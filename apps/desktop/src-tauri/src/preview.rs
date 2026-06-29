use crate::contracts::{
    AssetStatus, AssetSummary, AssetType, ConflictPreview, ConflictResolution,
    ConflictResolutionChoice, GitStatus, ImportPreview, MountPreview, MountTarget, PlanStep,
    PlanStepKind, PreviewConflictsInput, PreviewImportInput, PreviewMountInput, PreviewSyncInput,
    RiskLevel, RuntimeScope, ScanScope, SyncDirection, SyncPreview,
};
#[cfg(test)]
use crate::contracts::{BackupSummary, PreviewRestoreInput, RestorePreview};
#[cfg(test)]
use crate::path_utils::display_path;
use crate::path_utils::{
    expand_tilde, guard_existing_path, home_dir, validate_single_path_component,
};
use crate::read_only;
#[cfg(test)]
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub fn preview_import_command(input: PreviewImportInput) -> ImportPreview {
    preview_import(input)
}

pub fn preview_mount_command(input: PreviewMountInput) -> MountPreview {
    preview_mount(input)
}

pub fn preview_conflicts_command(input: PreviewConflictsInput) -> Vec<ConflictPreview> {
    match home_dir() {
        Some(home) => preview_conflicts_for_home(&home, input),
        None => preview_conflicts(input),
    }
}

pub fn preview_sync_command(input: PreviewSyncInput) -> SyncPreview {
    preview_sync(input, read_only::git_status_command())
}

pub fn preview_import(input: PreviewImportInput) -> ImportPreview {
    let preview_id = import_preview_id(&input.scope, &input.asset_ids, &input.conflict_resolutions);
    let mut warnings = vec!["Preview only: no files will be written.".into()];
    let mut invalid_ids = Vec::new();
    let assets = input
        .asset_ids
        .iter()
        .filter_map(|asset_id| match asset_from_id(asset_id) {
            Ok(asset) => Some(asset),
            Err(error) => {
                invalid_ids.push(error);
                None
            }
        })
        .collect::<Vec<_>>();
    let conflicts = input
        .conflict_resolutions
        .iter()
        .filter_map(|choice| match conflict_from_id(&choice.conflict_id) {
            Ok(conflict) => Some(conflict),
            Err(error) => {
                invalid_ids.push(error);
                None
            }
        })
        .collect::<Vec<_>>();
    warnings.extend(invalid_ids.iter().cloned());
    let mut steps = vec![step(
        "check-selection",
        PlanStepKind::Check,
        "校验选择资产",
        format!("预览 {} 个资产 ID。", input.asset_ids.len()),
        RiskLevel::None,
    )];

    if !conflicts.is_empty() {
        steps.push(step(
            "resolve-conflicts",
            PlanStepKind::Check,
            "应用冲突决策预览",
            format!("预览 {} 个冲突处理选择。", conflicts.len()),
            RiskLevel::Medium,
        ));
    }
    steps.push(step(
        "preview-import",
        PlanStepKind::Import,
        "生成导入计划",
        "仅生成计划，不复制、不移动、不创建文件。",
        RiskLevel::Low,
    ));

    ImportPreview {
        preview_id,
        scope: input.scope,
        assets,
        conflicts,
        steps,
        warnings,
        can_apply: !input.asset_ids.is_empty() && invalid_ids.is_empty(),
    }
}

pub fn preview_mount(input: PreviewMountInput) -> MountPreview {
    let preview_id = mount_preview_id(&input.asset_id, &input.target);
    let parsed_asset = asset_from_id(&input.asset_id);
    let asset_is_valid = parsed_asset.is_ok();
    let asset = parsed_asset.unwrap_or_else(|error| invalid_asset_from_id(&input.asset_id, &error));
    let target_path = input.target.runtime_path.clone();
    let is_mcp = asset.asset_type == AssetType::Mcp;
    let mut warnings = vec!["Preview only: no symlink or config file will be changed.".into()];
    if !asset_is_valid {
        warnings.push(format!("Invalid asset ID: {}.", input.asset_id));
    }
    if target_path.trim().is_empty() {
        warnings.push("Target runtime path is empty.".into());
    }

    MountPreview {
        preview_id,
        asset,
        target: input.target,
        steps: vec![
            step(
                "check-source",
                PlanStepKind::Check,
                "校验资产来源",
                "确认资产中心存在目标资产。",
                RiskLevel::None,
            ),
            step(
                "backup-target",
                PlanStepKind::Backup,
                "预览目标备份",
                format!("受影响目标：{}", target_path),
                RiskLevel::Low,
            ),
            step(
                "preview-mount",
                if is_mcp {
                    PlanStepKind::CompileMcp
                } else {
                    PlanStepKind::Mount
                },
                if is_mcp {
                    "预览 MCP 编译"
                } else {
                    "预览软链接挂载"
                },
                "仅生成挂载计划，不写入 runtime。",
                RiskLevel::Medium,
            ),
        ],
        warnings,
        backup_required: true,
        can_apply: asset_is_valid && !target_path.trim().is_empty(),
    }
}

pub fn preview_conflicts(input: PreviewConflictsInput) -> Vec<ConflictPreview> {
    input
        .asset_ids
        .iter()
        .filter_map(|asset_id| conflict_from_id(asset_id).ok())
        .collect()
}

pub fn preview_conflicts_for_home(
    home: &Path,
    input: PreviewConflictsInput,
) -> Vec<ConflictPreview> {
    input
        .asset_ids
        .iter()
        .filter_map(|asset_id| {
            real_conflict_from_id(home, &input.scope, asset_id)
                .ok()
                .flatten()
        })
        .collect()
}

#[cfg(test)]
pub fn preview_restore(input: PreviewRestoreInput) -> RestorePreview {
    let preview_id = restore_preview_id(&input.backup_id);
    let backup = BackupSummary {
        id: input.backup_id.clone(),
        label: format!("Restore preview for {}", input.backup_id),
        created_at: "preview-only".into(),
        size_bytes: 0,
        entry_count: 3,
    };
    let affected_paths = vec![
        format!("backups/{}/manifest.json", input.backup_id),
        "~/.claude/skills/review".into(),
        "~/workspace/project-a/.mcp.json".into(),
    ];

    RestorePreview {
        preview_id,
        backup,
        affected_paths,
        steps: vec![
            step(
                "check-backup",
                PlanStepKind::Check,
                "校验备份清单",
                "确认备份 ID 与 manifest 引用。",
                RiskLevel::None,
            ),
            step(
                "backup-current",
                PlanStepKind::Backup,
                "预览当前状态备份",
                "恢复前将先创建当前状态备份。",
                RiskLevel::Low,
            ),
            step(
                "preview-restore",
                PlanStepKind::Restore,
                "生成恢复影响预览",
                "仅展示受影响路径，不还原文件。",
                RiskLevel::High,
            ),
        ],
        warnings: vec!["Preview only: restore is not executed.".into()],
        backup_before_restore: true,
        can_apply: !input.backup_id.trim().is_empty(),
    }
}

#[cfg(test)]
pub fn preview_restore_for_home(home: &Path, input: PreviewRestoreInput) -> RestorePreview {
    if let Err(error) = validate_single_path_component(&input.backup_id, "backup ID") {
        let mut preview = preview_restore(input);
        preview.can_apply = false;
        preview.warnings.push(error);
        return preview;
    }
    let manifest_path = home
        .join(".my-agent-assets")
        .join("backups")
        .join(&input.backup_id)
        .join("manifest.json");
    let manifest_path = match guard_existing_path(home, &manifest_path) {
        Ok(path) => path,
        Err(error) => {
            let mut preview = preview_restore(input);
            preview.can_apply = false;
            preview.warnings.push(format!(
                "Could not safely resolve backup manifest {}; using synthetic restore preview: {}",
                display_path(&manifest_path),
                error
            ));
            return preview;
        }
    };
    let text = match fs::read_to_string(&manifest_path) {
        Ok(text) => text,
        Err(error) => {
            let mut preview = preview_restore(input);
            preview.warnings.push(format!(
                "Could not read backup manifest {}; using synthetic restore preview: {}",
                display_path(&manifest_path),
                error
            ));
            return preview;
        }
    };
    let manifest = match serde_json::from_str::<RestoreManifestPreview>(&text) {
        Ok(manifest) => manifest,
        Err(error) => {
            let mut preview = preview_restore(input);
            preview.warnings.push(format!(
                "Could not parse backup manifest {}; using synthetic restore preview: {}",
                display_path(&manifest_path),
                error
            ));
            return preview;
        }
    };
    let affected_paths = manifest
        .entries
        .iter()
        .map(|entry| entry.original_path.clone())
        .collect::<Vec<_>>();
    let size_bytes = manifest
        .entries
        .iter()
        .map(|entry| entry.size_bytes)
        .sum::<u64>();
    let entry_count = affected_paths.len() as u32;

    RestorePreview {
        preview_id: restore_preview_id(&input.backup_id),
        backup: BackupSummary {
            id: manifest.id,
            label: manifest.label,
            created_at: manifest.created_at,
            size_bytes,
            entry_count,
        },
        affected_paths,
        steps: vec![
            step(
                "check-backup",
                PlanStepKind::Check,
                "校验备份清单",
                format!("读取 manifest：{}", display_path(&manifest_path)),
                RiskLevel::None,
            ),
            step(
                "backup-current",
                PlanStepKind::Backup,
                "预览当前状态备份",
                "恢复前将先创建当前状态备份。",
                RiskLevel::Low,
            ),
            step(
                "preview-restore",
                PlanStepKind::Restore,
                "生成恢复影响预览",
                format!("仅展示 {} 个受影响路径，不还原文件。", entry_count),
                RiskLevel::High,
            ),
        ],
        warnings: vec!["Preview only: restore is not executed.".into()],
        backup_before_restore: true,
        can_apply: entry_count > 0,
    }
}

pub fn preview_sync(input: PreviewSyncInput, status: GitStatus) -> SyncPreview {
    let preview_id = sync_preview_id(&input.direction, &status);
    let direction_label = match input.direction {
        SyncDirection::Pull => "Pull",
        SyncDirection::Push => "Push",
    };
    let mut warnings = vec!["Preview only: no git pull, push, or fetch is executed.".into()];
    if !status.is_repository {
        warnings.push("Asset center is not a Git repository.".into());
    }
    if status.remote.is_none() {
        warnings.push("No upstream remote is configured.".into());
    }
    if !status.clean {
        warnings.push(format!(
            "{} local changed file(s) may need review before sync.",
            status.changed_files.len()
        ));
    }
    if !status.conflicts.is_empty() {
        warnings.push(format!(
            "{} conflict path(s) require manual resolution before sync.",
            status.conflicts.len()
        ));
    }

    let can_apply = status.is_repository
        && status.remote.is_some()
        && status.clean
        && status.conflicts.is_empty()
        && match input.direction {
            SyncDirection::Pull => status.behind > 0,
            SyncDirection::Push => status.ahead > 0,
        };

    SyncPreview {
        preview_id,
        direction: input.direction,
        repository_path: status.repository_path.clone(),
        branch: status.branch.clone(),
        remote: status.remote.clone(),
        steps: vec![
            step(
                "check-git-repository",
                PlanStepKind::Check,
                "校验本地 Git 仓库",
                status.status_message,
                RiskLevel::None,
            ),
            step(
                "check-sync-risk",
                PlanStepKind::Git,
                format!("预览 {} 风险", direction_label),
                format!(
                    "Ahead {} commit(s), behind {} commit(s), changed {} file(s), conflicts {}.",
                    status.ahead,
                    status.behind,
                    status.changed_files.len(),
                    status.conflicts.len()
                ),
                if status.conflicts.is_empty() {
                    RiskLevel::Low
                } else {
                    RiskLevel::High
                },
            ),
            step(
                "preview-git-sync",
                PlanStepKind::Git,
                format!("生成 {} 计划", direction_label),
                "仅生成同步计划，不执行 Git 同步命令。",
                RiskLevel::Medium,
            ),
        ],
        warnings,
        can_apply,
    }
}

pub fn import_preview_id(
    scope: &ScanScope,
    asset_ids: &[String],
    conflict_resolutions: &[ConflictResolutionChoice],
) -> String {
    let mut sorted_assets = asset_ids.to_vec();
    sorted_assets.sort();

    let mut sorted_resolutions = conflict_resolutions
        .iter()
        .map(|choice| {
            format!(
                "{}:{}:{}",
                choice.conflict_id,
                wire_json(&choice.resolution),
                choice.rename_to.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>();
    sorted_resolutions.sort();

    stable_preview_id(
        "import",
        &[
            canonical_scan_scope(scope),
            sorted_assets.join("\n"),
            sorted_resolutions.join("\n"),
        ],
    )
}

pub fn mount_preview_id(asset_id: &str, target: &MountTarget) -> String {
    stable_preview_id(
        "mount",
        &[
            asset_id.to_string(),
            wire_json(&target.scope),
            target.runtime_path.clone(),
            target.project_path.clone().unwrap_or_default(),
        ],
    )
}

#[cfg(test)]
pub fn restore_preview_id(backup_id: &str) -> String {
    stable_preview_id("restore", &[backup_id.to_string()])
}

pub fn sync_preview_id(direction: &SyncDirection, status: &GitStatus) -> String {
    stable_preview_id(
        "sync",
        &[
            wire_json(direction),
            status.repository_path.clone(),
            status.branch.clone(),
            status.remote.clone().unwrap_or_default(),
            status.ahead.to_string(),
            status.behind.to_string(),
            status.changed_files.join("\n"),
            status.conflicts.join("\n"),
        ],
    )
}

fn canonical_scan_scope(scope: &ScanScope) -> String {
    match scope {
        ScanScope::User => "user".into(),
        ScanScope::Project { project_path } => format!("project:{}", project_path),
        ScanScope::Custom { path } => format!("custom:{}", path),
    }
}

fn stable_preview_id(kind: &str, parts: &[String]) -> String {
    let mut canonical = String::new();
    canonical.push_str(kind);
    for part in parts {
        canonical.push('\u{1f}');
        canonical.push_str(part);
    }
    format!("preview:{}:{:016x}", kind, fnv1a64(canonical.as_bytes()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn wire_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".into())
}

fn asset_from_id(asset_id: &str) -> Result<AssetSummary, String> {
    let (asset_type, name) = parse_asset_id(asset_id)?;
    let prefix = asset_type_prefix(&asset_type);
    Ok(AssetSummary {
        id: format!("{}:{}", prefix, name),
        name: name.clone(),
        title: name.clone(),
        asset_type,
        status: AssetStatus::Ready,
        category: "preview".into(),
        description: format!("Preview summary for {}", name),
        source_path: format!("~/.my-agent-assets/assets/{}/{}", prefix, name),
        scope: Some(RuntimeScope::Local),
        updated_at: None,
        mount_targets: vec![],
    })
}

fn invalid_asset_from_id(asset_id: &str, error: &str) -> AssetSummary {
    AssetSummary {
        id: asset_id.into(),
        name: asset_id.into(),
        title: "Invalid asset ID".into(),
        asset_type: AssetType::Skill,
        status: AssetStatus::Invalid,
        category: "invalid".into(),
        description: error.into(),
        source_path: String::new(),
        scope: None,
        updated_at: None,
        mount_targets: vec![],
    }
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreManifestPreview {
    id: String,
    label: String,
    created_at: String,
    entries: Vec<RestoreEntryPreview>,
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreEntryPreview {
    original_path: String,
    size_bytes: u64,
}

fn conflict_from_id(asset_id: &str) -> Result<ConflictPreview, String> {
    let normalized_id = asset_id.strip_prefix("conflict:").unwrap_or(asset_id);
    let asset = asset_from_id(normalized_id)?;
    Ok(ConflictPreview {
        id: format!("conflict:{}", asset.id),
        asset_id: asset.id.clone(),
        asset_type: asset.asset_type,
        name: asset.name.clone(),
        reason: "同名资产预览冲突".into(),
        existing_content: format!("Existing preview content for {}", asset.name),
        incoming_content: format!("Incoming preview content for {}", asset.name),
        allowed_resolutions: vec![
            ConflictResolution::Skip,
            ConflictResolution::Rename,
            ConflictResolution::Overwrite,
        ],
    })
}

pub(crate) fn real_conflict_from_id(
    home: &Path,
    scope: &ScanScope,
    asset_id: &str,
) -> Result<Option<ConflictPreview>, String> {
    let (asset_type, name) = parse_asset_id(asset_id)?;
    let asset_root = home.join(".my-agent-assets").join("assets");
    let runtime_root = match scope {
        ScanScope::User => home.to_path_buf(),
        ScanScope::Project { project_path } => expand_tilde(project_path, home),
        ScanScope::Custom { path } => expand_tilde(path, home),
    };

    let (existing_path, incoming_path) = match asset_type {
        AssetType::Skill => (
            preferred_skill_path(&asset_root.join("skills"), &name),
            preferred_skill_path(&runtime_root.join(".claude").join("skills"), &name),
        ),
        AssetType::Command => (
            asset_root.join("commands").join(format!("{}.md", name)),
            runtime_root
                .join(".claude")
                .join("commands")
                .join(format!("{}.md", name)),
        ),
        AssetType::Mcp => (
            asset_root.join("mcps").join(format!("{}.json", name)),
            match scope {
                ScanScope::User => runtime_root.join(".claude.json"),
                ScanScope::Project { .. } | ScanScope::Custom { .. } => {
                    runtime_root.join(".mcp.json")
                }
            },
        ),
    };

    if !existing_path.exists() || !incoming_path.exists() {
        return Ok(None);
    }

    let existing_content = read_conflict_content(home, &existing_path, &asset_type, &name, false)?;
    let incoming_content = read_conflict_content(home, &incoming_path, &asset_type, &name, true)?;
    if existing_content == incoming_content {
        return Ok(None);
    }

    Ok(Some(ConflictPreview {
        id: format!("conflict:{}", asset_id),
        asset_id: asset_id.into(),
        asset_type,
        name,
        reason: "同名资产内容不同".into(),
        existing_content,
        incoming_content,
        allowed_resolutions: vec![
            ConflictResolution::Skip,
            ConflictResolution::Rename,
            ConflictResolution::Overwrite,
        ],
    }))
}

fn preferred_skill_path(root: &Path, name: &str) -> PathBuf {
    let directory = root.join(name);
    if directory.is_dir() {
        directory.join("SKILL.md")
    } else {
        root.join(format!("{}.md", name))
    }
}

fn read_conflict_content(
    home: &Path,
    path: &Path,
    asset_type: &AssetType,
    name: &str,
    extract_runtime_mcp: bool,
) -> Result<String, String> {
    let path = guard_existing_path(home, path).map_err(|error| error.to_string())?;
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    if *asset_type != AssetType::Mcp || !extract_runtime_mcp {
        if *asset_type == AssetType::Mcp {
            let value: Value = serde_json::from_str(&content).map_err(|error| error.to_string())?;
            return serde_json::to_string_pretty(&value).map_err(|error| error.to_string());
        }
        return Ok(content);
    }

    let config: Value = serde_json::from_str(&content).map_err(|error| error.to_string())?;
    let server = config
        .get("mcpServers")
        .and_then(Value::as_object)
        .and_then(|servers| servers.get(name))
        .ok_or_else(|| format!("mcpServers.{} was not found.", name))?;
    serde_json::to_string_pretty(server).map_err(|error| error.to_string())
}

fn parse_asset_id(asset_id: &str) -> Result<(AssetType, String), String> {
    let mut parts = asset_id.splitn(2, ':');
    let prefix = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default().trim();
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

fn asset_type_prefix(asset_type: &AssetType) -> &'static str {
    match asset_type {
        AssetType::Skill => "skill",
        AssetType::Command => "command",
        AssetType::Mcp => "mcp",
    }
}

fn step(
    id: &str,
    kind: PlanStepKind,
    label: impl Into<String>,
    description: impl Into<String>,
    risk: RiskLevel,
) -> PlanStep {
    PlanStep {
        id: id.into(),
        kind,
        label: label.into(),
        description: description.into(),
        risk,
    }
}
