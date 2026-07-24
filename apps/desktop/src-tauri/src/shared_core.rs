use crate::contracts::{AssetOpenResult, BackupRevealInput, BackupRevealResult};
use crate::path_utils::home_dir;
use my_agent_assets_core::adopt::{
    apply_adopt, preview_adopt, AdoptApplyRequest, AdoptApplyResult, AdoptPreview,
    AdoptPreviewRequest,
};
use my_agent_assets_core::asset_access::{
    load_canonical_asset_content, resolve_asset_open_target, AssetOpenAction, AssetOpenRequest,
    CanonicalAssetContent,
};
use my_agent_assets_core::audit_log::{read_audit_entries, AuditLogEntry};
use my_agent_assets_core::backup_delete::{
    apply_backup_delete, preview_backup_delete, BackupDeleteApplyRequest, BackupDeleteApplyResult,
    BackupDeletePreview, BackupDeletePreviewRequest,
};
use my_agent_assets_core::backup_history::{
    list_backups, resolve_backup_manifest, BackupHistoryEntry,
};
use my_agent_assets_core::batch_import::{
    apply_batch_import, preview_batch_import, BatchImportApplyRequest, BatchImportApplyResult,
    BatchImportPreview, BatchImportPreviewRequest,
};
use my_agent_assets_core::consistency_repair::{
    apply_consistency_repair, preview_consistency_repair, ConsistencyRepairApplyRequest,
    ConsistencyRepairApplyResult, ConsistencyRepairPreview, ConsistencyRepairPreviewRequest,
};
use my_agent_assets_core::delete::{
    apply_delete, preview_delete, DeleteApplyRequest, DeleteApplyResult, DeletePreview,
    DeletePreviewRequest,
};
use my_agent_assets_core::diagnostic_export::{
    apply_diagnostic_export, preview_diagnostic_export, DiagnosticExportApplyRequest,
    DiagnosticExportApplyResult, DiagnosticExportPreview,
};
use my_agent_assets_core::diagnostics::{doctor, DoctorReport};
use my_agent_assets_core::discovery::{discover, DiscoveryResult, DiscoveryScope};
use my_agent_assets_core::git_sync::{
    apply_sync, preview_sync, status as git_status, GitStatus, SyncApplyRequest, SyncApplyResult,
    SyncPreview, SyncPreviewRequest,
};
use my_agent_assets_core::import::{
    apply_import, preview_import, ImportApplyRequest, ImportApplyResult, ImportPreview,
    ImportPreviewRequest,
};
use my_agent_assets_core::initialization::{
    apply_initialization, preview_initialization, InitializationApplyRequest,
    InitializationApplyResult, InitializationPreview,
};
use my_agent_assets_core::mcp_management::{
    apply_mcp_save, load_mcp_asset, preview_mcp_save, McpAssetDefinition, McpSaveApplyRequest,
    McpSaveApplyResult, McpSavePreview, McpSavePreviewRequest,
};
use my_agent_assets_core::mount::{
    apply_mount, apply_unmount, preview_mount, preview_unmount, MountApplyRequest,
    MountApplyResult, MountPreview, MountPreviewRequest, UnmountApplyRequest, UnmountApplyResult,
    UnmountPreview, UnmountPreviewRequest,
};
use my_agent_assets_core::operation::{
    recover_incomplete, recovery_status, RecoveryReport, RecoveryStatus,
};
use my_agent_assets_core::project_registry::{
    apply_remove_project, apply_save_project, preview_remove_project, preview_save_project,
    ProjectChangePreview, ProjectChangeResult, ProjectRemoveApplyRequest, ProjectRemoveRequest,
    ProjectSaveApplyRequest, ProjectSaveRequest,
};
use my_agent_assets_core::query::{
    list_assets, list_projects, AssetQueryRequest, AssetSummary, ProjectSummary,
};
use my_agent_assets_core::target_management::{
    apply_register_target, apply_remove_target, preview_register_target, preview_remove_target,
    TargetChangePreview, TargetChangeResult, TargetRegistrationApplyRequest,
    TargetRegistrationPreviewRequest, TargetRemoveApplyRequest, TargetRemovePreviewRequest,
};
use my_agent_assets_core::targets::{load as load_targets, MountTarget};
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

pub fn initialization_preview_command() -> Result<InitializationPreview, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; initialization preview skipped.".to_string())?;
    preview_initialization(&home).map_err(|error| error.to_string())
}

pub fn list_audit_log_command() -> Result<Vec<AuditLogEntry>, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; audit log lookup skipped.".to_string())?;
    list_audit_log_for_home(&home)
}

pub fn list_audit_log_for_home(home: &Path) -> Result<Vec<AuditLogEntry>, String> {
    read_audit_entries(home).map_err(|error| error.to_string())
}

pub fn doctor_report_command() -> Result<DoctorReport, String> {
    let home = home_dir().ok_or_else(|| "HOME is unavailable; diagnostics skipped.".to_string())?;
    Ok(doctor(&home))
}

pub fn consistency_repair_preview_command(
    input: ConsistencyRepairPreviewRequest,
) -> Result<ConsistencyRepairPreview, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; consistency repair preview skipped.".to_string())?;
    preview_consistency_repair(&home, &input).map_err(|error| error.to_string())
}

pub fn consistency_repair_apply_command(
    input: ConsistencyRepairApplyRequest,
) -> Result<ConsistencyRepairApplyResult, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; consistency repair apply blocked.".to_string())?;
    apply_consistency_repair(&home, &input).map_err(|error| error.to_string())
}

pub fn diagnostic_export_preview_command() -> Result<DiagnosticExportPreview, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; diagnostic export preview skipped.".to_string())?;
    preview_diagnostic_export(&home).map_err(|error| error.to_string())
}

pub fn diagnostic_export_apply_command(
    input: DiagnosticExportApplyRequest,
) -> Result<DiagnosticExportApplyResult, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; diagnostic export blocked.".to_string())?;
    apply_diagnostic_export(&home, &input).map_err(|error| error.to_string())
}

pub fn initialization_apply_command(
    input: InitializationApplyRequest,
) -> Result<InitializationApplyResult, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; initialization apply blocked.".to_string())?;
    apply_initialization(&home, &input).map_err(|error| error.to_string())
}

pub fn discover_runtime_sources_command(scope: DiscoveryScope) -> Result<DiscoveryResult, String> {
    let home = home_dir().ok_or_else(|| "HOME is unavailable; discovery skipped.".to_string())?;
    Ok(discover(&home, scope))
}

pub fn list_assets_command(input: AssetQueryRequest) -> Result<Vec<AssetSummary>, String> {
    let home = home_dir().ok_or_else(|| "HOME is unavailable; asset list skipped.".to_string())?;
    list_assets_for_home(&home, input)
}

pub fn canonical_asset_content_command(asset_id: String) -> Result<CanonicalAssetContent, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; asset content lookup skipped.".to_string())?;
    load_canonical_asset_content(&home, &asset_id).map_err(|error| error.to_string())
}

pub fn canonical_asset_open_command(input: AssetOpenRequest) -> Result<AssetOpenResult, String> {
    let home = home_dir().ok_or_else(|| "HOME is unavailable; asset open blocked.".to_string())?;
    let target = resolve_asset_open_target(&home, &input).map_err(|error| error.to_string())?;
    let (program, arguments) = platform_asset_open_command(&target.path, target.action)?;
    Command::new(program)
        .args(arguments)
        .spawn()
        .map_err(|error| format!("无法打开 canonical asset：{error}"))?;
    Ok(AssetOpenResult {
        asset_id: target.asset_id,
        path: target.path.to_string_lossy().into_owned(),
    })
}

pub fn list_projects_command() -> Result<Vec<ProjectSummary>, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; project list skipped.".to_string())?;
    list_projects_for_home(&home)
}

pub fn list_assets_for_home(
    home: &Path,
    input: AssetQueryRequest,
) -> Result<Vec<AssetSummary>, String> {
    list_assets(home, &input).map_err(|error| error.to_string())
}

pub fn list_projects_for_home(home: &Path) -> Result<Vec<ProjectSummary>, String> {
    list_projects(home).map_err(|error| error.to_string())
}

pub fn project_save_preview_command(
    input: ProjectSaveRequest,
) -> Result<ProjectChangePreview, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; project save preview skipped.".to_string())?;
    preview_save_project(&home, &input).map_err(|error| error.to_string())
}

pub fn project_save_apply_command(
    input: ProjectSaveApplyRequest,
) -> Result<ProjectChangeResult, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; project save apply blocked.".to_string())?;
    apply_save_project(&home, &input).map_err(|error| error.to_string())
}

pub fn project_remove_preview_command(
    input: ProjectRemoveRequest,
) -> Result<ProjectChangePreview, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; project remove preview skipped.".to_string())?;
    preview_remove_project(&home, &input).map_err(|error| error.to_string())
}

pub fn project_remove_apply_command(
    input: ProjectRemoveApplyRequest,
) -> Result<ProjectChangeResult, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; project remove apply blocked.".to_string())?;
    apply_remove_project(&home, &input).map_err(|error| error.to_string())
}

pub fn canonical_import_preview_command(
    input: ImportPreviewRequest,
) -> Result<ImportPreview, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; import preview skipped.".to_string())?;
    preview_import(&home, &input).map_err(|error| error.to_string())
}

pub fn canonical_import_apply_command(
    input: ImportApplyRequest,
) -> Result<ImportApplyResult, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; import apply blocked.".to_string())?;
    apply_import(&home, &input).map_err(|error| error.to_string())
}

pub fn list_mount_targets_command() -> Result<Vec<MountTarget>, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; target listing skipped.".to_string())?;
    Ok(load_targets(&home)
        .map_err(|error| error.to_string())?
        .targets)
}

pub fn target_registration_preview_command(
    input: TargetRegistrationPreviewRequest,
) -> Result<TargetChangePreview, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; target registration preview skipped.".to_string())?;
    preview_register_target(&home, &input).map_err(|error| error.to_string())
}

pub fn target_registration_apply_command(
    input: TargetRegistrationApplyRequest,
) -> Result<TargetChangeResult, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; target registration apply blocked.".to_string())?;
    apply_register_target(&home, &input).map_err(|error| error.to_string())
}

pub fn target_removal_preview_command(
    input: TargetRemovePreviewRequest,
) -> Result<TargetChangePreview, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; target removal preview skipped.".to_string())?;
    preview_remove_target(&home, &input).map_err(|error| error.to_string())
}

pub fn target_removal_apply_command(
    input: TargetRemoveApplyRequest,
) -> Result<TargetChangeResult, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; target removal apply blocked.".to_string())?;
    apply_remove_target(&home, &input).map_err(|error| error.to_string())
}

pub fn list_backup_history_command() -> Result<Vec<BackupHistoryEntry>, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; backup history skipped.".to_string())?;
    Ok(list_backups(&home))
}

pub fn reveal_backup_manifest_command(
    input: BackupRevealInput,
) -> Result<BackupRevealResult, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; backup reveal blocked.".to_string())?;
    let manifest =
        resolve_backup_manifest(&home, &input.entry_id).map_err(|error| error.to_string())?;
    let (program, arguments) = platform_reveal_command(&manifest)?;
    Command::new(program)
        .args(arguments)
        .spawn()
        .map_err(|error| format!("无法在文件管理器中显示备份 manifest：{error}"))?;
    Ok(BackupRevealResult {
        manifest_path: manifest.to_string_lossy().into_owned(),
    })
}

pub fn backup_delete_preview_command(
    input: BackupDeletePreviewRequest,
) -> Result<BackupDeletePreview, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; backup deletion preview skipped.".to_string())?;
    preview_backup_delete(&home, &input).map_err(|error| error.to_string())
}

pub fn backup_delete_apply_command(
    input: BackupDeleteApplyRequest,
) -> Result<BackupDeleteApplyResult, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; backup deletion apply blocked.".to_string())?;
    apply_backup_delete(&home, &input).map_err(|error| error.to_string())
}

fn platform_reveal_command(path: &Path) -> Result<(OsString, Vec<OsString>), String> {
    #[cfg(target_os = "macos")]
    {
        Ok((
            OsString::from("open"),
            vec![OsString::from("-R"), path.as_os_str().to_owned()],
        ))
    }
    #[cfg(windows)]
    {
        Ok((
            OsString::from("explorer"),
            vec![OsString::from(format!("/select,{}", path.display()))],
        ))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let parent = path
            .parent()
            .ok_or_else(|| "backup manifest has no parent directory".to_string())?;
        Ok((
            OsString::from("xdg-open"),
            vec![parent.as_os_str().to_owned()],
        ))
    }
}

fn platform_asset_open_command(
    path: &Path,
    action: AssetOpenAction,
) -> Result<(OsString, Vec<OsString>), String> {
    #[cfg(target_os = "macos")]
    {
        let mut arguments = Vec::new();
        if action == AssetOpenAction::Reveal {
            arguments.push(OsString::from("-R"));
        }
        arguments.push(path.as_os_str().to_owned());
        Ok((OsString::from("open"), arguments))
    }
    #[cfg(windows)]
    {
        let argument = if action == AssetOpenAction::Reveal {
            OsString::from(format!("/select,{}", path.display()))
        } else {
            path.as_os_str().to_owned()
        };
        Ok((OsString::from("explorer"), vec![argument]))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let target = if action == AssetOpenAction::Reveal {
            path.parent()
                .ok_or_else(|| "asset content has no parent directory".to_string())?
        } else {
            path
        };
        Ok((
            OsString::from("xdg-open"),
            vec![target.as_os_str().to_owned()],
        ))
    }
}

pub fn git_status_command() -> Result<GitStatus, String> {
    let home = home_dir().ok_or_else(|| "HOME is unavailable; Git status skipped.".to_string())?;
    Ok(git_status(&home))
}

pub fn recovery_status_command() -> Result<RecoveryStatus, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; recovery status skipped.".to_string())?;
    recovery_status(&home).map_err(|error| error.to_string())
}

pub fn startup_recovery_command() -> Result<RecoveryReport, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; startup recovery skipped.".to_string())?;
    recover_incomplete(&home).map_err(|error| error.to_string())
}

pub fn sync_preview_command(input: SyncPreviewRequest) -> Result<SyncPreview, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; Git sync preview skipped.".to_string())?;
    preview_sync(&home, &input).map_err(|error| error.to_string())
}

pub fn sync_apply_command(input: SyncApplyRequest) -> Result<SyncApplyResult, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; Git sync apply blocked.".to_string())?;
    apply_sync(&home, &input).map_err(|error| error.to_string())
}

pub fn canonical_mount_preview_command(input: MountPreviewRequest) -> Result<MountPreview, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; mount preview skipped.".to_string())?;
    preview_mount(&home, &input).map_err(|error| error.to_string())
}

pub fn canonical_mount_apply_command(input: MountApplyRequest) -> Result<MountApplyResult, String> {
    let home = home_dir().ok_or_else(|| "HOME is unavailable; mount apply blocked.".to_string())?;
    apply_mount(&home, &input).map_err(|error| error.to_string())
}

pub fn canonical_mcp_save_preview_command(
    input: McpSavePreviewRequest,
) -> Result<McpSavePreview, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; MCP save preview skipped.".to_string())?;
    preview_mcp_save(&home, &input).map_err(|error| error.to_string())
}

pub fn canonical_mcp_get_command(asset_id: String) -> Result<McpAssetDefinition, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; MCP asset lookup skipped.".to_string())?;
    load_mcp_asset(&home, &asset_id).map_err(|error| error.to_string())
}

pub fn canonical_mcp_save_apply_command(
    input: McpSaveApplyRequest,
) -> Result<McpSaveApplyResult, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; MCP save apply blocked.".to_string())?;
    apply_mcp_save(&home, &input).map_err(|error| error.to_string())
}

pub fn canonical_unmount_preview_command(
    input: UnmountPreviewRequest,
) -> Result<UnmountPreview, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; unmount preview skipped.".to_string())?;
    preview_unmount(&home, &input).map_err(|error| error.to_string())
}

pub fn canonical_unmount_apply_command(
    input: UnmountApplyRequest,
) -> Result<UnmountApplyResult, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; unmount apply blocked.".to_string())?;
    apply_unmount(&home, &input).map_err(|error| error.to_string())
}

pub fn canonical_delete_preview_command(
    input: DeletePreviewRequest,
) -> Result<DeletePreview, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; delete preview skipped.".to_string())?;
    preview_delete(&home, &input).map_err(|error| error.to_string())
}

pub fn canonical_delete_apply_command(
    input: DeleteApplyRequest,
) -> Result<DeleteApplyResult, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; delete apply blocked.".to_string())?;
    apply_delete(&home, &input).map_err(|error| error.to_string())
}

pub fn preview_adopt_command(input: AdoptPreviewRequest) -> Result<AdoptPreview, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; adopt preview skipped.".to_string())?;
    preview_adopt(&home, &input).map_err(|error| error.to_string())
}

pub fn adopt_apply_command(input: AdoptApplyRequest) -> Result<AdoptApplyResult, String> {
    let home = home_dir().ok_or_else(|| "HOME is unavailable; adopt apply blocked.".to_string())?;
    apply_adopt(&home, &input).map_err(|error| error.to_string())
}

pub fn canonical_batch_import_preview_command(
    input: BatchImportPreviewRequest,
) -> Result<BatchImportPreview, String> {
    let home = home_dir()
        .ok_or_else(|| "HOME is unavailable; batch import preview skipped.".to_string())?;
    preview_batch_import(&home, &input).map_err(|error| error.to_string())
}

pub fn canonical_batch_import_apply_command(
    input: BatchImportApplyRequest,
) -> Result<BatchImportApplyResult, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; batch import apply blocked.".to_string())?;
    apply_batch_import(&home, &input).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use my_agent_assets_core::discovery::AssetKind;
    use my_agent_assets_core::import::{ImportApplyStatus, ImportResolution};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn adapter_contract_round_trip_works_with_fake_home() {
        let home = test_home("round-trip");
        initialize(&home);
        fs::create_dir_all(home.join(".claude/skills/review")).unwrap();
        fs::write(home.join(".claude/skills/review/SKILL.md"), "# Review").unwrap();

        let source = discover(&home, DiscoveryScope::User)
            .sources
            .into_iter()
            .find(|source| source.asset_kind == AssetKind::Skill)
            .unwrap();
        let request = ImportPreviewRequest {
            scope: DiscoveryScope::User,
            source_id: source.source_id,
            resolution: ImportResolution::Unresolved,
        };
        let preview = preview_import(&home, &request).unwrap();
        let result = apply_import(
            &home,
            &ImportApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap();
        assert_eq!(result.status, ImportApplyStatus::Imported);
        assert!(home
            .join(".my-agent-assets/assets/skills/review/SKILL.md")
            .is_file());
        let assets = list_assets_for_home(
            &home,
            AssetQueryRequest {
                asset_type: Some(AssetKind::Skill),
            },
        )
        .unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].id, "skill:review");
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn project_query_adapter_lists_only_explicitly_registered_projects() {
        let home = test_home("project-query");
        initialize(&home);
        fs::create_dir_all(home.join("workspace/group/project-a")).unwrap();
        fs::write(home.join("workspace/group/project-a/package.json"), "{}").unwrap();

        assert!(list_projects_for_home(&home).unwrap().is_empty());
        let request = my_agent_assets_core::project_registry::ProjectSaveRequest {
            id: None,
            name: "project-a".into(),
            title: "Project A".into(),
            path: home.join("workspace/group/project-a"),
            description: "explicit project".into(),
        };
        let preview =
            my_agent_assets_core::project_registry::preview_save_project(&home, &request).unwrap();
        my_agent_assets_core::project_registry::apply_save_project(
            &home,
            &my_agent_assets_core::project_registry::ProjectSaveApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap();

        let projects = list_projects_for_home(&home).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "project-a");
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn audit_log_adapter_returns_only_shared_core_redacted_entries() {
        let home = test_home("audit-log");
        initialize(&home);
        my_agent_assets_core::audit_log::append_operation(
            &home,
            "mount",
            my_agent_assets_core::audit_log::AuditOutcome::Completed,
        )
        .unwrap();
        let entries = list_audit_log_for_home(&home).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].operation_type, "mount");
        assert_eq!(
            entries[0].outcome,
            my_agent_assets_core::audit_log::AuditOutcome::Completed
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn backup_reveal_uses_a_platform_command_without_a_shell() {
        let path = PathBuf::from("/tmp/backups/manifest.yaml");
        let (program, arguments) = platform_reveal_command(&path).unwrap();
        assert!(!program.is_empty());
        assert!(!arguments.is_empty());
        assert_ne!(program, OsString::from("sh"));
        assert_ne!(program, OsString::from("cmd"));
    }

    #[test]
    fn asset_open_uses_argument_arrays_without_a_shell() {
        let path = PathBuf::from("/tmp/assets/commands/commit.md");
        for action in [AssetOpenAction::Reveal, AssetOpenAction::OpenExternal] {
            let (program, arguments) = platform_asset_open_command(&path, action).unwrap();
            assert!(!program.is_empty());
            assert!(!arguments.is_empty());
            assert_ne!(program, OsString::from("sh"));
            assert_ne!(program, OsString::from("cmd"));
        }
    }

    fn initialize(home: &Path) {
        let preview = preview_initialization(home).unwrap();
        apply_initialization(
            home,
            &InitializationApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
            },
        )
        .unwrap();
    }

    fn test_home(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "maa-shared-core-adapter-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
}
