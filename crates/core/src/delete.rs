use crate::asset_registry::{
    canonical_path, load as load_assets, parse_asset_id, registry_path as asset_registry_path,
    save as save_assets,
};
use crate::mount::{
    copy_any, discard_runtime_snapshot, guard_target_path, preview_unmount, remove_path_if_present,
    remove_runtime_mount, restore_runtime_snapshot, snapshot_runtime_path, RuntimeSnapshot,
    UnmountPreviewRequest,
};
use crate::mount_registry::{
    load as load_mounts, registry_path as mount_registry_path, save as save_mounts,
};
use crate::operation::{OperationJournal, OperationLock, RecoveryTarget};
use crate::path_safety::guard_write_path;
use crate::targets::load as load_targets;
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PREVIEW_TTL_SECONDS: u64 = 300;
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteMode {
    #[serde(rename = "require_unmounted")]
    RequireUnmounted,
    #[serde(rename = "unmount_all")]
    UnmountAll,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePreviewRequest {
    pub asset_id: String,
    pub mode: DeleteMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteBindingImpact {
    pub target_id: String,
    pub target_path: PathBuf,
    pub can_unmount: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePreview {
    pub preview_id: String,
    pub asset_id: String,
    pub canonical_path: PathBuf,
    pub bindings: Vec<DeleteBindingImpact>,
    pub planned_effects: Vec<String>,
    pub warnings: Vec<String>,
    pub backup_required: bool,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: DeletePreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteApplyResult {
    pub preview_id: String,
    pub asset_id: String,
    pub deleted: bool,
    pub portable_backup_id: String,
    pub local_backup_id: String,
    pub affected_paths: Vec<PathBuf>,
    pub journal_path: PathBuf,
}

pub fn preview_delete(home: &Path, request: &DeletePreviewRequest) -> Result<DeletePreview> {
    preview_delete_at(home, request, epoch_seconds())
}

fn preview_delete_at(
    home: &Path,
    request: &DeletePreviewRequest,
    generated_at_epoch_seconds: u64,
) -> Result<DeletePreview> {
    let (kind, name) =
        parse_asset_id(&request.asset_id).map_err(|error| MaaError::new(error.to_string()))?;
    let assets = load_assets(home).map_err(|error| MaaError::new(error.to_string()))?;
    if assets.get(kind, &name).is_none() {
        return Err(MaaError::new(format!(
            "asset '{}' is not registered",
            request.asset_id
        )));
    }
    let canonical = canonical_path(home, kind, &name);
    if !canonical.exists() {
        return Err(MaaError::new(format!(
            "canonical content is missing: {}",
            canonical.display()
        )));
    }
    let mounts = load_mounts(home).map_err(|error| MaaError::new(error.to_string()))?;
    let mut impacts = Vec::new();
    for binding in mounts.for_asset(&request.asset_id) {
        let unmount = preview_unmount(
            home,
            &UnmountPreviewRequest {
                asset_id: request.asset_id.clone(),
                target_id: binding.target_id.clone(),
            },
        )?;
        impacts.push(DeleteBindingImpact {
            target_id: binding.target_id.clone(),
            target_path: unmount.affected_target_path,
            can_unmount: unmount.can_apply,
            warnings: unmount.warnings,
        });
    }
    impacts.sort_by(|left, right| left.target_id.cmp(&right.target_id));

    let mut warnings = Vec::new();
    let can_apply = match request.mode {
        DeleteMode::RequireUnmounted if !impacts.is_empty() => {
            warnings.push(format!(
                "{} active binding(s) must be unmounted before direct deletion",
                impacts.len()
            ));
            false
        }
        DeleteMode::UnmountAll => impacts.iter().all(|impact| impact.can_unmount),
        DeleteMode::RequireUnmounted => true,
    };
    for impact in &impacts {
        warnings.extend(impact.warnings.clone());
    }
    let mut planned_effects = impacts
        .iter()
        .map(|impact| {
            format!(
                "unmount '{}' at {}",
                impact.target_id,
                impact.target_path.display()
            )
        })
        .collect::<Vec<_>>();
    planned_effects.push(format!("delete canonical content {}", canonical.display()));
    planned_effects.push("remove the canonical assets.yaml record".into());
    let preview_id = delete_fingerprint(
        home,
        request,
        &canonical,
        &impacts,
        generated_at_epoch_seconds,
    )?;
    Ok(DeletePreview {
        preview_id,
        asset_id: request.asset_id.clone(),
        canonical_path: canonical,
        bindings: impacts,
        planned_effects,
        warnings,
        backup_required: true,
        can_apply,
        generated_at_epoch_seconds,
        expires_at_epoch_seconds: generated_at_epoch_seconds.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

pub fn apply_delete(home: &Path, request: &DeleteApplyRequest) -> Result<DeleteApplyResult> {
    apply_delete_inner(home, request, None)
}

fn apply_delete_inner(
    home: &Path,
    request: &DeleteApplyRequest,
    fail_after_unmounts: Option<usize>,
) -> Result<DeleteApplyResult> {
    let _operation_lock = OperationLock::acquire(home)?;
    if epoch_seconds()
        > request
            .preview_generated_at_epoch_seconds
            .saturating_add(PREVIEW_TTL_SECONDS)
    {
        return Err(MaaError::new(
            "delete preview expired; generate a new preview before applying",
        ));
    }
    let preview = preview_delete_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    if preview.preview_id != request.preview_id {
        return Err(MaaError::new(
            "delete preview is stale; generate a new preview before applying",
        ));
    }
    if !preview.can_apply {
        return Err(MaaError::new(
            preview
                .warnings
                .first()
                .cloned()
                .unwrap_or_else(|| "delete is blocked".into()),
        ));
    }

    let (kind, name) = parse_asset_id(&request.request.asset_id)
        .map_err(|error| MaaError::new(error.to_string()))?;
    let operation_id = operation_id();
    let root = home.join(".my-agent-assets");
    let staging = guard_write_path(
        &root,
        &root
            .join("operations")
            .join(format!("{operation_id}-canonical")),
    )?;
    let mut recovery_targets = vec![
        RecoveryTarget::asset_center(asset_registry_path(home)),
        RecoveryTarget::asset_center(mount_registry_path(home)),
        RecoveryTarget::asset_center(preview.canonical_path.clone()),
        RecoveryTarget::asset_center(staging.clone()),
    ];
    recovery_targets.extend(preview.bindings.iter().map(|impact| {
        RecoveryTarget::registered_target(impact.target_id.clone(), impact.target_path.clone())
    }));
    let mut journal =
        OperationJournal::start_recoverable(home, &operation_id, "delete_asset", recovery_targets)?;
    let original_assets = fs::read(asset_registry_path(home))?;
    let original_mounts = fs::read(mount_registry_path(home))?;
    let portable_backup_id = create_portable_backup(
        home,
        &operation_id,
        &preview.canonical_path,
        &original_assets,
    )?;
    let local_backup_id =
        create_local_backup(home, &operation_id, &preview.bindings, &original_mounts)?;
    journal.record_step("backups_created")?;

    let targets = load_targets(home)?;
    let mut snapshots = BTreeMap::<PathBuf, RuntimeSnapshot>::new();
    for impact in &preview.bindings {
        let target = targets.resolve(&impact.target_id)?;
        guard_target_path(target, &impact.target_path)?;
        if !snapshots.contains_key(&impact.target_path) {
            snapshots.insert(
                impact.target_path.clone(),
                snapshot_runtime_path(&impact.target_path)?,
            );
        }
    }

    let mut canonical_moved = false;
    let result = (|| -> Result<(Vec<PathBuf>, crate::asset_registry::AssetRegistry, crate::mount_registry::MountRegistry)> {
        let mut affected = Vec::new();
        for (index, impact) in preview.bindings.iter().enumerate() {
            let target = targets.resolve(&impact.target_id)?;
            remove_runtime_mount(&impact.target_path, target, kind, &name)?;
            affected.push(impact.target_path.clone());
            journal.record_step(format!("unmounted:{}", impact.target_id))?;
            if fail_after_unmounts.is_some_and(|count| index + 1 >= count) {
                return Err(MaaError::new("injected delete failure"));
            }
        }

        fs::rename(&preview.canonical_path, &staging)?;
        canonical_moved = true;
        journal.record_step("canonical_staged_for_delete")?;

        let mut assets = load_assets(home).map_err(|error| MaaError::new(error.to_string()))?;
        assets.remove(kind, &name);
        save_assets(home, &assets).map_err(|error| MaaError::new(error.to_string()))?;
        let mut mounts = load_mounts(home).map_err(|error| MaaError::new(error.to_string()))?;
        mounts.remove_asset(&request.request.asset_id);
        save_mounts(home, &mounts).map_err(|error| MaaError::new(error.to_string()))?;
        journal.record_step("registries_updated")?;
        Ok((affected, assets, mounts))
    })();

    let affected = match result {
        Ok((affected, _, _)) => affected,
        Err(error) => {
            let mut rollback_errors = Vec::new();
            if canonical_moved && staging.exists() {
                if let Err(rollback) = fs::rename(&staging, &preview.canonical_path) {
                    rollback_errors.push(format!("canonical restore failed: {rollback}"));
                }
            }
            if let Err(rollback) = fs::write(asset_registry_path(home), &original_assets) {
                rollback_errors.push(format!("assets.yaml restore failed: {rollback}"));
            }
            if let Err(rollback) = fs::write(mount_registry_path(home), &original_mounts) {
                rollback_errors.push(format!("mounts.yaml restore failed: {rollback}"));
            }
            for (path, snapshot) in snapshots {
                if let Err(rollback) = restore_runtime_snapshot(&path, snapshot) {
                    rollback_errors.push(format!("{} restore failed: {rollback}", path.display()));
                }
            }
            match journal.rollback_now(home) {
                Ok(_) => return Err(error),
                Err(persistent_error) => {
                    rollback_errors.push(format!("persistent recovery failed: {persistent_error}"));
                    let message = format!(
                        "{error}; automatic rollback incomplete: {}",
                        rollback_errors.join("; ")
                    );
                    let _ = journal.mark_rollback_required(&message);
                    return Err(MaaError::new(message));
                }
            }
        }
    };

    remove_path_if_present(&staging)?;
    for snapshot in snapshots.into_values() {
        discard_runtime_snapshot(snapshot)?;
    }
    journal.record_step("canonical_deleted")?;
    journal.complete()?;
    let mut affected_paths = affected;
    affected_paths.extend([
        preview.canonical_path,
        asset_registry_path(home),
        mount_registry_path(home),
    ]);
    Ok(DeleteApplyResult {
        preview_id: preview.preview_id,
        asset_id: request.request.asset_id.clone(),
        deleted: true,
        portable_backup_id,
        local_backup_id,
        affected_paths,
        journal_path: journal.path().to_path_buf(),
    })
}

fn create_portable_backup(
    home: &Path,
    operation_id: &str,
    canonical: &Path,
    assets: &[u8],
) -> Result<String> {
    let root = home.join(".my-agent-assets");
    let id = format!("delete-{operation_id}");
    let backup = guard_write_path(&root, &root.join("backups/portable").join(&id))?;
    fs::create_dir_all(&backup)?;
    copy_any(canonical, &backup.join("content"))?;
    fs::write(backup.join("assets.yaml"), assets)?;
    fs::write(
        backup.join("manifest.yaml"),
        format!(
            "schemaVersion: 1\noperation: delete\ncanonicalPath: {}\n",
            canonical
                .strip_prefix(&root)
                .map_err(|_| MaaError::new("canonical path escaped asset center"))?
                .display()
        ),
    )?;
    Ok(id)
}

fn create_local_backup(
    home: &Path,
    operation_id: &str,
    impacts: &[DeleteBindingImpact],
    mounts: &[u8],
) -> Result<String> {
    let root = home.join(".my-agent-assets");
    let id = format!("delete-{operation_id}");
    let backup = guard_write_path(&root, &root.join("backups/local").join(&id))?;
    fs::create_dir_all(&backup)?;
    for (index, impact) in impacts.iter().enumerate() {
        if fs::symlink_metadata(&impact.target_path).is_ok() {
            copy_any(
                &impact.target_path,
                &backup.join(format!("runtime-{index}")),
            )?;
        }
    }
    fs::write(backup.join("mounts.yaml"), mounts)?;
    let paths = impacts
        .iter()
        .map(|impact| format!("  - {}", impact.target_path.display()))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        backup.join("manifest.yaml"),
        format!("schemaVersion: 1\noperation: delete\nruntimePaths:\n{paths}\n"),
    )?;
    Ok(id)
}

fn delete_fingerprint(
    home: &Path,
    request: &DeletePreviewRequest,
    canonical: &Path,
    impacts: &[DeleteBindingImpact],
    generated_at: u64,
) -> Result<String> {
    let mut hash = Fnv64::new();
    hash.write(
        serde_json::to_string(request)
            .map_err(|error| MaaError::new(error.to_string()))?
            .as_bytes(),
    );
    hash.write(&generated_at.to_le_bytes());
    fingerprint_path(canonical, &mut hash)?;
    for impact in impacts {
        if fs::symlink_metadata(&impact.target_path).is_ok() {
            fingerprint_path(&impact.target_path, &mut hash)?;
        }
    }
    for path in [
        asset_registry_path(home),
        mount_registry_path(home),
        crate::targets::registry_path(home),
    ] {
        fingerprint_path(&path, &mut hash)?;
    }
    Ok(format!("delete-{:016x}", hash.finish()))
}

fn fingerprint_path(path: &Path, hash: &mut Fnv64) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    hash.write(path.to_string_lossy().as_bytes());
    if metadata.file_type().is_symlink() {
        hash.write(fs::read_link(path)?.to_string_lossy().as_bytes());
    } else if metadata.is_dir() {
        let mut entries = fs::read_dir(path)?.collect::<std::result::Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            fingerprint_path(&entry.path(), hash)?;
        }
    } else {
        let mut file = fs::File::open(path)?;
        let mut buffer = [0_u8; 8192];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hash.write(&buffer[..read]);
        }
    }
    Ok(())
}

fn operation_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos}-{}-{counter}", std::process::id())
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

struct Fnv64(u64);

impl Fnv64 {
    fn new() -> Self {
        Self(0xcbf29ce484222325)
    }
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
    fn finish(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_registry::{save as save_assets, AssetRecord, AssetRegistry};
    use crate::mount::{apply_mount, preview_mount, MountApplyRequest, MountPreviewRequest};
    use crate::mount_registry::{save as save_mounts, MountRegistry};
    use crate::targets::{
        save as save_targets, AssetKind, MountAdapter, ProviderState, TargetRegistry,
    };

    #[cfg(unix)]
    #[test]
    fn direct_delete_is_blocked_with_bindings_and_unmount_all_succeeds() {
        let home = initialized_home("success");
        register_skill(&home, "review");
        mount_skill(&home, "review", "claude-user-skills");
        mount_skill(&home, "review", "codex-user-skills");

        let direct = preview_delete(
            &home,
            &DeletePreviewRequest {
                asset_id: "skill:review".into(),
                mode: DeleteMode::RequireUnmounted,
            },
        )
        .unwrap();
        assert!(!direct.can_apply);
        assert_eq!(direct.bindings.len(), 2);

        let request = DeletePreviewRequest {
            asset_id: "skill:review".into(),
            mode: DeleteMode::UnmountAll,
        };
        let preview = preview_delete(&home, &request).unwrap();
        let result = apply_delete(
            &home,
            &DeleteApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap();
        assert!(result.deleted);
        assert!(!canonical_path(&home, AssetKind::Skill, "review").exists());
        assert!(!home.join(".claude/skills/review").exists());
        assert!(!home.join(".agents/skills/review").exists());
        assert!(load_mounts(&home)
            .unwrap()
            .for_asset("skill:review")
            .is_empty());
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(unix)]
    #[test]
    fn injected_mid_delete_failure_rolls_back_every_target_and_registry() {
        let home = initialized_home("rollback");
        register_skill(&home, "review");
        mount_skill(&home, "review", "claude-user-skills");
        mount_skill(&home, "review", "codex-user-skills");
        let assets_before = fs::read(asset_registry_path(&home)).unwrap();
        let mounts_before = fs::read(mount_registry_path(&home)).unwrap();
        let request = DeletePreviewRequest {
            asset_id: "skill:review".into(),
            mode: DeleteMode::UnmountAll,
        };
        let preview = preview_delete(&home, &request).unwrap();
        let error = apply_delete_inner(
            &home,
            &DeleteApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
            Some(1),
        )
        .unwrap_err();
        assert!(error.to_string().contains("injected"));
        assert!(canonical_path(&home, AssetKind::Skill, "review").exists());
        assert!(home.join(".claude/skills/review").exists());
        assert!(home.join(".agents/skills/review").exists());
        assert_eq!(fs::read(asset_registry_path(&home)).unwrap(), assets_before);
        assert_eq!(fs::read(mount_registry_path(&home)).unwrap(), mounts_before);
        let _ = fs::remove_dir_all(home);
    }

    fn initialized_home(name: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!(
            "maa-delete-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = home.join(".my-agent-assets");
        for path in [
            root.join("assets/skills"),
            root.join("assets/commands"),
            root.join("assets/mcps"),
            root.join("backups/portable"),
            root.join("backups/local"),
        ] {
            fs::create_dir_all(path).unwrap();
        }
        save_assets(&home, &AssetRegistry::default()).unwrap();
        save_mounts(&home, &MountRegistry::default()).unwrap();
        save_targets(
            &home,
            &TargetRegistry::standard_user_targets(
                &home,
                ProviderState::Initialized,
                ProviderState::Initialized,
                MountAdapter::SymlinkDirectory,
            )
            .unwrap(),
        )
        .unwrap();
        home
    }

    fn register_skill(home: &Path, name: &str) {
        let mut assets = load_assets(home).unwrap();
        assets
            .upsert(AssetRecord::new(AssetKind::Skill, name).unwrap())
            .unwrap();
        save_assets(home, &assets).unwrap();
        let path = canonical_path(home, AssetKind::Skill, name);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("SKILL.md"), "# Review").unwrap();
    }

    fn mount_skill(home: &Path, name: &str, target_id: &str) {
        let request = MountPreviewRequest {
            asset_id: format!("skill:{name}"),
            target_id: target_id.into(),
        };
        let preview = preview_mount(home, &request).unwrap();
        apply_mount(
            home,
            &MountApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap();
    }
}
