use crate::contracts::{
    ApplyMode, ApplyResult, ApplyStepResult, ApplyStepStatus, AssetType, BackupManifestSummary,
    ConflictApplyInput, ConflictResolution, ConflictResolutionChoice, ImportApplyInput,
    MountApplyInput, PlanStepKind, RestoreApplyInput, ScanScope,
};
use crate::path_utils::{
    display_path, expand_tilde, guard_existing_path, guard_write_path, home_dir, modified_time_iso,
    path_is_within, validate_single_path_component,
};
use crate::preview;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
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

#[tauri::command]
pub fn conflict_apply_command(input: ConflictApplyInput) -> ApplyResult {
    match home_dir() {
        Some(home) => conflict_apply_for_home(&home, input),
        None => ApplyResult {
            mode: input.mode,
            ok: false,
            preview_id: input.preview_id,
            backup: None,
            steps: vec![],
            warnings: vec![],
            errors: vec!["Could not resolve HOME for conflict apply.".into()],
        },
    }
}

pub fn conflict_apply_for_home(home: &Path, input: ConflictApplyInput) -> ApplyResult {
    if let Err(error) = validate_conflict_apply_input(&input) {
        return ApplyResult {
            mode: input.mode,
            ok: false,
            preview_id: input.preview_id,
            backup: None,
            steps: vec![],
            warnings: vec![],
            errors: vec![error],
        };
    }

    import_apply_for_home(
        home,
        ImportApplyInput {
            preview_id: input.preview_id,
            mode: input.mode,
            scope: input.scope,
            asset_ids: input.asset_ids,
            conflict_resolutions: input.conflict_resolutions,
            backup_before_apply: input.backup_before_apply,
        },
    )
}

#[tauri::command]
pub fn mount_apply_command(input: MountApplyInput) -> ApplyResult {
    match home_dir() {
        Some(home) => mount_apply_for_home(&home, input),
        None => ApplyResult {
            mode: input.mode,
            ok: false,
            preview_id: input.preview_id,
            backup: None,
            steps: vec![],
            warnings: vec![],
            errors: vec!["Could not resolve HOME for mount apply.".into()],
        },
    }
}

pub fn mount_apply_for_home(home: &Path, input: MountApplyInput) -> ApplyResult {
    let mut runner = MountApplyRunner::new(home, input);
    runner.run()
}

#[tauri::command]
pub fn restore_apply_command(input: RestoreApplyInput) -> ApplyResult {
    match home_dir() {
        Some(home) => restore_apply_for_home(&home, input),
        None => ApplyResult {
            mode: input.mode,
            ok: false,
            preview_id: input.preview_id,
            backup: None,
            steps: vec![],
            warnings: vec![],
            errors: vec!["Could not resolve HOME for restore apply.".into()],
        },
    }
}

pub fn restore_apply_for_home(home: &Path, input: RestoreApplyInput) -> ApplyResult {
    let mut runner = RestoreApplyRunner::new(home, input);
    runner.run()
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
        if !self.validate_preview_id() {
            return ApplyResult {
                mode: self.input.mode.clone(),
                ok: false,
                preview_id: self.input.preview_id.clone(),
                backup: None,
                steps: self.steps.clone(),
                warnings: self.warnings.clone(),
                errors: self.errors.clone(),
            };
        }

        if self.input.asset_ids.is_empty() {
            self.warnings
                .push("No asset IDs were selected for import apply.".into());
        }

        for asset_id in self.input.asset_ids.clone() {
            let resolution = self.resolution_for(&asset_id).cloned();
            if matches!(
                resolution.as_ref().map(|choice| &choice.resolution),
                Some(ConflictResolution::Skip)
            ) {
                self.steps.push(ApplyStepResult {
                    step_id: format!("skip-conflict-{}", sanitize_step_id(&asset_id)),
                    kind: PlanStepKind::Import,
                    label: format!("跳过冲突 {}", asset_id),
                    status: ApplyStepStatus::Skipped,
                    message: "Conflict resolution is skip; no files were written.".into(),
                    affected_paths: vec![],
                });
                continue;
            }

            let mut intent =
                match ImportIntent::from_asset_id(self.home, &self.input.scope, &asset_id) {
                    Ok(intent) => intent,
                    Err(error) => {
                        self.push_failed_step(&asset_id, "解析资产 ID", error, vec![]);
                        continue;
                    }
                };
            if let Some(choice) = resolution.as_ref() {
                if choice.resolution == ConflictResolution::Rename {
                    let rename_to = choice
                        .rename_to
                        .as_deref()
                        .expect("rename input was validated before runner creation");
                    if let Err(error) = intent.rename_destination(rename_to) {
                        self.push_failed_step(
                            &asset_id,
                            "校验重命名",
                            error,
                            vec![display_path(&intent.destination)],
                        );
                        continue;
                    }
                    if intent.destination.exists() {
                        self.push_failed_step(
                            &asset_id,
                            "校验重命名目标",
                            format!(
                                "Rename target already exists: {}",
                                display_path(&intent.destination)
                            ),
                            vec![display_path(&intent.destination)],
                        );
                        continue;
                    }
                }
            }
            if let Err(error) = intent.validate(self.home) {
                self.push_failed_step(
                    &asset_id,
                    "校验导入路径",
                    error.to_string(),
                    vec![display_path(&intent.destination)],
                );
                continue;
            }

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

    fn validate_preview_id(&mut self) -> bool {
        let expected = preview::import_preview_id(
            &self.input.scope,
            &self.input.asset_ids,
            &self.input.conflict_resolutions,
        );
        if self.input.preview_id == expected {
            return true;
        }

        let preview_id = self.input.preview_id.clone();
        self.push_failed_step(
            &preview_id,
            "校验预览 ID",
            format!(
                "Preview ID does not match import input. Expected {}, got {}.",
                expected, self.input.preview_id
            ),
            vec![],
        );
        false
    }

    fn resolution_for(&self, asset_id: &str) -> Option<&ConflictResolutionChoice> {
        self.input.conflict_resolutions.iter().find(|choice| {
            choice.conflict_id == asset_id || choice.conflict_id == format!("conflict:{}", asset_id)
        })
    }

    fn apply_intent(&mut self, intent: &ImportIntent) -> io::Result<()> {
        match intent.asset_type {
            AssetType::Skill | AssetType::Command => self.apply_filesystem_intent(intent),
            AssetType::Mcp => self.apply_mcp_intent(intent),
        }
    }

    fn apply_filesystem_intent(&mut self, intent: &ImportIntent) -> io::Result<()> {
        let source = guard_existing_path(self.home, &intent.source)?;
        let destination = guard_write_path(self.home, &intent.destination)?;

        self.backup_destination_if_needed(&destination)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        if source.is_dir() {
            let temp = temp_path_for(&destination);
            if temp.exists() {
                remove_path(&temp)?;
            }
            copy_dir_recursive(&source, &temp)?;
            verify_dir_equal(&source, &temp)?;
            replace_path(&temp, &destination)?;
            verify_dir_equal(&source, &destination)?;
        } else {
            copy_file_verified(&source, &destination)?;
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
        let source = guard_existing_path(self.home, &intent.source)?;
        let destination = guard_write_path(self.home, &intent.destination)?;
        let config_text = fs::read_to_string(source)?;
        let config: Value = serde_json::from_str(&config_text).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Could not parse MCP config JSON: {}", error),
            )
        })?;
        let server = config
            .get("mcpServers")
            .and_then(Value::as_object)
            .and_then(|servers| servers.get(&intent.source_name))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("mcpServers.{} was not found.", intent.source_name),
                )
            })?;

        self.backup_destination_if_needed(&destination)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        let pretty = serde_json::to_vec_pretty(server).map_err(io::Error::other)?;
        write_file_verified(&pretty, &destination)?;
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
            self.backup = Some(BackupBuilder::create(
                self.home,
                &self.input.preview_id,
                "import",
                "Import apply backup",
            )?);
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

struct MountApplyRunner<'a> {
    home: &'a Path,
    input: MountApplyInput,
    steps: Vec<ApplyStepResult>,
    warnings: Vec<String>,
    errors: Vec<String>,
    backup: Option<BackupBuilder>,
}

struct RestoreApplyRunner<'a> {
    home: &'a Path,
    input: RestoreApplyInput,
    steps: Vec<ApplyStepResult>,
    warnings: Vec<String>,
    errors: Vec<String>,
    backup: Option<BackupBuilder>,
}

impl<'a> RestoreApplyRunner<'a> {
    fn new(home: &'a Path, input: RestoreApplyInput) -> Self {
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
        if !self.validate_preview_id() {
            return self.result();
        }

        let manifest_path = self
            .home
            .join(".my-agent-assets")
            .join("backups")
            .join(&self.input.backup_id)
            .join("manifest.json");
        if let Err(error) = validate_single_path_component(&self.input.backup_id, "backup ID") {
            self.push_failed_step("校验备份 ID", error, vec![]);
            return self.result();
        }
        let manifest_path = match guard_existing_path(self.home, &manifest_path) {
            Ok(path) => path,
            Err(error) => {
                self.push_failed_step(
                    "校验备份清单路径",
                    error.to_string(),
                    vec![display_path(&manifest_path)],
                );
                return self.result();
            }
        };

        let manifest = match read_backup_manifest(&manifest_path) {
            Ok(manifest) => manifest,
            Err(error) => {
                self.push_failed_step(
                    "读取备份清单",
                    error.to_string(),
                    vec![display_path(&manifest_path)],
                );
                return self.result();
            }
        };
        if manifest.id != self.input.backup_id {
            self.push_failed_step(
                "校验备份清单",
                "Backup manifest ID does not match the requested backup ID.",
                vec![display_path(&manifest_path)],
            );
            return self.result();
        }
        if PathBuf::from(&manifest.runtime_root) != self.home {
            self.push_failed_step(
                "校验备份清单",
                "Backup manifest runtimeRoot does not match resolved HOME.",
                vec![display_path(&manifest_path)],
            );
            return self.result();
        }
        if let Err(error) = manifest.entries.iter().try_for_each(|entry| {
            self.validate_restore_entry(entry, &manifest_path)
                .map(|_| ())
        }) {
            self.push_failed_step(
                "校验备份清单",
                error.to_string(),
                vec![display_path(&manifest_path)],
            );
            return self.result();
        }

        if self.input.mode == ApplyMode::PlanOnly {
            self.steps.push(ApplyStepResult {
                step_id: format!("plan-restore-{}", sanitize_step_id(&self.input.backup_id)),
                kind: PlanStepKind::Restore,
                label: format!("预览恢复 {}", self.input.backup_id),
                status: ApplyStepStatus::Skipped,
                message: format!(
                    "Plan-only mode: {} backup entries would be restored.",
                    manifest.entries.len()
                ),
                affected_paths: manifest
                    .entries
                    .iter()
                    .map(|entry| entry.original_path.clone())
                    .collect(),
            });
            return self.result();
        }

        for entry in &manifest.entries {
            if let Err(error) = self.restore_entry(entry, &manifest_path) {
                self.push_failed_step(
                    "恢复备份条目",
                    error.to_string(),
                    vec![entry.original_path.clone()],
                );
                break;
            }
        }

        self.result()
    }

    fn validate_preview_id(&mut self) -> bool {
        let expected = preview::restore_preview_id(&self.input.backup_id);
        if self.input.preview_id == expected {
            return true;
        }

        self.push_failed_step(
            "校验预览 ID",
            format!(
                "Preview ID does not match restore input. Expected {}, got {}.",
                expected, self.input.preview_id
            ),
            vec![],
        );
        false
    }

    fn restore_entry(&mut self, entry: &BackupEntry, manifest_path: &Path) -> io::Result<()> {
        let (original, backup) = self.validate_restore_entry(entry, manifest_path)?;

        self.backup_current_if_needed(&original)?;
        if let Some(parent) = original.parent() {
            fs::create_dir_all(parent)?;
        }

        match entry.kind.as_str() {
            "directory" => {
                let temp = temp_path_for(&original);
                if path_exists_no_follow(&temp) {
                    remove_path(&temp)?;
                }
                copy_dir_recursive(&backup, &temp)?;
                verify_dir_equal(&backup, &temp)?;
                replace_path(&temp, &original)?;
                verify_dir_equal(&backup, &original)?;
            }
            "file" => copy_file_verified(&backup, &original)?,
            "symlink" => {
                let target = PathBuf::from(fs::read_to_string(&backup)?);
                let temp = temp_path_for(&original);
                if path_exists_no_follow(&temp) {
                    remove_path(&temp)?;
                }
                create_symlink(&target, &temp)?;
                verify_symlink_target(&temp, &target)?;
                replace_path(&temp, &original)?;
                verify_symlink_target(&original, &target)?;
            }
            _ => unreachable!("entry kind was validated before restore"),
        }

        self.steps.push(ApplyStepResult {
            step_id: format!("restore-{}", sanitize_step_id(&entry.original_path)),
            kind: PlanStepKind::Restore,
            label: "恢复备份条目".into(),
            status: ApplyStepStatus::Success,
            message: "Restored path from backup manifest.".into(),
            affected_paths: vec![entry.original_path.clone()],
        });
        Ok(())
    }

    fn validate_restore_entry(
        &self,
        entry: &BackupEntry,
        manifest_path: &Path,
    ) -> io::Result<(PathBuf, PathBuf)> {
        let original = guard_write_path(self.home, Path::new(&entry.original_path))?;
        let backup_root = manifest_path
            .parent()
            .ok_or_else(|| io::Error::other("Backup manifest has no parent directory."))?;
        let backup_files_root = backup_root.join("files");
        let backup = guard_existing_path(&backup_files_root, Path::new(&entry.backup_path))?;
        if path_is_within(backup_root, &original)? {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Restore target must not overwrite the selected backup directory.",
            ));
        }
        if !matches!(entry.kind.as_str(), "directory" | "file" | "symlink") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported backup entry kind: {}", entry.kind),
            ));
        }
        if entry.kind == "symlink" {
            self.validate_restored_symlink(&original, &backup)?;
        }
        Ok((original, backup))
    }

    fn validate_restored_symlink(&self, original: &Path, backup: &Path) -> io::Result<()> {
        let target = PathBuf::from(fs::read_to_string(backup)?);
        let resolved = if target.is_absolute() {
            target
        } else {
            original
                .parent()
                .ok_or_else(|| io::Error::other("Restore symlink has no parent."))?
                .join(target)
        };
        guard_write_path(self.home, &resolved)?;
        Ok(())
    }

    fn backup_current_if_needed(&mut self, original: &Path) -> io::Result<()> {
        if !path_exists_no_follow(original) {
            return Ok(());
        }
        if !self.input.backup_before_restore {
            self.warnings.push(format!(
                "Replacing current path without backup: {}",
                display_path(original)
            ));
            return Ok(());
        }

        if self.backup.is_none() {
            self.backup = Some(BackupBuilder::create(
                self.home,
                &self.input.preview_id,
                "restore",
                "Restore apply backup",
            )?);
        }
        if let Some(backup) = &mut self.backup {
            backup.add_path(original)?;
            self.steps.push(ApplyStepResult {
                step_id: format!(
                    "backup-current-{}",
                    sanitize_step_id(&display_path(original))
                ),
                kind: PlanStepKind::Backup,
                label: "备份当前状态".into(),
                status: ApplyStepStatus::Success,
                message: "Backed up current path before restore.".into(),
                affected_paths: vec![display_path(original)],
            });
        }
        Ok(())
    }

    fn result(&mut self) -> ApplyResult {
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

    fn push_failed_step(
        &mut self,
        label: &str,
        message: impl Into<String>,
        affected_paths: Vec<String>,
    ) {
        let message = message.into();
        self.errors.push(message.clone());
        self.steps.push(ApplyStepResult {
            step_id: format!("failed-restore-{}", sanitize_step_id(&self.input.backup_id)),
            kind: PlanStepKind::Restore,
            label: label.into(),
            status: ApplyStepStatus::Failed,
            message,
            affected_paths,
        });
    }
}

impl<'a> MountApplyRunner<'a> {
    fn new(home: &'a Path, input: MountApplyInput) -> Self {
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
        if !self.validate_preview_id() {
            return self.result();
        }

        let intent = match MountIntent::from_input(self.home, &self.input) {
            Ok(intent) => intent,
            Err(error) => {
                self.push_failed_step("解析挂载请求", error, vec![]);
                return self.result();
            }
        };
        if let Err(error) = intent.validate(self.home) {
            self.push_failed_step(
                "校验挂载路径",
                error.to_string(),
                vec![display_path(&intent.destination)],
            );
            return self.result();
        }

        if self.input.mode == ApplyMode::PlanOnly {
            self.steps.push(ApplyStepResult {
                step_id: format!("plan-mount-{}", sanitize_step_id(&self.input.asset_id)),
                kind: PlanStepKind::Mount,
                label: format!("预览挂载 {}", self.input.asset_id),
                status: ApplyStepStatus::Skipped,
                message: "Plan-only mode: no symlink was created.".into(),
                affected_paths: vec![display_path(&intent.destination)],
            });
            return self.result();
        }

        if let Err(error) = self.apply_intent(&intent) {
            self.push_failed_step(
                "挂载资产",
                error.to_string(),
                vec![display_path(&intent.destination)],
            );
        }

        self.result()
    }

    fn validate_preview_id(&mut self) -> bool {
        let expected = preview::mount_preview_id(&self.input.asset_id, &self.input.target);
        if self.input.preview_id == expected {
            return true;
        }

        self.push_failed_step(
            "校验预览 ID",
            format!(
                "Preview ID does not match mount input. Expected {}, got {}.",
                expected, self.input.preview_id
            ),
            vec![],
        );
        false
    }

    fn apply_intent(&mut self, intent: &MountIntent) -> io::Result<()> {
        if intent.asset_type == AssetType::Mcp {
            return self.apply_mcp_compile_intent(intent);
        }
        let asset_root = self.home.join(".my-agent-assets").join("assets");
        let source = guard_existing_path(&asset_root, &intent.source)?;
        let destination = guard_write_path(self.home, &intent.destination)?;

        self.backup_destination_if_needed(&destination)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp = temp_path_for(&destination);
        if temp.exists() {
            remove_path(&temp)?;
        }
        create_symlink(&source, &temp)?;
        verify_symlink_target(&temp, &source)?;
        replace_path(&temp, &destination)?;
        verify_symlink_target(&destination, &source)?;

        self.steps.push(ApplyStepResult {
            step_id: format!("mount-{}", sanitize_step_id(&self.input.asset_id)),
            kind: PlanStepKind::Mount,
            label: format!("挂载 {}", self.input.asset_id),
            status: ApplyStepStatus::Success,
            message: "Created runtime symlink to the asset center.".into(),
            affected_paths: vec![display_path(&intent.destination)],
        });
        Ok(())
    }

    fn apply_mcp_compile_intent(&mut self, intent: &MountIntent) -> io::Result<()> {
        let asset_root = self.home.join(".my-agent-assets").join("assets");
        let source = guard_existing_path(&asset_root, &intent.source)?;
        let destination = guard_write_path(self.home, &intent.destination)?;

        let server = read_json_file(&source)?;
        let mut runtime_config = if destination.exists() {
            let value = read_json_file(&destination)?;
            match value {
                Value::Object(object) => object,
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Existing MCP runtime config must be a JSON object.",
                    ));
                }
            }
        } else {
            Map::new()
        };

        match runtime_config.get_mut("mcpServers") {
            Some(Value::Object(servers)) => {
                servers.insert(intent.name.clone(), server);
            }
            Some(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Existing mcpServers field must be a JSON object.",
                ));
            }
            None => {
                let mut servers = Map::new();
                servers.insert(intent.name.clone(), server);
                runtime_config.insert("mcpServers".into(), Value::Object(servers));
            }
        }

        self.backup_destination_if_needed(&destination)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes =
            serde_json::to_vec_pretty(&Value::Object(runtime_config)).map_err(io::Error::other)?;
        write_file_verified(&bytes, &destination)?;

        self.steps.push(ApplyStepResult {
            step_id: format!("compile-mcp-{}", sanitize_step_id(&self.input.asset_id)),
            kind: PlanStepKind::CompileMcp,
            label: format!("编译 {}", self.input.asset_id),
            status: ApplyStepStatus::Success,
            message: "Compiled MCP server JSON into the runtime config.".into(),
            affected_paths: vec![display_path(&intent.destination)],
        });
        Ok(())
    }

    fn backup_destination_if_needed(&mut self, destination: &Path) -> io::Result<()> {
        if !path_exists_no_follow(destination) {
            return Ok(());
        }
        if !self.input.backup_before_apply {
            self.warnings.push(format!(
                "Replacing existing mount target without backup: {}",
                display_path(destination)
            ));
            return Ok(());
        }

        if self.backup.is_none() {
            self.backup = Some(BackupBuilder::create(
                self.home,
                &self.input.preview_id,
                "mount",
                "Mount apply backup",
            )?);
        }
        if let Some(backup) = &mut self.backup {
            backup.add_path(destination)?;
            self.steps.push(ApplyStepResult {
                step_id: format!("backup-{}", sanitize_step_id(&display_path(destination))),
                kind: PlanStepKind::Backup,
                label: "备份已有挂载目标".into(),
                status: ApplyStepStatus::Success,
                message: "Backed up existing mount target before replacement.".into(),
                affected_paths: vec![display_path(destination)],
            });
        }
        Ok(())
    }

    fn result(&mut self) -> ApplyResult {
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

    fn push_failed_step(
        &mut self,
        label: &str,
        message: impl Into<String>,
        affected_paths: Vec<String>,
    ) {
        let message = message.into();
        self.errors.push(message.clone());
        self.steps.push(ApplyStepResult {
            step_id: format!("failed-{}", sanitize_step_id(&self.input.asset_id)),
            kind: PlanStepKind::Mount,
            label: label.into(),
            status: ApplyStepStatus::Failed,
            message,
            affected_paths,
        });
    }
}

struct MountIntent {
    asset_type: AssetType,
    name: String,
    source: PathBuf,
    destination: PathBuf,
}

impl MountIntent {
    fn from_input(home: &Path, input: &MountApplyInput) -> Result<Self, String> {
        let (asset_type, name) = parse_asset_id(&input.asset_id)?;
        let asset_center = home.join(".my-agent-assets").join("assets");
        let source = match asset_type {
            AssetType::Skill => {
                let skill_dir = asset_center.join("skills").join(&name);
                let skill_file = asset_center.join("skills").join(format!("{}.md", name));
                if skill_dir.exists() {
                    skill_dir
                } else {
                    skill_file
                }
            }
            AssetType::Command => asset_center.join("commands").join(format!("{}.md", name)),
            AssetType::Mcp => asset_center.join("mcps").join(format!("{}.json", name)),
        };

        Ok(Self {
            asset_type,
            name,
            source,
            destination: expand_tilde(&input.target.runtime_path, home),
        })
    }

    fn validate(&self, home: &Path) -> io::Result<()> {
        let asset_root = home.join(".my-agent-assets").join("assets");
        guard_existing_path(&asset_root, &self.source)?;
        guard_write_path(home, &self.destination)?;
        Ok(())
    }
}

struct ImportIntent {
    asset_id: String,
    asset_type: AssetType,
    name: String,
    source_name: String,
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
            source_name: name.clone(),
            name,
            source,
            destination,
        })
    }

    fn validate(&self, home: &Path) -> io::Result<()> {
        let source = guard_existing_path(home, &self.source)?;
        if source.is_dir() {
            ensure_tree_has_no_symlinks(&source)?;
        }
        guard_write_path(home, &self.destination)?;
        Ok(())
    }

    fn rename_destination(&mut self, rename_to: &str) -> Result<(), String> {
        validate_single_path_component(rename_to, "Conflict rename target")
            .map_err(|error| error.to_string())?;
        self.name = rename_to.into();
        self.destination = match self.asset_type {
            AssetType::Skill if self.source.is_dir() => self
                .destination
                .parent()
                .expect("skill destination should have a parent")
                .join(rename_to),
            AssetType::Skill | AssetType::Command => self
                .destination
                .parent()
                .expect("asset destination should have a parent")
                .join(format!("{}.md", rename_to)),
            AssetType::Mcp => self
                .destination
                .parent()
                .expect("MCP destination should have a parent")
                .join(format!("{}.json", rename_to)),
        };
        Ok(())
    }
}

fn validate_conflict_apply_input(input: &ConflictApplyInput) -> Result<(), String> {
    if input.asset_ids.is_empty() {
        return Err("Conflict apply requires at least one asset ID.".into());
    }
    if input.conflict_resolutions.len() != input.asset_ids.len() {
        return Err("Conflict apply requires exactly one resolution for every asset.".into());
    }

    for asset_id in &input.asset_ids {
        parse_asset_id(asset_id)?;
        let matches = input
            .conflict_resolutions
            .iter()
            .filter(|choice| {
                choice.conflict_id == *asset_id
                    || choice.conflict_id == format!("conflict:{}", asset_id)
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(format!(
                "Conflict apply requires one unambiguous resolution for {}.",
                asset_id
            ));
        }
        let choice = matches[0];
        match choice.resolution {
            ConflictResolution::Rename => {
                let rename_to = choice.rename_to.as_deref().ok_or_else(|| {
                    format!("Rename resolution for {} requires renameTo.", asset_id)
                })?;
                validate_single_path_component(rename_to, "Conflict rename target")
                    .map_err(|error| error.to_string())?;
            }
            ConflictResolution::Skip | ConflictResolution::Overwrite => {
                if choice.rename_to.is_some() {
                    return Err(format!(
                        "renameTo is only allowed for rename resolution: {}.",
                        asset_id
                    ));
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupManifest {
    id: String,
    label: String,
    created_at: String,
    runtime_root: String,
    entries: Vec<BackupEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
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
    fn create(
        home: &Path,
        preview_id: &str,
        id_prefix: &str,
        label_prefix: &str,
    ) -> io::Result<Self> {
        let created_at = modified_time_iso(SystemTime::now());
        let id = format!(
            "{}-{}-{}",
            id_prefix,
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
            label: format!("{} for {}", label_prefix, preview_id),
            created_at,
            root,
            manifest_path,
            runtime_root: home.to_path_buf(),
            entries: vec![],
        })
    }

    fn add_path(&mut self, original: &Path) -> io::Result<()> {
        let original = guard_write_path(&self.runtime_root, original)?;
        let relative = original
            .strip_prefix(&self.runtime_root)
            .map_err(|_| io::Error::other("Backup source escaped runtime root."))?
            .to_path_buf();
        let backup_path = self.root.join("files").join(&relative);
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let metadata = fs::symlink_metadata(&original)?;
        let (kind, size_bytes) = if metadata.file_type().is_symlink() {
            let target = fs::read_link(&original)?;
            fs::write(&backup_path, target.to_string_lossy().as_bytes())?;
            ("symlink".into(), fs::metadata(&backup_path)?.len())
        } else if metadata.is_dir() {
            copy_dir_recursive(&original, &backup_path)?;
            ("directory".into(), dir_size(&original)?)
        } else {
            fs::copy(&original, &backup_path)?;
            ("file".into(), fs::metadata(&original)?.len())
        };

        self.entries.push(BackupEntry {
            original_path: display_path(&original),
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
    let name = parts.next().unwrap_or_default();
    if name.is_empty() {
        return Err(format!("Invalid asset ID '{}': missing name.", asset_id));
    }
    validate_single_path_component(name, "asset name")?;
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

fn read_json_file(path: &Path) -> io::Result<Value> {
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Could not parse JSON file {}: {}",
                display_path(path),
                error
            ),
        )
    })
}

fn read_backup_manifest(path: &Path) -> io::Result<BackupManifest> {
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Could not parse backup manifest {}: {}",
                display_path(path),
                error
            ),
        )
    })
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
    if path_exists_no_follow(destination) {
        remove_path(destination)?;
    }
    fs::rename(temp, destination)
}

fn remove_path(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn path_exists_no_follow(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

#[cfg(unix)]
fn create_symlink(source: &Path, destination: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, destination)
}

#[cfg(windows)]
fn create_symlink(source: &Path, destination: &Path) -> io::Result<()> {
    if source.is_dir() {
        std::os::windows::fs::symlink_dir(source, destination)
    } else {
        std::os::windows::fs::symlink_file(source, destination)
    }
}

fn verify_symlink_target(link: &Path, expected: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(link)?;
    if !metadata.file_type().is_symlink() {
        return Err(io::Error::other(format!(
            "Mount target is not a symlink: {}",
            display_path(link)
        )));
    }
    let actual = fs::read_link(link)?;
    if actual != expected {
        return Err(io::Error::other(format!(
            "Symlink verification failed. Expected {}, got {}.",
            display_path(expected),
            display_path(&actual)
        )));
    }
    Ok(())
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
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "Symlinks are forbidden inside copied directories: {}",
                    display_path(&entry.path())
                ),
            ));
        }
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if file_type.is_dir() {
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

fn ensure_tree_has_no_symlinks(root: &Path) -> io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "Symlinks are forbidden inside copied directories: {}",
                    display_path(&entry.path())
                ),
            ));
        }
        if file_type.is_dir() {
            ensure_tree_has_no_symlinks(&entry.path())?;
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
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "Symlinks are forbidden inside verified directories: {}",
                    display_path(&entry.path())
                ),
            ));
        }
        let path = entry.path();
        if file_type.is_dir() {
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
