use crate::asset_registry::{inspect_content, load as load_assets, ContentState};
use crate::managed_projects::{
    is_generated_project_target, load as load_managed_projects, record_check, ManagedProject,
    ProjectCheckSummary,
};
use crate::mount_registry::{
    load as load_mounts, registry_path as mount_registry_path, BindingStatus, MountRegistry,
};
use crate::path_safety::display_path;
use crate::targets::{
    load as load_targets, registry_path as target_registry_path, AssetKind, RuntimeProvider,
    TargetRegistry, TargetScope,
};
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetQueryStatus {
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "mounted")]
    Mounted,
    #[serde(rename = "unmounted")]
    Unmounted,
    #[serde(rename = "conflict")]
    Conflict,
    #[serde(rename = "invalid")]
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectQueryStatus {
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "unchecked")]
    Unchecked,
    #[serde(rename = "needs_attention")]
    NeedsAttention,
    #[serde(rename = "missing_path")]
    MissingPath,
    #[serde(rename = "invalid")]
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetQueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_type: Option<AssetKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSummary {
    pub id: String,
    pub name: String,
    pub title: String,
    pub asset_type: AssetKind,
    pub status: AssetQueryStatus,
    pub category: String,
    pub description: String,
    pub source_path: PathBuf,
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    pub mount_targets: Vec<String>,
}

/// A locally registered runtime binding. This intentionally exposes target
/// metadata instead of asking a renderer to infer bindings from filesystem
/// paths, which is especially important for JSON and TOML MCP renderers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountBindingSummary {
    pub asset_id: String,
    pub target_id: String,
    pub status: BindingStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<RuntimeProvider>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<TargetScope>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetCounts {
    pub total: u32,
    pub skills: u32,
    pub commands: u32,
    pub mcps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub title: String,
    pub path: PathBuf,
    pub status: ProjectQueryStatus,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checked_at: Option<String>,
    pub path_available: bool,
    pub warning_count: u32,
    pub asset_counts: AssetCounts,
    pub mounts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInspectionRequest {
    /// An empty list means every explicitly managed project.
    #[serde(default)]
    pub project_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInspection {
    pub project: ProjectSummary,
    pub warnings: Vec<String>,
}

pub fn list_assets(home: &Path, request: &AssetQueryRequest) -> Result<Vec<AssetSummary>> {
    let asset_center = home.join(".my-agent-assets");
    if !asset_center.exists() {
        return Ok(Vec::new());
    }
    let registry = load_assets(home).map_err(|error| MaaError::new(error.to_string()))?;
    let diagnostics =
        inspect_content(home, &registry).map_err(|error| MaaError::new(error.to_string()))?;
    let mounts = load_mounts_or_empty(home)?;
    let targets = load_targets_or_empty(home)?;
    let target_paths = targets
        .targets
        .into_iter()
        .map(|target| (target.id, display_path(&target.path)))
        .collect::<BTreeMap<_, _>>();

    let mut summaries = Vec::new();
    for diagnostic in diagnostics {
        if request
            .asset_type
            .is_some_and(|asset_type| asset_type != diagnostic.asset_type)
        {
            continue;
        }
        let bindings = mounts.for_asset(&diagnostic.asset_id);
        let mount_targets = bindings
            .iter()
            .map(|binding| {
                target_paths
                    .get(&binding.target_id)
                    .cloned()
                    .unwrap_or_else(|| binding.target_id.clone())
            })
            .collect::<Vec<_>>();
        let status = match diagnostic.state {
            ContentState::Ready
                if bindings.iter().any(|binding| {
                    matches!(
                        binding.status,
                        BindingStatus::Mounted | BindingStatus::OutOfSync
                    )
                }) =>
            {
                AssetQueryStatus::Mounted
            }
            ContentState::Ready => AssetQueryStatus::Ready,
            ContentState::MissingContent
            | ContentState::Unregistered
            | ContentState::InvalidContent => AssetQueryStatus::Invalid,
        };
        let record = registry.assets.get(&diagnostic.asset_id);
        let description = record
            .and_then(|record| record.description.clone())
            .or(diagnostic.message.clone())
            .unwrap_or_else(|| format!("本地 {} 资产", asset_kind_label(diagnostic.asset_type)));
        summaries.push(AssetSummary {
            id: diagnostic.asset_id,
            name: diagnostic.name.clone(),
            title: record
                .and_then(|record| record.title.clone())
                .unwrap_or_else(|| diagnostic.name.clone()),
            asset_type: diagnostic.asset_type,
            status,
            category: if diagnostic.state == ContentState::Unregistered {
                "未登记内容".into()
            } else {
                "资产中心".into()
            },
            description,
            source_path: diagnostic.path.clone(),
            scope: "local".into(),
            updated_at: modified_time(&diagnostic.path),
            mount_targets,
        });
    }
    summaries.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(summaries)
}

pub fn list_mount_bindings(home: &Path) -> Result<Vec<MountBindingSummary>> {
    let mounts = load_mounts_or_empty(home)?;
    let targets = load_targets_or_empty(home)?;
    let target_map = targets
        .targets
        .iter()
        .map(|target| (target.id.as_str(), target))
        .collect::<BTreeMap<_, _>>();
    let mut summaries = mounts
        .bindings
        .values()
        .map(|binding| {
            let target = target_map.get(binding.target_id.as_str()).copied();
            MountBindingSummary {
                asset_id: binding.asset_id.clone(),
                target_id: binding.target_id.clone(),
                status: binding.status,
                target_path: target.map(|target| target.path.clone()),
                provider: target.map(|target| target.provider),
                scope: target.map(|target| target.scope),
            }
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| {
        left.asset_id
            .cmp(&right.asset_id)
            .then(left.target_id.cmp(&right.target_id))
    });
    Ok(summaries)
}

pub fn list_projects(home: &Path) -> Result<Vec<ProjectSummary>> {
    let projects = load_managed_projects(home)?;
    let targets = load_targets_or_empty(home)?;
    let mounts = load_mounts_or_empty(home)?;
    let asset_registry = load_assets(home).ok();

    let mut summaries = Vec::new();
    for project in projects.projects {
        let project_target_ids = targets
            .targets
            .iter()
            .filter(|target| {
                is_generated_project_target(&project.id, &target.id)
                    || target.project_path.as_deref() == Some(project.path.as_path())
            })
            .map(|target| target.id.as_str())
            .collect::<BTreeSet<_>>();
        let mut mounted_assets = mounts
            .bindings
            .values()
            .filter(|binding| project_target_ids.contains(binding.target_id.as_str()))
            .map(|binding| binding.asset_id.clone())
            .collect::<Vec<_>>();
        mounted_assets.sort();
        mounted_assets.dedup();
        let mounts = mounted_assets
            .into_iter()
            .map(|asset_id| asset_name(&asset_id, asset_registry.as_ref()))
            .collect::<Vec<_>>();
        let path_available = project.path.is_dir();
        let (asset_counts, warning_count, last_checked_at) = project
            .last_check
            .as_ref()
            .map(|check| {
                (
                    check.asset_counts.clone(),
                    check.warning_count,
                    epoch_to_rfc3339(check.checked_at_epoch_seconds),
                )
            })
            .unwrap_or_default();
        summaries.push(ProjectSummary {
            id: project.id.clone(),
            name: project.name.clone(),
            title: project.name.clone(),
            path: project.path.clone(),
            status: managed_project_status(&project, path_available),
            description: "用户显式维护的本地项目。".into(),
            updated_at: epoch_to_rfc3339(project.updated_at_epoch_seconds),
            last_checked_at,
            path_available,
            warning_count,
            asset_counts,
            mounts,
        });
    }
    summaries.sort_by(|left, right| left.name.cmp(&right.name).then(left.path.cmp(&right.path)));
    Ok(summaries)
}

/// User-triggered project inspection. The filesystem scan is read-only; the
/// resulting derived summary is persisted locally so the project list can show
/// the most recent explicit check after restart.
pub fn inspect_projects(
    home: &Path,
    request: &ProjectInspectionRequest,
) -> Result<Vec<ProjectInspection>> {
    let registry = load_managed_projects(home)?;
    let selected = registry
        .projects
        .iter()
        .filter(|project| {
            request.project_ids.is_empty() || request.project_ids.contains(&project.id)
        })
        .map(|project| project.id.clone())
        .collect::<Vec<_>>();
    let mut inspections = Vec::new();
    for project_id in selected {
        let discovery = crate::discovery::discover(
            home,
            crate::discovery::DiscoveryScope::ManagedProjects {
                project_ids: vec![project_id.clone()],
            },
        );
        let counts = count_sources(&discovery.sources);
        let project = record_check(
            home,
            &project_id,
            ProjectCheckSummary {
                checked_at_epoch_seconds: epoch_seconds(),
                asset_counts: counts,
                warning_count: discovery.warnings.len() as u32,
                path_available: registry
                    .projects
                    .iter()
                    .find(|project| project.id == project_id)
                    .is_some_and(|project| project.path.is_dir()),
            },
        )?;
        let summary = list_projects(home)?
            .into_iter()
            .find(|summary| summary.id == project.id)
            .ok_or_else(|| MaaError::new("managed project disappeared during inspection"))?;
        inspections.push(ProjectInspection {
            project: summary,
            warnings: discovery.warnings,
        });
    }
    for project_id in &request.project_ids {
        if !registry
            .projects
            .iter()
            .any(|project| &project.id == project_id)
        {
            return Err(MaaError::new(format!(
                "managed project not found: {project_id}"
            )));
        }
    }
    Ok(inspections)
}

fn load_targets_or_empty(home: &Path) -> Result<TargetRegistry> {
    if !target_registry_path(home).exists() {
        return TargetRegistry::new(Vec::new());
    }
    load_targets(home)
}

fn load_mounts_or_empty(home: &Path) -> Result<MountRegistry> {
    if !mount_registry_path(home).exists() {
        return Ok(MountRegistry::default());
    }
    load_mounts(home).map_err(|error| MaaError::new(error.to_string()))
}

fn managed_project_status(project: &ManagedProject, path_available: bool) -> ProjectQueryStatus {
    if !path_available {
        ProjectQueryStatus::MissingPath
    } else if project.last_check.is_none() {
        ProjectQueryStatus::Unchecked
    } else if project
        .last_check
        .as_ref()
        .is_some_and(|check| check.warning_count > 0 || !check.path_available)
    {
        ProjectQueryStatus::NeedsAttention
    } else {
        ProjectQueryStatus::Ready
    }
}

fn asset_name(asset_id: &str, registry: Option<&crate::asset_registry::AssetRegistry>) -> String {
    registry
        .and_then(|registry| registry.assets.get(asset_id))
        .map(|record| record.name.clone())
        .unwrap_or_else(|| {
            asset_id
                .split_once(':')
                .map(|(_, name)| name)
                .unwrap_or(asset_id)
                .to_string()
        })
}

fn epoch_to_rfc3339(seconds: u64) -> Option<String> {
    UNIX_EPOCH
        .checked_add(Duration::from_secs(seconds))
        .map(|time| humantime::format_rfc3339_seconds(time).to_string())
}

fn count_sources(sources: &[crate::discovery::DiscoveredSource]) -> AssetCounts {
    let mut counts = AssetCounts::default();
    for source in sources {
        match source.asset_kind {
            AssetKind::Skill => counts.skills += 1,
            AssetKind::Command => counts.commands += 1,
            AssetKind::Mcp => counts.mcps += 1,
        }
    }
    counts.total = counts.skills + counts.commands + counts.mcps;
    counts
}

fn epoch_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn modified_time(path: &Path) -> Option<String> {
    fs::symlink_metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .map(|time| humantime::format_rfc3339_seconds(time).to_string())
}

fn asset_kind_label(kind: AssetKind) -> &'static str {
    match kind {
        AssetKind::Skill => "Skill",
        AssetKind::Command => "Command",
        AssetKind::Mcp => "MCP",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_registry::{save as save_assets, AssetRecord, AssetRegistry};
    use crate::mount_registry::{save as save_mounts, MountBinding, MountRegistry};
    use crate::targets::{save as save_targets, MountAdapter, ProviderState, TargetRegistry};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn home(label: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!(
            "maa-query-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = home.join(".my-agent-assets");
        fs::create_dir_all(root.join("assets/skills/review")).unwrap();
        fs::create_dir_all(root.join("assets/commands")).unwrap();
        fs::create_dir_all(root.join("assets/mcps")).unwrap();
        fs::write(root.join("assets/skills/review/SKILL.md"), "# Review").unwrap();
        let mut assets = AssetRegistry::default();
        assets
            .upsert(AssetRecord::new(AssetKind::Skill, "review").unwrap())
            .unwrap();
        save_assets(&home, &assets).unwrap();
        save_mounts(&home, &MountRegistry::default()).unwrap();
        save_targets(&home, &TargetRegistry::new(Vec::new()).unwrap()).unwrap();
        home
    }

    #[test]
    fn lists_registered_and_unregistered_assets_without_writing() {
        let home = home("assets");
        fs::write(
            home.join(".my-agent-assets/assets/commands/orphan.md"),
            "# Orphan",
        )
        .unwrap();
        let before = fs::read(home.join(".my-agent-assets/assets.yaml")).unwrap();
        let assets = list_assets(&home, &AssetQueryRequest { asset_type: None }).unwrap();
        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0].id, "command:orphan");
        assert_eq!(assets[0].status, AssetQueryStatus::Invalid);
        assert_eq!(assets[1].id, "skill:review");
        assert_eq!(
            fs::read(home.join(".my-agent-assets/assets.yaml")).unwrap(),
            before
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn lists_registered_mount_bindings_with_target_metadata() {
        let home = home("mount-bindings");
        let targets = TargetRegistry::standard_user_targets(
            &home,
            ProviderState::Initialized,
            ProviderState::NotInstalled,
            MountAdapter::SymlinkDirectory,
        )
        .unwrap();
        save_targets(&home, &targets).unwrap();
        let mut mounts = MountRegistry::default();
        mounts
            .upsert(
                MountBinding::new("skill:review", "claude-user-skills", BindingStatus::Mounted)
                    .unwrap(),
            )
            .unwrap();
        save_mounts(&home, &mounts).unwrap();

        let bindings = list_mount_bindings(&home).unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].asset_id, "skill:review");
        assert_eq!(bindings[0].target_id, "claude-user-skills");
        assert_eq!(bindings[0].provider, Some(RuntimeProvider::ClaudeCode));
        assert_eq!(bindings[0].scope, Some(TargetScope::User));
        assert_eq!(
            bindings[0].target_path.as_deref(),
            Some(home.join(".claude/skills").as_path())
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn lists_only_explicitly_managed_projects_and_uses_cached_check_summary() {
        let home = home("projects");
        let root = home.join("workspace/project-a");
        fs::create_dir_all(root.join("group/.claude/skills/review")).unwrap();
        fs::write(
            root.join("group/.claude/skills/review/SKILL.md"),
            "# Review",
        )
        .unwrap();
        fs::create_dir_all(home.join("workspace/not-managed/.claude")).unwrap();
        let add = crate::managed_projects::ProjectAddPreviewRequest {
            path: root,
            name: None,
        };
        let preview = crate::managed_projects::preview_add_project(&home, &add).unwrap();
        crate::managed_projects::apply_add_project(
            &home,
            &crate::managed_projects::ProjectAddApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request: add,
            },
        )
        .unwrap();
        let projects = list_projects(&home).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "project-a");
        assert_eq!(projects[0].status, ProjectQueryStatus::Unchecked);
        let checked = inspect_projects(
            &home,
            &ProjectInspectionRequest {
                project_ids: vec![projects[0].id.clone()],
            },
        )
        .unwrap();
        assert_eq!(checked[0].project.asset_counts.skills, 1);
        assert_eq!(checked[0].project.status, ProjectQueryStatus::Ready);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn missing_managed_path_is_reported_without_auto_discovery() {
        let home = home("missing-project");
        let root = home.join("workspace/project-a");
        fs::create_dir_all(&root).unwrap();
        let add = crate::managed_projects::ProjectAddPreviewRequest {
            path: root.clone(),
            name: None,
        };
        let preview = crate::managed_projects::preview_add_project(&home, &add).unwrap();
        crate::managed_projects::apply_add_project(
            &home,
            &crate::managed_projects::ProjectAddApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request: add,
            },
        )
        .unwrap();
        fs::remove_dir_all(root).unwrap();
        let projects = list_projects(&home).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].status, ProjectQueryStatus::MissingPath);
        let _ = fs::remove_dir_all(home);
    }
}
