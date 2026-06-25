mod apply;
mod contracts;
mod path_utils;
mod preview;
mod read_only;

use contracts::{
    AppInfo, ApplyResult, AssetSummary, ConflictPreview, DesktopSettings, GitStatus,
    ImportApplyInput, ImportPreview, ListAssetsInput, MountApplyInput, MountPreview,
    PreviewConflictsInput, PreviewImportInput, PreviewMountInput, PreviewRestoreInput,
    ProjectSummary, RestoreApplyInput, RestorePreview, ScanAssetsInput, ScanResult,
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
fn settings_load() -> DesktopSettings {
    read_only::settings_load_command()
}

#[tauri::command]
fn git_status() -> GitStatus {
    read_only::git_status_command()
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
fn preview_restore(input: PreviewRestoreInput) -> RestorePreview {
    preview::preview_restore_command(input)
}

#[tauri::command]
fn import_apply(input: ImportApplyInput) -> ApplyResult {
    apply::import_apply_command(input)
}

#[tauri::command]
fn mount_apply(input: MountApplyInput) -> ApplyResult {
    apply::mount_apply_command(input)
}

#[tauri::command]
fn restore_apply(input: RestoreApplyInput) -> ApplyResult {
    apply::restore_apply_command(input)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_info,
            settings_load,
            git_status,
            list_assets,
            list_projects,
            scan_assets,
            preview_import,
            preview_mount,
            preview_conflicts,
            preview_restore,
            import_apply,
            mount_apply,
            restore_apply
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
