mod apply;
mod codex;
mod contracts;
mod path_utils;
mod preview;
mod read_only;
mod settings;
mod shared_core;
#[cfg(test)]
mod sync_apply;

use contracts::{
    AppInfo, ApplyResult, AssetSummary, CodexDiscoveryInput, CodexMcpListResult,
    CodexSkillListResult, ConflictApplyInput, ConflictPreview, DesktopSettings, ImportApplyInput,
    ImportPreview, ListAssetsInput, MountApplyInput, MountPreview, PreviewConflictsInput,
    PreviewImportInput, PreviewMountInput, ProjectSummary, ScanAssetsInput, ScanResult,
    SettingsSaveInput,
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
fn list_assets(input: ListAssetsInput) -> Vec<AssetSummary> {
    read_only::list_assets_command(input)
}

#[tauri::command]
fn list_projects() -> Vec<ProjectSummary> {
    read_only::list_projects_command()
}

#[tauri::command]
fn list_backups() -> Result<Vec<my_agent_assets_core::backup_history::BackupHistoryEntry>, String> {
    shared_core::list_backup_history_command()
}

#[tauri::command]
fn list_codex_skills(input: CodexDiscoveryInput) -> CodexSkillListResult {
    codex::list_codex_skills_command(input)
}

#[tauri::command]
fn list_codex_mcp_servers(input: CodexDiscoveryInput) -> CodexMcpListResult {
    codex::list_codex_mcp_servers_command(input)
}

#[tauri::command]
fn scan_assets(input: ScanAssetsInput) -> ScanResult {
    read_only::scan_assets_command(input)
}

#[tauri::command]
fn preview_import(input: PreviewImportInput) -> ImportPreview {
    preview::preview_import_command(input)
}

#[tauri::command]
fn preview_mount(input: PreviewMountInput) -> MountPreview {
    preview::preview_mount_command(input)
}

#[tauri::command]
fn preview_conflicts(input: PreviewConflictsInput) -> Vec<ConflictPreview> {
    preview::preview_conflicts_command(input)
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
fn import_apply(input: ImportApplyInput) -> ApplyResult {
    apply::import_apply_command(input)
}

#[tauri::command]
fn conflict_apply(input: ConflictApplyInput) -> ApplyResult {
    apply::conflict_apply_command(input)
}

#[tauri::command]
fn mount_apply(input: MountApplyInput) -> ApplyResult {
    apply::mount_apply_command(input)
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
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_info,
            settings_load,
            settings_save,
            git_status,
            list_assets,
            list_projects,
            list_backups,
            list_codex_skills,
            list_codex_mcp_servers,
            scan_assets,
            preview_import,
            preview_mount,
            preview_conflicts,
            preview_sync,
            sync_apply,
            import_apply,
            conflict_apply,
            mount_apply,
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
mod read_only_tests;

#[cfg(test)]
mod preview_tests;

#[cfg(test)]
mod apply_tests;

#[cfg(test)]
mod conflict_apply_tests;

#[cfg(test)]
mod settings_tests;

#[cfg(test)]
mod write_safety_e2e_tests;

#[cfg(test)]
mod codex_tests;
