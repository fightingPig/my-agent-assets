use crate::asset_registry::{
    asset_id, canonical_path, load as load_assets, registry_path as asset_registry_path,
    save as save_assets, AssetRecord,
};
use crate::fingerprint::PreviewFingerprint;
use crate::mcp::CanonicalMcp;
use crate::mount::validate_mcp_target_preview;
use crate::mount_registry::{
    load as load_mounts, registry_path as mount_registry_path, save as save_mounts, BindingStatus,
};
use crate::operation::{OperationJournal, OperationLock, RecoveryTarget};
use crate::path_safety::guard_write_path;
use crate::targets::{load as load_targets, AssetKind};
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PREVIEW_TTL_SECONDS: u64 = 300;
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpSavePreviewRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    pub canonical: CanonicalMcp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpSaveOperation {
    #[serde(rename = "create")]
    Create,
    #[serde(rename = "edit")]
    Edit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpTargetCompatibility {
    pub target_id: String,
    pub compatible: bool,
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpSavePreview {
    pub preview_id: String,
    pub operation: McpSaveOperation,
    pub asset_id: String,
    pub canonical_path: PathBuf,
    pub registry_path: PathBuf,
    pub out_of_sync_target_ids: Vec<String>,
    pub target_compatibility: Vec<McpTargetCompatibility>,
    pub planned_effects: Vec<String>,
    pub warnings: Vec<String>,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpSaveApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: McpSavePreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpSaveApplyResult {
    pub preview_id: String,
    pub operation: McpSaveOperation,
    pub asset_id: String,
    pub canonical_path: PathBuf,
    pub out_of_sync_target_ids: Vec<String>,
    pub affected_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpBindingSummary {
    pub target_id: String,
    pub status: BindingStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpAssetDefinition {
    pub asset_id: String,
    pub canonical: CanonicalMcp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub bindings: Vec<McpBindingSummary>,
}

pub fn load_mcp_asset(home: &Path, asset_id_value: &str) -> Result<McpAssetDefinition> {
    let (kind, name) = crate::asset_registry::parse_asset_id(asset_id_value)
        .map_err(|error| MaaError::new(error.to_string()))?;
    if kind != AssetKind::Mcp {
        return Err(MaaError::new("MCP asset lookup requires an mcp: asset ID"));
    }
    let assets = load_assets(home).map_err(|error| MaaError::new(error.to_string()))?;
    let record = assets
        .get(AssetKind::Mcp, &name)
        .ok_or_else(|| MaaError::new(format!("MCP asset '{asset_id_value}' is not registered")))?;
    let path = canonical_path(home, AssetKind::Mcp, &name);
    let canonical: CanonicalMcp = serde_json::from_slice(&fs::read(&path)?)
        .map_err(|error| MaaError::new(format!("invalid canonical MCP: {error}")))?;
    canonical
        .validate()
        .map_err(|error| MaaError::new(error.to_string()))?;
    if canonical.name != name {
        return Err(MaaError::new(
            "canonical MCP name does not match the asset ID",
        ));
    }
    let mounts = load_mounts(home).map_err(|error| MaaError::new(error.to_string()))?;
    let mut bindings = mounts
        .for_asset(asset_id_value)
        .into_iter()
        .map(|binding| McpBindingSummary {
            target_id: binding.target_id.clone(),
            status: binding.status,
            last_synced_at: binding.last_synced_at.clone(),
        })
        .collect::<Vec<_>>();
    bindings.sort_by(|left, right| left.target_id.cmp(&right.target_id));
    Ok(McpAssetDefinition {
        asset_id: asset_id_value.to_string(),
        canonical,
        title: record.title.clone(),
        description: record.description.clone(),
        bindings,
    })
}

pub fn preview_mcp_save(home: &Path, request: &McpSavePreviewRequest) -> Result<McpSavePreview> {
    preview_mcp_save_at(home, request, epoch_seconds())
}

fn preview_mcp_save_at(
    home: &Path,
    request: &McpSavePreviewRequest,
    generated_at_epoch_seconds: u64,
) -> Result<McpSavePreview> {
    request
        .canonical
        .validate()
        .map_err(|error| MaaError::new(error.to_string()))?;
    let expected_asset_id = asset_id(AssetKind::Mcp, &request.canonical.name);
    if let Some(existing_asset_id) = &request.asset_id {
        let (kind, name) = crate::asset_registry::parse_asset_id(existing_asset_id)
            .map_err(|error| MaaError::new(error.to_string()))?;
        if kind != AssetKind::Mcp {
            return Err(MaaError::new("MCP edit requires an mcp: asset ID"));
        }
        if name != request.canonical.name {
            return Err(MaaError::new(
                "existing MCP name and asset ID are read-only",
            ));
        }
    }

    let assets = load_assets(home).map_err(|error| MaaError::new(error.to_string()))?;
    let canonical_path = canonical_path(home, AssetKind::Mcp, &request.canonical.name);
    let existing = assets.get(AssetKind::Mcp, &request.canonical.name);
    let operation = if request.asset_id.is_some() {
        if existing.is_none() {
            return Err(MaaError::new(format!(
                "MCP asset '{expected_asset_id}' is not registered"
            )));
        }
        McpSaveOperation::Edit
    } else {
        if existing.is_some() || canonical_path.exists() {
            return Err(MaaError::new(format!(
                "MCP asset '{expected_asset_id}' already exists; edit it by asset ID"
            )));
        }
        McpSaveOperation::Create
    };

    let mounts = load_mounts(home).map_err(|error| MaaError::new(error.to_string()))?;
    let targets = load_targets(home)?;
    let mut out_of_sync_target_ids = Vec::new();
    let mut target_compatibility = Vec::new();
    let mut warnings = Vec::new();
    let mut can_apply = true;
    for binding in mounts.for_asset(&expected_asset_id) {
        out_of_sync_target_ids.push(binding.target_id.clone());
        match targets.resolve(&binding.target_id) {
            Ok(target) => match validate_mcp_target_preview(target, &request.canonical) {
                Ok(renderer_warnings) => {
                    warnings.extend(renderer_warnings.iter().cloned());
                    target_compatibility.push(McpTargetCompatibility {
                        target_id: binding.target_id.clone(),
                        compatible: true,
                        warnings: renderer_warnings,
                        blocked_reason: None,
                    });
                }
                Err(error) => {
                    can_apply = false;
                    target_compatibility.push(McpTargetCompatibility {
                        target_id: binding.target_id.clone(),
                        compatible: false,
                        warnings: Vec::new(),
                        blocked_reason: Some(error.to_string()),
                    });
                }
            },
            Err(error) => {
                can_apply = false;
                target_compatibility.push(McpTargetCompatibility {
                    target_id: binding.target_id.clone(),
                    compatible: false,
                    warnings: Vec::new(),
                    blocked_reason: Some(error.to_string()),
                });
            }
        }
    }
    out_of_sync_target_ids.sort();
    target_compatibility.sort_by(|left, right| left.target_id.cmp(&right.target_id));
    warnings.sort();
    warnings.dedup();

    let mut planned_effects = vec![
        format!(
            "write canonical MCP definition {}",
            canonical_path.display()
        ),
        format!(
            "update canonical asset index {}",
            asset_registry_path(home).display()
        ),
    ];
    if !out_of_sync_target_ids.is_empty() {
        planned_effects.push(format!(
            "mark {} existing target binding(s) outOfSync without writing live config",
            out_of_sync_target_ids.len()
        ));
    }
    let preview_id =
        preview_fingerprint(home, request, &canonical_path, generated_at_epoch_seconds)?;
    Ok(McpSavePreview {
        preview_id,
        operation,
        asset_id: expected_asset_id,
        canonical_path,
        registry_path: asset_registry_path(home),
        out_of_sync_target_ids,
        target_compatibility,
        planned_effects,
        warnings,
        can_apply,
        generated_at_epoch_seconds,
        expires_at_epoch_seconds: generated_at_epoch_seconds.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

pub fn apply_mcp_save(home: &Path, request: &McpSaveApplyRequest) -> Result<McpSaveApplyResult> {
    let _lock = OperationLock::acquire(home)?;
    if epoch_seconds()
        > request
            .preview_generated_at_epoch_seconds
            .saturating_add(PREVIEW_TTL_SECONDS)
    {
        return Err(MaaError::new(
            "MCP save preview expired; generate a new preview before applying",
        ));
    }
    let preview = preview_mcp_save_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    if preview.preview_id != request.preview_id {
        return Err(MaaError::new(
            "MCP save preview is stale; generate a new preview before applying",
        ));
    }
    if !preview.can_apply {
        return Err(MaaError::new(
            "MCP save is blocked by target compatibility errors",
        ));
    }

    let operation_id = operation_id();
    let mut recovery_targets = vec![
        RecoveryTarget::asset_center(preview.canonical_path.clone()),
        RecoveryTarget::asset_center(asset_registry_path(home)),
    ];
    if !preview.out_of_sync_target_ids.is_empty() {
        recovery_targets.push(RecoveryTarget::asset_center(mount_registry_path(home)));
    }
    let mut journal =
        OperationJournal::start_recoverable(home, &operation_id, "mcp_save", recovery_targets)?;
    let result = apply_mcp_save_locked(home, request, &preview);
    match result {
        Ok(result) => {
            journal.record_step("canonical_mcp_saved")?;
            journal.complete()?;
            Ok(result)
        }
        Err(error) => {
            let original = error.to_string();
            journal.rollback_now(home).map_err(|rollback| {
                MaaError::new(format!(
                    "{original}; persistent MCP save rollback failed: {rollback}"
                ))
            })?;
            Err(error)
        }
    }
}

fn apply_mcp_save_locked(
    home: &Path,
    request: &McpSaveApplyRequest,
    preview: &McpSavePreview,
) -> Result<McpSaveApplyResult> {
    let mut assets = load_assets(home).map_err(|error| MaaError::new(error.to_string()))?;
    let mut record = AssetRecord::new(AssetKind::Mcp, request.request.canonical.name.clone())
        .map_err(|error| MaaError::new(error.to_string()))?;
    record.title = normalized_optional(&request.request.title);
    record.description = normalized_optional(&request.request.description);
    assets
        .upsert(record)
        .map_err(|error| MaaError::new(error.to_string()))?;

    let content = serde_json::to_vec_pretty(&request.request.canonical)
        .map_err(|error| MaaError::new(error.to_string()))?;
    atomic_write_asset(home, &preview.canonical_path, &content)?;
    save_assets(home, &assets).map_err(|error| MaaError::new(error.to_string()))?;
    if !preview.out_of_sync_target_ids.is_empty() {
        let mut mounts = load_mounts(home).map_err(|error| MaaError::new(error.to_string()))?;
        mounts.mark_asset_out_of_sync(&preview.asset_id);
        save_mounts(home, &mounts).map_err(|error| MaaError::new(error.to_string()))?;
    }

    let mut affected_paths = vec![preview.canonical_path.clone(), asset_registry_path(home)];
    if !preview.out_of_sync_target_ids.is_empty() {
        affected_paths.push(mount_registry_path(home));
    }

    Ok(McpSaveApplyResult {
        preview_id: preview.preview_id.clone(),
        operation: preview.operation,
        asset_id: preview.asset_id.clone(),
        canonical_path: preview.canonical_path.clone(),
        out_of_sync_target_ids: preview.out_of_sync_target_ids.clone(),
        affected_paths,
    })
}

fn preview_fingerprint(
    home: &Path,
    request: &McpSavePreviewRequest,
    canonical_path: &Path,
    generated_at_epoch_seconds: u64,
) -> Result<String> {
    let mut fingerprint = PreviewFingerprint::new("mcp-save");
    fingerprint.add_bytes(
        "request",
        &serde_json::to_vec(request).map_err(|error| MaaError::new(error.to_string()))?,
    );
    fingerprint.add_u64("generated-at", generated_at_epoch_seconds);
    for path in [
        canonical_path.to_path_buf(),
        asset_registry_path(home),
        mount_registry_path(home),
        crate::targets::registry_path(home),
    ] {
        fingerprint.add_path_if_present("state", &path)?;
    }
    Ok(fingerprint.finish("mcp-save"))
}

fn atomic_write_asset(home: &Path, path: &Path, content: &[u8]) -> Result<()> {
    let root = home.join(".my-agent-assets");
    let path = guard_write_path(&root, path)?;
    let parent = path
        .parent()
        .ok_or_else(|| MaaError::new("canonical MCP path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = guard_write_path(
        &root,
        &parent.join(format!(".mcp-save-{}.tmp", operation_id())),
    )?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(content)?;
    file.sync_all()?;
    drop(file);
    if path.exists() {
        fs::remove_file(&path)?;
    }
    fs::rename(&temporary, &path)?;
    Ok(())
}

fn normalized_optional(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
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
    use crate::mcp::{McpSpec, McpTransport};
    use crate::mount::{apply_mount, preview_mount, MountApplyRequest, MountPreviewRequest};
    use crate::mount_registry::{
        load as load_mounts, save as save_mounts, BindingStatus, MountBinding, MountRegistry,
    };
    use crate::operation::{crash_test, recover_incomplete};
    use crate::targets::{save as save_targets, MountAdapter, ProviderState, TargetRegistry};
    use serde_json::Map;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    fn temp_home(name: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!("maa-mcp-save-{name}-{}", operation_id()));
        let root = home.join(".my-agent-assets");
        for path in [
            root.join("assets/mcps"),
            root.join("backups/local"),
            root.join("operations"),
        ] {
            fs::create_dir_all(path).unwrap();
        }
        save_assets(&home, &AssetRegistry::default()).unwrap();
        save_mounts(&home, &MountRegistry::default()).unwrap();
        let targets = TargetRegistry::standard_user_targets(
            &home,
            ProviderState::Initialized,
            ProviderState::Initialized,
            MountAdapter::SymlinkDirectory,
        )
        .unwrap();
        save_targets(&home, &targets).unwrap();
        home
    }

    fn canonical(name: &str, command: &str) -> CanonicalMcp {
        CanonicalMcp {
            schema_version: 1,
            name: name.into(),
            spec: McpSpec {
                transport: Some(McpTransport::Stdio),
                command: Some(command.into()),
                args: vec![],
                env: Default::default(),
                cwd: None,
                url: None,
                headers: Default::default(),
                extra: Map::new(),
            },
            provider_extensions: Map::new(),
        }
    }

    #[test]
    fn create_and_edit_only_update_canonical_registries() {
        let home = temp_home("create-edit");
        let create_request = McpSavePreviewRequest {
            asset_id: None,
            canonical: canonical("filesystem", "first"),
            title: Some("Filesystem".into()),
            description: Some("Local files".into()),
        };
        let generated_at = epoch_seconds();
        let preview = preview_mcp_save_at(&home, &create_request, generated_at).unwrap();
        crate::fingerprint::assert_sha256_preview_id(&preview.preview_id, "mcp-save-");
        let result = apply_mcp_save(
            &home,
            &McpSaveApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: generated_at,
                request: create_request,
            },
        )
        .unwrap();
        assert_eq!(result.operation, McpSaveOperation::Create);
        assert!(load_assets(&home)
            .unwrap()
            .get(AssetKind::Mcp, "filesystem")
            .is_some());
        assert!(!home.join(".claude.json").exists());
        assert!(!home.join(".codex/config.toml").exists());

        let mut mounts = load_mounts(&home).unwrap();
        mounts
            .upsert(
                MountBinding::new("mcp:filesystem", "claude-user-mcp", BindingStatus::Mounted)
                    .unwrap(),
            )
            .unwrap();
        save_mounts(&home, &mounts).unwrap();
        let edit_request = McpSavePreviewRequest {
            asset_id: Some("mcp:filesystem".into()),
            canonical: canonical("filesystem", "second"),
            title: None,
            description: None,
        };
        let generated_at = epoch_seconds();
        let preview = preview_mcp_save_at(&home, &edit_request, generated_at).unwrap();
        assert_eq!(
            preview.out_of_sync_target_ids,
            vec!["claude-user-mcp".to_string()]
        );
        let result = apply_mcp_save(
            &home,
            &McpSaveApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: generated_at,
                request: edit_request,
            },
        )
        .unwrap();
        assert_eq!(result.operation, McpSaveOperation::Edit);
        assert_eq!(
            load_mounts(&home).unwrap().for_asset("mcp:filesystem")[0].status,
            BindingStatus::OutOfSync
        );
        let saved: CanonicalMcp =
            serde_json::from_slice(&fs::read(result.canonical_path).unwrap()).unwrap();
        assert_eq!(saved.spec.command.as_deref(), Some("second"));
        assert!(!home.join(".claude.json").exists());
        assert!(!home.join(".codex/config.toml").exists());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn edit_rejects_name_changes_and_create_rejects_duplicates() {
        let home = temp_home("identity");
        let create = McpSavePreviewRequest {
            asset_id: None,
            canonical: canonical("one", "cmd"),
            title: None,
            description: None,
        };
        let generated_at = epoch_seconds();
        let preview = preview_mcp_save_at(&home, &create, generated_at).unwrap();
        apply_mcp_save(
            &home,
            &McpSaveApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: generated_at,
                request: create.clone(),
            },
        )
        .unwrap();
        assert!(preview_mcp_save(&home, &create)
            .unwrap_err()
            .to_string()
            .contains("already exists"));
        let renamed = McpSavePreviewRequest {
            asset_id: Some("mcp:one".into()),
            canonical: canonical("two", "cmd"),
            title: None,
            description: None,
        };
        assert!(preview_mcp_save(&home, &renamed)
            .unwrap_err()
            .to_string()
            .contains("read-only"));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn codex_incompatible_edit_is_blocked_without_touching_live_config() {
        let home = temp_home("codex-blocked");
        let create = McpSavePreviewRequest {
            asset_id: None,
            canonical: canonical("events", "server"),
            title: None,
            description: None,
        };
        let generated_at = epoch_seconds();
        let preview = preview_mcp_save_at(&home, &create, generated_at).unwrap();
        apply_mcp_save(
            &home,
            &McpSaveApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: generated_at,
                request: create,
            },
        )
        .unwrap();

        let live = home.join(".codex/config.toml");
        fs::create_dir_all(live.parent().unwrap()).unwrap();
        fs::write(&live, "model = \"test\"\n").unwrap();
        let original_live = fs::read(&live).unwrap();
        let mut mounts = load_mounts(&home).unwrap();
        mounts
            .upsert(
                MountBinding::new("mcp:events", "codex-user-mcp", BindingStatus::Mounted).unwrap(),
            )
            .unwrap();
        save_mounts(&home, &mounts).unwrap();

        let mut proposed = canonical("events", "unused");
        proposed.spec.transport = Some(McpTransport::Sse);
        proposed.spec.command = None;
        proposed.spec.url = Some("https://example.invalid/events".into());
        let request = McpSavePreviewRequest {
            asset_id: Some("mcp:events".into()),
            canonical: proposed,
            title: None,
            description: None,
        };
        let preview = preview_mcp_save(&home, &request).unwrap();
        assert!(!preview.can_apply);
        assert_eq!(preview.target_compatibility.len(), 1);
        assert!(preview.target_compatibility[0]
            .blocked_reason
            .as_deref()
            .unwrap()
            .contains("SSE"));
        assert_eq!(fs::read(&live).unwrap(), original_live);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn stale_preview_is_rejected_without_writing() {
        let home = temp_home("stale");
        let request = McpSavePreviewRequest {
            asset_id: None,
            canonical: canonical("filesystem", "first"),
            title: None,
            description: None,
        };
        let generated_at = epoch_seconds();
        let preview = preview_mcp_save_at(&home, &request, generated_at).unwrap();
        let mut changed = request.clone();
        changed.canonical.spec.command = Some("second".into());
        let result = apply_mcp_save(
            &home,
            &McpSaveApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: generated_at,
                request: changed,
            },
        );
        assert!(result.unwrap_err().to_string().contains("stale"));
        assert!(!canonical_path(&home, AssetKind::Mcp, "filesystem").exists());
        assert!(load_assets(&home)
            .unwrap()
            .get(AssetKind::Mcp, "filesystem")
            .is_none());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn mcp_save_recovers_when_process_crashes_after_persisted_step() {
        let home = temp_home("crash-recovery");
        let request = McpSavePreviewRequest {
            asset_id: None,
            canonical: canonical("filesystem", "first"),
            title: Some("Filesystem".into()),
            description: None,
        };
        let generated_at = epoch_seconds();
        let preview = preview_mcp_save_at(&home, &request, generated_at).unwrap();
        let crash = crash_test::crash_after_step("mcp_save", "canonical_mcp_saved");

        let result = catch_unwind(AssertUnwindSafe(|| {
            apply_mcp_save(
                &home,
                &McpSaveApplyRequest {
                    preview_id: preview.preview_id,
                    preview_generated_at_epoch_seconds: generated_at,
                    request,
                },
            )
        }));
        assert!(result.is_err());
        drop(crash);

        assert!(canonical_path(&home, AssetKind::Mcp, "filesystem").exists());
        assert!(load_assets(&home)
            .unwrap()
            .get(AssetKind::Mcp, "filesystem")
            .is_some());
        let report = recover_incomplete(&home).unwrap();
        assert!(report.attempted);
        assert!(!report.writes_blocked);
        assert!(report.attempts[0].recovered);
        assert!(!canonical_path(&home, AssetKind::Mcp, "filesystem").exists());
        assert!(load_assets(&home)
            .unwrap()
            .get(AssetKind::Mcp, "filesystem")
            .is_none());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn explicit_target_sync_compiles_saved_canonical_after_out_of_sync() {
        let home = temp_home("explicit-sync");
        let create = McpSavePreviewRequest {
            asset_id: None,
            canonical: canonical("filesystem", "first"),
            title: None,
            description: None,
        };
        let generated_at = epoch_seconds();
        let preview = preview_mcp_save_at(&home, &create, generated_at).unwrap();
        apply_mcp_save(
            &home,
            &McpSaveApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: generated_at,
                request: create,
            },
        )
        .unwrap();

        let mount_request = MountPreviewRequest {
            asset_id: "mcp:filesystem".into(),
            target_id: "claude-user-mcp".into(),
        };
        let mount_preview = preview_mount(&home, &mount_request).unwrap();
        apply_mount(
            &home,
            &MountApplyRequest {
                preview_id: mount_preview.preview_id,
                preview_generated_at_epoch_seconds: mount_preview.generated_at_epoch_seconds,
                request: mount_request.clone(),
            },
        )
        .unwrap();
        let live_path = home.join(".claude.json");
        let first_live = fs::read_to_string(&live_path).unwrap();
        assert!(first_live.contains("\"command\": \"first\""));

        let edit = McpSavePreviewRequest {
            asset_id: Some("mcp:filesystem".into()),
            canonical: canonical("filesystem", "second"),
            title: None,
            description: None,
        };
        let generated_at = epoch_seconds();
        let preview = preview_mcp_save_at(&home, &edit, generated_at).unwrap();
        apply_mcp_save(
            &home,
            &McpSaveApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: generated_at,
                request: edit,
            },
        )
        .unwrap();
        assert_eq!(fs::read_to_string(&live_path).unwrap(), first_live);
        assert_eq!(
            load_mounts(&home).unwrap().for_asset("mcp:filesystem")[0].status,
            BindingStatus::OutOfSync
        );

        let mount_preview = preview_mount(&home, &mount_request).unwrap();
        apply_mount(
            &home,
            &MountApplyRequest {
                preview_id: mount_preview.preview_id,
                preview_generated_at_epoch_seconds: mount_preview.generated_at_epoch_seconds,
                request: mount_request,
            },
        )
        .unwrap();
        let second_live = fs::read_to_string(&live_path).unwrap();
        assert!(second_live.contains("\"command\": \"second\""));
        assert_eq!(
            load_mounts(&home).unwrap().for_asset("mcp:filesystem")[0].status,
            BindingStatus::Mounted
        );
        let _ = fs::remove_dir_all(home);
    }
}
