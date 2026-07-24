mod command_error;
mod contracts;
mod path_utils;
mod settings;
mod shared_core;

use command_error::DesktopCommandError;
use contracts::{
    AppInfo, AssetOpenResult, BackupRevealInput, BackupRevealResult, CanonicalAssetContentInput,
    CanonicalMcpGetInput, DesktopSettings, SettingsSaveInput,
};

type CommandResult<T> = Result<T, DesktopCommandError>;

fn command_result<T>(result: Result<T, String>) -> CommandResult<T> {
    result.map_err(DesktopCommandError::from_core_message)
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: "My Agent Assets",
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        backend_ready: true,
    }
}

#[tauri::command]
fn settings_load() -> CommandResult<DesktopSettings> {
    command_result(settings::settings_load_command())
}

#[tauri::command]
fn settings_save(input: SettingsSaveInput) -> CommandResult<DesktopSettings> {
    command_result(settings::settings_save_command(input))
}

#[tauri::command]
fn git_status() -> CommandResult<my_agent_assets_core::git_sync::GitStatus> {
    command_result(shared_core::git_status_command())
}

#[tauri::command]
fn recovery_status() -> CommandResult<my_agent_assets_core::operation::RecoveryStatus> {
    command_result(shared_core::recovery_status_command())
}

#[tauri::command]
fn list_audit_log() -> CommandResult<Vec<my_agent_assets_core::audit_log::AuditLogEntry>> {
    command_result(shared_core::list_audit_log_command())
}

#[tauri::command]
fn doctor_report() -> CommandResult<my_agent_assets_core::diagnostics::DoctorReport> {
    command_result(shared_core::doctor_report_command())
}

#[tauri::command]
fn consistency_repair_preview(
    input: my_agent_assets_core::consistency_repair::ConsistencyRepairPreviewRequest,
) -> CommandResult<my_agent_assets_core::consistency_repair::ConsistencyRepairPreview> {
    command_result(shared_core::consistency_repair_preview_command(input))
}

#[tauri::command]
fn consistency_repair_apply(
    input: my_agent_assets_core::consistency_repair::ConsistencyRepairApplyRequest,
) -> CommandResult<my_agent_assets_core::consistency_repair::ConsistencyRepairApplyResult> {
    command_result(shared_core::consistency_repair_apply_command(input))
}

#[tauri::command]
fn diagnostic_export_preview(
) -> CommandResult<my_agent_assets_core::diagnostic_export::DiagnosticExportPreview> {
    command_result(shared_core::diagnostic_export_preview_command())
}

#[tauri::command]
fn diagnostic_export_apply(
    input: my_agent_assets_core::diagnostic_export::DiagnosticExportApplyRequest,
) -> CommandResult<my_agent_assets_core::diagnostic_export::DiagnosticExportApplyResult> {
    command_result(shared_core::diagnostic_export_apply_command(input))
}

#[tauri::command]
fn initialization_preview(
) -> CommandResult<my_agent_assets_core::initialization::InitializationPreview> {
    command_result(shared_core::initialization_preview_command())
}

#[tauri::command]
fn initialization_apply(
    input: my_agent_assets_core::initialization::InitializationApplyRequest,
) -> CommandResult<my_agent_assets_core::initialization::InitializationApplyResult> {
    command_result(shared_core::initialization_apply_command(input))
}

#[tauri::command]
fn list_assets(
    input: my_agent_assets_core::query::AssetQueryRequest,
) -> CommandResult<Vec<my_agent_assets_core::query::AssetSummary>> {
    command_result(shared_core::list_assets_command(input))
}

#[tauri::command]
fn canonical_asset_content(
    input: CanonicalAssetContentInput,
) -> CommandResult<my_agent_assets_core::asset_access::CanonicalAssetContent> {
    command_result(shared_core::canonical_asset_content_command(input.asset_id))
}

#[tauri::command]
fn canonical_asset_open(
    input: my_agent_assets_core::asset_access::AssetOpenRequest,
) -> CommandResult<AssetOpenResult> {
    command_result(shared_core::canonical_asset_open_command(input))
}

#[tauri::command]
fn list_projects() -> CommandResult<Vec<my_agent_assets_core::query::ProjectSummary>> {
    command_result(shared_core::list_projects_command())
}

#[tauri::command]
fn project_save_preview(
    input: my_agent_assets_core::project_registry::ProjectSaveRequest,
) -> CommandResult<my_agent_assets_core::project_registry::ProjectChangePreview> {
    command_result(shared_core::project_save_preview_command(input))
}

#[tauri::command]
fn project_save_apply(
    input: my_agent_assets_core::project_registry::ProjectSaveApplyRequest,
) -> CommandResult<my_agent_assets_core::project_registry::ProjectChangeResult> {
    command_result(shared_core::project_save_apply_command(input))
}

#[tauri::command]
fn project_remove_preview(
    input: my_agent_assets_core::project_registry::ProjectRemoveRequest,
) -> CommandResult<my_agent_assets_core::project_registry::ProjectChangePreview> {
    command_result(shared_core::project_remove_preview_command(input))
}

#[tauri::command]
fn project_remove_apply(
    input: my_agent_assets_core::project_registry::ProjectRemoveApplyRequest,
) -> CommandResult<my_agent_assets_core::project_registry::ProjectChangeResult> {
    command_result(shared_core::project_remove_apply_command(input))
}

#[tauri::command]
fn list_backups() -> CommandResult<Vec<my_agent_assets_core::backup_history::BackupHistoryEntry>> {
    command_result(shared_core::list_backup_history_command())
}

#[tauri::command]
fn reveal_backup_manifest(input: BackupRevealInput) -> CommandResult<BackupRevealResult> {
    command_result(shared_core::reveal_backup_manifest_command(input))
}

#[tauri::command]
fn backup_delete_preview(
    input: my_agent_assets_core::backup_delete::BackupDeletePreviewRequest,
) -> CommandResult<my_agent_assets_core::backup_delete::BackupDeletePreview> {
    command_result(shared_core::backup_delete_preview_command(input))
}

#[tauri::command]
fn backup_delete_apply(
    input: my_agent_assets_core::backup_delete::BackupDeleteApplyRequest,
) -> CommandResult<my_agent_assets_core::backup_delete::BackupDeleteApplyResult> {
    command_result(shared_core::backup_delete_apply_command(input))
}

#[tauri::command]
fn preview_sync(
    input: my_agent_assets_core::git_sync::SyncPreviewRequest,
) -> CommandResult<my_agent_assets_core::git_sync::SyncPreview> {
    command_result(shared_core::sync_preview_command(input))
}

#[tauri::command]
fn sync_apply(
    input: my_agent_assets_core::git_sync::SyncApplyRequest,
) -> CommandResult<my_agent_assets_core::git_sync::SyncApplyResult> {
    command_result(shared_core::sync_apply_command(input))
}

#[tauri::command]
fn discover_runtime_sources(
    input: my_agent_assets_core::discovery::DiscoveryScope,
) -> CommandResult<my_agent_assets_core::discovery::DiscoveryResult> {
    command_result(shared_core::discover_runtime_sources_command(input))
}

#[tauri::command]
fn canonical_import_preview(
    input: my_agent_assets_core::import::ImportPreviewRequest,
) -> CommandResult<my_agent_assets_core::import::ImportPreview> {
    command_result(shared_core::canonical_import_preview_command(input))
}

#[tauri::command]
fn canonical_import_apply(
    input: my_agent_assets_core::import::ImportApplyRequest,
) -> CommandResult<my_agent_assets_core::import::ImportApplyResult> {
    command_result(shared_core::canonical_import_apply_command(input))
}

#[tauri::command]
fn list_mount_targets() -> CommandResult<Vec<my_agent_assets_core::targets::MountTarget>> {
    command_result(shared_core::list_mount_targets_command())
}

#[tauri::command]
fn target_registration_preview(
    input: my_agent_assets_core::target_management::TargetRegistrationPreviewRequest,
) -> CommandResult<my_agent_assets_core::target_management::TargetChangePreview> {
    command_result(shared_core::target_registration_preview_command(input))
}

#[tauri::command]
fn target_registration_apply(
    input: my_agent_assets_core::target_management::TargetRegistrationApplyRequest,
) -> CommandResult<my_agent_assets_core::target_management::TargetChangeResult> {
    command_result(shared_core::target_registration_apply_command(input))
}

#[tauri::command]
fn target_removal_preview(
    input: my_agent_assets_core::target_management::TargetRemovePreviewRequest,
) -> CommandResult<my_agent_assets_core::target_management::TargetChangePreview> {
    command_result(shared_core::target_removal_preview_command(input))
}

#[tauri::command]
fn target_removal_apply(
    input: my_agent_assets_core::target_management::TargetRemoveApplyRequest,
) -> CommandResult<my_agent_assets_core::target_management::TargetChangeResult> {
    command_result(shared_core::target_removal_apply_command(input))
}

#[tauri::command]
fn canonical_mount_preview(
    input: my_agent_assets_core::mount::MountPreviewRequest,
) -> CommandResult<my_agent_assets_core::mount::MountPreview> {
    command_result(shared_core::canonical_mount_preview_command(input))
}

#[tauri::command]
fn canonical_mount_apply(
    input: my_agent_assets_core::mount::MountApplyRequest,
) -> CommandResult<my_agent_assets_core::mount::MountApplyResult> {
    command_result(shared_core::canonical_mount_apply_command(input))
}

#[tauri::command]
fn canonical_mcp_save_preview(
    input: my_agent_assets_core::mcp_management::McpSavePreviewRequest,
) -> CommandResult<my_agent_assets_core::mcp_management::McpSavePreview> {
    command_result(shared_core::canonical_mcp_save_preview_command(input))
}

#[tauri::command]
fn canonical_mcp_get(
    input: CanonicalMcpGetInput,
) -> CommandResult<my_agent_assets_core::mcp_management::McpAssetDefinition> {
    command_result(shared_core::canonical_mcp_get_command(input.asset_id))
}

#[tauri::command]
fn canonical_mcp_save_apply(
    input: my_agent_assets_core::mcp_management::McpSaveApplyRequest,
) -> CommandResult<my_agent_assets_core::mcp_management::McpSaveApplyResult> {
    command_result(shared_core::canonical_mcp_save_apply_command(input))
}

#[tauri::command]
fn canonical_unmount_preview(
    input: my_agent_assets_core::mount::UnmountPreviewRequest,
) -> CommandResult<my_agent_assets_core::mount::UnmountPreview> {
    command_result(shared_core::canonical_unmount_preview_command(input))
}

#[tauri::command]
fn canonical_unmount_apply(
    input: my_agent_assets_core::mount::UnmountApplyRequest,
) -> CommandResult<my_agent_assets_core::mount::UnmountApplyResult> {
    command_result(shared_core::canonical_unmount_apply_command(input))
}

#[tauri::command]
fn canonical_delete_preview(
    input: my_agent_assets_core::delete::DeletePreviewRequest,
) -> CommandResult<my_agent_assets_core::delete::DeletePreview> {
    command_result(shared_core::canonical_delete_preview_command(input))
}

#[tauri::command]
fn canonical_delete_apply(
    input: my_agent_assets_core::delete::DeleteApplyRequest,
) -> CommandResult<my_agent_assets_core::delete::DeleteApplyResult> {
    command_result(shared_core::canonical_delete_apply_command(input))
}

#[tauri::command]
fn preview_adopt(
    input: my_agent_assets_core::adopt::AdoptPreviewRequest,
) -> CommandResult<my_agent_assets_core::adopt::AdoptPreview> {
    command_result(shared_core::preview_adopt_command(input))
}

#[tauri::command]
fn adopt_apply(
    input: my_agent_assets_core::adopt::AdoptApplyRequest,
) -> CommandResult<my_agent_assets_core::adopt::AdoptApplyResult> {
    command_result(shared_core::adopt_apply_command(input))
}

#[tauri::command]
fn canonical_batch_import_preview(
    input: my_agent_assets_core::batch_import::BatchImportPreviewRequest,
) -> CommandResult<my_agent_assets_core::batch_import::BatchImportPreview> {
    command_result(shared_core::canonical_batch_import_preview_command(input))
}

#[tauri::command]
fn canonical_batch_import_apply(
    input: my_agent_assets_core::batch_import::BatchImportApplyRequest,
) -> CommandResult<my_agent_assets_core::batch_import::BatchImportApplyResult> {
    command_result(shared_core::canonical_batch_import_apply_command(input))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(error) = shared_core::startup_recovery_command() {
        eprintln!("[startup-recovery] {error}");
    }
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_info,
            settings_load,
            settings_save,
            git_status,
            recovery_status,
            list_audit_log,
            doctor_report,
            consistency_repair_preview,
            consistency_repair_apply,
            diagnostic_export_preview,
            diagnostic_export_apply,
            initialization_preview,
            initialization_apply,
            list_assets,
            canonical_asset_content,
            canonical_asset_open,
            list_projects,
            project_save_preview,
            project_save_apply,
            project_remove_preview,
            project_remove_apply,
            list_backups,
            reveal_backup_manifest,
            backup_delete_preview,
            backup_delete_apply,
            preview_sync,
            sync_apply,
            discover_runtime_sources,
            canonical_import_preview,
            canonical_import_apply,
            list_mount_targets,
            target_registration_preview,
            target_registration_apply,
            target_removal_preview,
            target_removal_apply,
            canonical_mount_preview,
            canonical_mount_apply,
            canonical_mcp_get,
            canonical_mcp_save_preview,
            canonical_mcp_save_apply,
            canonical_unmount_preview,
            canonical_unmount_apply,
            canonical_delete_preview,
            canonical_delete_apply,
            preview_adopt,
            adopt_apply,
            canonical_batch_import_preview,
            canonical_batch_import_apply
        ])
        .run(tauri::generate_context!())
        .expect("error while running My Agent Assets");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_is_read_only_platform_metadata() {
        let info = app_info();
        assert_eq!(info.name, "My Agent Assets");
        assert!(info.backend_ready);
    }
}

#[cfg(test)]
mod settings_tests;
