use crate::asset_registry::{inspect_content, load as load_assets, ContentState};
use crate::discovery::{discover, DiscoveryScope};
use crate::mount_registry::{
    load as load_mounts, registry_path as mount_registry_path, BindingStatus, MountRegistry,
};
use crate::path_safety::{display_path, expand_tilde, is_link_or_junction};
use crate::settings;
use crate::targets::{
    load as load_targets, registry_path as target_registry_path, AssetKind, TargetRegistry,
};
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    #[serde(rename = "changed")]
    Changed,
    #[serde(rename = "needsSync")]
    NeedsSync,
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
    pub asset_counts: AssetCounts,
    pub mounts: Vec<String>,
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

pub fn list_projects(home: &Path) -> Result<Vec<ProjectSummary>> {
    let settings = settings::load(home).map_err(|error| MaaError::new(error.to_string()))?;
    let targets = load_targets_or_empty(home)?;
    let mounts = load_mounts_or_empty(home)?;
    let mut project_paths = BTreeSet::new();
    for root in settings
        .scan_roots
        .iter()
        .map(|root| expand_tilde(root, home))
        .filter(|root| root.is_dir())
    {
        discover_projects(&root, 0, settings.max_depth as usize, &mut project_paths);
    }

    let mut projects = Vec::new();
    for path in project_paths {
        let discovery = discover(
            home,
            DiscoveryScope::Project {
                project_path: path.clone(),
            },
        );
        let counts = count_sources(&discovery.sources);
        let project_target_ids = targets
            .targets
            .iter()
            .filter(|target| target.project_path.as_deref() == Some(path.as_path()))
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
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("project")
            .to_string();
        projects.push(ProjectSummary {
            id: display_path(&path),
            name: name.clone(),
            title: name,
            path: path.clone(),
            status: project_status(&path),
            description: "由共享 Rust Core 从配置的扫描根目录发现。".into(),
            updated_at: modified_time(&path),
            asset_counts: counts,
            mounts: mounted_assets,
        });
    }
    projects.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(projects)
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

fn discover_projects(
    path: &Path,
    depth: usize,
    max_depth: usize,
    projects: &mut BTreeSet<PathBuf>,
) {
    if depth > max_depth || should_skip(path) || is_symlink(path) {
        return;
    }
    if depth > 0 && is_project(path) {
        projects.insert(path.to_path_buf());
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    let mut children = entries
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    children.sort();
    for child in children {
        if child.is_dir() {
            discover_projects(&child, depth + 1, max_depth, projects);
        }
    }
}

fn is_project(path: &Path) -> bool {
    [
        "package.json",
        "Cargo.toml",
        ".git",
        ".claude",
        ".agents",
        ".mcp.json",
        ".codex",
    ]
    .iter()
    .any(|marker| path.join(marker).exists())
}

fn should_skip(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | "node_modules" | "dist" | "build" | "target" | ".venv" | "__pycache__")
    )
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| is_link_or_junction(&metadata))
        .unwrap_or(false)
}

fn project_status(path: &Path) -> ProjectQueryStatus {
    let output = Command::new("git")
        .current_dir(path)
        .args(["status", "--porcelain=v1"])
        .output();
    match output {
        Ok(output) if output.status.success() && !output.stdout.is_empty() => {
            ProjectQueryStatus::Changed
        }
        Ok(output) if output.status.success() => ProjectQueryStatus::Ready,
        _ => ProjectQueryStatus::Ready,
    }
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
    use crate::mount_registry::{save as save_mounts, MountRegistry};
    use crate::targets::{save as save_targets, TargetRegistry};
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
    fn discovers_nested_projects_to_configured_depth_without_following_symlinks() {
        let home = home("projects");
        let root = home.join("workspace");
        fs::create_dir_all(root.join("group/project-a/.claude/skills/review")).unwrap();
        fs::write(
            root.join("group/project-a/.claude/skills/review/SKILL.md"),
            "# Review",
        )
        .unwrap();
        fs::create_dir_all(root.join("too/deep/project-b")).unwrap();
        fs::write(root.join("too/deep/project-b/package.json"), "{}").unwrap();
        let mut settings = settings::Settings::defaults_for_home(&home);
        settings.scan_roots = vec![display_path(&root)];
        settings.max_depth = 2;
        settings::save(&home, &settings).unwrap();

        let projects = list_projects(&home).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "project-a");
        assert_eq!(projects[0].asset_counts.skills, 1);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn project_query_uses_defaults_before_asset_center_initialization() {
        let home = std::env::temp_dir().join(format!(
            "maa-query-uninitialized-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(home.join("workspace/project-a")).unwrap();
        fs::write(home.join("workspace/project-a/package.json"), "{}").unwrap();

        let projects = list_projects(&home).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "project-a");
        assert!(!home.join(".my-agent-assets").exists());
        let _ = fs::remove_dir_all(home);
    }
}
