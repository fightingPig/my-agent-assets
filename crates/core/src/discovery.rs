use crate::managed_projects::load as load_managed_projects;
use crate::path_safety::{expand_tilde, is_link_or_junction, validate_single_path_component};
use crate::settings;
pub use crate::targets::{AssetKind, RuntimeProvider};
use crate::{
    mcp::{import_claude_server, import_codex_server, CanonicalMcp},
    MaaError, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::Document;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceFormat {
    #[serde(rename = "skill_directory")]
    SkillDirectory,
    #[serde(rename = "markdown")]
    Markdown,
    #[serde(rename = "claude_mcp_json")]
    ClaudeMcpJson,
    #[serde(rename = "codex_mcp_toml")]
    CodexMcpToml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceScope {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "project")]
    Project,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiscoveryScope {
    User,
    Project {
        #[serde(rename = "projectPath")]
        project_path: PathBuf,
    },
    ManagedProjects {
        #[serde(rename = "projectIds", default)]
        project_ids: Vec<String>,
    },
    Custom {
        path: PathBuf,
        #[serde(rename = "assetKind")]
        asset_kind: AssetKind,
        #[serde(rename = "sourceFormat")]
        source_format: SourceFormat,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredSource {
    pub source_id: String,
    pub provider: RuntimeProvider,
    pub source_path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_path: Option<PathBuf>,
    pub asset_kind: AssetKind,
    pub asset_name: String,
    pub source_format: SourceFormat,
    pub scope: SourceScope,
    pub is_managed: bool,
    pub is_symlink: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symlink_target: Option<PathBuf>,
    #[serde(default)]
    pub warnings: Vec<String>,
    pub eligible_import: bool,
    pub eligible_adopt: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResult {
    pub sources: Vec<DiscoveredSource>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedMcpSource {
    pub canonical: CanonicalMcp,
    pub raw_source: String,
}

pub fn discover(home: &Path, scope: DiscoveryScope) -> DiscoveryResult {
    let mut result = DiscoveryResult {
        sources: Vec::new(),
        warnings: Vec::new(),
    };
    match scope {
        DiscoveryScope::User => {
            scan_skill_dir(
                home,
                &home.join(".claude/skills"),
                RuntimeProvider::ClaudeCode,
                SourceScope::User,
                &mut result,
            );
            scan_command_dir(
                home,
                &home.join(".claude/commands"),
                RuntimeProvider::ClaudeCode,
                SourceScope::User,
                &mut result,
            );
            scan_claude_mcp(
                home,
                &home.join(".claude.json"),
                SourceScope::User,
                &mut result,
            );
            scan_skill_dir(
                home,
                &home.join(".agents/skills"),
                RuntimeProvider::Codex,
                SourceScope::User,
                &mut result,
            );
            scan_codex_mcp(
                home,
                &home.join(".codex/config.toml"),
                SourceScope::User,
                &mut result,
            );
        }
        DiscoveryScope::Project { project_path } => {
            scan_project_runtime(home, &expand_input_path(&project_path, home), &mut result);
        }
        DiscoveryScope::ManagedProjects { project_ids } => {
            scan_managed_projects(home, &project_ids, &mut result);
        }
        DiscoveryScope::Custom {
            path,
            asset_kind,
            source_format,
        } => {
            let path = expand_input_path(&path, home);
            match (asset_kind, source_format) {
                (AssetKind::Skill, SourceFormat::SkillDirectory)
                | (AssetKind::Skill, SourceFormat::Markdown) => scan_skill_dir(
                    home,
                    &path,
                    RuntimeProvider::Custom,
                    SourceScope::Custom,
                    &mut result,
                ),
                (AssetKind::Command, SourceFormat::Markdown) => scan_command_dir(
                    home,
                    &path,
                    RuntimeProvider::Custom,
                    SourceScope::Custom,
                    &mut result,
                ),
                (AssetKind::Mcp, SourceFormat::ClaudeMcpJson) => scan_claude_mcp_with_provider(
                    home,
                    &path,
                    RuntimeProvider::Custom,
                    SourceScope::Custom,
                    &mut result,
                ),
                (AssetKind::Mcp, SourceFormat::CodexMcpToml) => scan_codex_mcp_with_provider(
                    home,
                    &path,
                    RuntimeProvider::Custom,
                    SourceScope::Custom,
                    &mut result,
                ),
                _ => result.warnings.push(format!(
                    "custom source format {:?} is incompatible with {:?}",
                    source_format, asset_kind
                )),
            }
        }
    }
    result.sources.sort_by(|left, right| {
        (left.asset_kind as u8, &left.asset_name, &left.source_id).cmp(&(
            right.asset_kind as u8,
            &right.asset_name,
            &right.source_id,
        ))
    });
    result
}

fn scan_project_runtime(home: &Path, project: &Path, result: &mut DiscoveryResult) {
    scan_skill_dir(
        home,
        &project.join(".claude/skills"),
        RuntimeProvider::ClaudeCode,
        SourceScope::Project,
        result,
    );
    scan_command_dir(
        home,
        &project.join(".claude/commands"),
        RuntimeProvider::ClaudeCode,
        SourceScope::Project,
        result,
    );
    scan_claude_mcp(
        home,
        &project.join(".mcp.json"),
        SourceScope::Project,
        result,
    );
    scan_skill_dir(
        home,
        &project.join(".agents/skills"),
        RuntimeProvider::Codex,
        SourceScope::Project,
        result,
    );
    scan_codex_mcp(
        home,
        &project.join(".codex/config.toml"),
        SourceScope::Project,
        result,
    );
}

fn scan_managed_projects(home: &Path, requested_ids: &[String], result: &mut DiscoveryResult) {
    let registry = match load_managed_projects(home) {
        Ok(registry) => registry,
        Err(error) => {
            result.warnings.push(format!(
                "managed project registry could not be read: {error}"
            ));
            return;
        }
    };
    let settings = match settings::load(home) {
        Ok(settings) => settings,
        Err(error) => {
            result
                .warnings
                .push(format!("scan settings could not be read: {error}"));
            return;
        }
    };
    let requested = requested_ids
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    for project in &registry.projects {
        if !requested.is_empty() && !requested.contains(&project.id) {
            continue;
        }
        if !project.path.is_dir() {
            result.warnings.push(format!(
                "managed project path is unavailable: {}",
                project.path.display()
            ));
            continue;
        }
        scan_managed_project_tree(home, &project.path, 0, settings.max_depth as usize, result);
    }
    for requested_id in requested_ids {
        if !registry
            .projects
            .iter()
            .any(|project| &project.id == requested_id)
        {
            result
                .warnings
                .push(format!("managed project is unavailable: {requested_id}"));
        }
    }
}

fn scan_managed_project_tree(
    home: &Path,
    directory: &Path,
    depth: usize,
    max_depth: usize,
    result: &mut DiscoveryResult,
) {
    if depth > max_depth || should_skip_project_child(directory) || is_project_symlink(directory) {
        return;
    }
    scan_project_runtime(home, directory, result);
    if depth == max_depth {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        result.warnings.push(format!(
            "cannot read managed project directory: {}",
            directory.display()
        ));
        return;
    };
    let mut children = entries
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    children.sort();
    for child in children.into_iter().filter(|child| child.is_dir()) {
        scan_managed_project_tree(home, &child, depth + 1, max_depth, result);
    }
}

fn should_skip_project_child(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(
            ".git"
                | "node_modules"
                | "dist"
                | "build"
                | "target"
                | ".venv"
                | "__pycache__"
                | ".claude"
                | ".agents"
                | ".codex"
        )
    )
}

fn is_project_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| is_link_or_junction(&metadata))
        .unwrap_or(true)
}

pub fn load_mcp_source(source: &DiscoveredSource) -> Result<LoadedMcpSource> {
    if source.asset_kind != AssetKind::Mcp {
        return Err(MaaError::new("discovered source is not an MCP asset"));
    }
    let config_path = source
        .config_path
        .as_deref()
        .ok_or_else(|| MaaError::new("MCP source is missing configPath"))?;
    let text = fs::read_to_string(config_path).map_err(|error| {
        MaaError::new(format!(
            "failed to read MCP source {}: {error}",
            config_path.display()
        ))
    })?;
    match source.source_format {
        SourceFormat::ClaudeMcpJson => {
            let root: JsonValue = serde_json::from_str(&text).map_err(|error| {
                MaaError::new(format!(
                    "invalid Claude MCP JSON {}: {error}",
                    config_path.display()
                ))
            })?;
            let server = root
                .get("mcpServers")
                .and_then(JsonValue::as_object)
                .and_then(|servers| servers.get(&source.asset_name))
                .ok_or_else(|| {
                    MaaError::new(format!(
                        "MCP server '{}' no longer exists in {}",
                        source.asset_name,
                        config_path.display()
                    ))
                })?;
            let canonical = import_claude_server(&source.asset_name, server)
                .map_err(|error| MaaError::new(error.to_string()))?;
            Ok(LoadedMcpSource {
                canonical,
                raw_source: serde_json::to_string_pretty(server)
                    .map_err(|error| MaaError::new(error.to_string()))?,
            })
        }
        SourceFormat::CodexMcpToml => {
            let root: toml::Value = toml::from_str(&text).map_err(|error| {
                MaaError::new(format!(
                    "invalid Codex MCP TOML {}: {error}",
                    config_path.display()
                ))
            })?;
            let server = root
                .get("mcp_servers")
                .and_then(toml::Value::as_table)
                .and_then(|servers| servers.get(&source.asset_name))
                .ok_or_else(|| {
                    MaaError::new(format!(
                        "MCP server '{}' no longer exists in {}",
                        source.asset_name,
                        config_path.display()
                    ))
                })?;
            let canonical = import_codex_server(&source.asset_name, server)
                .map_err(|error| MaaError::new(error.to_string()))?;
            Ok(LoadedMcpSource {
                canonical,
                raw_source: toml::to_string_pretty(server)
                    .map_err(|error| MaaError::new(error.to_string()))?,
            })
        }
        _ => Err(MaaError::new(format!(
            "source format {:?} cannot contain MCP configuration",
            source.source_format
        ))),
    }
}

fn scan_skill_dir(
    home: &Path,
    directory: &Path,
    provider: RuntimeProvider,
    scope: SourceScope,
    result: &mut DiscoveryResult,
) {
    let entries = match read_dir_optional(directory, result) {
        Some(entries) => entries,
        None => return,
    };
    for path in entries {
        if path.is_dir() && path.join("SKILL.md").is_file() {
            let name = file_stem_or_name(&path);
            push_file_source(
                home,
                path,
                provider,
                scope,
                AssetKind::Skill,
                name,
                SourceFormat::SkillDirectory,
                result,
            );
        } else if is_extension(&path, "md") {
            let name = file_stem_or_name(&path);
            push_file_source(
                home,
                path,
                provider,
                scope,
                AssetKind::Skill,
                name,
                SourceFormat::Markdown,
                result,
            );
        }
    }
}

fn scan_command_dir(
    home: &Path,
    directory: &Path,
    provider: RuntimeProvider,
    scope: SourceScope,
    result: &mut DiscoveryResult,
) {
    let entries = match read_dir_optional(directory, result) {
        Some(entries) => entries,
        None => return,
    };
    for path in entries.into_iter().filter(|path| is_extension(path, "md")) {
        let name = file_stem_or_name(&path);
        push_file_source(
            home,
            path,
            provider,
            scope,
            AssetKind::Command,
            name,
            SourceFormat::Markdown,
            result,
        );
    }
}

fn scan_claude_mcp(
    home: &Path,
    config_path: &Path,
    scope: SourceScope,
    result: &mut DiscoveryResult,
) {
    scan_claude_mcp_with_provider(
        home,
        config_path,
        RuntimeProvider::ClaudeCode,
        scope,
        result,
    );
}

fn scan_claude_mcp_with_provider(
    home: &Path,
    config_path: &Path,
    provider: RuntimeProvider,
    scope: SourceScope,
    result: &mut DiscoveryResult,
) {
    if !config_path.exists() {
        return;
    }
    let content = match fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(error) => {
            result.warnings.push(format!(
                "could not read MCP JSON {}: {error}",
                config_path.display()
            ));
            return;
        }
    };
    let root: JsonValue = match serde_json::from_str(&content) {
        Ok(root) => root,
        Err(error) => {
            result.warnings.push(format!(
                "invalid MCP JSON {}: {error}",
                config_path.display()
            ));
            return;
        }
    };
    let Some(servers) = root.get("mcpServers").and_then(JsonValue::as_object) else {
        return;
    };
    for (name, value) in servers {
        let mut warnings = Vec::new();
        let valid =
            validate_single_path_component(name, "MCP server name").is_ok() && value.is_object();
        if !value.is_object() {
            warnings.push("MCP server entry must be a JSON object".into());
        }
        push_mcp_source(
            home,
            config_path,
            provider,
            scope,
            name,
            SourceFormat::ClaudeMcpJson,
            warnings,
            valid,
            result,
        );
    }
}

fn scan_codex_mcp(
    home: &Path,
    config_path: &Path,
    scope: SourceScope,
    result: &mut DiscoveryResult,
) {
    scan_codex_mcp_with_provider(home, config_path, RuntimeProvider::Codex, scope, result);
}

fn scan_codex_mcp_with_provider(
    home: &Path,
    config_path: &Path,
    provider: RuntimeProvider,
    scope: SourceScope,
    result: &mut DiscoveryResult,
) {
    if !config_path.exists() {
        return;
    }
    let content = match fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(error) => {
            result.warnings.push(format!(
                "could not read MCP TOML {}: {error}",
                config_path.display()
            ));
            return;
        }
    };
    let document = match content.parse::<Document>() {
        Ok(document) => document,
        Err(error) => {
            result.warnings.push(format!(
                "invalid MCP TOML {}: {error}",
                config_path.display()
            ));
            return;
        }
    };
    let Some(servers) = document.get("mcp_servers").and_then(|item| item.as_table()) else {
        return;
    };
    for (name, item) in servers.iter() {
        let mut warnings = Vec::new();
        let valid =
            validate_single_path_component(name, "MCP server name").is_ok() && item.is_table_like();
        if !item.is_table_like() {
            warnings.push("Codex MCP server entry must be a TOML table".into());
        }
        push_mcp_source(
            home,
            config_path,
            provider,
            scope,
            name,
            SourceFormat::CodexMcpToml,
            warnings,
            valid,
            result,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_file_source(
    home: &Path,
    path: PathBuf,
    provider: RuntimeProvider,
    scope: SourceScope,
    asset_kind: AssetKind,
    name: String,
    source_format: SourceFormat,
    result: &mut DiscoveryResult,
) {
    let mut warnings = Vec::new();
    let valid = validate_single_path_component(&name, "asset name").is_ok();
    if !valid {
        warnings.push("asset name is not a safe single path component".into());
    }
    let (is_symlink, symlink_target) = link_metadata(&path, &mut warnings);
    let is_managed = symlink_target
        .as_deref()
        .is_some_and(|target| target.starts_with(home.join(".my-agent-assets/assets")));
    result.sources.push(DiscoveredSource {
        source_id: source_id(provider, scope, asset_kind, &name, &path),
        provider,
        source_path: path,
        config_path: None,
        asset_kind,
        asset_name: name,
        source_format,
        scope,
        is_managed,
        is_symlink,
        symlink_target,
        warnings,
        eligible_import: valid && !is_managed,
        eligible_adopt: valid && !is_managed,
    });
}

#[allow(clippy::too_many_arguments)]
fn push_mcp_source(
    home: &Path,
    config_path: &Path,
    provider: RuntimeProvider,
    scope: SourceScope,
    name: &str,
    source_format: SourceFormat,
    mut warnings: Vec<String>,
    valid: bool,
    result: &mut DiscoveryResult,
) {
    if validate_single_path_component(name, "MCP server name").is_err() {
        warnings.push("MCP name is not a safe single path component".into());
    }
    result.sources.push(DiscoveredSource {
        source_id: source_id(provider, scope, AssetKind::Mcp, name, config_path),
        provider,
        source_path: config_path.to_path_buf(),
        config_path: Some(config_path.to_path_buf()),
        asset_kind: AssetKind::Mcp,
        asset_name: name.to_string(),
        source_format,
        scope,
        is_managed: config_path.starts_with(home.join(".my-agent-assets")),
        is_symlink: false,
        symlink_target: None,
        warnings,
        eligible_import: valid && validate_single_path_component(name, "MCP server name").is_ok(),
        eligible_adopt: false,
    });
}

fn read_dir_optional(directory: &Path, result: &mut DiscoveryResult) -> Option<Vec<PathBuf>> {
    if !directory.exists() {
        return None;
    }
    match fs::read_dir(directory) {
        Ok(entries) => {
            let mut paths = entries
                .filter_map(|entry| match entry {
                    Ok(entry) => Some(entry.path()),
                    Err(error) => {
                        result.warnings.push(format!(
                            "could not read entry in {}: {error}",
                            directory.display()
                        ));
                        None
                    }
                })
                .collect::<Vec<_>>();
            paths.sort();
            Some(paths)
        }
        Err(error) => {
            result.warnings.push(format!(
                "could not scan source directory {}: {error}",
                directory.display()
            ));
            None
        }
    }
}

fn link_metadata(path: &Path, warnings: &mut Vec<String>) -> (bool, Option<PathBuf>) {
    match fs::symlink_metadata(path) {
        Ok(metadata) if is_link_or_junction(&metadata) => match fs::read_link(path) {
            Ok(target) => {
                let target = if target.is_absolute() {
                    target
                } else {
                    path.parent().unwrap_or_else(|| Path::new(".")).join(target)
                };
                (true, Some(target))
            }
            Err(error) => {
                warnings.push(format!("could not read symlink target: {error}"));
                (true, None)
            }
        },
        Ok(_) => (false, None),
        Err(error) => {
            warnings.push(format!("could not inspect source: {error}"));
            (false, None)
        }
    }
}

fn source_id(
    provider: RuntimeProvider,
    scope: SourceScope,
    kind: AssetKind,
    name: &str,
    path: &Path,
) -> String {
    let identity = format!("{provider:?}|{scope:?}|{kind:?}|{}|{name}", path.display());
    let hash = identity
        .bytes()
        .fold(0xcbf29ce484222325_u64, |value, byte| {
            value.wrapping_mul(0x100000001b3) ^ u64::from(byte)
        });
    format!(
        "{}:{}:{}:{hash:016x}:{name}",
        provider_wire(provider),
        scope_wire(scope),
        kind_wire(kind)
    )
}

fn provider_wire(provider: RuntimeProvider) -> &'static str {
    match provider {
        RuntimeProvider::ClaudeCode => "claude",
        RuntimeProvider::Codex => "codex",
        RuntimeProvider::Custom => "custom",
    }
}

fn scope_wire(scope: SourceScope) -> &'static str {
    match scope {
        SourceScope::User => "user",
        SourceScope::Project => "project",
        SourceScope::Custom => "custom",
    }
}

fn kind_wire(kind: AssetKind) -> &'static str {
    match kind {
        AssetKind::Skill => "skill",
        AssetKind::Command => "command",
        AssetKind::Mcp => "mcp",
    }
}

fn is_extension(path: &Path, extension: &str) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

fn expand_input_path(path: &Path, home: &Path) -> PathBuf {
    expand_tilde(&path.to_string_lossy(), home)
}

fn file_stem_or_name(path: &Path) -> String {
    if path.is_dir() {
        path.file_name()
    } else {
        path.file_stem()
    }
    .and_then(|value| value.to_str())
    .unwrap_or_default()
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn user_scan_discovers_claude_and_codex_without_project_guessing() {
        let home = test_dir("user");
        fs::create_dir_all(home.join(".claude/skills/review")).unwrap();
        fs::write(home.join(".claude/skills/review/SKILL.md"), "# Review").unwrap();
        fs::create_dir_all(home.join(".claude/commands")).unwrap();
        fs::write(home.join(".claude/commands/commit.md"), "# Commit").unwrap();
        fs::write(
            home.join(".claude.json"),
            r#"{"mcpServers":{"postgres":{"command":"npx"}}}"#,
        )
        .unwrap();
        fs::create_dir_all(home.join(".agents/skills/codex-review")).unwrap();
        fs::write(
            home.join(".agents/skills/codex-review/SKILL.md"),
            "# Codex Review",
        )
        .unwrap();
        fs::create_dir_all(home.join(".codex")).unwrap();
        fs::write(
            home.join(".codex/config.toml"),
            "[mcp_servers.filesystem]\ncommand = \"npx\"\n",
        )
        .unwrap();
        fs::create_dir_all(home.join("random/.claude/skills/ignored")).unwrap();
        fs::write(
            home.join("random/.claude/skills/ignored/SKILL.md"),
            "# Ignored",
        )
        .unwrap();

        let result = discover(&home, DiscoveryScope::User);
        let names = result
            .sources
            .iter()
            .map(|source| source.asset_name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            ["codex-review", "review", "commit", "filesystem", "postgres"]
        );
        assert!(!names.contains(&"ignored"));
        assert!(result.warnings.is_empty());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn project_scan_requires_and_limits_itself_to_explicit_project() {
        let home = test_dir("project");
        let selected = home.join("workspace/selected");
        let other = home.join("workspace/other");
        for root in [&selected, &other] {
            fs::create_dir_all(root.join(".agents/skills/api-design")).unwrap();
            fs::write(
                root.join(".agents/skills/api-design/SKILL.md"),
                "# API design",
            )
            .unwrap();
        }
        fs::create_dir_all(selected.join(".claude/skills/review")).unwrap();
        fs::write(selected.join(".claude/skills/review/SKILL.md"), "# Review").unwrap();
        let result = discover(
            &home,
            DiscoveryScope::Project {
                project_path: selected.clone(),
            },
        );
        assert_eq!(result.sources.len(), 2);
        assert!(result
            .sources
            .iter()
            .all(|source| source.source_path.starts_with(&selected)));
        assert!(result
            .sources
            .iter()
            .all(|source| !source.source_path.starts_with(&other)));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn custom_sources_require_a_known_kind_and_format_pair() {
        let home = test_dir("custom");
        let config = home.join("servers.toml");
        fs::create_dir_all(&home).unwrap();
        fs::write(&config, "[mcp_servers.sqlite]\ncommand = \"uvx\"\n").unwrap();
        let result = discover(
            &home,
            DiscoveryScope::Custom {
                path: config,
                asset_kind: AssetKind::Mcp,
                source_format: SourceFormat::CodexMcpToml,
            },
        );
        assert_eq!(result.sources.len(), 1);
        assert_eq!(result.sources[0].provider, RuntimeProvider::Custom);
        assert_eq!(result.sources[0].asset_name, "sqlite");
        assert!(!result.sources[0].eligible_adopt);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn damaged_configs_warn_without_failing_other_sources() {
        let home = test_dir("damaged");
        fs::create_dir_all(home.join(".claude/skills/direct")).unwrap();
        fs::write(home.join(".claude/skills/direct/SKILL.md"), "# Direct").unwrap();
        fs::write(home.join(".claude.json"), "{bad").unwrap();
        fs::create_dir_all(home.join(".codex")).unwrap();
        fs::write(home.join(".codex/config.toml"), "[bad").unwrap();

        let result = discover(&home, DiscoveryScope::User);
        assert_eq!(result.sources.len(), 1);
        assert_eq!(result.warnings.len(), 2);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn mcp_sources_reload_exact_entries_into_the_same_canonical_model() {
        let home = test_dir("mcp-load");
        fs::create_dir_all(&home).unwrap();
        fs::write(
            home.join(".claude.json"),
            r#"{"mcpServers":{"remote":{"type":"http","url":"https://example.test/mcp","headers":{"X-Test":"value"}}}}"#,
        )
        .unwrap();
        fs::create_dir_all(home.join(".codex")).unwrap();
        fs::write(
            home.join(".codex/config.toml"),
            "[mcp_servers.remote]\nurl = \"https://example.test/mcp\"\nhttp_headers = { X-Test = \"value\" }\n",
        )
        .unwrap();

        let result = discover(&home, DiscoveryScope::User);
        let mut remote = result
            .sources
            .iter()
            .filter(|source| source.asset_name == "remote")
            .map(load_mcp_source)
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(remote.len(), 2);
        assert_eq!(remote.remove(0).canonical, remote.remove(0).canonical);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn mcp_reload_fails_if_selected_entry_changed_after_discovery() {
        let home = test_dir("mcp-stale");
        fs::create_dir_all(&home).unwrap();
        let config = home.join(".claude.json");
        fs::write(
            &config,
            r#"{"mcpServers":{"filesystem":{"command":"npx"}}}"#,
        )
        .unwrap();
        let source = discover(&home, DiscoveryScope::User)
            .sources
            .into_iter()
            .next()
            .unwrap();
        fs::write(&config, r#"{"mcpServers":{}}"#).unwrap();
        let error = load_mcp_source(&source).unwrap_err();
        assert!(error.to_string().contains("no longer exists"));
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(unix)]
    #[test]
    fn managed_symlink_is_not_importable_or_adoptable() {
        use std::os::unix::fs::symlink;

        let home = test_dir("managed");
        let canonical = home.join(".my-agent-assets/assets/skills/review");
        fs::create_dir_all(&canonical).unwrap();
        fs::write(canonical.join("SKILL.md"), "# Review").unwrap();
        fs::create_dir_all(home.join(".claude/skills")).unwrap();
        symlink(&canonical, home.join(".claude/skills/review")).unwrap();

        let result = discover(&home, DiscoveryScope::User);
        assert_eq!(result.sources.len(), 1);
        assert!(result.sources[0].is_managed);
        assert!(!result.sources[0].eligible_import);
        assert!(!result.sources[0].eligible_adopt);
        let _ = fs::remove_dir_all(home);
    }

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "maa-discovery-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&path);
        path
    }
}
