use crate::asset_registry::{
    canonical_path, inspect_content, load as load_assets, parse_asset_id, save as save_assets,
    AssetRecord, ContentDiagnostic, ContentState,
};
use crate::fingerprint::PreviewFingerprint;
use crate::mount::remove_path_if_present;
use crate::operation::{OperationJournal, OperationLock, RecoveryTarget};
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PREVIEW_TTL_SECONDS: u64 = 600;
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// An explicit, state-bound repair for a single registry/content mismatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsistencyRepairAction {
    #[serde(rename = "remove_missing_registry_record")]
    RemoveMissingRegistryRecord,
    #[serde(rename = "register_unregistered_content")]
    RegisterUnregisteredContent,
    #[serde(rename = "delete_unregistered_content")]
    DeleteUnregisteredContent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencyRepairPreviewRequest {
    pub asset_id: String,
    pub action: ConsistencyRepairAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencyRepairPreview {
    pub preview_id: String,
    pub request: ConsistencyRepairPreviewRequest,
    pub diagnostic: ContentDiagnostic,
    pub planned_effects: Vec<String>,
    pub warnings: Vec<String>,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencyRepairApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: ConsistencyRepairPreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencyRepairApplyResult {
    pub preview_id: String,
    pub asset_id: String,
    pub action: ConsistencyRepairAction,
    pub affected_paths: Vec<PathBuf>,
    pub journal_path: PathBuf,
}

pub fn preview_consistency_repair(
    home: &Path,
    request: &ConsistencyRepairPreviewRequest,
) -> Result<ConsistencyRepairPreview> {
    preview_consistency_repair_at(home, request, epoch_seconds())
}

fn preview_consistency_repair_at(
    home: &Path,
    request: &ConsistencyRepairPreviewRequest,
    generated_at_epoch_seconds: u64,
) -> Result<ConsistencyRepairPreview> {
    let (asset_type, name) =
        parse_asset_id(&request.asset_id).map_err(|error| MaaError::new(error.to_string()))?;
    let registry = load_assets(home).map_err(|error| MaaError::new(error.to_string()))?;
    let diagnostic = inspect_content(home, &registry)
        .map_err(|error| MaaError::new(error.to_string()))?
        .into_iter()
        .find(|entry| entry.asset_id == request.asset_id)
        .ok_or_else(|| {
            MaaError::new(format!(
                "no registry/content diagnostic for {}",
                request.asset_id
            ))
        })?;

    if diagnostic.asset_type != asset_type || diagnostic.name != name {
        return Err(MaaError::new(
            "registry/content diagnostic identity is invalid",
        ));
    }

    let mut planned_effects = Vec::new();
    let mut warnings = vec![
        "这是高风险一致性修复：Apply 会先保存可恢复事务快照，并在写入前重新验证 Preview。".into(),
    ];
    let can_apply = match (request.action, diagnostic.state) {
        (ConsistencyRepairAction::RemoveMissingRegistryRecord, ContentState::MissingContent) => {
            planned_effects.push(format!(
                "remove stale assets.yaml record for {} without recreating missing content",
                diagnostic.asset_id
            ));
            true
        }
        (ConsistencyRepairAction::RegisterUnregisteredContent, ContentState::Unregistered) => {
            planned_effects.push(format!(
                "add {} to assets.yaml using existing canonical content",
                diagnostic.asset_id
            ));
            true
        }
        (ConsistencyRepairAction::DeleteUnregisteredContent, ContentState::Unregistered) => {
            planned_effects.push(format!(
                "delete unregistered canonical content at {}",
                diagnostic.path.display()
            ));
            warnings.push(
                "删除孤立内容后无法通过应用内历史 Restore 恢复；请确认不需要该文件或目录。".into(),
            );
            true
        }
        (_, ContentState::InvalidContent) => {
            warnings.push(
                "canonical 内容已损坏。为避免覆盖或丢失原始证据，本版本只报告诊断，不提供自动修复。"
                    .into(),
            );
            false
        }
        (action, state) => {
            warnings.push(format!(
                "repair action {action:?} is not valid for diagnostic state {state:?}."
            ));
            false
        }
    };

    let preview_id = repair_fingerprint(home, request, &diagnostic, generated_at_epoch_seconds)?;
    Ok(ConsistencyRepairPreview {
        preview_id,
        request: request.clone(),
        diagnostic,
        planned_effects,
        warnings,
        can_apply,
        generated_at_epoch_seconds,
        expires_at_epoch_seconds: generated_at_epoch_seconds.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

pub fn apply_consistency_repair(
    home: &Path,
    request: &ConsistencyRepairApplyRequest,
) -> Result<ConsistencyRepairApplyResult> {
    let _lock = OperationLock::acquire(home)?;
    if epoch_seconds()
        > request
            .preview_generated_at_epoch_seconds
            .saturating_add(PREVIEW_TTL_SECONDS)
    {
        return Err(MaaError::new(
            "consistency repair preview expired; generate a new preview before applying",
        ));
    }
    let preview = preview_consistency_repair_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    if preview.preview_id != request.preview_id {
        return Err(MaaError::new(
            "consistency repair preview is stale; generate a new preview before applying",
        ));
    }
    if !preview.can_apply {
        return Err(MaaError::new(
            preview
                .warnings
                .last()
                .cloned()
                .unwrap_or_else(|| "consistency repair is blocked".into()),
        ));
    }

    let registry_path = home.join(".my-agent-assets/assets.yaml");
    let content_path = canonical_path(
        home,
        preview.diagnostic.asset_type,
        &preview.diagnostic.name,
    );
    let mut recovery_targets = vec![RecoveryTarget::asset_center(registry_path.clone())];
    if matches!(
        preview.request.action,
        ConsistencyRepairAction::DeleteUnregisteredContent
    ) {
        recovery_targets.push(RecoveryTarget::asset_center(content_path.clone()));
    }
    let operation_id = operation_id();
    let mut journal = OperationJournal::start_recoverable(
        home,
        &operation_id,
        "consistency_repair",
        recovery_targets,
    )?;

    let mut affected_paths = vec![registry_path];
    match preview.request.action {
        ConsistencyRepairAction::RemoveMissingRegistryRecord => {
            let mut registry =
                load_assets(home).map_err(|error| MaaError::new(error.to_string()))?;
            registry.assets.remove(&preview.diagnostic.asset_id);
            save_assets(home, &registry).map_err(|error| MaaError::new(error.to_string()))?;
            journal.record_step("stale_registry_record_removed")?;
        }
        ConsistencyRepairAction::RegisterUnregisteredContent => {
            let mut registry =
                load_assets(home).map_err(|error| MaaError::new(error.to_string()))?;
            registry
                .upsert(
                    AssetRecord::new(preview.diagnostic.asset_type, &preview.diagnostic.name)
                        .map_err(|error| MaaError::new(error.to_string()))?,
                )
                .map_err(|error| MaaError::new(error.to_string()))?;
            save_assets(home, &registry).map_err(|error| MaaError::new(error.to_string()))?;
            journal.record_step("unregistered_content_registered")?;
        }
        ConsistencyRepairAction::DeleteUnregisteredContent => {
            remove_path_if_present(&content_path)?;
            affected_paths.push(content_path);
            journal.record_step("unregistered_content_deleted")?;
        }
    }
    journal.complete()?;
    Ok(ConsistencyRepairApplyResult {
        preview_id: preview.preview_id,
        asset_id: preview.diagnostic.asset_id,
        action: preview.request.action,
        affected_paths,
        journal_path: journal.path().to_path_buf(),
    })
}

fn repair_fingerprint(
    home: &Path,
    request: &ConsistencyRepairPreviewRequest,
    diagnostic: &ContentDiagnostic,
    generated_at_epoch_seconds: u64,
) -> Result<String> {
    let mut fingerprint = PreviewFingerprint::new("consistency-repair");
    fingerprint.add_bytes(
        "request",
        &serde_json::to_vec(request).map_err(|error| MaaError::new(error.to_string()))?,
    );
    fingerprint.add_u64("generated-at", generated_at_epoch_seconds);
    fingerprint.add_path_if_present("registry", &home.join(".my-agent-assets/assets.yaml"))?;
    fingerprint.add_path_if_present("canonical-content", &diagnostic.path)?;
    fingerprint.add_path_if_present("operations", &home.join(".my-agent-assets/operations"))?;
    Ok(fingerprint.finish("consistency-repair"))
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
    use crate::asset_registry::{save, AssetRegistry};
    use crate::operation::{crash_test, recover_incomplete};
    use crate::targets::AssetKind;
    use std::fs;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    fn home(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "maa-consistency-repair-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn setup(home: &Path) {
        fs::create_dir_all(home.join(".my-agent-assets/assets/skills/orphan")).unwrap();
        fs::write(
            home.join(".my-agent-assets/assets/skills/orphan/SKILL.md"),
            "# Orphan",
        )
        .unwrap();
        fs::create_dir_all(home.join(".my-agent-assets/assets/commands")).unwrap();
        let mut registry = AssetRegistry::default();
        registry
            .upsert(AssetRecord::new(AssetKind::Command, "missing").unwrap())
            .unwrap();
        save(home, &registry).unwrap();
    }

    fn apply(
        home: &Path,
        request: ConsistencyRepairPreviewRequest,
    ) -> ConsistencyRepairApplyResult {
        let preview = preview_consistency_repair(home, &request).unwrap();
        assert!(preview.can_apply, "{:?}", preview.warnings);
        apply_consistency_repair(
            home,
            &ConsistencyRepairApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap()
    }

    #[test]
    fn removes_missing_record_registers_orphan_and_deletes_only_selected_orphan() {
        let home = home("all-actions");
        setup(&home);
        apply(
            &home,
            ConsistencyRepairPreviewRequest {
                asset_id: "command:missing".into(),
                action: ConsistencyRepairAction::RemoveMissingRegistryRecord,
            },
        );
        assert!(!load_assets(&home)
            .unwrap()
            .assets
            .contains_key("command:missing"));

        apply(
            &home,
            ConsistencyRepairPreviewRequest {
                asset_id: "skill:orphan".into(),
                action: ConsistencyRepairAction::RegisterUnregisteredContent,
            },
        );
        assert!(load_assets(&home)
            .unwrap()
            .assets
            .contains_key("skill:orphan"));

        let mut registry = load_assets(&home).unwrap();
        registry.assets.remove("skill:orphan");
        save_assets(&home, &registry).unwrap();
        let result = apply(
            &home,
            ConsistencyRepairPreviewRequest {
                asset_id: "skill:orphan".into(),
                action: ConsistencyRepairAction::DeleteUnregisteredContent,
            },
        );
        assert!(result
            .affected_paths
            .iter()
            .any(|path| path.ends_with("skills/orphan")));
        assert!(!home.join(".my-agent-assets/assets/skills/orphan").exists());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn rejects_incompatible_action_and_stale_preview() {
        let home = home("blocked");
        setup(&home);
        let wrong = preview_consistency_repair(
            &home,
            &ConsistencyRepairPreviewRequest {
                asset_id: "skill:orphan".into(),
                action: ConsistencyRepairAction::RemoveMissingRegistryRecord,
            },
        )
        .unwrap();
        assert!(!wrong.can_apply);

        let request = ConsistencyRepairPreviewRequest {
            asset_id: "skill:orphan".into(),
            action: ConsistencyRepairAction::RegisterUnregisteredContent,
        };
        let preview = preview_consistency_repair(&home, &request).unwrap();
        fs::write(
            home.join(".my-agent-assets/assets/skills/orphan/SKILL.md"),
            "# Changed",
        )
        .unwrap();
        assert!(apply_consistency_repair(
            &home,
            &ConsistencyRepairApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .is_err());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn interrupted_delete_is_recovered_from_journal_snapshot() {
        let home = home("recovery");
        setup(&home);
        let _guard =
            crash_test::crash_after_step("consistency_repair", "unregistered_content_deleted");
        let panic = catch_unwind(AssertUnwindSafe(|| {
            apply(
                &home,
                ConsistencyRepairPreviewRequest {
                    asset_id: "skill:orphan".into(),
                    action: ConsistencyRepairAction::DeleteUnregisteredContent,
                },
            );
        }));
        assert!(panic.is_err());
        assert!(!home.join(".my-agent-assets/assets/skills/orphan").exists());
        let report = recover_incomplete(&home).unwrap();
        assert!(report.attempted);
        assert!(home
            .join(".my-agent-assets/assets/skills/orphan/SKILL.md")
            .is_file());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn repair_action_wire_values_are_explicit_and_stable() {
        let cases = [
            (
                ConsistencyRepairAction::RemoveMissingRegistryRecord,
                "remove_missing_registry_record",
            ),
            (
                ConsistencyRepairAction::RegisterUnregisteredContent,
                "register_unregistered_content",
            ),
            (
                ConsistencyRepairAction::DeleteUnregisteredContent,
                "delete_unregistered_content",
            ),
        ];
        for (action, expected) in cases {
            assert_eq!(
                serde_json::to_string(&action).unwrap(),
                format!("\"{expected}\"")
            );
            assert_eq!(
                serde_json::from_str::<ConsistencyRepairAction>(&format!("\"{expected}\""))
                    .unwrap(),
                action
            );
        }
    }
}
