use crate::fingerprint::PreviewFingerprint;
use crate::fs_sync::sync_directory;
use crate::mount_registry::{load as load_mounts, registry_path as mount_registry_path};
use crate::operation::{OperationJournal, OperationLock, RecoveryTarget};
use crate::path_safety::{display_path, guard_write_path, is_link_or_junction};
use crate::query::AssetCounts;
use crate::targets::{
    load as load_targets, registry_path as target_registry_path, save as save_targets, MountTarget,
    MountTargetKind, TargetRegistry,
};
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MANAGED_PROJECTS_SCHEMA_VERSION: u32 = 1;
const PREVIEW_TTL_SECONDS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCheckSummary {
    pub checked_at_epoch_seconds: u64,
    pub asset_counts: AssetCounts,
    pub warning_count: u32,
    pub path_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedProject {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub created_at_epoch_seconds: u64,
    pub updated_at_epoch_seconds: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_check: Option<ProjectCheckSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedProjectRegistry {
    pub schema_version: u32,
    #[serde(default)]
    pub projects: Vec<ManagedProject>,
}

impl Default for ManagedProjectRegistry {
    fn default() -> Self {
        Self {
            schema_version: MANAGED_PROJECTS_SCHEMA_VERSION,
            projects: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAddPreviewRequest {
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEditPreviewRequest {
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRemovePreviewRequest {
    pub project_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectChangePreview {
    pub preview_id: String,
    pub operation: String,
    pub project: ManagedProject,
    pub affected_paths: Vec<PathBuf>,
    pub blocking_bindings: Vec<String>,
    pub warnings: Vec<String>,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAddApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: ProjectAddPreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEditApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: ProjectEditPreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRemoveApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: ProjectRemovePreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectChangeResult {
    pub preview_id: String,
    pub operation: String,
    pub project: ManagedProject,
    pub registry_path: PathBuf,
    pub affected_paths: Vec<PathBuf>,
}

pub fn registry_path(home: &Path) -> PathBuf {
    home.join(".my-agent-assets/projects.yaml")
}

pub fn load(home: &Path) -> Result<ManagedProjectRegistry> {
    let path = registry_path(home);
    match fs::read_to_string(&path) {
        Ok(text) => {
            let registry: ManagedProjectRegistry =
                serde_yaml::from_str(&text).map_err(|error| {
                    MaaError::new(format!("invalid managed project registry: {error}"))
                })?;
            validate_registry(&registry)?;
            Ok(registry)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(ManagedProjectRegistry::default())
        }
        Err(error) => Err(MaaError::new(format!(
            "failed to read managed project registry {}: {error}",
            path.display()
        ))),
    }
}

pub fn save(home: &Path, registry: &ManagedProjectRegistry) -> Result<()> {
    validate_registry(registry)?;
    let root = home.join(".my-agent-assets");
    let path = guard_write_path(&root, &registry_path(home))?;
    let parent = path
        .parent()
        .ok_or_else(|| MaaError::new("managed project registry path has no parent"))?;
    fs::create_dir_all(parent)?;
    let yaml = serde_yaml::to_string(registry)
        .map_err(|error| MaaError::new(format!("failed to serialize managed projects: {error}")))?;
    write_atomic(&path, yaml.as_bytes())
}

pub fn preview_add_project(
    home: &Path,
    request: &ProjectAddPreviewRequest,
) -> Result<ProjectChangePreview> {
    preview_add_project_at(home, request, epoch_seconds())
}

pub fn preview_edit_project(
    home: &Path,
    request: &ProjectEditPreviewRequest,
) -> Result<ProjectChangePreview> {
    preview_edit_project_at(home, request, epoch_seconds())
}

pub fn preview_remove_project(
    home: &Path,
    request: &ProjectRemovePreviewRequest,
) -> Result<ProjectChangePreview> {
    preview_remove_project_at(home, request, epoch_seconds())
}

pub fn apply_add_project(
    home: &Path,
    request: &ProjectAddApplyRequest,
) -> Result<ProjectChangeResult> {
    validate_preview_time(request.preview_generated_at_epoch_seconds)?;
    let _lock = OperationLock::acquire(home)?;
    let preview = preview_add_project_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    validate_preview(&request.preview_id, &preview)?;

    let mut registry = load(home)?;
    registry.projects.push(preview.project.clone());
    let mut targets = load_targets(home)?;
    targets.targets.extend(project_targets(&preview.project)?);
    targets.validate()?;
    save_project_state_transaction(home, &registry, &targets, "project_add")?;
    Ok(ProjectChangeResult {
        preview_id: preview.preview_id,
        operation: "add".into(),
        project: preview.project,
        registry_path: registry_path(home),
        affected_paths: vec![registry_path(home), target_registry_path(home)],
    })
}

pub fn apply_edit_project(
    home: &Path,
    request: &ProjectEditApplyRequest,
) -> Result<ProjectChangeResult> {
    validate_preview_time(request.preview_generated_at_epoch_seconds)?;
    let _lock = OperationLock::acquire(home)?;
    let preview = preview_edit_project_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    validate_preview(&request.preview_id, &preview)?;

    let mut registry = load(home)?;
    let project = registry
        .projects
        .iter_mut()
        .find(|project| project.id == request.request.project_id)
        .ok_or_else(|| MaaError::new("managed project no longer exists"))?;
    *project = preview.project.clone();
    let mut targets = load_targets(home)?;
    targets
        .targets
        .retain(|target| !is_generated_project_target(&preview.project.id, &target.id));
    targets.targets.extend(project_targets(&preview.project)?);
    targets.validate()?;
    save_project_state_transaction(home, &registry, &targets, "project_edit")?;
    Ok(ProjectChangeResult {
        preview_id: preview.preview_id,
        operation: "edit".into(),
        project: preview.project,
        registry_path: registry_path(home),
        affected_paths: vec![registry_path(home), target_registry_path(home)],
    })
}

pub fn apply_remove_project(
    home: &Path,
    request: &ProjectRemoveApplyRequest,
) -> Result<ProjectChangeResult> {
    validate_preview_time(request.preview_generated_at_epoch_seconds)?;
    let _lock = OperationLock::acquire(home)?;
    let preview = preview_remove_project_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    validate_preview(&request.preview_id, &preview)?;

    let mut registry = load(home)?;
    registry
        .projects
        .retain(|project| project.id != preview.project.id);
    let mut targets = load_targets(home)?;
    targets
        .targets
        .retain(|target| !is_generated_project_target(&preview.project.id, &target.id));
    targets.validate()?;
    save_project_state_transaction(home, &registry, &targets, "project_remove")?;
    Ok(ProjectChangeResult {
        preview_id: preview.preview_id,
        operation: "remove".into(),
        project: preview.project,
        registry_path: registry_path(home),
        affected_paths: vec![registry_path(home), target_registry_path(home)],
    })
}

/// Persists only user-triggered derived inspection metadata. It never changes
/// assets, mounts, targets, or runtime paths.
pub fn record_check(
    home: &Path,
    project_id: &str,
    summary: ProjectCheckSummary,
) -> Result<ManagedProject> {
    let _lock = OperationLock::acquire(home)?;
    let mut registry = load(home)?;
    let project = registry
        .projects
        .iter_mut()
        .find(|project| project.id == project_id)
        .ok_or_else(|| MaaError::new("managed project not found"))?;
    project.last_check = Some(summary);
    project.updated_at_epoch_seconds = epoch_seconds();
    let updated = project.clone();
    save(home, &registry)?;
    Ok(updated)
}

pub fn project_target_id(project_id: &str, kind: MountTargetKind) -> String {
    let suffix = match kind {
        MountTargetKind::ClaudeProjectSkills => "claude-skills",
        MountTargetKind::CodexProjectSkills => "codex-skills",
        MountTargetKind::ClaudeProjectCommands => "claude-commands",
        MountTargetKind::ClaudeProjectMcpJson => "claude-mcp",
        MountTargetKind::CodexProjectMcpToml => "codex-mcp",
        _ => "invalid",
    };
    format!("managed-{project_id}-{suffix}")
}

pub fn is_generated_project_target(project_id: &str, target_id: &str) -> bool {
    target_id.starts_with(&format!("managed-{project_id}-"))
}

pub fn project_targets(project: &ManagedProject) -> Result<Vec<MountTarget>> {
    [
        MountTargetKind::ClaudeProjectSkills,
        MountTargetKind::CodexProjectSkills,
        MountTargetKind::ClaudeProjectCommands,
        MountTargetKind::ClaudeProjectMcpJson,
        MountTargetKind::CodexProjectMcpToml,
    ]
    .into_iter()
    .map(|kind| {
        MountTarget::project(
            project_target_id(&project.id, kind),
            kind,
            project.path.clone(),
        )
    })
    .collect()
}

fn preview_add_project_at(
    home: &Path,
    request: &ProjectAddPreviewRequest,
    generated_at: u64,
) -> Result<ProjectChangePreview> {
    let registry = load(home)?;
    let path = canonical_project_path(&request.path)?;
    let project = ManagedProject {
        id: project_id(&path),
        name: project_name(&path, request.name.as_deref())?,
        path,
        created_at_epoch_seconds: generated_at,
        updated_at_epoch_seconds: generated_at,
        last_check: None,
    };
    let mut candidate = registry.clone();
    candidate.projects.push(project.clone());
    let validation = validate_registry(&candidate);
    let warnings = validation
        .as_ref()
        .err()
        .map(|error| vec![error.to_string()])
        .unwrap_or_default();
    build_preview(
        home,
        "add",
        project,
        Vec::new(),
        warnings,
        validation.is_ok(),
        generated_at,
    )
}

fn preview_edit_project_at(
    home: &Path,
    request: &ProjectEditPreviewRequest,
    generated_at: u64,
) -> Result<ProjectChangePreview> {
    let registry = load(home)?;
    let existing = registry
        .projects
        .iter()
        .find(|project| project.id == request.project_id)
        .cloned()
        .ok_or_else(|| MaaError::new("managed project not found"))?;
    let blocking_bindings = bindings_for_project(home, &existing.id)?;
    let changing_path = request.path.is_some();
    let path = match request.path.as_deref() {
        Some(path) => canonical_project_path(path)?,
        None => existing.path.clone(),
    };
    let project = ManagedProject {
        name: project_name(&path, request.name.as_deref().or(Some(&existing.name)))?,
        path,
        updated_at_epoch_seconds: generated_at,
        ..existing
    };
    let mut candidate = registry.clone();
    if let Some(entry) = candidate
        .projects
        .iter_mut()
        .find(|entry| entry.id == project.id)
    {
        *entry = project.clone();
    }
    let validation = validate_registry(&candidate);
    let mut warnings = validation
        .as_ref()
        .err()
        .map(|error| vec![error.to_string()])
        .unwrap_or_default();
    if changing_path && !blocking_bindings.is_empty() {
        warnings
            .push("unmount every related asset before changing the managed project path".into());
    }
    build_preview(
        home,
        "edit",
        project,
        blocking_bindings.clone(),
        warnings,
        validation.is_ok() && (!changing_path || blocking_bindings.is_empty()),
        generated_at,
    )
}

fn preview_remove_project_at(
    home: &Path,
    request: &ProjectRemovePreviewRequest,
    generated_at: u64,
) -> Result<ProjectChangePreview> {
    let registry = load(home)?;
    let project = registry
        .projects
        .iter()
        .find(|project| project.id == request.project_id)
        .cloned()
        .ok_or_else(|| MaaError::new("managed project not found"))?;
    let blocking_bindings = bindings_for_project(home, &project.id)?;
    let warnings = if blocking_bindings.is_empty() {
        vec!["removing management does not delete the project directory or canonical assets".into()]
    } else {
        vec!["unmount every related asset before removing this managed project".into()]
    };
    build_preview(
        home,
        "remove",
        project,
        blocking_bindings.clone(),
        warnings,
        blocking_bindings.is_empty(),
        generated_at,
    )
}

fn build_preview(
    home: &Path,
    operation: &str,
    project: ManagedProject,
    blocking_bindings: Vec<String>,
    warnings: Vec<String>,
    can_apply: bool,
    generated_at: u64,
) -> Result<ProjectChangePreview> {
    let preview_id = fingerprint(home, operation, &project, generated_at)?;
    Ok(ProjectChangePreview {
        preview_id,
        operation: operation.into(),
        affected_paths: vec![registry_path(home), target_registry_path(home)],
        project,
        blocking_bindings,
        warnings,
        can_apply,
        generated_at_epoch_seconds: generated_at,
        expires_at_epoch_seconds: generated_at.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

fn bindings_for_project(home: &Path, project_id: &str) -> Result<Vec<String>> {
    let mounts = match load_mounts(home) {
        Ok(mounts) => mounts,
        Err(_error) if !mount_registry_path(home).exists() => return Ok(Vec::new()),
        Err(error) => return Err(MaaError::new(error.to_string())),
    };
    let mut bindings = mounts
        .bindings
        .values()
        .filter(|binding| is_generated_project_target(project_id, &binding.target_id))
        .map(|binding| format!("{} -> {}", binding.asset_id, binding.target_id))
        .collect::<Vec<_>>();
    bindings.sort();
    Ok(bindings)
}

fn save_project_state_transaction(
    home: &Path,
    projects: &ManagedProjectRegistry,
    targets: &TargetRegistry,
    operation: &str,
) -> Result<()> {
    let mut journal = OperationJournal::start_recoverable(
        home,
        &format!("{operation}-{}", epoch_nanos()),
        operation,
        vec![
            RecoveryTarget::asset_center(registry_path(home)),
            RecoveryTarget::asset_center(target_registry_path(home)),
        ],
    )?;
    let result = (|| -> Result<()> {
        save(home, projects)?;
        save_targets(home, targets)?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            journal.record_step("managed_project_registry_saved")?;
            journal.complete()
        }
        Err(error) => {
            let original = error.to_string();
            journal.rollback_now(home).map_err(|rollback| {
                MaaError::new(format!(
                    "{original}; managed project rollback failed: {rollback}"
                ))
            })?;
            Err(error)
        }
    }
}

fn validate_registry(registry: &ManagedProjectRegistry) -> Result<()> {
    if registry.schema_version != MANAGED_PROJECTS_SCHEMA_VERSION {
        return Err(MaaError::new(format!(
            "unsupported managed project registry schemaVersion: {}",
            registry.schema_version
        )));
    }
    for (index, project) in registry.projects.iter().enumerate() {
        if project.id.trim().is_empty()
            || project.name.trim().is_empty()
            || !project.path.is_absolute()
        {
            return Err(MaaError::new(
                "managed project contains an invalid id, name, or path",
            ));
        }
        for other in registry.projects.iter().skip(index + 1) {
            if project.id == other.id {
                return Err(MaaError::new("managed project IDs must be unique"));
            }
            if paths_overlap(&project.path, &other.path) {
                return Err(MaaError::new(format!(
                    "managed project paths overlap: {} and {}",
                    display_path(&project.path),
                    display_path(&other.path)
                )));
            }
        }
    }
    Ok(())
}

fn canonical_project_path(path: &Path) -> Result<PathBuf> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        MaaError::new(format!(
            "managed project directory is unavailable: {}: {error}",
            path.display()
        ))
    })?;
    let metadata = fs::symlink_metadata(&canonical)?;
    if !metadata.is_dir() || is_link_or_junction(&metadata) {
        return Err(MaaError::new(
            "managed project path must be a real directory",
        ));
    }
    Ok(canonical)
}

fn project_name(path: &Path, requested: Option<&str>) -> Result<String> {
    let name = requested
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
        })
        .ok_or_else(|| MaaError::new("managed project needs a display name"))?;
    if name.len() > 120 || name.contains(['\n', '\r']) {
        return Err(MaaError::new("managed project display name is invalid"));
    }
    Ok(name)
}

fn project_id(path: &Path) -> String {
    let digest = Sha256::digest(path.to_string_lossy().as_bytes());
    let encoded = digest
        .iter()
        .take(12)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("project-{encoded}")
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    left.starts_with(right) || right.starts_with(left)
}

fn fingerprint(
    home: &Path,
    operation: &str,
    project: &ManagedProject,
    generated_at: u64,
) -> Result<String> {
    let mut fingerprint = PreviewFingerprint::new("managed-project");
    fingerprint.add_bytes("operation", operation.as_bytes());
    fingerprint.add_bytes("project-id", project.id.as_bytes());
    fingerprint.add_bytes("project-name", project.name.as_bytes());
    fingerprint.add_bytes("project-path", project.path.to_string_lossy().as_bytes());
    fingerprint.add_u64("generated-at", generated_at);
    fingerprint.add_path_if_present("project-registry", &registry_path(home))?;
    fingerprint.add_path_if_present("target-registry", &target_registry_path(home))?;
    fingerprint.add_path_if_present("mount-registry", &mount_registry_path(home))?;
    Ok(fingerprint.finish("managed-project"))
}

fn validate_preview_time(generated_at: u64) -> Result<()> {
    if epoch_seconds() > generated_at.saturating_add(PREVIEW_TTL_SECONDS) {
        return Err(MaaError::new(
            "managed project preview expired; generate a new preview",
        ));
    }
    Ok(())
}

fn validate_preview(expected: &str, preview: &ProjectChangePreview) -> Result<()> {
    if expected != preview.preview_id {
        return Err(MaaError::new(
            "managed project preview is stale; generate a new preview",
        ));
    }
    if !preview.can_apply {
        return Err(MaaError::new(
            preview
                .warnings
                .first()
                .cloned()
                .unwrap_or_else(|| "managed project change is blocked".into()),
        ));
    }
    Ok(())
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

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| MaaError::new("managed project path has no parent"))?;
    let temporary = parent.join(format!(
        ".projects.yaml.tmp-{}-{}",
        std::process::id(),
        epoch_nanos()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        sync_directory(parent)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(MaaError::new(format!(
            "failed to save managed projects: {error}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mount_registry::{save as save_mounts, BindingStatus, MountBinding, MountRegistry};
    use crate::targets::{directory_mount_adapter, ProviderState};

    fn home(label: &str) -> PathBuf {
        let home =
            std::env::temp_dir().join(format!("maa-managed-project-{label}-{}", epoch_nanos()));
        fs::create_dir_all(home.join(".my-agent-assets")).unwrap();
        let targets = TargetRegistry::standard_user_targets(
            &home,
            ProviderState::Initialized,
            ProviderState::Initialized,
            directory_mount_adapter(),
        )
        .unwrap();
        save_targets(&home, &targets).unwrap();
        save_mounts(&home, &MountRegistry::default()).unwrap();
        home
    }

    #[test]
    fn adds_project_with_hidden_targets_and_rejects_overlaps() {
        let home = home("add");
        let project_path = home.join("workspace/project-a");
        fs::create_dir_all(&project_path).unwrap();
        let request = ProjectAddPreviewRequest {
            path: project_path.clone(),
            name: None,
        };
        let preview = preview_add_project(&home, &request).unwrap();
        assert!(preview.can_apply);
        let result = apply_add_project(
            &home,
            &ProjectAddApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap();
        assert_eq!(result.project.name, "project-a");
        assert_eq!(load(&home).unwrap().projects.len(), 1);
        let targets = load_targets(&home).unwrap();
        assert_eq!(
            targets
                .targets
                .iter()
                .filter(|target| is_generated_project_target(&result.project.id, &target.id))
                .count(),
            5
        );

        let nested = home.join("workspace/project-a/packages/app");
        fs::create_dir_all(&nested).unwrap();
        let overlap = preview_add_project(
            &home,
            &ProjectAddPreviewRequest {
                path: nested,
                name: None,
            },
        )
        .unwrap();
        assert!(!overlap.can_apply);
        assert!(overlap
            .warnings
            .iter()
            .any(|warning| warning.contains("overlap")));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn path_edit_and_remove_require_manual_unmount() {
        let home = home("bindings");
        let project_path = home.join("workspace/project-a");
        fs::create_dir_all(&project_path).unwrap();
        let request = ProjectAddPreviewRequest {
            path: project_path,
            name: None,
        };
        let preview = preview_add_project(&home, &request).unwrap();
        let project = apply_add_project(
            &home,
            &ProjectAddApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap()
        .project;
        let target_id = project_target_id(&project.id, MountTargetKind::ClaudeProjectSkills);
        let mut mounts = MountRegistry::default();
        mounts
            .upsert(MountBinding::new("skill:review", target_id, BindingStatus::Mounted).unwrap())
            .unwrap();
        save_mounts(&home, &mounts).unwrap();

        let new_path = home.join("workspace/project-b");
        fs::create_dir_all(&new_path).unwrap();
        let edit = preview_edit_project(
            &home,
            &ProjectEditPreviewRequest {
                project_id: project.id.clone(),
                name: None,
                path: Some(new_path),
            },
        )
        .unwrap();
        assert!(!edit.can_apply);
        assert_eq!(edit.blocking_bindings.len(), 1);
        let remove = preview_remove_project(
            &home,
            &ProjectRemovePreviewRequest {
                project_id: project.id,
            },
        )
        .unwrap();
        assert!(!remove.can_apply);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn records_derived_check_without_touching_targets_or_mounts() {
        let home = home("check");
        let path = home.join("workspace/project-a");
        fs::create_dir_all(&path).unwrap();
        let request = ProjectAddPreviewRequest { path, name: None };
        let preview = preview_add_project(&home, &request).unwrap();
        let project = apply_add_project(
            &home,
            &ProjectAddApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap()
        .project;
        let targets_before = fs::read(target_registry_path(&home)).unwrap();
        let mounts_before = fs::read(mount_registry_path(&home)).unwrap();
        let recorded = record_check(
            &home,
            &project.id,
            ProjectCheckSummary {
                checked_at_epoch_seconds: epoch_seconds(),
                asset_counts: AssetCounts {
                    total: 2,
                    skills: 1,
                    commands: 0,
                    mcps: 1,
                },
                warning_count: 1,
                path_available: true,
            },
        )
        .unwrap();
        assert_eq!(recorded.last_check.unwrap().asset_counts.total, 2);
        assert_eq!(
            fs::read(target_registry_path(&home)).unwrap(),
            targets_before
        );
        assert_eq!(fs::read(mount_registry_path(&home)).unwrap(), mounts_before);
        let _ = fs::remove_dir_all(home);
    }
}
