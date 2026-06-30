#[cfg(test)]
use crate::contracts::{AppearanceTheme, DensityPreference, DesktopSettings, LogLevel};
use crate::contracts::{
    AssetCounts, AssetStatus, AssetSummary, AssetType, ListAssetsInput, ProjectStatus,
    ProjectSummary, RuntimeScope,
};
#[cfg(test)]
use crate::contracts::{BackupSummary, GitStatus};
use crate::path_utils::{display_path, home_dir, modified_time_iso};
#[cfg(test)]
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn list_assets_command(input: ListAssetsInput) -> Vec<AssetSummary> {
    match home_dir() {
        Some(home) => list_assets_for_home(&home, input),
        None => vec![],
    }
}

pub fn list_projects_command() -> Vec<ProjectSummary> {
    match home_dir() {
        Some(home) => list_projects_for_home(&home),
        None => vec![],
    }
}

#[cfg(test)]
pub fn settings_for_home(home: Option<&Path>) -> DesktopSettings {
    let (asset_center_path, scan_roots) = match home {
        Some(home) => (
            display_path(&home.join(".my-agent-assets")),
            vec![
                display_path(&home.join(".claude")),
                display_path(&home.join("workspace")),
                display_path(&home.join("code")),
            ],
        ),
        None => (
            "~/.my-agent-assets".into(),
            vec!["~/.claude".into(), "~/workspace".into(), "~/code".into()],
        ),
    };

    DesktopSettings {
        asset_center_path,
        scan_roots,
        max_depth: 5,
        backup_before_apply: true,
        plan_only_by_default: true,
        git_default_branch: "main".into(),
        git_remote: "origin".into(),
        appearance_theme: AppearanceTheme::System,
        density: DensityPreference::Compact,
        log_level: LogLevel::Info,
        log_retention_days: 14,
        cli_path: "maa".into(),
    }
}

pub fn list_assets_for_home(home: &Path, input: ListAssetsInput) -> Vec<AssetSummary> {
    let asset_root = home.join(".my-agent-assets").join("assets");
    let mut assets = Vec::new();

    if input.asset_type.is_none() || input.asset_type == Some(AssetType::Skill) {
        assets.extend(read_skill_assets(&asset_root.join("skills")));
    }
    if input.asset_type.is_none() || input.asset_type == Some(AssetType::Command) {
        assets.extend(read_file_assets(
            &asset_root.join("commands"),
            AssetType::Command,
            RuntimeScope::Local,
            "local",
            "Command",
        ));
    }
    if input.asset_type.is_none() || input.asset_type == Some(AssetType::Mcp) {
        assets.extend(read_mcp_assets(
            &asset_root.join("mcps"),
            RuntimeScope::Local,
        ));
    }

    for asset in &mut assets {
        asset.mount_targets = discover_asset_mounts(home, asset);
        if !asset.mount_targets.is_empty() && asset.status == AssetStatus::Ready {
            asset.status = AssetStatus::Mounted;
        }
    }
    assets.sort_by(|left, right| left.id.cmp(&right.id));
    assets
}

pub fn list_projects_for_home(home: &Path) -> Vec<ProjectSummary> {
    let mut projects = discover_project_paths(home)
        .iter()
        .filter_map(|path| project_summary_from_path(path))
        .collect::<Vec<_>>();

    projects.sort_by(|left, right| left.name.cmp(&right.name));
    projects
}

#[cfg(test)]
pub fn list_backups_for_home(home: &Path) -> Vec<BackupSummary> {
    let backup_root = home.join(".my-agent-assets").join("backups");
    let Ok(entries) = fs::read_dir(&backup_root) else {
        return vec![];
    };

    let mut backups = entries
        .flatten()
        .filter_map(|entry| backup_summary_from_manifest(&entry.path().join("manifest.json")))
        .collect::<Vec<_>>();
    backups.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    backups
}

#[cfg(test)]
pub fn git_status_for_home(home: &Path) -> GitStatus {
    let repository_path = home.join(".my-agent-assets");
    let display_repository_path = display_path(&repository_path);

    if !repository_path.exists() {
        return safe_git_status(
            display_repository_path,
            "Asset center directory does not exist.",
        );
    }

    if !git_success(&repository_path, &["rev-parse", "--is-inside-work-tree"]) {
        return safe_git_status(
            display_repository_path,
            "Asset center is not a Git repository.",
        );
    }

    let branch = git_output(&repository_path, &["branch", "--show-current"]).unwrap_or_default();
    let status_output =
        git_output(&repository_path, &["status", "--porcelain"]).unwrap_or_default();
    let changed_files = parse_changed_files(&status_output);
    let conflicts = parse_conflicts(&status_output);
    let upstream = git_output(
        &repository_path,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    );
    let (ahead, behind, remote, upstream_message) = match upstream {
        Some(upstream) if !upstream.is_empty() => {
            let (ahead, behind) = git_output(
                &repository_path,
                &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
            )
            .map(|value| parse_ahead_behind(&value))
            .unwrap_or((0, 0));
            (
                ahead,
                behind,
                Some(upstream),
                "Git repository is available.",
            )
        }
        _ => (0, 0, None, "Git repository has no upstream."),
    };

    GitStatus {
        repository_path: display_repository_path,
        is_repository: true,
        status_message: upstream_message.into(),
        branch,
        remote,
        clean: changed_files.is_empty() && conflicts.is_empty(),
        ahead,
        behind,
        changed_files,
        conflicts,
        last_synced_at: None,
    }
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupManifestProbe {
    id: String,
    label: String,
    created_at: String,
    entries: Vec<BackupEntryProbe>,
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupEntryProbe {
    size_bytes: u64,
}

#[cfg(test)]
fn backup_summary_from_manifest(path: &Path) -> Option<BackupSummary> {
    let text = fs::read_to_string(path).ok()?;
    let manifest = serde_json::from_str::<BackupManifestProbe>(&text).ok()?;
    Some(BackupSummary {
        id: manifest.id,
        label: manifest.label,
        created_at: manifest.created_at,
        size_bytes: manifest
            .entries
            .iter()
            .map(|entry| entry.size_bytes)
            .sum::<u64>(),
        entry_count: manifest.entries.len() as u32,
    })
}

fn scan_runtime_root(
    root: &Path,
    scope: RuntimeScope,
    warnings: &mut Vec<String>,
) -> Vec<AssetSummary> {
    if !root.exists() {
        warnings.push(format!(
            "Runtime root does not exist: {}",
            display_path(root)
        ));
        return vec![];
    }

    let mut assets = Vec::new();
    let claude_root = root.join(".claude");
    assets.extend(read_runtime_skill_assets(
        &claude_root.join("skills"),
        scope.clone(),
        "project runtime",
    ));
    assets.extend(read_runtime_markdown_assets(
        &claude_root.join("commands"),
        AssetType::Command,
        scope.clone(),
        "project runtime",
        "Command",
    ));
    assets.extend(read_mcp_config_file(
        &root.join(".mcp.json"),
        scope,
        warnings,
    ));
    assets
}

fn read_skill_assets(dir: &Path) -> Vec<AssetSummary> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };

    let mut assets = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let Some(name) = file_stem_or_name(&path) else {
                continue;
            };
            let description = first_readable_markdown_description(&path)
                .unwrap_or_else(|| format!("Local skill asset {}", name));
            assets.push(asset_summary(
                AssetType::Skill,
                RuntimeScope::Local,
                &name,
                &path,
                AssetStatus::Ready,
                "local",
                description,
            ));
        } else if has_extension(&path, "md") {
            let Some(name) = file_stem_or_name(&path) else {
                continue;
            };
            let description =
                first_paragraph(&path).unwrap_or_else(|| format!("Local skill asset {}", name));
            assets.push(asset_summary(
                AssetType::Skill,
                RuntimeScope::Local,
                &name,
                &path,
                AssetStatus::Ready,
                "local",
                description,
            ));
        }
    }
    assets
}

fn read_file_assets(
    dir: &Path,
    asset_type: AssetType,
    scope: RuntimeScope,
    category: &str,
    label: &str,
) -> Vec<AssetSummary> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };

    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && has_extension(path, "md"))
        .filter_map(|path| {
            let name = file_stem_or_name(&path)?;
            let description =
                first_paragraph(&path).unwrap_or_else(|| format!("{} asset {}", label, name));
            Some(asset_summary(
                asset_type.clone(),
                scope.clone(),
                &name,
                &path,
                AssetStatus::Ready,
                category,
                description,
            ))
        })
        .collect()
}

fn read_runtime_markdown_assets(
    dir: &Path,
    asset_type: AssetType,
    scope: RuntimeScope,
    category: &str,
    label: &str,
) -> Vec<AssetSummary> {
    read_file_assets(dir, asset_type, scope, category, label)
}

fn read_runtime_skill_assets(dir: &Path, scope: RuntimeScope, category: &str) -> Vec<AssetSummary> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };

    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                let skill_file = path.join("SKILL.md");
                if !skill_file.is_file() {
                    return None;
                }
                let name = file_stem_or_name(&path)?;
                let description =
                    first_paragraph(&skill_file).unwrap_or_else(|| format!("Skill asset {}", name));
                return Some(asset_summary(
                    AssetType::Skill,
                    scope.clone(),
                    &name,
                    &path,
                    AssetStatus::Ready,
                    category,
                    description,
                ));
            }

            if !path.is_file() || !has_extension(&path, "md") {
                return None;
            }
            let name = file_stem_or_name(&path)?;
            let description =
                first_paragraph(&path).unwrap_or_else(|| format!("Skill asset {}", name));
            Some(asset_summary(
                AssetType::Skill,
                scope.clone(),
                &name,
                &path,
                AssetStatus::Ready,
                category,
                description,
            ))
        })
        .collect()
}

fn read_mcp_assets(dir: &Path, scope: RuntimeScope) -> Vec<AssetSummary> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };

    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && has_extension(path, "json"))
        .filter_map(|path| {
            let name = file_stem_or_name(&path)?;
            let status = if json_file_is_valid(&path) {
                AssetStatus::Ready
            } else {
                AssetStatus::Invalid
            };
            let description = if status == AssetStatus::Invalid {
                format!("Invalid MCP JSON asset {}", name)
            } else {
                format!("MCP server configuration {}", name)
            };
            Some(asset_summary(
                AssetType::Mcp,
                scope.clone(),
                &name,
                &path,
                status,
                "local",
                description,
            ))
        })
        .collect()
}

fn read_mcp_config_file(
    config_path: &Path,
    scope: RuntimeScope,
    warnings: &mut Vec<String>,
) -> Vec<AssetSummary> {
    if !config_path.exists() {
        return vec![];
    }

    let content = match fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(error) => {
            warnings.push(format!(
                "Could not read MCP config {}: {}",
                display_path(config_path),
                error
            ));
            return vec![];
        }
    };

    let value: Value = match serde_json::from_str(&content) {
        Ok(value) => value,
        Err(error) => {
            warnings.push(format!(
                "Could not parse MCP config {}: {}",
                display_path(config_path),
                error
            ));
            return vec![];
        }
    };

    let Some(servers) = value.get("mcpServers").and_then(Value::as_object) else {
        return vec![];
    };

    servers
        .keys()
        .map(|name| {
            asset_summary(
                AssetType::Mcp,
                scope.clone(),
                name,
                config_path,
                AssetStatus::Ready,
                "runtime config",
                format!("MCP server {} from top-level mcpServers", name),
            )
        })
        .collect()
}

fn project_summary_from_path(path: &Path) -> Option<ProjectSummary> {
    let package = path.join("package.json").exists();
    let cargo = path.join("Cargo.toml").exists();
    let git = path.join(".git").exists();
    let claude = path.join(".claude").exists();

    if !package && !cargo && !git && !claude {
        return None;
    }

    let name = path.file_name()?.to_string_lossy().into_owned();
    let status = if git
        && git_output(path, &["status", "--porcelain"]).is_some_and(|status| !status.is_empty())
    {
        ProjectStatus::Changed
    } else {
        ProjectStatus::Ready
    };

    let mut warnings = vec![];
    let runtime_assets = scan_runtime_root(path, RuntimeScope::Project, &mut warnings);
    let mounts = runtime_assets
        .iter()
        .map(|asset| asset.name.clone())
        .collect::<Vec<_>>();

    Some(ProjectSummary {
        id: display_path(path),
        name: name.clone(),
        title: name.clone(),
        path: display_path(path),
        status,
        description: "Local project detected from filesystem markers.".into(),
        updated_at: metadata_modified(path),
        asset_counts: count_assets(&runtime_assets),
        mounts,
    })
}

fn discover_project_paths(home: &Path) -> Vec<PathBuf> {
    let mut paths = vec![];
    for root_name in ["workspace", "code"] {
        let root = home.join(root_name);
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        paths.extend(
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir()),
        );
    }
    paths.sort();
    paths
}

fn discover_asset_mounts(home: &Path, asset: &AssetSummary) -> Vec<String> {
    let mut targets = vec![];
    let mut runtime_roots = vec![home.to_path_buf()];
    runtime_roots.extend(discover_project_paths(home));

    for runtime_root in runtime_roots {
        let is_user = runtime_root == home;
        match asset.asset_type {
            AssetType::Skill => {
                let root = runtime_root.join(".claude").join("skills");
                for candidate in [
                    root.join(&asset.name),
                    root.join(format!("{}.md", asset.name)),
                ] {
                    if symlink_points_to(&candidate, Path::new(&asset.source_path)) {
                        targets.push(display_path(&candidate));
                    }
                }
            }
            AssetType::Command => {
                let candidate = runtime_root
                    .join(".claude")
                    .join("commands")
                    .join(format!("{}.md", asset.name));
                if symlink_points_to(&candidate, Path::new(&asset.source_path)) {
                    targets.push(display_path(&candidate));
                }
            }
            AssetType::Mcp => {
                let config_path = if is_user {
                    runtime_root.join(".claude.json")
                } else {
                    runtime_root.join(".mcp.json")
                };
                if mcp_config_contains(&config_path, &asset.name) {
                    targets.push(display_path(&config_path));
                }
            }
        }
    }
    targets.sort();
    targets.dedup();
    targets
}

fn symlink_points_to(candidate: &Path, expected: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(candidate) else {
        return false;
    };
    if !metadata.file_type().is_symlink() {
        return false;
    }
    let Ok(target) = fs::read_link(candidate) else {
        return false;
    };
    let resolved = if target.is_absolute() {
        target
    } else {
        candidate
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(target)
    };
    match (fs::canonicalize(resolved), fs::canonicalize(expected)) {
        (Ok(resolved), Ok(expected)) => resolved == expected,
        _ => false,
    }
}

fn mcp_config_contains(path: &Path, name: &str) -> bool {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|value| {
            value
                .get("mcpServers")
                .and_then(Value::as_object)
                .map(|servers| servers.contains_key(name))
        })
        .unwrap_or(false)
}

fn asset_summary(
    asset_type: AssetType,
    scope: RuntimeScope,
    name: &str,
    path: &Path,
    status: AssetStatus,
    category: &str,
    description: String,
) -> AssetSummary {
    let id_prefix = match asset_type {
        AssetType::Skill => "skill",
        AssetType::Command => "command",
        AssetType::Mcp => "mcp",
    };

    AssetSummary {
        id: format!("{}:{}", id_prefix, name),
        name: name.into(),
        title: name.into(),
        asset_type,
        status,
        category: category.into(),
        description,
        source_path: display_path(path),
        scope: Some(scope),
        updated_at: metadata_modified(path),
        mount_targets: vec![],
    }
}

fn count_assets(assets: &[AssetSummary]) -> AssetCounts {
    let skills = assets
        .iter()
        .filter(|asset| asset.asset_type == AssetType::Skill)
        .count() as u32;
    let commands = assets
        .iter()
        .filter(|asset| asset.asset_type == AssetType::Command)
        .count() as u32;
    let mcps = assets
        .iter()
        .filter(|asset| asset.asset_type == AssetType::Mcp)
        .count() as u32;
    AssetCounts {
        total: skills + commands + mcps,
        skills,
        commands,
        mcps,
    }
}

#[cfg(test)]
fn safe_git_status(repository_path: String, message: &str) -> GitStatus {
    GitStatus {
        repository_path,
        is_repository: false,
        status_message: message.into(),
        branch: "".into(),
        remote: None,
        clean: true,
        ahead: 0,
        behind: 0,
        changed_files: vec![],
        conflicts: vec![],
        last_synced_at: None,
    }
}

#[cfg(test)]
fn git_success(cwd: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn git_output(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
fn parse_changed_files(status: &str) -> Vec<String> {
    status
        .lines()
        .filter_map(|line| line.get(3..).map(str::trim))
        .filter(|file| !file.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
fn parse_conflicts(status: &str) -> Vec<String> {
    status
        .lines()
        .filter(|line| {
            let code = line.get(0..2).unwrap_or("");
            matches!(code, "UU" | "AA" | "DD" | "AU" | "UA" | "DU" | "UD")
        })
        .filter_map(|line| line.get(3..).map(str::trim))
        .filter(|file| !file.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
fn parse_ahead_behind(value: &str) -> (u32, u32) {
    let mut parts = value.split_whitespace();
    let ahead = parts.next().and_then(|item| item.parse().ok()).unwrap_or(0);
    let behind = parts.next().and_then(|item| item.parse().ok()).unwrap_or(0);
    (ahead, behind)
}

fn first_readable_markdown_description(dir: &Path) -> Option<String> {
    for name in ["SKILL.md", "README.md"] {
        let path = dir.join(name);
        if let Some(description) = first_paragraph(&path) {
            return Some(description);
        }
    }
    None
}

fn first_paragraph(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let mut lines = Vec::new();
    for line in content.lines().map(str::trim) {
        if line.is_empty() {
            if !lines.is_empty() {
                break;
            }
            continue;
        }
        lines.push(line.trim_start_matches('#').trim().to_string());
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join(" "))
    }
}

fn json_file_is_valid(path: &Path) -> bool {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .is_some()
}

fn metadata_modified(path: &Path) -> Option<String> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .map(modified_time_iso)
}

fn file_stem_or_name(path: &Path) -> Option<String> {
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
}

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}
