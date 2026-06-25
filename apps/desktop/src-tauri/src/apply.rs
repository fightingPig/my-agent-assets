use crate::contracts::{
    ApplyMode, ApplyResult, ApplyStepResult, ApplyStepStatus, AssetType, BackupManifestSummary,
    ImportApplyInput, PlanStepKind, ScanScope,
};
use crate::path_utils::{display_path, expand_tilde, home_dir, modified_time_iso};
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[tauri::command]
pub fn import_apply_command(input: ImportApplyInput) -> ApplyResult {
    match home_dir() {
        Some(home) => import_apply_for_home(&home, input),
        None => ApplyResult {
            mode: input.mode,
            ok: false,
            preview_id: input.preview_id,
            backup: None,
            steps: vec![],
            warnings: vec![],
            errors: vec!["Could not resolve HOME for import apply.".into()],
        },
    }
}

pub fn import_apply_for_home(home: &Path, input: ImportApplyInput) -> ApplyResult {
    let mut result = ImportApplyRunner::new(home, input);
    result.run()
}

struct ImportApplyRunner<'a> {
    home: &'a Path,
    input: ImportApplyInput,
    steps: Vec<ApplyStepResult>,
    warnings: Vec<String>,
    errors: Vec<String>,
    backup: Option<BackupBuilder>,
}

impl<'a> ImportApplyRunner<'a> {
    fn new(home: &'a Path, input: ImportApplyInput) -> Self {
        Self {
            home,
            input,
            steps: vec![],
            warnings: vec![],
            errors: vec![],
            backup: None,
        }
    }

    fn run(&mut self) -> ApplyResult {
        if self.input.asset_ids.is_empty() {
            self.warnings
                .push("No asset IDs were selected for import apply.".into());
        }

        for asset_id in self.input.asset_ids.clone() {
            let intent = match ImportIntent::from_asset_id(self.home, &self.input.scope, &asset_id)
            {
                Ok(intent) => intent,
                Err(error) => {
                    self.push_failed_step(&asset_id, "解析资产 ID", error, vec![]);
                    continue;
                }
            };

            if self.input.mode == ApplyMode::PlanOnly {
                self.steps.push(ApplyStepResult {
                    step_id: format!("plan-import-{}", sanitize_step_id(&asset_id)),
                    kind: PlanStepKind::Import,
                    label: format!("预览导入 {}", asset_id),
                    status: ApplyStepStatus::Skipped,
                    message: "Plan-only mode: no files were written.".into(),
                    affected_paths: vec![display_path(&intent.destination)],
                });
                continue;
            }

            if let Err(error) = self.apply_intent(&intent) {
                self.push_failed_step(
                    &asset_id,
                    "导入资产",
                    error.to_string(),
                    vec![display_path(&intent.destination)],
                );
                break;
            }
        }

        let backup = match self.input.mode {
            ApplyMode::PlanOnly => None,
            ApplyMode::Apply => self.backup.take().and_then(|backup| match backup.finish() {
                Ok(summary) => Some(summary),
                Err(error) => {
                    self.errors
                        .push(format!("Could not write backup manifest: {}", error));
                    None
                }
            }),
        };

        ApplyResult {
            mode: self.input.mode.clone(),
            ok: self.errors.is_empty(),
            preview_id: self.input.preview_id.clone(),
            backup,
            steps: self.steps.clone(),
            warnings: self.warnings.clone(),
            errors: self.errors.clone(),
        }
    }

    fn apply_intent(&mut self, intent: &ImportIntent) -> io::Result<()> {
        match intent.asset_type {
            AssetType::Skill | AssetType::Command => self.apply_filesystem_intent(intent),
            AssetType::Mcp => self.apply_mcp_intent(intent),
        }
    }

    fn apply_filesystem_intent(&mut self, intent: &ImportIntent) -> io::Result<()> {
        if !intent.source.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Source path does not exist: {}",
                    display_path(&intent.source)
                ),
            ));
        }

        self.backup_destination_if_needed(&intent.destination)?;
        if let Some(parent) = intent.destination.parent() {
            fs::create_dir_all(parent)?;
        }

        if intent.source.is_dir() {
            let temp = temp_path_for(&intent.destination);
            if temp.exists() {
                remove_path(&temp)?;
            }
            copy_dir_recursive(&intent.source, &temp)?;
            verify_dir_equal(&intent.source, &temp)?;
            replace_path(&temp, &intent.destination)?;
            verify_dir_equal(&intent.source, &intent.destination)?;
        } else {
            copy_file_verified(&intent.source, &intent.destination)?;
        }

        self.steps.push(ApplyStepResult {
            step_id: format!("import-{}", sanitize_step_id(&intent.asset_id)),
            kind: PlanStepKind::Import,
            label: format!("导入 {}", intent.asset_id),
            status: ApplyStepStatus::Success,
            message: "Imported asset into the asset center.".into(),
            affected_paths: vec![display_path(&intent.destination)],
        });
        Ok(())
    }

    fn apply_mcp_intent(&mut self, intent: &ImportIntent) -> io::Result<()> {
        let config_text = fs::read_to_string(&intent.source)?;
        let config: Value = serde_json::from_str(&config_text).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Could not parse MCP config JSON: {}", error),
            )
        })?;
        let server = config
            .get("mcpServers")
            .and_then(Value::as_object)
            .and_then(|servers| servers.get(&intent.name))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("mcpServers.{} was not found.", intent.name),
                )
            })?;

        self.backup_destination_if_needed(&intent.destination)?;
        if let Some(parent) = intent.destination.parent() {
            fs::create_dir_all(parent)?;
        }

        let pretty = serde_json::to_vec_pretty(server).map_err(io::Error::other)?;
        write_file_verified(&pretty, &intent.destination)?;
        self.steps.push(ApplyStepResult {
            step_id: format!("import-{}", sanitize_step_id(&intent.asset_id)),
            kind: PlanStepKind::Import,
            label: format!("导入 {}", intent.asset_id),
            status: ApplyStepStatus::Success,
            message: "Imported MCP server JSON into the asset center.".into(),
            affected_paths: vec![display_path(&intent.destination)],
        });
        Ok(())
    }

    fn backup_destination_if_needed(&mut self, destination: &Path) -> io::Result<()> {
        if !destination.exists() {
            return Ok(());
        }
        if !self.input.backup_before_apply {
            self.warnings.push(format!(
                "Replacing existing path without backup: {}",
                display_path(destination)
            ));
            return Ok(());
        }

        if self.backup.is_none() {
            self.backup = Some(BackupBuilder::create(self.home, &self.input.preview_id)?);
        }
        if let Some(backup) = &mut self.backup {
            backup.add_path(destination)?;
            self.steps.push(ApplyStepResult {
                step_id: format!("backup-{}", sanitize_step_id(&display_path(destination))),
                kind: PlanStepKind::Backup,
                label: "备份已有资产".into(),
                status: ApplyStepStatus::Success,
                message: "Backed up existing destination before replacement.".into(),
                affected_paths: vec![display_path(destination)],
            });
        }
        Ok(())
    }

    fn push_failed_step(
        &mut self,
        asset_id: &str,
        label: &str,
        message: impl Into<String>,
        affected_paths: Vec<String>,
    ) {
        let message = message.into();
        self.errors.push(message.clone());
        self.steps.push(ApplyStepResult {
            step_id: format!("failed-{}", sanitize_step_id(asset_id)),
            kind: PlanStepKind::Import,
            label: label.into(),
            status: ApplyStepStatus::Failed,
            message,
            affected_paths,
        });
    }
}

struct ImportIntent {
    asset_id: String,
    asset_type: AssetType,
    name: String,
    source: PathBuf,
    destination: PathBuf,
}

impl ImportIntent {
    fn from_asset_id(home: &Path, scope: &ScanScope, asset_id: &str) -> Result<Self, String> {
        let (asset_type, name) = parse_asset_id(asset_id)?;
        let runtime_root = runtime_root(home, scope);
        let asset_center = home.join(".my-agent-assets").join("assets");
        let (source, destination) = match asset_type {
            AssetType::Skill => {
                let skill_dir = runtime_root.join(".claude").join("skills").join(&name);
                let skill_file = runtime_root
                    .join(".claude")
                    .join("skills")
                    .join(format!("{}.md", name));
                if skill_dir.exists() {
                    (skill_dir, asset_center.join("skills").join(&name))
                } else {
                    (
                        skill_file,
                        asset_center.join("skills").join(format!("{}.md", name)),
                    )
                }
            }
            AssetType::Command => (
                runtime_root
                    .join(".claude")
                    .join("commands")
                    .join(format!("{}.md", name)),
                asset_center.join("commands").join(format!("{}.md", name)),
            ),
            AssetType::Mcp => {
                let config = match scope {
                    ScanScope::User => runtime_root.join(".claude.json"),
                    ScanScope::Project { .. } | ScanScope::Custom { .. } => {
                        runtime_root.join(".mcp.json")
                    }
                };
                (
                    config,
                    asset_center.join("mcps").join(format!("{}.json", name)),
                )
            }
        };

        Ok(Self {
            asset_id: asset_id.into(),
            asset_type,
            name,
            source,
            destination,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupManifest {
    id: String,
    label: String,
    created_at: String,
    runtime_root: String,
    entries: Vec<BackupEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupEntry {
    original_path: String,
    backup_path: String,
    kind: String,
    size_bytes: u64,
}

struct BackupBuilder {
    id: String,
    label: String,
    created_at: String,
    root: PathBuf,
    manifest_path: PathBuf,
    runtime_root: PathBuf,
    entries: Vec<BackupEntry>,
}

impl BackupBuilder {
    fn create(home: &Path, preview_id: &str) -> io::Result<Self> {
        let created_at = modified_time_iso(SystemTime::now());
        let id = format!(
            "import-{}-{}",
            created_at
                .replace([':', '-'], "")
                .replace('T', "-")
                .trim_end_matches('Z'),
            std::process::id()
        );
        let root = home.join(".my-agent-assets").join("backups").join(&id);
        fs::create_dir_all(&root)?;
        let manifest_path = root.join("manifest.json");
        Ok(Self {
            id,
            label: format!("Import apply backup for {}", preview_id),
            created_at,
            root,
            manifest_path,
            runtime_root: home.to_path_buf(),
            entries: vec![],
        })
    }

    fn add_path(&mut self, original: &Path) -> io::Result<()> {
        let relative = original
            .strip_prefix(&self.runtime_root)
            .unwrap_or(original)
            .to_path_buf();
        let backup_path = self.root.join("files").join(&relative);
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let (kind, size_bytes) = if original.is_dir() {
            copy_dir_recursive(original, &backup_path)?;
            ("directory".into(), dir_size(original)?)
        } else {
            fs::copy(original, &backup_path)?;
            ("file".into(), fs::metadata(original)?.len())
        };

        self.entries.push(BackupEntry {
            original_path: display_path(original),
            backup_path: display_path(&backup_path),
            kind,
            size_bytes,
        });
        Ok(())
    }

    fn finish(self) -> io::Result<BackupManifestSummary> {
        let size_bytes = self
            .entries
            .iter()
            .map(|entry| entry.size_bytes)
            .sum::<u64>();
        let affected_paths = self
            .entries
            .iter()
            .map(|entry| entry.original_path.clone())
            .collect::<Vec<_>>();
        let manifest = BackupManifest {
            id: self.id.clone(),
            label: self.label.clone(),
            created_at: self.created_at.clone(),
            runtime_root: display_path(&self.runtime_root),
            entries: self.entries,
        };
        let bytes = serde_json::to_vec_pretty(&manifest).map_err(io::Error::other)?;
        fs::write(&self.manifest_path, bytes)?;

        Ok(BackupManifestSummary {
            id: self.id,
            label: self.label,
            created_at: self.created_at,
            size_bytes,
            entry_count: affected_paths.len() as u32,
            manifest_path: display_path(&self.manifest_path),
            runtime_root: display_path(&self.runtime_root),
            affected_paths,
        })
    }
}

fn parse_asset_id(asset_id: &str) -> Result<(AssetType, String), String> {
    let mut parts = asset_id.splitn(2, ':');
    let prefix = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default().trim();
    if name.is_empty() {
        return Err(format!("Invalid asset ID '{}': missing name.", asset_id));
    }
    let asset_type = match prefix {
        "skill" => AssetType::Skill,
        "command" => AssetType::Command,
        "mcp" => AssetType::Mcp,
        _ => return Err(format!("Invalid asset ID '{}': unknown type.", asset_id)),
    };
    Ok((asset_type, name.into()))
}

fn runtime_root(home: &Path, scope: &ScanScope) -> PathBuf {
    match scope {
        ScanScope::User => home.to_path_buf(),
        ScanScope::Project { project_path } => expand_tilde(project_path, home),
        ScanScope::Custom { path } => expand_tilde(path, home),
    }
}

fn copy_file_verified(source: &Path, destination: &Path) -> io::Result<()> {
    let bytes = fs::read(source)?;
    write_file_verified(&bytes, destination)
}

fn write_file_verified(bytes: &[u8], destination: &Path) -> io::Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = temp_path_for(destination);
    fs::write(&temp, bytes)?;
    let temp_bytes = fs::read(&temp)?;
    if temp_bytes != bytes {
        let _ = fs::remove_file(&temp);
        return Err(io::Error::other("Temporary file verification failed."));
    }
    replace_path(&temp, destination)?;
    let destination_bytes = fs::read(destination)?;
    if destination_bytes != bytes {
        return Err(io::Error::other("Destination file verification failed."));
    }
    Ok(())
}

fn replace_path(temp: &Path, destination: &Path) -> io::Result<()> {
    if destination.exists() {
        remove_path(destination)?;
    }
    fs::rename(temp, destination)
}

fn remove_path(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn temp_path_for(destination: &Path) -> PathBuf {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("asset");
    destination.with_file_name(format!(
        ".tmp-my-agent-assets-{}-{}",
        std::process::id(),
        file_name
    ))
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn verify_dir_equal(source: &Path, destination: &Path) -> io::Result<()> {
    let source_files = collect_files(source, source)?;
    let destination_files = collect_files(destination, destination)?;
    if source_files != destination_files {
        return Err(io::Error::other("Directory file list verification failed."));
    }
    for relative in source_files {
        let source_bytes = fs::read(source.join(&relative))?;
        let destination_bytes = fs::read(destination.join(&relative))?;
        if source_bytes != destination_bytes {
            return Err(io::Error::other(format!(
                "Directory file verification failed for {}.",
                display_path(&relative)
            )));
        }
    }
    Ok(())
}

fn collect_files(root: &Path, current: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = vec![];
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_files(root, &path)?);
        } else {
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn dir_size(path: &Path) -> io::Result<u64> {
    let mut size = 0;
    for relative in collect_files(path, path)? {
        size += fs::metadata(path.join(relative))?.len();
    }
    Ok(size)
}

fn sanitize_step_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect()
}
