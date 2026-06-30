use crate::asset_registry::registry_path as asset_registry_path;
use crate::import::{
    apply_import_locked, preview_import_at, ImportApplyRequest, ImportApplyResult, ImportPreview,
    ImportPreviewRequest, ImportResolution,
};
use crate::mount::{
    discard_runtime_snapshot, restore_runtime_snapshot, snapshot_runtime_path, RuntimeSnapshot,
};
use crate::mount_registry::registry_path as mount_registry_path;
use crate::operation::{OperationJournal, OperationLock, RecoveryTarget};
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PREVIEW_TTL_SECONDS: u64 = 300;
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImportSelection {
    pub source_id: String,
    pub resolution: ImportResolution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImportPreviewRequest {
    pub scope: crate::discovery::DiscoveryScope,
    pub selections: Vec<BatchImportSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImportPreview {
    pub preview_id: String,
    pub items: Vec<ImportPreview>,
    pub warnings: Vec<String>,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImportApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: BatchImportPreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImportApplyResult {
    pub preview_id: String,
    pub items: Vec<ImportApplyResult>,
    pub affected_paths: Vec<PathBuf>,
    pub journal_path: PathBuf,
}

pub fn preview_batch_import(
    home: &Path,
    request: &BatchImportPreviewRequest,
) -> Result<BatchImportPreview> {
    preview_batch_import_at(home, request, epoch_seconds())
}

fn preview_batch_import_at(
    home: &Path,
    request: &BatchImportPreviewRequest,
    generated_at: u64,
) -> Result<BatchImportPreview> {
    if request.selections.is_empty() {
        return Err(MaaError::new(
            "batch import requires at least one selected source",
        ));
    }
    let mut source_ids = BTreeSet::new();
    let mut destination_ids = BTreeSet::new();
    let mut items = Vec::new();
    let mut warnings = Vec::new();
    for selection in &request.selections {
        if !source_ids.insert(selection.source_id.clone()) {
            return Err(MaaError::new(format!(
                "duplicate import source id: {}",
                selection.source_id
            )));
        }
        let item = preview_import_at(
            home,
            &ImportPreviewRequest {
                scope: request.scope.clone(),
                source_id: selection.source_id.clone(),
                resolution: selection.resolution.clone(),
            },
            generated_at,
        )?;
        if selection.resolution != ImportResolution::Skip
            && !destination_ids.insert(item.asset_id.clone())
        {
            return Err(MaaError::new(format!(
                "multiple sources resolve to canonical asset '{}'",
                item.asset_id
            )));
        }
        warnings.extend(item.warnings.clone());
        items.push(item);
    }
    let can_apply = items.iter().all(|item| {
        item.can_apply || item.disposition == crate::import::ImportDisposition::Unchanged
    });
    let preview_id = batch_fingerprint(request, &items, generated_at)?;
    Ok(BatchImportPreview {
        preview_id,
        items,
        warnings,
        can_apply,
        generated_at_epoch_seconds: generated_at,
        expires_at_epoch_seconds: generated_at.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

pub fn apply_batch_import(
    home: &Path,
    request: &BatchImportApplyRequest,
) -> Result<BatchImportApplyResult> {
    apply_batch_import_inner(home, request, None)
}

fn apply_batch_import_inner(
    home: &Path,
    request: &BatchImportApplyRequest,
    fail_after_items: Option<usize>,
) -> Result<BatchImportApplyResult> {
    let _operation_lock = OperationLock::acquire(home)?;
    if epoch_seconds()
        > request
            .preview_generated_at_epoch_seconds
            .saturating_add(PREVIEW_TTL_SECONDS)
    {
        return Err(MaaError::new(
            "batch import preview expired; generate a new preview before applying",
        ));
    }
    let preview = preview_batch_import_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    if preview.preview_id != request.preview_id {
        return Err(MaaError::new(
            "batch import preview is stale; generate a new preview before applying",
        ));
    }
    if !preview.can_apply {
        return Err(MaaError::new("batch import contains unresolved conflicts"));
    }

    let operation_id = operation_id();
    let mut recovery_targets = vec![
        RecoveryTarget::asset_center(asset_registry_path(home)),
        RecoveryTarget::asset_center(mount_registry_path(home)),
    ];
    recovery_targets.extend(
        preview
            .items
            .iter()
            .map(|item| RecoveryTarget::asset_center(item.destination_path.clone())),
    );
    let mut journal =
        OperationJournal::start_recoverable(home, &operation_id, "batch_import", recovery_targets)?;
    let assets_before = fs::read(asset_registry_path(home))?;
    let mounts_before = fs::read(mount_registry_path(home))?;
    let mut snapshots = BTreeMap::<PathBuf, RuntimeSnapshot>::new();
    for item in &preview.items {
        snapshots
            .entry(item.destination_path.clone())
            .or_insert(snapshot_runtime_path(&item.destination_path)?);
    }
    journal.record_step("rollback_snapshots_created")?;

    let result = (|| -> Result<(Vec<ImportApplyResult>, Vec<PathBuf>)> {
        let mut results = Vec::new();
        let mut affected = Vec::new();
        for (index, selection) in request.request.selections.iter().enumerate() {
            let import_request = ImportPreviewRequest {
                scope: request.request.scope.clone(),
                source_id: selection.source_id.clone(),
                resolution: selection.resolution.clone(),
            };
            let current = preview_import_at(
                home,
                &import_request,
                request.preview_generated_at_epoch_seconds,
            )?;
            let applied = apply_import_locked(
                home,
                &ImportApplyRequest {
                    preview_id: current.preview_id,
                    preview_generated_at_epoch_seconds: request.preview_generated_at_epoch_seconds,
                    request: import_request,
                },
            )?;
            journal.record_step(format!("imported:{}", applied.asset_id))?;
            affected.extend(applied.affected_paths.clone());
            results.push(applied);
            if fail_after_items.is_some_and(|count| index + 1 >= count) {
                return Err(MaaError::new("injected batch import failure"));
            }
        }
        Ok((results, affected))
    })();

    let (items, affected_paths) = match result {
        Ok(result) => result,
        Err(error) => {
            let mut rollback_errors = Vec::new();
            for (path, snapshot) in snapshots {
                if let Err(rollback) = restore_runtime_snapshot(&path, snapshot) {
                    rollback_errors.push(format!("{} restore failed: {rollback}", path.display()));
                }
            }
            if let Err(rollback) = fs::write(asset_registry_path(home), &assets_before) {
                rollback_errors.push(format!("assets.yaml restore failed: {rollback}"));
            }
            if let Err(rollback) = fs::write(mount_registry_path(home), &mounts_before) {
                rollback_errors.push(format!("mounts.yaml restore failed: {rollback}"));
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
    for snapshot in snapshots.into_values() {
        discard_runtime_snapshot(snapshot)?;
    }
    journal.complete()?;
    Ok(BatchImportApplyResult {
        preview_id: preview.preview_id,
        items,
        affected_paths,
        journal_path: journal.path().to_path_buf(),
    })
}

fn batch_fingerprint(
    request: &BatchImportPreviewRequest,
    items: &[ImportPreview],
    generated_at: u64,
) -> Result<String> {
    let mut hash = Fnv64::new();
    hash.write(
        serde_json::to_string(request)
            .map_err(|error| MaaError::new(error.to_string()))?
            .as_bytes(),
    );
    hash.write(&generated_at.to_le_bytes());
    for item in items {
        hash.write(item.preview_id.as_bytes());
    }
    Ok(format!("batch-import-{:016x}", hash.finish()))
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
    use crate::asset_registry::{load as load_assets, save as save_assets, AssetRegistry};
    use crate::discovery::{discover, DiscoveryScope};
    use crate::mount_registry::{save as save_mounts, MountRegistry};

    #[test]
    fn imports_multiple_sources_atomically_and_rolls_back_injected_failure() {
        let home = initialized_home();
        fs::create_dir_all(home.join(".claude/commands")).unwrap();
        fs::write(home.join(".claude/commands/one.md"), "one").unwrap();
        fs::write(home.join(".claude/commands/two.md"), "two").unwrap();
        let selections = discover(&home, DiscoveryScope::User)
            .sources
            .into_iter()
            .map(|source| BatchImportSelection {
                source_id: source.source_id,
                resolution: ImportResolution::Unresolved,
            })
            .collect::<Vec<_>>();
        let request = BatchImportPreviewRequest {
            scope: DiscoveryScope::User,
            selections,
        };
        let preview = preview_batch_import(&home, &request).unwrap();
        assert!(preview.can_apply);
        let failed = apply_batch_import_inner(
            &home,
            &BatchImportApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request: request.clone(),
            },
            Some(1),
        )
        .unwrap_err();
        assert!(failed.to_string().contains("injected"));
        assert!(load_assets(&home).unwrap().assets.is_empty());

        let preview = preview_batch_import(&home, &request).unwrap();
        let applied = apply_batch_import(
            &home,
            &BatchImportApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap();
        assert_eq!(applied.items.len(), 2);
        assert_eq!(load_assets(&home).unwrap().assets.len(), 2);
        let _ = fs::remove_dir_all(home);
    }

    fn initialized_home() -> PathBuf {
        let home = std::env::temp_dir().join(format!(
            "maa-batch-import-{}",
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
        home
    }
}
