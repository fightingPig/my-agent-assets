use crate::fingerprint::PreviewFingerprint;
use crate::mount_registry::load as load_mounts;
use crate::operation::{OperationJournal, OperationLock, RecoveryTarget};
use crate::targets::{load as load_targets, save as save_targets, MountTarget, TargetRegistry};
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROJECT_REGISTRY_SCHEMA_VERSION: u32 = 1;
const PREVIEW_TTL_SECONDS: u64 = 300;
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedProject {
    pub id: String,
    pub name: String,
    pub title: String,
    pub path: PathBuf,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRegistry {
    pub schema_version: u32,
    #[serde(default)]
    pub projects: Vec<ManagedProject>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSaveRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub title: String,
    pub path: PathBuf,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRemoveRequest {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectChangePreview {
    pub preview_id: String,
    pub operation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ManagedProject>,
    pub affected_paths: Vec<PathBuf>,
    pub migrated_target_ids: Vec<String>,
    pub blocking_bindings: Vec<String>,
    pub warnings: Vec<String>,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSaveApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: ProjectSaveRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRemoveApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: ProjectRemoveRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectChangeResult {
    pub preview_id: String,
    pub operation: String,
    pub project_id: String,
    pub registry_path: PathBuf,
    pub affected_paths: Vec<PathBuf>,
}

impl Default for ProjectRegistry {
    fn default() -> Self {
        Self {
            schema_version: PROJECT_REGISTRY_SCHEMA_VERSION,
            projects: Vec::new(),
        }
    }
}

impl ProjectRegistry {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != PROJECT_REGISTRY_SCHEMA_VERSION {
            return Err(MaaError::new(format!(
                "unsupported project registry schemaVersion: {}",
                self.schema_version
            )));
        }
        let mut ids = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for project in &self.projects {
            validate_project(&project)?;
            if !ids.insert(project.id.clone()) {
                return Err(MaaError::new(format!(
                    "duplicate managed project id: {}",
                    project.id
                )));
            }
            if !paths.insert(path_key(&project.path)) {
                return Err(MaaError::new(format!(
                    "duplicate managed project path: {}",
                    project.path.display()
                )));
            }
        }
        Ok(())
    }

    fn find(&self, id: &str) -> Result<&ManagedProject> {
        validate_id(id, "project id")?;
        self.projects
            .iter()
            .find(|project| project.id == id)
            .ok_or_else(|| MaaError::new(format!("unknown managed project id: {id}")))
    }
}

pub fn registry_path(home: &Path) -> PathBuf {
    home.join(".my-agent-assets/projects.yaml")
}

pub fn load(home: &Path) -> Result<ProjectRegistry> {
    let path = registry_path(home);
    if !path.exists() {
        return Ok(ProjectRegistry::default());
    }
    let content = fs::read_to_string(&path).map_err(|error| {
        MaaError::new(format!(
            "failed to read project registry {}: {error}",
            path.display()
        ))
    })?;
    let registry: ProjectRegistry = serde_yaml::from_str(&content)
        .map_err(|error| MaaError::new(format!("invalid project registry YAML: {error}")))?;
    registry.validate()?;
    Ok(registry)
}

pub fn save(home: &Path, registry: &ProjectRegistry) -> Result<()> {
    registry.validate()?;
    let path = registry_path(home);
    write_atomic(
        &path,
        serde_yaml::to_string(registry)
            .map_err(|error| MaaError::new(error.to_string()))?
            .as_bytes(),
    )
}

pub fn preview_save_project(
    home: &Path,
    request: &ProjectSaveRequest,
) -> Result<ProjectChangePreview> {
    preview_save_project_at(home, request, epoch_seconds())
}

pub fn preview_remove_project(
    home: &Path,
    request: &ProjectRemoveRequest,
) -> Result<ProjectChangePreview> {
    preview_remove_project_at(home, request, epoch_seconds())
}

pub fn apply_save_project(
    home: &Path,
    request: &ProjectSaveApplyRequest,
) -> Result<ProjectChangeResult> {
    validate_preview_time(request.preview_generated_at_epoch_seconds)?;
    let _lock = OperationLock::acquire(home)?;
    let preview = preview_save_project_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    validate_preview(&request.preview_id, &preview)?;

    let mut registry = load(home)?;
    let project = project_from_request(home, &request.request)?;
    let old = request.request.id().and_then(|id| {
        registry
            .projects
            .iter()
            .find(|entry| entry.id == id)
            .cloned()
    });
    if let Some(old) = &old {
        registry.projects.retain(|entry| entry.id != old.id);
    }
    registry.projects.push(project.clone());
    registry
        .projects
        .sort_by(|left, right| left.path.cmp(&right.path));
    registry.validate()?;

    let mut targets = load_targets(home)?;
    if let Some(old) = &old {
        if old.path != project.path {
            migrate_project_targets(&mut targets, &old.path, &project.path)?;
        }
    }
    commit_change(
        home,
        "project_save",
        &registry,
        &targets,
        &preview,
        project.id,
    )
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
    let project = preview
        .project
        .clone()
        .ok_or_else(|| MaaError::new("remove preview has no project"))?;
    let mut registry = load(home)?;
    registry.projects.retain(|entry| entry.id != project.id);
    registry.validate()?;
    let mut targets = load_targets(home)?;
    targets
        .targets
        .retain(|target| target.project_path.as_deref() != Some(project.path.as_path()));
    targets.validate()?;
    commit_change(
        home,
        "project_remove",
        &registry,
        &targets,
        &preview,
        project.id,
    )
}

fn preview_save_project_at(
    home: &Path,
    request: &ProjectSaveRequest,
    generated_at: u64,
) -> Result<ProjectChangePreview> {
    let registry = load(home)?;
    let project = project_from_request(home, request)?;
    let mut candidate = registry.clone();
    let previous = request
        .id()
        .map(|id| registry.find(id).cloned())
        .transpose()?;
    if let Some(previous) = &previous {
        candidate.projects.retain(|entry| entry.id != previous.id);
    }
    candidate.projects.push(project.clone());
    let mut warnings = Vec::new();
    if let Err(error) = candidate.validate() {
        warnings.push(error.to_string());
    }
    let targets = load_targets(home)?;
    let migrated_target_ids = previous
        .as_ref()
        .filter(|old| old.path != project.path)
        .map(|old| project_target_ids(&targets, &old.path))
        .unwrap_or_default();
    let blocking_bindings = blocking_bindings(home, &migrated_target_ids)?;
    if !blocking_bindings.is_empty() {
        warnings.push("project path cannot change while mount bindings exist; unmount or migrate bindings first".into());
    }
    let can_apply = warnings.is_empty();
    build_preview(
        home,
        "save",
        Some(project),
        migrated_target_ids,
        blocking_bindings,
        warnings,
        can_apply,
        generated_at,
    )
}

fn preview_remove_project_at(
    home: &Path,
    request: &ProjectRemoveRequest,
    generated_at: u64,
) -> Result<ProjectChangePreview> {
    let registry = load(home)?;
    let project = registry.find(&request.id)?.clone();
    let targets = load_targets(home)?;
    let target_ids = project_target_ids(&targets, &project.path);
    let blocking_bindings = blocking_bindings(home, &target_ids)?;
    let warnings = if blocking_bindings.is_empty() {
        vec!["removing a managed project deletes only local registry records; the project directory is preserved".into()]
    } else {
        vec!["project still has mount bindings; unmount or migrate bindings before removal".into()]
    };
    build_preview(
        home,
        "remove",
        Some(project),
        target_ids,
        blocking_bindings.clone(),
        warnings,
        blocking_bindings.is_empty(),
        generated_at,
    )
}

fn build_preview(
    home: &Path,
    operation: &str,
    project: Option<ManagedProject>,
    migrated_target_ids: Vec<String>,
    blocking_bindings: Vec<String>,
    warnings: Vec<String>,
    can_apply: bool,
    generated_at: u64,
) -> Result<ProjectChangePreview> {
    let mut fingerprint = PreviewFingerprint::new("project-change");
    fingerprint.add_bytes(
        "request",
        &serde_json::to_vec(&(
            operation,
            &project,
            &migrated_target_ids,
            &blocking_bindings,
        ))
        .map_err(|error| MaaError::new(error.to_string()))?,
    );
    fingerprint.add_u64("generated-at", generated_at);
    fingerprint.add_path_if_present("project-registry", &registry_path(home))?;
    fingerprint.add_path_if_present("target-registry", &crate::targets::registry_path(home))?;
    if let Some(project) = &project {
        fingerprint.add_bytes("project-path", project.path.to_string_lossy().as_bytes());
        let metadata = fs::metadata(&project.path).map_err(|error| {
            MaaError::new(format!("project path is no longer readable: {error}"))
        })?;
        fingerprint.add_u64("project-path-modified", modified_seconds(&metadata));
    }
    Ok(ProjectChangePreview {
        preview_id: fingerprint.finish(&format!("project-{operation}")),
        operation: operation.into(),
        project,
        affected_paths: vec![registry_path(home), crate::targets::registry_path(home)],
        migrated_target_ids,
        blocking_bindings,
        warnings,
        can_apply,
        generated_at_epoch_seconds: generated_at,
        expires_at_epoch_seconds: generated_at.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

fn commit_change(
    home: &Path,
    operation: &str,
    registry: &ProjectRegistry,
    targets: &TargetRegistry,
    preview: &ProjectChangePreview,
    project_id: String,
) -> Result<ProjectChangeResult> {
    let mut journal = OperationJournal::start_recoverable(
        home,
        &format!(
            "{operation}-{}-{}",
            epoch_nanos(),
            OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed)
        ),
        operation,
        vec![
            RecoveryTarget::asset_center(registry_path(home)),
            RecoveryTarget::asset_center(crate::targets::registry_path(home)),
        ],
    )?;
    let result: Result<ProjectChangeResult> = (|| {
        save(home, registry)?;
        save_targets(home, targets)?;
        journal.record_step("project_registry_saved")?;
        journal.complete()?;
        Ok(ProjectChangeResult {
            preview_id: preview.preview_id.clone(),
            operation: preview.operation.clone(),
            project_id,
            registry_path: registry_path(home),
            affected_paths: preview.affected_paths.clone(),
        })
    })();
    if let Err(error) = result {
        let original = error.to_string();
        journal.rollback_now(home).map_err(|rollback| {
            MaaError::new(format!(
                "{original}; project registry rollback failed: {rollback}"
            ))
        })?;
        return Err(error);
    }
    result
}

fn migrate_project_targets(
    targets: &mut TargetRegistry,
    old_path: &Path,
    new_path: &Path,
) -> Result<()> {
    for target in &mut targets.targets {
        if target.project_path.as_deref() == Some(old_path) {
            *target = MountTarget::project(target.id.clone(), target.kind, new_path.to_path_buf())?;
        }
    }
    targets.validate()
}

fn project_target_ids(targets: &TargetRegistry, path: &Path) -> Vec<String> {
    targets
        .targets
        .iter()
        .filter(|target| target.project_path.as_deref() == Some(path))
        .map(|target| target.id.clone())
        .collect()
}

fn blocking_bindings(home: &Path, target_ids: &[String]) -> Result<Vec<String>> {
    let mounts = load_mounts(home).map_err(|error| MaaError::new(error.to_string()))?;
    let ids = target_ids.iter().collect::<BTreeSet<_>>();
    let mut bindings = mounts
        .bindings
        .values()
        .filter(|binding| ids.contains(&binding.target_id))
        .map(|binding| binding.asset_id.clone())
        .collect::<Vec<_>>();
    bindings.sort();
    bindings.dedup();
    Ok(bindings)
}

fn project_from_request(home: &Path, request: &ProjectSaveRequest) -> Result<ManagedProject> {
    let path = canonical_project_path(home, &request.path)?;
    let name = request.name.trim();
    if name.is_empty() {
        return Err(MaaError::new("project name must not be empty"));
    }
    let id = request
        .id
        .clone()
        .unwrap_or_else(|| new_project_id(name, &path));
    let project = ManagedProject {
        id,
        name: name.into(),
        title: if request.title.trim().is_empty() {
            name.into()
        } else {
            request.title.trim().into()
        },
        path,
        description: request.description.trim().into(),
    };
    validate_project(&project)?;
    Ok(project)
}

fn new_project_id(name: &str, path: &Path) -> String {
    let slug = name
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() {
                (byte as char).to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let slug = slug.trim_matches('-');
    let mut hash = Sha256::new();
    hash.update(path.to_string_lossy().as_bytes());
    let digest = hash.finalize();
    let suffix = digest[..4]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!(
        "project-{}-{suffix}",
        if slug.is_empty() { "local" } else { slug }
    )
}

impl ProjectSaveRequest {
    fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
}

fn canonical_project_path(home: &Path, path: &Path) -> Result<PathBuf> {
    let expanded = if path == Path::new("~") {
        home.to_path_buf()
    } else if let Ok(rest) = path.strip_prefix("~") {
        home.join(rest)
    } else {
        path.to_path_buf()
    };
    let metadata = fs::symlink_metadata(&expanded).map_err(|error| {
        MaaError::new(format!(
            "project path must exist: {} ({error})",
            expanded.display()
        ))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(MaaError::new("project path must not be a symlink"));
    }
    let canonical =
        fs::canonicalize(&expanded).map_err(|error| MaaError::new(error.to_string()))?;
    if !canonical.is_dir() {
        return Err(MaaError::new("project path must be a directory"));
    }
    Ok(canonical)
}

fn validate_project(project: &ManagedProject) -> Result<()> {
    validate_id(&project.id, "project id")?;
    if project.name.trim().is_empty() || project.name.len() > 160 {
        return Err(MaaError::new("project name must be 1-160 characters"));
    }
    if project.title.len() > 200 || project.description.len() > 2000 {
        return Err(MaaError::new(
            "project metadata exceeds the supported length",
        ));
    }
    if !project.path.is_absolute()
        || project.path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(MaaError::new(
            "project path must be an absolute normalized path",
        ));
    }
    Ok(())
}

fn validate_id(value: &str, label: &str) -> Result<()> {
    if !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        Ok(())
    } else {
        Err(MaaError::new(format!("unsafe {label}: {value:?}")))
    }
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_owned()
}

fn validate_preview_time(generated_at: u64) -> Result<()> {
    let now = epoch_seconds();
    if generated_at > now.saturating_add(5)
        || now.saturating_sub(generated_at) > PREVIEW_TTL_SECONDS
    {
        return Err(MaaError::new(
            "project preview expired; generate a new preview",
        ));
    }
    Ok(())
}

fn validate_preview(preview_id: &str, preview: &ProjectChangePreview) -> Result<()> {
    if preview_id != preview.preview_id {
        return Err(MaaError::new(
            "project registry changed after preview; generate a new preview",
        ));
    }
    if !preview.can_apply {
        return Err(MaaError::new(
            preview
                .warnings
                .first()
                .cloned()
                .unwrap_or_else(|| "project change is blocked".into()),
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

fn modified_seconds(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_secs())
        .unwrap_or_default()
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
        .ok_or_else(|| MaaError::new("project registry path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".projects.yaml.tmp-{}-{}",
        std::process::id(),
        epoch_nanos()
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        OpenOptions::new().read(true).open(parent)?.sync_all()
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(MaaError::new(format!(
            "failed to save project registry: {error}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::initialization::{
        apply_initialization, preview_initialization, InitializationApplyRequest,
    };
    use crate::mount::{apply_mount, preview_mount, MountApplyRequest, MountPreviewRequest};

    fn home(label: &str) -> PathBuf {
        let home =
            std::env::temp_dir().join(format!("maa-project-registry-{label}-{}", epoch_nanos()));
        fs::create_dir_all(home.join("workspace/project-a")).unwrap();
        let preview = preview_initialization(&home).unwrap();
        apply_initialization(
            &home,
            &InitializationApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
            },
        )
        .unwrap();
        home
    }

    fn save_request(path: PathBuf) -> ProjectSaveRequest {
        ProjectSaveRequest {
            id: None,
            name: "project-a".into(),
            title: "Project A".into(),
            path,
            description: "managed locally".into(),
        }
    }

    #[test]
    fn explicit_projects_are_saved_and_missing_registry_is_empty() {
        let home = home("save");
        let request = save_request(home.join("workspace/project-a"));
        let preview = preview_save_project(&home, &request).unwrap();
        assert!(preview.can_apply);
        let result = apply_save_project(
            &home,
            &ProjectSaveApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap();
        assert!(result.registry_path.is_file());
        let projects = load(&home).unwrap();
        assert_eq!(projects.projects.len(), 1);
        assert_eq!(projects.projects[0].name, "project-a");
        assert!(home.join("workspace/project-a").is_dir());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn changing_path_or_removing_is_blocked_by_active_mounts() {
        let home = home("bindings");
        let request = save_request(home.join("workspace/project-a"));
        let preview = preview_save_project(&home, &request).unwrap();
        let saved = apply_save_project(
            &home,
            &ProjectSaveApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap();
        let mut assets = crate::asset_registry::load(&home).unwrap();
        assets
            .upsert(
                crate::asset_registry::AssetRecord::new(crate::targets::AssetKind::Skill, "review")
                    .unwrap(),
            )
            .unwrap();
        crate::asset_registry::save(&home, &assets).unwrap();
        fs::create_dir_all(home.join(".my-agent-assets/assets/skills/review")).unwrap();
        fs::write(
            home.join(".my-agent-assets/assets/skills/review/SKILL.md"),
            "# Review",
        )
        .unwrap();
        let target_request = crate::target_management::TargetRegistrationPreviewRequest {
            id: "project-a-skills".into(),
            kind: crate::targets::MountTargetKind::ClaudeProjectSkills,
            location: home.join("workspace/project-a"),
        };
        let target_preview =
            crate::target_management::preview_register_target(&home, &target_request).unwrap();
        crate::target_management::apply_register_target(
            &home,
            &crate::target_management::TargetRegistrationApplyRequest {
                preview_id: target_preview.preview_id,
                preview_generated_at_epoch_seconds: target_preview.generated_at_epoch_seconds,
                request: target_request,
            },
        )
        .unwrap();
        let mount_request = MountPreviewRequest {
            asset_id: "skill:review".into(),
            target_id: "project-a-skills".into(),
        };
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
        fs::create_dir_all(home.join("workspace/project-b")).unwrap();
        let edit = ProjectSaveRequest {
            id: Some(saved.project_id.clone()),
            name: "project-a".into(),
            title: "Project A".into(),
            path: home.join("workspace/project-b"),
            description: String::new(),
        };
        assert!(!preview_save_project(&home, &edit).unwrap().can_apply);
        assert!(
            !preview_remove_project(
                &home,
                &ProjectRemoveRequest {
                    id: saved.project_id
                }
            )
            .unwrap()
            .can_apply
        );
        let _ = fs::remove_dir_all(home);
    }
}
