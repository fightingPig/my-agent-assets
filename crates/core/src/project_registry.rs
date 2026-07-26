use crate::discovery::{discover, DiscoveryScope};
use crate::fingerprint::PreviewFingerprint;
use crate::mount_registry::load as load_mounts;
use crate::operation::{OperationJournal, OperationLock, RecoveryTarget};
use crate::path_safety::is_link_or_junction;
use crate::targets::{
    load as load_targets, save as save_targets, AssetKind, MountTarget, MountTargetKind,
    TargetRegistry,
};
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
const PROJECTS_GITIGNORE_ENTRY: &str = "projects.yaml";
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_inspection: Option<ProjectInspectionSummary>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInspectionSummary {
    pub checked_at_epoch_seconds: u64,
    pub skills: u32,
    pub commands: u32,
    pub mcps: u32,
    pub path_healthy: bool,
    #[serde(default)]
    pub warnings: Vec<String>,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRefreshRequest {
    #[serde(default)]
    pub project_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRefreshResult {
    pub refreshed_project_ids: Vec<String>,
    pub registry_path: PathBuf,
    pub warnings: Vec<String>,
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
            validate_project(project)?;
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
        for (index, project) in self.projects.iter().enumerate() {
            for other in self.projects.iter().skip(index + 1) {
                if project.path.starts_with(&other.path) || other.path.starts_with(&project.path) {
                    return Err(MaaError::new(format!(
                        "managed project paths must not overlap: {} and {}",
                        project.path.display(),
                        other.path.display()
                    )));
                }
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

fn gitignore_path(home: &Path) -> PathBuf {
    home.join(".my-agent-assets/.gitignore")
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
    ensure_standard_project_targets(&mut targets, &project)?;
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

pub fn refresh_projects(
    home: &Path,
    request: &ProjectRefreshRequest,
) -> Result<ProjectRefreshResult> {
    let _lock = OperationLock::acquire(home)?;
    let mut registry = load(home)?;
    let selected = if request.project_ids.is_empty() {
        registry
            .projects
            .iter()
            .map(|project| project.id.clone())
            .collect::<BTreeSet<_>>()
    } else {
        request
            .project_ids
            .iter()
            .map(|id| {
                validate_id(id, "project id")?;
                registry.find(id)?;
                Ok(id.clone())
            })
            .collect::<Result<BTreeSet<_>>>()?
    };
    let checked_at = epoch_seconds();
    let mut warnings = Vec::new();
    let mut refreshed_project_ids = Vec::new();
    for project in &mut registry.projects {
        if !selected.contains(&project.id) {
            continue;
        }
        let mut inspection = ProjectInspectionSummary {
            checked_at_epoch_seconds: checked_at,
            path_healthy: project.path.is_dir(),
            ..ProjectInspectionSummary::default()
        };
        if inspection.path_healthy {
            let discovered = discover(
                home,
                DiscoveryScope::Project {
                    project_path: project.path.clone(),
                },
            );
            for source in discovered.sources {
                match source.asset_kind {
                    AssetKind::Skill => inspection.skills += 1,
                    AssetKind::Command => inspection.commands += 1,
                    AssetKind::Mcp => inspection.mcps += 1,
                }
            }
            inspection.warnings = discovered.warnings;
        } else {
            inspection
                .warnings
                .push("项目目录不存在或不可读取。".to_string());
        }
        warnings.extend(
            inspection
                .warnings
                .iter()
                .map(|warning| format!("{}: {warning}", project.name)),
        );
        project.last_inspection = Some(inspection);
        refreshed_project_ids.push(project.id.clone());
    }
    save(home, &registry)?;
    Ok(ProjectRefreshResult {
        refreshed_project_ids,
        registry_path: registry_path(home),
        warnings,
    })
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
        generated_at,
        ProjectPreviewParts {
            operation: "save",
            project: Some(project),
            migrated_target_ids,
            blocking_bindings,
            warnings,
            can_apply,
        },
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
        generated_at,
        ProjectPreviewParts {
            operation: "remove",
            project: Some(project),
            migrated_target_ids: target_ids,
            blocking_bindings: blocking_bindings.clone(),
            warnings,
            can_apply: blocking_bindings.is_empty(),
        },
    )
}

struct ProjectPreviewParts {
    operation: &'static str,
    project: Option<ManagedProject>,
    migrated_target_ids: Vec<String>,
    blocking_bindings: Vec<String>,
    warnings: Vec<String>,
    can_apply: bool,
}

fn build_preview(
    home: &Path,
    generated_at: u64,
    parts: ProjectPreviewParts,
) -> Result<ProjectChangePreview> {
    let ProjectPreviewParts {
        operation,
        project,
        migrated_target_ids,
        blocking_bindings,
        warnings,
        can_apply,
    } = parts;
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
    fingerprint.add_path_if_present("gitignore", &gitignore_path(home))?;
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
        affected_paths: vec![
            registry_path(home),
            crate::targets::registry_path(home),
            gitignore_path(home),
        ],
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
            RecoveryTarget::asset_center(gitignore_path(home)),
        ],
    )?;
    let result: Result<ProjectChangeResult> = (|| {
        save(home, registry)?;
        save_targets(home, targets)?;
        ensure_project_registry_ignored(home)?;
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

fn ensure_project_registry_ignored(home: &Path) -> Result<()> {
    let path = gitignore_path(home);
    let mut content = if path.exists() {
        let metadata = fs::symlink_metadata(&path)?;
        if is_link_or_junction(&metadata) || !metadata.is_file() {
            return Err(MaaError::new(
                "asset center .gitignore must be a regular file",
            ));
        }
        fs::read_to_string(&path)
            .map_err(|error| MaaError::new(format!("failed to read .gitignore: {error}")))?
    } else {
        String::new()
    };
    if content
        .lines()
        .any(|line| line.trim() == PROJECTS_GITIGNORE_ENTRY)
    {
        return Ok(());
    }
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(PROJECTS_GITIGNORE_ENTRY);
    content.push('\n');
    write_atomic(&path, content.as_bytes())
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

fn ensure_standard_project_targets(
    targets: &mut TargetRegistry,
    project: &ManagedProject,
) -> Result<()> {
    let kinds = [
        (MountTargetKind::ClaudeProjectSkills, "claude-skills"),
        (MountTargetKind::CodexProjectSkills, "codex-skills"),
        (MountTargetKind::ClaudeProjectCommands, "claude-commands"),
        (MountTargetKind::ClaudeProjectMcpJson, "claude-mcp"),
        (MountTargetKind::CodexProjectMcpToml, "codex-mcp"),
    ];
    for (kind, suffix) in kinds {
        let id = format!("{}-{suffix}", project.id);
        let generated = MountTarget::project(id.clone(), kind, project.path.clone())?;
        if let Some(existing) = targets.targets.iter_mut().find(|target| target.id == id) {
            if existing.kind != kind {
                return Err(MaaError::new(format!(
                    "generated project target id conflicts with an existing target: {id}"
                )));
            }
            *existing = generated;
        } else {
            targets.targets.push(generated);
        }
    }
    targets
        .targets
        .sort_by(|left, right| left.id.cmp(&right.id));
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
        last_inspection: request.id.as_deref().and_then(|id| {
            load(home)
                .ok()
                .and_then(|registry| registry.projects.into_iter().find(|item| item.id == id))
                .and_then(|item| item.last_inspection)
        }),
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
        crate::operation::sync_directory(parent)
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
        fs::remove_file(registry_path(&home)).unwrap();
        let ignore_path = gitignore_path(&home);
        let legacy_ignore = fs::read_to_string(&ignore_path)
            .unwrap()
            .replace("projects.yaml\n", "");
        fs::write(&ignore_path, legacy_ignore).unwrap();
        let request = save_request(home.join("workspace/project-a"));
        let preview = preview_save_project(&home, &request).unwrap();
        assert!(preview.can_apply);
        assert!(preview.affected_paths.contains(&ignore_path));
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
        let targets = load_targets(&home).unwrap();
        let generated = targets
            .targets
            .iter()
            .filter(|target| {
                target.project_path.as_deref() == Some(projects.projects[0].path.as_path())
            })
            .collect::<Vec<_>>();
        assert_eq!(generated.len(), 5);
        for kind in [
            MountTargetKind::ClaudeProjectSkills,
            MountTargetKind::CodexProjectSkills,
            MountTargetKind::ClaudeProjectCommands,
            MountTargetKind::ClaudeProjectMcpJson,
            MountTargetKind::CodexProjectMcpToml,
        ] {
            assert!(generated.iter().any(|target| target.kind == kind));
        }
        assert!(fs::read_to_string(ignore_path)
            .unwrap()
            .lines()
            .any(|line| line == "projects.yaml"));
        assert!(home.join("workspace/project-a").is_dir());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn overlapping_managed_project_paths_are_rejected() {
        let home = home("overlap");
        let parent_request = save_request(home.join("workspace/project-a"));
        let parent_preview = preview_save_project(&home, &parent_request).unwrap();
        apply_save_project(
            &home,
            &ProjectSaveApplyRequest {
                preview_id: parent_preview.preview_id,
                preview_generated_at_epoch_seconds: parent_preview.generated_at_epoch_seconds,
                request: parent_request,
            },
        )
        .unwrap();

        let nested = home.join("workspace/project-a/packages/nested");
        fs::create_dir_all(&nested).unwrap();
        let request = ProjectSaveRequest {
            id: None,
            name: "nested".into(),
            title: "Nested".into(),
            path: nested,
            description: String::new(),
        };
        let preview = preview_save_project(&home, &request).unwrap();
        assert!(!preview.can_apply);
        assert!(preview
            .warnings
            .iter()
            .any(|warning| warning.contains("must not overlap")));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn refresh_persists_runtime_asset_health() {
        let home = home("refresh");
        let project_path = home.join("workspace/project-a");
        fs::create_dir_all(project_path.join(".claude/skills/review")).unwrap();
        fs::write(
            project_path.join(".claude/skills/review/SKILL.md"),
            "# Review",
        )
        .unwrap();
        fs::create_dir_all(project_path.join(".claude/commands")).unwrap();
        fs::write(project_path.join(".claude/commands/commit.md"), "# Commit").unwrap();
        let request = save_request(project_path);
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

        let result = refresh_projects(
            &home,
            &ProjectRefreshRequest {
                project_ids: vec![saved.project_id.clone()],
            },
        )
        .unwrap();
        assert_eq!(result.refreshed_project_ids, vec![saved.project_id]);
        let inspection = load(&home).unwrap().projects[0]
            .last_inspection
            .clone()
            .unwrap();
        assert!(inspection.path_healthy);
        assert_eq!(inspection.skills, 1);
        assert_eq!(inspection.commands, 1);
        assert_eq!(inspection.mcps, 0);
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
        let target_id = format!("{}-claude-skills", saved.project_id);
        let mount_request = MountPreviewRequest {
            asset_id: "skill:review".into(),
            target_id,
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
