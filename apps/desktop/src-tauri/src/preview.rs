use crate::contracts::{
    AssetStatus, AssetSummary, AssetType, BackupSummary, ConflictPreview, ConflictResolution,
    GitStatus, ImportPreview, MountPreview, PlanStep, PlanStepKind, PreviewConflictsInput,
    PreviewImportInput, PreviewMountInput, PreviewRestoreInput, PreviewSyncInput, RestorePreview,
    RiskLevel, RuntimeScope, SyncDirection, SyncPreview,
};
use crate::path_utils::{display_path, home_dir};
use crate::read_only;
use serde::Deserialize;
use std::fs;
use std::path::Path;

pub fn preview_import_command(input: PreviewImportInput) -> ImportPreview {
    preview_import(input)
}

pub fn preview_mount_command(input: PreviewMountInput) -> MountPreview {
    preview_mount(input)
}

pub fn preview_conflicts_command(input: PreviewConflictsInput) -> Vec<ConflictPreview> {
    preview_conflicts(input)
}

pub fn preview_restore_command(input: PreviewRestoreInput) -> RestorePreview {
    match home_dir() {
        Some(home) => preview_restore_for_home(&home, input),
        None => {
            let mut preview = preview_restore(input);
            preview
                .warnings
                .push("HOME is unavailable; using synthetic restore preview.".into());
            preview
        }
    }
}

pub fn preview_sync_command(input: PreviewSyncInput) -> SyncPreview {
    preview_sync(input, read_only::git_status_command())
}

pub fn preview_import(input: PreviewImportInput) -> ImportPreview {
    let assets = input
        .asset_ids
        .iter()
        .map(|asset_id| asset_from_id(asset_id))
        .collect::<Vec<_>>();
    let conflicts = input
        .conflict_resolutions
        .iter()
        .map(|choice| conflict_from_id(&choice.conflict_id))
        .collect::<Vec<_>>();
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
        scope: input.scope,
        assets,
        conflicts,
        steps,
        warnings: vec!["Preview only: no files will be written.".into()],
        can_apply: !input.asset_ids.is_empty(),
    }
}

pub fn preview_mount(input: PreviewMountInput) -> MountPreview {
    let asset = asset_from_id(&input.asset_id);
    let target_path = input.target.runtime_path.clone();
    let is_mcp = asset.asset_type == AssetType::Mcp;
    let mut warnings = vec!["Preview only: no symlink or config file will be changed.".into()];
    if target_path.trim().is_empty() {
        warnings.push("Target runtime path is empty.".into());
    }

    MountPreview {
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
        can_apply: !target_path.trim().is_empty(),
    }
}

pub fn preview_conflicts(input: PreviewConflictsInput) -> Vec<ConflictPreview> {
    input
        .asset_ids
        .iter()
        .map(|asset_id| conflict_from_id(asset_id))
        .collect()
}

pub fn preview_restore(input: PreviewRestoreInput) -> RestorePreview {
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

pub fn preview_restore_for_home(home: &Path, input: PreviewRestoreInput) -> RestorePreview {
    let manifest_path = home
        .join(".my-agent-assets")
        .join("backups")
        .join(&input.backup_id)
        .join("manifest.json");
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
        && status.conflicts.is_empty()
        && match input.direction {
            SyncDirection::Pull => status.behind > 0,
            SyncDirection::Push => status.ahead > 0,
        };

    SyncPreview {
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

fn asset_from_id(asset_id: &str) -> AssetSummary {
    let (asset_type, name) = parse_asset_id(asset_id);
    let prefix = asset_type_prefix(&asset_type);
    AssetSummary {
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
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreManifestPreview {
    id: String,
    label: String,
    created_at: String,
    entries: Vec<RestoreEntryPreview>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreEntryPreview {
    original_path: String,
    size_bytes: u64,
}

fn conflict_from_id(asset_id: &str) -> ConflictPreview {
    let asset = asset_from_id(asset_id);
    ConflictPreview {
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
    }
}

fn parse_asset_id(asset_id: &str) -> (AssetType, String) {
    let mut parts = asset_id.splitn(2, ':');
    let prefix = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or(prefix).trim();
    let asset_type = match prefix {
        "command" => AssetType::Command,
        "mcp" => AssetType::Mcp,
        _ => AssetType::Skill,
    };
    let fallback_name = if name.is_empty() { "unknown" } else { name };
    (asset_type, fallback_name.into())
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
