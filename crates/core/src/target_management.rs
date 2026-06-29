use crate::mount_registry::load as load_mounts;
use crate::operation::OperationLock;
use crate::targets::{load as load_targets, registry_path, save as save_targets, MountTarget};
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PREVIEW_TTL_SECONDS: u64 = 300;
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetAddPreviewRequest {
    pub target: MountTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetRemovePreviewRequest {
    pub target_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetChangePreview {
    pub preview_id: String,
    pub operation: String,
    pub target: MountTarget,
    pub affected_paths: Vec<PathBuf>,
    pub blocking_bindings: Vec<String>,
    pub warnings: Vec<String>,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetAddApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: TargetAddPreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetRemoveApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: TargetRemovePreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetChangeResult {
    pub preview_id: String,
    pub operation: String,
    pub target_id: String,
    pub registry_path: PathBuf,
    pub backup_path: PathBuf,
}

pub fn preview_add_target(
    home: &Path,
    request: &TargetAddPreviewRequest,
) -> Result<TargetChangePreview> {
    preview_add_target_at(home, request, epoch_seconds())
}

pub fn preview_remove_target(
    home: &Path,
    request: &TargetRemovePreviewRequest,
) -> Result<TargetChangePreview> {
    preview_remove_target_at(home, request, epoch_seconds())
}

pub fn apply_add_target(
    home: &Path,
    request: &TargetAddApplyRequest,
) -> Result<TargetChangeResult> {
    validate_preview_time(request.preview_generated_at_epoch_seconds)?;
    let _lock = OperationLock::acquire(home)?;
    let preview = preview_add_target_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    validate_apply_preview(&request.preview_id, &preview)?;

    let mut registry = load_targets(home)?;
    registry.targets.push(request.request.target.clone());
    registry.validate()?;
    let backup_path = backup_registry(home)?;
    save_targets(home, &registry)?;
    Ok(TargetChangeResult {
        preview_id: preview.preview_id,
        operation: "add".into(),
        target_id: request.request.target.id.clone(),
        registry_path: registry_path(home),
        backup_path,
    })
}

pub fn apply_remove_target(
    home: &Path,
    request: &TargetRemoveApplyRequest,
) -> Result<TargetChangeResult> {
    validate_preview_time(request.preview_generated_at_epoch_seconds)?;
    let _lock = OperationLock::acquire(home)?;
    let preview = preview_remove_target_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    validate_apply_preview(&request.preview_id, &preview)?;

    let mut registry = load_targets(home)?;
    registry
        .targets
        .retain(|target| target.id != request.request.target_id);
    registry.validate()?;
    let backup_path = backup_registry(home)?;
    save_targets(home, &registry)?;
    Ok(TargetChangeResult {
        preview_id: preview.preview_id,
        operation: "remove".into(),
        target_id: request.request.target_id.clone(),
        registry_path: registry_path(home),
        backup_path,
    })
}

fn preview_add_target_at(
    home: &Path,
    request: &TargetAddPreviewRequest,
    generated_at: u64,
) -> Result<TargetChangePreview> {
    request.target.validate()?;
    let registry = load_targets(home)?;
    let mut candidate = registry.clone();
    candidate.targets.push(request.target.clone());
    let validation = candidate.validate();
    let warnings = validation
        .as_ref()
        .err()
        .map(|error| vec![error.to_string()])
        .unwrap_or_default();
    build_preview(
        home,
        "add",
        request.target.clone(),
        Vec::new(),
        warnings,
        validation.is_ok(),
        generated_at,
    )
}

fn preview_remove_target_at(
    home: &Path,
    request: &TargetRemovePreviewRequest,
    generated_at: u64,
) -> Result<TargetChangePreview> {
    let registry = load_targets(home)?;
    let target = registry.resolve(&request.target_id)?.clone();
    let mounts = load_mounts(home).map_err(|error| MaaError::new(error.to_string()))?;
    let mut blocking_bindings = mounts
        .bindings
        .values()
        .filter(|binding| binding.target_id == request.target_id)
        .map(|binding| binding.asset_id.clone())
        .collect::<Vec<_>>();
    blocking_bindings.sort();
    let warnings = if blocking_bindings.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "target '{}' still has {} mount binding(s); unmount them before removal",
            request.target_id,
            blocking_bindings.len()
        )]
    };
    build_preview(
        home,
        "remove",
        target,
        blocking_bindings.clone(),
        warnings,
        blocking_bindings.is_empty(),
        generated_at,
    )
}

fn build_preview(
    home: &Path,
    operation: &str,
    target: MountTarget,
    blocking_bindings: Vec<String>,
    warnings: Vec<String>,
    can_apply: bool,
    generated_at: u64,
) -> Result<TargetChangePreview> {
    let registry_text = fs::read_to_string(registry_path(home))?;
    let request_text = serde_json::to_string(&(operation, &target, &blocking_bindings))
        .map_err(|error| MaaError::new(error.to_string()))?;
    let mut hasher = DefaultHasher::new();
    registry_text.hash(&mut hasher);
    request_text.hash(&mut hasher);
    generated_at.hash(&mut hasher);
    let preview_id = format!("target-{operation}-{:016x}", hasher.finish());
    Ok(TargetChangePreview {
        preview_id,
        operation: operation.into(),
        affected_paths: vec![registry_path(home), target.path.clone()],
        target,
        blocking_bindings,
        warnings,
        can_apply,
        generated_at_epoch_seconds: generated_at,
        expires_at_epoch_seconds: generated_at.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

fn validate_apply_preview(preview_id: &str, preview: &TargetChangePreview) -> Result<()> {
    if preview_id != preview.preview_id {
        return Err(MaaError::new(
            "target registry changed after preview; generate a new preview",
        ));
    }
    if !preview.can_apply {
        return Err(MaaError::new(
            preview
                .warnings
                .first()
                .cloned()
                .unwrap_or_else(|| "target change is blocked".into()),
        ));
    }
    Ok(())
}

fn validate_preview_time(generated_at: u64) -> Result<()> {
    let now = epoch_seconds();
    if generated_at > now.saturating_add(5)
        || now.saturating_sub(generated_at) > PREVIEW_TTL_SECONDS
    {
        return Err(MaaError::new(
            "target preview expired; generate a new preview",
        ));
    }
    Ok(())
}

fn backup_registry(home: &Path) -> Result<PathBuf> {
    let id = format!(
        "target-registry-{}-{}",
        epoch_nanos(),
        OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let backup = home
        .join(".my-agent-assets/backups/local")
        .join(id)
        .join("targets.yaml");
    let parent = backup
        .parent()
        .ok_or_else(|| MaaError::new("target registry backup path has no parent"))?;
    fs::create_dir_all(parent)?;
    fs::copy(registry_path(home), &backup)?;
    Ok(backup)
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn epoch_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mount_registry::{save as save_mounts, BindingStatus, MountBinding, MountRegistry};
    use crate::targets::{MountAdapter, MountTargetKind, ProviderState, TargetRegistry};

    fn home(label: &str) -> PathBuf {
        let home =
            std::env::temp_dir().join(format!("maa-target-management-{label}-{}", epoch_nanos()));
        fs::create_dir_all(home.join(".my-agent-assets/backups/local")).unwrap();
        let registry = TargetRegistry::standard_user_targets(
            &home,
            ProviderState::Initialized,
            ProviderState::Initialized,
            if cfg!(windows) {
                MountAdapter::WindowsDirectoryJunction
            } else {
                MountAdapter::SymlinkDirectory
            },
        )
        .unwrap();
        save_targets(&home, &registry).unwrap();
        save_mounts(&home, &MountRegistry::default()).unwrap();
        home
    }

    #[test]
    fn project_target_add_requires_preview_and_creates_backup() {
        let home = home("add");
        let project = home.join("workspace/project-a");
        fs::create_dir_all(&project).unwrap();
        let target = MountTarget::project(
            "project-a-claude-skills",
            MountTargetKind::ClaudeProjectSkills,
            project,
        )
        .unwrap();
        let request = TargetAddPreviewRequest { target };
        let preview = preview_add_target(&home, &request).unwrap();
        assert!(preview.can_apply);
        let result = apply_add_target(
            &home,
            &TargetAddApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap();
        assert!(result.backup_path.exists());
        assert!(load_targets(&home)
            .unwrap()
            .resolve("project-a-claude-skills")
            .is_ok());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn target_remove_is_blocked_until_bindings_are_removed() {
        let home = home("remove");
        let mut mounts = MountRegistry::default();
        mounts
            .upsert(
                MountBinding::new("skill:review", "claude-user-skills", BindingStatus::Mounted)
                    .unwrap(),
            )
            .unwrap();
        save_mounts(&home, &mounts).unwrap();
        let request = TargetRemovePreviewRequest {
            target_id: "claude-user-skills".into(),
        };
        let blocked = preview_remove_target(&home, &request).unwrap();
        assert!(!blocked.can_apply);
        assert_eq!(blocked.blocking_bindings, vec!["skill:review"]);

        save_mounts(&home, &MountRegistry::default()).unwrap();
        let preview = preview_remove_target(&home, &request).unwrap();
        apply_remove_target(
            &home,
            &TargetRemoveApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap();
        assert!(load_targets(&home)
            .unwrap()
            .resolve("claude-user-skills")
            .is_err());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn stale_registry_blocks_target_apply() {
        let home = home("stale");
        let request = TargetAddPreviewRequest {
            target: MountTarget::custom(
                "custom-skills",
                MountTargetKind::CustomSkillDirectory,
                home.join("custom/skills"),
            )
            .unwrap(),
        };
        let preview = preview_add_target(&home, &request).unwrap();
        let mut registry = load_targets(&home).unwrap();
        registry.targets.push(
            MountTarget::custom(
                "other-skills",
                MountTargetKind::CustomSkillDirectory,
                home.join("other/skills"),
            )
            .unwrap(),
        );
        save_targets(&home, &registry).unwrap();
        let error = apply_add_target(
            &home,
            &TargetAddApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("changed after preview"));
        let _ = fs::remove_dir_all(home);
    }
}
