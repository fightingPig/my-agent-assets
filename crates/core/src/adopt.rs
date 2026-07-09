use crate::asset_registry::registry_path as asset_registry_path;
use crate::discovery::{AssetKind, DiscoveredSource, DiscoveryScope};
use crate::fingerprint::PreviewFingerprint;
use crate::import::{
    apply_import_locked, find_source, preview_import_at, ImportApplyRequest, ImportApplyStatus,
    ImportPreview, ImportPreviewRequest, ImportResolution,
};
use crate::mount::{
    apply_mount_locked, discard_runtime_snapshot, preview_mount_at, restore_runtime_snapshot,
    snapshot_runtime_path, target_asset_path, MountApplyRequest, MountApplyResult,
    MountPreviewRequest, RuntimeSnapshot,
};
use crate::mount_registry::registry_path as mount_registry_path;
use crate::operation::{OperationJournal, OperationLock, RecoveryTarget};
use crate::targets::{load as load_targets, MountTarget};
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
pub struct AdoptSelection {
    pub source_id: String,
    pub resolution: ImportResolution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptPreviewRequest {
    pub scope: DiscoveryScope,
    pub selections: Vec<AdoptSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptItemPreview {
    pub source_id: String,
    pub import_plan: ImportPreview,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_path: Option<PathBuf>,
    pub backup_required: bool,
    pub warnings: Vec<String>,
    pub can_apply: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptPreview {
    pub preview_id: String,
    pub items: Vec<AdoptItemPreview>,
    pub import_plan: Vec<String>,
    pub mount_plan: Vec<String>,
    pub backup_plan: Vec<String>,
    pub warnings: Vec<String>,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: AdoptPreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptItemResult {
    pub source_id: String,
    pub asset_id: String,
    pub target_id: Option<String>,
    pub imported: bool,
    pub mounted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptApplyResult {
    pub preview_id: String,
    pub items: Vec<AdoptItemResult>,
    pub affected_paths: Vec<PathBuf>,
    pub journal_path: PathBuf,
}

pub fn preview_adopt(home: &Path, request: &AdoptPreviewRequest) -> Result<AdoptPreview> {
    preview_adopt_at(home, request, epoch_seconds())
}

fn preview_adopt_at(
    home: &Path,
    request: &AdoptPreviewRequest,
    generated_at_epoch_seconds: u64,
) -> Result<AdoptPreview> {
    if request.selections.is_empty() {
        return Err(MaaError::new("adopt requires at least one selected source"));
    }
    let mut source_ids = BTreeSet::new();
    let mut destination_ids = BTreeSet::new();
    let targets = load_targets(home)?;
    let mut items = Vec::new();
    let mut all_warnings = Vec::new();
    for selection in &request.selections {
        if !source_ids.insert(selection.source_id.clone()) {
            return Err(MaaError::new(format!(
                "duplicate adopt source id: {}",
                selection.source_id
            )));
        }
        let source = find_source(home, &request.scope, &selection.source_id)?;
        let import_plan = preview_import_at(
            home,
            &ImportPreviewRequest {
                scope: request.scope.clone(),
                source_id: selection.source_id.clone(),
                resolution: selection.resolution.clone(),
            },
            generated_at_epoch_seconds,
        )?;
        if selection.resolution != ImportResolution::Skip
            && !destination_ids.insert(import_plan.asset_id.clone())
        {
            return Err(MaaError::new(format!(
                "multiple adopt sources resolve to the same canonical asset '{}'",
                import_plan.asset_id
            )));
        }
        let mut warnings = Vec::new();
        let mut can_apply = import_plan.can_apply
            || import_plan.disposition == crate::import::ImportDisposition::Unchanged;
        let (target_id, target_path) = if selection.resolution == ImportResolution::Skip {
            (None, None)
        } else if matches!(selection.resolution, ImportResolution::Rename { .. }) {
            warnings.push(
                "Rename is available for Import only; Adopt must preserve the runtime asset name."
                    .into(),
            );
            can_apply = false;
            (None, None)
        } else if !source.eligible_adopt && source.asset_kind != AssetKind::Mcp {
            warnings.push("source is not eligible for adoption".into());
            can_apply = false;
            (None, None)
        } else {
            match find_source_target(&targets.targets, &source, &import_plan.destination_name) {
                Some((target, path)) => {
                    let compatibility = target.compatibility(source.asset_kind);
                    if !compatibility.compatible {
                        warnings.push(
                            compatibility
                                .reason
                                .unwrap_or_else(|| "target is incompatible".into()),
                        );
                        can_apply = false;
                    }
                    (Some(target.id.clone()), Some(path))
                }
                None => {
                    warnings.push(
                        "no authorized Mount Target matches the selected runtime source".into(),
                    );
                    can_apply = false;
                    (None, None)
                }
            }
        };
        all_warnings.extend(warnings.clone());
        items.push(AdoptItemPreview {
            source_id: selection.source_id.clone(),
            import_plan,
            target_id,
            target_path,
            backup_required: selection.resolution != ImportResolution::Skip,
            warnings,
            can_apply,
        });
    }
    let can_apply = items.iter().all(|item| item.can_apply);
    let import_plan = items
        .iter()
        .map(|item| {
            format!(
                "{} canonical asset {}",
                match item.import_plan.disposition {
                    crate::import::ImportDisposition::Overwrite => "overwrite",
                    crate::import::ImportDisposition::Unchanged => "reuse",
                    crate::import::ImportDisposition::Skip => "skip",
                    _ => "import",
                },
                item.import_plan.asset_id
            )
        })
        .collect();
    let mount_plan = items
        .iter()
        .filter_map(|item| {
            item.target_path.as_ref().map(|path| {
                format!(
                    "mount {} through target '{}' at {}",
                    item.import_plan.asset_id,
                    item.target_id.as_deref().unwrap_or_default(),
                    path.display()
                )
            })
        })
        .collect();
    let backup_plan = items
        .iter()
        .filter_map(|item| {
            item.target_path
                .as_ref()
                .map(|path| format!("backup original runtime source {}", path.display()))
        })
        .collect();
    let preview_id = adopt_fingerprint(home, request, &items, generated_at_epoch_seconds)?;
    Ok(AdoptPreview {
        preview_id,
        items,
        import_plan,
        mount_plan,
        backup_plan,
        warnings: all_warnings,
        can_apply,
        generated_at_epoch_seconds,
        expires_at_epoch_seconds: generated_at_epoch_seconds.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

pub fn apply_adopt(home: &Path, request: &AdoptApplyRequest) -> Result<AdoptApplyResult> {
    apply_adopt_inner(home, request, None)
}

fn apply_adopt_inner(
    home: &Path,
    request: &AdoptApplyRequest,
    fail_after_items: Option<usize>,
) -> Result<AdoptApplyResult> {
    let _operation_lock = OperationLock::acquire(home)?;
    if epoch_seconds()
        > request
            .preview_generated_at_epoch_seconds
            .saturating_add(PREVIEW_TTL_SECONDS)
    {
        return Err(MaaError::new(
            "adopt preview expired; generate a new preview before applying",
        ));
    }
    let preview = preview_adopt_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    if preview.preview_id != request.preview_id {
        return Err(MaaError::new(
            "adopt preview is stale; generate a new preview before applying",
        ));
    }
    if !preview.can_apply {
        return Err(MaaError::new(
            preview
                .warnings
                .first()
                .cloned()
                .unwrap_or_else(|| "adopt is blocked".into()),
        ));
    }

    let operation_id = operation_id();
    let mut recovery_targets = vec![
        RecoveryTarget::asset_center(asset_registry_path(home)),
        RecoveryTarget::asset_center(mount_registry_path(home)),
    ];
    for item in &preview.items {
        if item.import_plan.disposition == crate::import::ImportDisposition::Skip {
            continue;
        }
        recovery_targets.push(RecoveryTarget::asset_center(
            item.import_plan.destination_path.clone(),
        ));
        if let (Some(target_id), Some(target_path)) = (&item.target_id, &item.target_path) {
            recovery_targets.push(RecoveryTarget::registered_target(
                target_id.clone(),
                target_path.clone(),
            ));
        }
    }
    let mut journal = OperationJournal::start_recoverable(
        home,
        &operation_id,
        "import_and_adopt",
        recovery_targets,
    )?;
    let assets_before = fs::read(asset_registry_path(home))?;
    let mounts_before = fs::read(mount_registry_path(home))?;
    let mut snapshots = BTreeMap::<PathBuf, RuntimeSnapshot>::new();
    for item in &preview.items {
        if item.import_plan.disposition == crate::import::ImportDisposition::Skip {
            continue;
        }
        if let Some(path) = &item.target_path {
            snapshots
                .entry(path.clone())
                .or_insert(snapshot_runtime_path(path)?);
        }
        snapshots
            .entry(item.import_plan.destination_path.clone())
            .or_insert(snapshot_runtime_path(&item.import_plan.destination_path)?);
    }
    journal.record_step("rollback_snapshots_created")?;

    let result = (|| -> Result<(Vec<AdoptItemResult>, Vec<PathBuf>)> {
        let mut results = Vec::new();
        let mut affected = Vec::new();
        for (index, (selection, item)) in request
            .request
            .selections
            .iter()
            .zip(&preview.items)
            .enumerate()
        {
            if selection.resolution == ImportResolution::Skip {
                results.push(AdoptItemResult {
                    source_id: selection.source_id.clone(),
                    asset_id: item.import_plan.asset_id.clone(),
                    target_id: None,
                    imported: false,
                    mounted: false,
                });
                continue;
            }
            let import_request = ImportPreviewRequest {
                scope: request.request.scope.clone(),
                source_id: selection.source_id.clone(),
                resolution: selection.resolution.clone(),
            };
            let current_import_preview = preview_import_at(
                home,
                &import_request,
                request.preview_generated_at_epoch_seconds,
            )?;
            let import_result = apply_import_locked(
                home,
                &ImportApplyRequest {
                    preview_id: current_import_preview.preview_id,
                    preview_generated_at_epoch_seconds: request.preview_generated_at_epoch_seconds,
                    request: import_request,
                },
                None,
            )?;
            journal.record_step(format!("imported:{}", import_result.asset_id))?;

            let target_id = item
                .target_id
                .as_ref()
                .ok_or_else(|| MaaError::new("adopt item is missing targetId"))?;
            let mount_request = MountPreviewRequest {
                asset_id: import_result.asset_id.clone(),
                target_id: target_id.clone(),
            };
            let mount_preview = preview_mount_at(
                home,
                &mount_request,
                request.preview_generated_at_epoch_seconds,
            )?;
            let mount_result: MountApplyResult = apply_mount_locked(
                home,
                &MountApplyRequest {
                    preview_id: mount_preview.preview_id,
                    preview_generated_at_epoch_seconds: request.preview_generated_at_epoch_seconds,
                    request: mount_request,
                },
            )?;
            journal.record_step(format!("mounted:{}@{target_id}", import_result.asset_id))?;
            affected.extend(import_result.affected_paths);
            affected.extend(mount_result.affected_paths);
            results.push(AdoptItemResult {
                source_id: selection.source_id.clone(),
                asset_id: import_result.asset_id,
                target_id: Some(target_id.clone()),
                imported: import_result.status == ImportApplyStatus::Imported,
                mounted: mount_result.mounted,
            });
            if fail_after_items.is_some_and(|count| index + 1 >= count) {
                return Err(MaaError::new("injected adopt failure"));
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
    Ok(AdoptApplyResult {
        preview_id: preview.preview_id,
        items,
        affected_paths,
        journal_path: journal.path().to_path_buf(),
    })
}

fn find_source_target<'a>(
    targets: &'a [MountTarget],
    source: &DiscoveredSource,
    destination_name: &str,
) -> Option<(&'a MountTarget, PathBuf)> {
    targets.iter().find_map(|target| {
        if target.provider != source.provider || !target.accepts.contains(&source.asset_kind) {
            return None;
        }
        let path = target_asset_path(target, source.asset_kind, destination_name);
        let matches_source = if source.asset_kind == AssetKind::Mcp {
            source.config_path.as_deref() == Some(target.path.as_path())
        } else {
            path == source.source_path
        };
        matches_source.then_some((target, path))
    })
}

fn adopt_fingerprint(
    home: &Path,
    request: &AdoptPreviewRequest,
    items: &[AdoptItemPreview],
    generated_at: u64,
) -> Result<String> {
    let mut fingerprint = PreviewFingerprint::new("adopt");
    fingerprint.add_bytes(
        "request",
        &serde_json::to_vec(request).map_err(|error| MaaError::new(error.to_string()))?,
    );
    fingerprint.add_u64("generated-at", generated_at);
    for item in items {
        fingerprint.add_bytes("import-preview-id", item.import_plan.preview_id.as_bytes());
        if let Some(target_id) = &item.target_id {
            fingerprint.add_bytes("target-id", target_id.as_bytes());
        }
    }
    for path in [
        asset_registry_path(home),
        mount_registry_path(home),
        crate::targets::registry_path(home),
    ] {
        fingerprint.add_path("registry", &path)?;
    }
    Ok(fingerprint.finish("adopt"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_registry::{load as load_assets, save as save_assets, AssetRegistry};
    use crate::mount_registry::{load as load_mounts, save as save_mounts, MountRegistry};
    use crate::operation::{crash_test, recover_incomplete};
    use crate::targets::{save as save_targets, MountAdapter, ProviderState, TargetRegistry};
    use serde_json::Value as JsonValue;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[cfg(unix)]
    #[test]
    fn adopts_claude_codex_skill_command_and_mcp_sources_in_one_transaction() {
        let home = initialized_home("all-sources");
        create_sources(&home);
        let source_bytes = source_snapshots(&home);
        let discovered = crate::discovery::discover(&home, DiscoveryScope::User);
        let selections = discovered
            .sources
            .iter()
            .map(|source| AdoptSelection {
                source_id: source.source_id.clone(),
                resolution: ImportResolution::Unresolved,
            })
            .collect::<Vec<_>>();
        assert_eq!(selections.len(), 5);
        let request = AdoptPreviewRequest {
            scope: DiscoveryScope::User,
            selections,
        };
        let preview = preview_adopt(&home, &request).unwrap();
        crate::fingerprint::assert_sha256_preview_id(&preview.preview_id, "adopt-");
        assert!(preview.can_apply, "{:?}", preview.warnings);
        assert_eq!(preview.items.len(), 5);
        let result = apply_adopt(
            &home,
            &AdoptApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap();
        assert_eq!(result.items.len(), 5);
        assert!(result.items.iter().all(|item| item.mounted));

        for path in [
            home.join(".claude/skills/review"),
            home.join(".agents/skills/api-design"),
            home.join(".claude/commands/commit.md"),
        ] {
            assert!(fs::symlink_metadata(path).unwrap().file_type().is_symlink());
        }
        let claude: JsonValue =
            serde_json::from_str(&fs::read_to_string(home.join(".claude.json")).unwrap()).unwrap();
        assert_eq!(claude["other"], "keep");
        assert_eq!(
            claude["mcpServers"]["postgres"]["command"],
            source_bytes["claude_mcp"]
        );
        let codex = fs::read_to_string(home.join(".codex/config.toml")).unwrap();
        assert!(codex.contains("# keep"));
        assert!(codex.contains("[mcp_servers.filesystem]"));
        assert_eq!(load_assets(&home).unwrap().assets.len(), 5);
        assert_eq!(load_mounts(&home).unwrap().bindings.len(), 5);
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(unix)]
    #[test]
    fn mid_adopt_failure_restores_all_sources_and_canonical_state() {
        let home = initialized_home("rollback");
        fs::create_dir_all(home.join(".claude/skills/one")).unwrap();
        fs::create_dir_all(home.join(".agents/skills/two")).unwrap();
        fs::write(home.join(".claude/skills/one/SKILL.md"), "# One").unwrap();
        fs::write(home.join(".agents/skills/two/SKILL.md"), "# Two").unwrap();
        let discovered = crate::discovery::discover(&home, DiscoveryScope::User);
        let request = AdoptPreviewRequest {
            scope: DiscoveryScope::User,
            selections: discovered
                .sources
                .iter()
                .map(|source| AdoptSelection {
                    source_id: source.source_id.clone(),
                    resolution: ImportResolution::Unresolved,
                })
                .collect(),
        };
        let preview = preview_adopt(&home, &request).unwrap();
        let assets_before = fs::read(asset_registry_path(&home)).unwrap();
        let mounts_before = fs::read(mount_registry_path(&home)).unwrap();
        let error = apply_adopt_inner(
            &home,
            &AdoptApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
            Some(1),
        )
        .unwrap_err();
        assert!(error.to_string().contains("injected"));
        assert!(home.join(".claude/skills/one/SKILL.md").is_file());
        assert!(home.join(".agents/skills/two/SKILL.md").is_file());
        assert!(!home.join(".my-agent-assets/assets/skills/one").exists());
        assert!(!home.join(".my-agent-assets/assets/skills/two").exists());
        assert_eq!(fs::read(asset_registry_path(&home)).unwrap(), assets_before);
        assert_eq!(fs::read(mount_registry_path(&home)).unwrap(), mounts_before);
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(unix)]
    #[test]
    fn adopt_recovers_when_process_crashes_after_mounted_step() {
        let home = initialized_home("crash-recovery");
        fs::create_dir_all(home.join(".claude/skills/one")).unwrap();
        fs::write(home.join(".claude/skills/one/SKILL.md"), "# One").unwrap();
        let discovered = crate::discovery::discover(&home, DiscoveryScope::User);
        let request = AdoptPreviewRequest {
            scope: DiscoveryScope::User,
            selections: discovered
                .sources
                .iter()
                .map(|source| AdoptSelection {
                    source_id: source.source_id.clone(),
                    resolution: ImportResolution::Unresolved,
                })
                .collect(),
        };
        let preview = preview_adopt(&home, &request).unwrap();
        let runtime_path = home.join(".claude/skills/one");
        let crash = crash_test::crash_after_step(
            "import_and_adopt",
            "mounted:skill:one@claude-user-skills",
        );

        let result = catch_unwind(AssertUnwindSafe(|| {
            apply_adopt(
                &home,
                &AdoptApplyRequest {
                    preview_id: preview.preview_id,
                    preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                    request,
                },
            )
        }));
        assert!(result.is_err());
        drop(crash);

        assert!(fs::symlink_metadata(&runtime_path)
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(load_assets(&home)
            .unwrap()
            .get(crate::targets::AssetKind::Skill, "one")
            .is_some());
        let report = recover_incomplete(&home).unwrap();
        assert!(report.attempted);
        assert!(!report.writes_blocked);
        assert!(report.attempts[0].recovered);
        assert!(runtime_path.join("SKILL.md").is_file());
        assert!(!fs::symlink_metadata(&runtime_path)
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(load_assets(&home).unwrap().assets.is_empty());
        assert!(load_mounts(&home).unwrap().bindings.is_empty());
        let _ = fs::remove_dir_all(home);
    }

    fn initialized_home(name: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!(
            "maa-adopt-{name}-{}",
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

    fn create_sources(home: &Path) {
        fs::create_dir_all(home.join(".claude/skills/review")).unwrap();
        fs::write(home.join(".claude/skills/review/SKILL.md"), "# Review").unwrap();
        fs::create_dir_all(home.join(".agents/skills/api-design")).unwrap();
        fs::write(
            home.join(".agents/skills/api-design/SKILL.md"),
            "# API Design",
        )
        .unwrap();
        fs::create_dir_all(home.join(".claude/commands")).unwrap();
        fs::write(home.join(".claude/commands/commit.md"), "# Commit").unwrap();
        fs::write(
            home.join(".claude.json"),
            r#"{"other":"keep","mcpServers":{"postgres":{"command":"postgres-server"}}}"#,
        )
        .unwrap();
        fs::create_dir_all(home.join(".codex")).unwrap();
        fs::write(
            home.join(".codex/config.toml"),
            "# keep\nmodel = \"gpt-5\"\n[mcp_servers.filesystem]\ncommand = \"filesystem-server\"\n",
        )
        .unwrap();
    }

    fn source_snapshots(_home: &Path) -> BTreeMap<&'static str, JsonValue> {
        BTreeMap::from([("claude_mcp", JsonValue::String("postgres-server".into()))])
    }
}
