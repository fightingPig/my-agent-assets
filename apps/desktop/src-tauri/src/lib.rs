mod contracts;
mod path_utils;
mod settings;
mod shared_core;

use contracts::{
    AppInfo, AssetOpenResult, BackupRevealInput, BackupRevealResult, CanonicalAssetContentInput,
    CanonicalMcpGetInput, DesktopSettings, SettingsSaveInput,
};

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
fn settings_load() -> Result<DesktopSettings, String> {
    settings::settings_load_command()
}

#[tauri::command]
fn settings_save(input: SettingsSaveInput) -> Result<DesktopSettings, String> {
    settings::settings_save_command(input)
}

#[tauri::command]
fn git_status() -> Result<my_agent_assets_core::git_sync::GitStatus, String> {
    shared_core::git_status_command()
}

#[tauri::command]
fn recovery_status() -> Result<my_agent_assets_core::operation::RecoveryStatus, String> {
    shared_core::recovery_status_command()
}

#[tauri::command]
fn list_audit_log() -> Result<Vec<my_agent_assets_core::audit_log::AuditLogEntry>, String> {
    shared_core::list_audit_log_command()
}

#[tauri::command]
fn doctor_report() -> Result<my_agent_assets_core::diagnostics::DoctorReport, String> {
    shared_core::doctor_report_command()
}

#[tauri::command]
fn consistency_repair_preview(
    input: my_agent_assets_core::consistency_repair::ConsistencyRepairPreviewRequest,
) -> Result<my_agent_assets_core::consistency_repair::ConsistencyRepairPreview, String> {
    shared_core::consistency_repair_preview_command(input)
}

#[tauri::command]
fn consistency_repair_apply(
    input: my_agent_assets_core::consistency_repair::ConsistencyRepairApplyRequest,
) -> Result<my_agent_assets_core::consistency_repair::ConsistencyRepairApplyResult, String> {
    shared_core::consistency_repair_apply_command(input)
}

#[tauri::command]
fn diagnostic_export_preview(
) -> Result<my_agent_assets_core::diagnostic_export::DiagnosticExportPreview, String> {
    shared_core::diagnostic_export_preview_command()
}

#[tauri::command]
fn diagnostic_export_apply(
    input: my_agent_assets_core::diagnostic_export::DiagnosticExportApplyRequest,
) -> Result<my_agent_assets_core::diagnostic_export::DiagnosticExportApplyResult, String> {
    shared_core::diagnostic_export_apply_command(input)
}

#[tauri::command]
fn initialization_preview(
) -> Result<my_agent_assets_core::initialization::InitializationPreview, String> {
    shared_core::initialization_preview_command()
}

#[tauri::command]
fn initialization_apply(
    input: my_agent_assets_core::initialization::InitializationApplyRequest,
) -> Result<my_agent_assets_core::initialization::InitializationApplyResult, String> {
    shared_core::initialization_apply_command(input)
}

#[tauri::command]
fn list_assets(
    input: my_agent_assets_core::query::AssetQueryRequest,
) -> Result<Vec<my_agent_assets_core::query::AssetSummary>, String> {
    shared_core::list_assets_command(input)
}

#[tauri::command]
fn canonical_asset_content(
    input: CanonicalAssetContentInput,
) -> Result<my_agent_assets_core::asset_access::CanonicalAssetContent, String> {
    shared_core::canonical_asset_content_command(input.asset_id)
}

#[tauri::command]
fn canonical_asset_open(
    input: my_agent_assets_core::asset_access::AssetOpenRequest,
) -> Result<AssetOpenResult, String> {
    shared_core::canonical_asset_open_command(input)
}

#[tauri::command]
fn list_projects() -> Result<Vec<my_agent_assets_core::query::ProjectSummary>, String> {
    shared_core::list_projects_command()
}

#[tauri::command]
fn list_backups() -> Result<Vec<my_agent_assets_core::backup_history::BackupHistoryEntry>, String> {
    shared_core::list_backup_history_command()
}

#[tauri::command]
fn reveal_backup_manifest(input: BackupRevealInput) -> Result<BackupRevealResult, String> {
    shared_core::reveal_backup_manifest_command(input)
}

#[tauri::command]
fn backup_delete_preview(
    input: my_agent_assets_core::backup_delete::BackupDeletePreviewRequest,
) -> Result<my_agent_assets_core::backup_delete::BackupDeletePreview, String> {
    shared_core::backup_delete_preview_command(input)
}

#[tauri::command]
fn backup_delete_apply(
    input: my_agent_assets_core::backup_delete::BackupDeleteApplyRequest,
) -> Result<my_agent_assets_core::backup_delete::BackupDeleteApplyResult, String> {
    shared_core::backup_delete_apply_command(input)
}

#[tauri::command]
fn preview_sync(
    input: my_agent_assets_core::git_sync::SyncPreviewRequest,
) -> Result<my_agent_assets_core::git_sync::SyncPreview, String> {
    shared_core::sync_preview_command(input)
}

#[tauri::command]
fn sync_apply(
    input: my_agent_assets_core::git_sync::SyncApplyRequest,
) -> Result<my_agent_assets_core::git_sync::SyncApplyResult, String> {
    shared_core::sync_apply_command(input)
}

#[tauri::command]
fn discover_runtime_sources(
    input: my_agent_assets_core::discovery::DiscoveryScope,
) -> Result<my_agent_assets_core::discovery::DiscoveryResult, String> {
    shared_core::discover_runtime_sources_command(input)
}

#[tauri::command]
fn canonical_import_preview(
    input: my_agent_assets_core::import::ImportPreviewRequest,
) -> Result<my_agent_assets_core::import::ImportPreview, String> {
    shared_core::canonical_import_preview_command(input)
}

#[tauri::command]
fn canonical_import_apply(
    input: my_agent_assets_core::import::ImportApplyRequest,
) -> Result<my_agent_assets_core::import::ImportApplyResult, String> {
    shared_core::canonical_import_apply_command(input)
}

#[tauri::command]
fn list_mount_targets() -> Result<Vec<my_agent_assets_core::targets::MountTarget>, String> {
    shared_core::list_mount_targets_command()
}

#[tauri::command]
fn target_registration_preview(
    input: my_agent_assets_core::target_management::TargetRegistrationPreviewRequest,
) -> Result<my_agent_assets_core::target_management::TargetChangePreview, String> {
    shared_core::target_registration_preview_command(input)
}

#[tauri::command]
fn target_registration_apply(
    input: my_agent_assets_core::target_management::TargetRegistrationApplyRequest,
) -> Result<my_agent_assets_core::target_management::TargetChangeResult, String> {
    shared_core::target_registration_apply_command(input)
}

#[tauri::command]
fn target_removal_preview(
    input: my_agent_assets_core::target_management::TargetRemovePreviewRequest,
) -> Result<my_agent_assets_core::target_management::TargetChangePreview, String> {
    shared_core::target_removal_preview_command(input)
}

#[tauri::command]
fn target_removal_apply(
    input: my_agent_assets_core::target_management::TargetRemoveApplyRequest,
) -> Result<my_agent_assets_core::target_management::TargetChangeResult, String> {
    shared_core::target_removal_apply_command(input)
}

#[tauri::command]
fn canonical_mount_preview(
    input: my_agent_assets_core::mount::MountPreviewRequest,
) -> Result<my_agent_assets_core::mount::MountPreview, String> {
    shared_core::canonical_mount_preview_command(input)
}

#[tauri::command]
fn canonical_mount_apply(
    input: my_agent_assets_core::mount::MountApplyRequest,
) -> Result<my_agent_assets_core::mount::MountApplyResult, String> {
    shared_core::canonical_mount_apply_command(input)
}

#[tauri::command]
fn canonical_mcp_save_preview(
    input: my_agent_assets_core::mcp_management::McpSavePreviewRequest,
) -> Result<my_agent_assets_core::mcp_management::McpSavePreview, String> {
    shared_core::canonical_mcp_save_preview_command(input)
}

#[tauri::command]
fn canonical_mcp_get(
    input: CanonicalMcpGetInput,
) -> Result<my_agent_assets_core::mcp_management::McpAssetDefinition, String> {
    shared_core::canonical_mcp_get_command(input.asset_id)
}

#[tauri::command]
fn canonical_mcp_save_apply(
    input: my_agent_assets_core::mcp_management::McpSaveApplyRequest,
) -> Result<my_agent_assets_core::mcp_management::McpSaveApplyResult, String> {
    shared_core::canonical_mcp_save_apply_command(input)
}

#[tauri::command]
fn canonical_unmount_preview(
    input: my_agent_assets_core::mount::UnmountPreviewRequest,
) -> Result<my_agent_assets_core::mount::UnmountPreview, String> {
    shared_core::canonical_unmount_preview_command(input)
}

#[tauri::command]
fn canonical_unmount_apply(
    input: my_agent_assets_core::mount::UnmountApplyRequest,
) -> Result<my_agent_assets_core::mount::UnmountApplyResult, String> {
    shared_core::canonical_unmount_apply_command(input)
}

#[tauri::command]
fn canonical_delete_preview(
    input: my_agent_assets_core::delete::DeletePreviewRequest,
) -> Result<my_agent_assets_core::delete::DeletePreview, String> {
    shared_core::canonical_delete_preview_command(input)
}

#[tauri::command]
fn canonical_delete_apply(
    input: my_agent_assets_core::delete::DeleteApplyRequest,
) -> Result<my_agent_assets_core::delete::DeleteApplyResult, String> {
    shared_core::canonical_delete_apply_command(input)
}

#[tauri::command]
fn preview_adopt(
    input: my_agent_assets_core::adopt::AdoptPreviewRequest,
) -> Result<my_agent_assets_core::adopt::AdoptPreview, String> {
    shared_core::preview_adopt_command(input)
}

#[tauri::command]
fn adopt_apply(
    input: my_agent_assets_core::adopt::AdoptApplyRequest,
) -> Result<my_agent_assets_core::adopt::AdoptApplyResult, String> {
    shared_core::adopt_apply_command(input)
}

#[tauri::command]
fn canonical_batch_import_preview(
    input: my_agent_assets_core::batch_import::BatchImportPreviewRequest,
) -> Result<my_agent_assets_core::batch_import::BatchImportPreview, String> {
    shared_core::canonical_batch_import_preview_command(input)
}

#[tauri::command]
fn canonical_batch_import_apply(
    input: my_agent_assets_core::batch_import::BatchImportApplyRequest,
) -> Result<my_agent_assets_core::batch_import::BatchImportApplyResult, String> {
    shared_core::canonical_batch_import_apply_command(input)
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
