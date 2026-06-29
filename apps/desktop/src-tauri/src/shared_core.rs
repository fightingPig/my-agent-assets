use crate::path_utils::home_dir;
use my_agent_assets_core::discovery::{discover, DiscoveryResult, DiscoveryScope};
use my_agent_assets_core::import::{
    apply_import, preview_import, ImportApplyRequest, ImportApplyResult, ImportPreview,
    ImportPreviewRequest,
};
use my_agent_assets_core::mount::{
    apply_mount, apply_unmount, preview_mount, preview_unmount, MountApplyRequest,
    MountApplyResult, MountPreview, MountPreviewRequest, UnmountApplyRequest, UnmountApplyResult,
    UnmountPreview, UnmountPreviewRequest,
};
use my_agent_assets_core::targets::{load as load_targets, MountTarget};

pub fn discover_runtime_sources_command(scope: DiscoveryScope) -> Result<DiscoveryResult, String> {
    let home = home_dir().ok_or_else(|| "HOME is unavailable; discovery skipped.".to_string())?;
    Ok(discover(&home, scope))
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

pub fn canonical_mount_preview_command(input: MountPreviewRequest) -> Result<MountPreview, String> {
    let home =
        home_dir().ok_or_else(|| "HOME is unavailable; mount preview skipped.".to_string())?;
    preview_mount(&home, &input).map_err(|error| error.to_string())
}

pub fn canonical_mount_apply_command(input: MountApplyRequest) -> Result<MountApplyResult, String> {
    let home = home_dir().ok_or_else(|| "HOME is unavailable; mount apply blocked.".to_string())?;
    apply_mount(&home, &input).map_err(|error| error.to_string())
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
        let _ = fs::remove_dir_all(home);
    }

    fn initialize(home: &Path) {
        let root = home.join(".my-agent-assets");
        fs::create_dir_all(root.join("assets/skills")).unwrap();
        fs::create_dir_all(root.join("assets/commands")).unwrap();
        fs::create_dir_all(root.join("assets/mcps")).unwrap();
        fs::create_dir_all(root.join("backups/portable")).unwrap();
        fs::write(root.join("assets.yaml"), "schemaVersion: 1\nassets: {}\n").unwrap();
        fs::write(root.join("mounts.yaml"), "schemaVersion: 1\nbindings: {}\n").unwrap();
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
        path
    }
}
