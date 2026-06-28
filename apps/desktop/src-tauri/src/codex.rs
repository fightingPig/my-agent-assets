use crate::contracts::{
    AssetStatus, CodexDiscoveryInput, CodexMcpListResult, CodexMcpServerSummary, CodexMcpTransport,
    CodexScope, CodexSkillListResult, CodexSkillSummary,
};
use crate::path_utils::{display_path, expand_tilde, home_dir, modified_time_iso};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

pub fn list_codex_skills_command(input: CodexDiscoveryInput) -> CodexSkillListResult {
    let Some(home) = home_dir() else {
        return CodexSkillListResult {
            skills: vec![],
            warnings: vec!["HOME is unavailable; Codex Skill discovery skipped.".into()],
        };
    };
    let project_start = resolve_project_start(&home, input.project_path.as_deref());
    list_codex_skills_for_context(
        &home,
        project_start.as_deref(),
        Some(Path::new("/etc/codex/skills")),
    )
}

pub fn list_codex_mcp_servers_command(input: CodexDiscoveryInput) -> CodexMcpListResult {
    let Some(home) = home_dir() else {
        return CodexMcpListResult {
            servers: vec![],
            warnings: vec!["HOME is unavailable; Codex MCP discovery skipped.".into()],
        };
    };
    let project_start = resolve_project_start(&home, input.project_path.as_deref());
    list_codex_mcp_servers_for_context(&home, project_start.as_deref())
}

pub fn list_codex_skills_for_context(
    home: &Path,
    project_start: Option<&Path>,
    system_skills_root: Option<&Path>,
) -> CodexSkillListResult {
    let mut skills = Vec::new();
    let mut warnings = Vec::new();
    let mut seen = HashSet::new();

    read_skill_root(
        &home.join(".agents/skills"),
        CodexScope::Global,
        &mut skills,
        &mut warnings,
        &mut seen,
    );

    if let Some(start) = project_start {
        for root in project_ancestor_roots(start, home) {
            read_skill_root(
                &root.join(".agents/skills"),
                CodexScope::Project,
                &mut skills,
                &mut warnings,
                &mut seen,
            );
        }
    }

    if let Some(root) = system_skills_root {
        read_skill_root(
            root,
            CodexScope::System,
            &mut skills,
            &mut warnings,
            &mut seen,
        );
    }

    skills.sort_by(|left, right| {
        scope_rank(&left.scope)
            .cmp(&scope_rank(&right.scope))
            .then(left.name.cmp(&right.name))
            .then(left.path.cmp(&right.path))
    });
    CodexSkillListResult { skills, warnings }
}

pub fn list_codex_mcp_servers_for_context(
    home: &Path,
    project_start: Option<&Path>,
) -> CodexMcpListResult {
    let mut servers = Vec::new();
    let mut warnings = Vec::new();
    read_mcp_config(
        &home.join(".codex/config.toml"),
        CodexScope::Global,
        &mut servers,
        &mut warnings,
    );

    if let Some(start) = project_start {
        let root = find_repo_root(start, home).unwrap_or_else(|| start.to_path_buf());
        read_mcp_config(
            &root.join(".codex/config.toml"),
            CodexScope::Project,
            &mut servers,
            &mut warnings,
        );
    }

    servers.sort_by(|left, right| {
        scope_rank(&left.scope)
            .cmp(&scope_rank(&right.scope))
            .then(left.name.cmp(&right.name))
    });
    CodexMcpListResult { servers, warnings }
}

fn resolve_project_start(home: &Path, project_path: Option<&str>) -> Option<PathBuf> {
    project_path
        .map(|path| expand_tilde(path, home))
        .or_else(|| std::env::current_dir().ok())
        .filter(|path| path.exists())
}

fn project_ancestor_roots(start: &Path, home: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut current = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        roots.push(current.clone());
        if is_repo_root(&current) || current == home {
            break;
        }
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent.to_path_buf();
    }
    roots
}

fn find_repo_root(start: &Path, home: &Path) -> Option<PathBuf> {
    project_ancestor_roots(start, home)
        .into_iter()
        .find(|path| is_repo_root(path))
}

fn is_repo_root(path: &Path) -> bool {
    path.join(".git").exists()
        || path.join("Cargo.toml").is_file()
        || path.join("package.json").is_file()
}

fn read_skill_root(
    root: &Path,
    scope: CodexScope,
    skills: &mut Vec<CodexSkillSummary>,
    warnings: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    if !root.exists() {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        warnings.push(format!(
            "Could not read Codex Skill root {}.",
            display_path(root)
        ));
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_file = path.join("SKILL.md");
        if !skill_file.is_file() {
            continue;
        }

        let identity = fs::canonicalize(&path)
            .map(|path| display_path(&path))
            .unwrap_or_else(|_| display_path(&path));
        if !seen.insert(identity) {
            continue;
        }

        let text = match fs::read_to_string(&skill_file) {
            Ok(text) => text,
            Err(error) => {
                warnings.push(format!(
                    "Could not read Codex Skill {}: {}",
                    display_path(&skill_file),
                    error
                ));
                continue;
            }
        };
        let folder_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".into());
        let metadata = parse_skill_metadata(&text, &folder_name);
        let mut skill_warnings = Vec::new();
        if metadata.description.is_empty() {
            skill_warnings.push("SKILL.md does not provide a description.".into());
        }
        let symlink_target = fs::symlink_metadata(&path)
            .ok()
            .filter(|metadata| metadata.file_type().is_symlink())
            .and_then(|_| fs::read_link(&path).ok())
            .map(|target| {
                if target.is_absolute() {
                    display_path(&target)
                } else {
                    display_path(&root.join(target))
                }
            });
        let updated_at = fs::metadata(&skill_file)
            .and_then(|metadata| metadata.modified())
            .ok()
            .map(modified_time_iso);

        skills.push(CodexSkillSummary {
            id: format!("codex-skill:{}:{}", scope_wire(&scope), metadata.name),
            name: metadata.name,
            description: metadata.description,
            scope: scope.clone(),
            path: display_path(&path),
            status: AssetStatus::Ready,
            has_scripts: path.join("scripts").is_dir(),
            has_references: path.join("references").is_dir(),
            has_assets: path.join("assets").is_dir(),
            has_openai_metadata: path.join("agents/openai.yaml").is_file(),
            symlink_target,
            updated_at,
            warnings: skill_warnings,
        });
    }
}

fn read_mcp_config(
    config_path: &Path,
    scope: CodexScope,
    servers: &mut Vec<CodexMcpServerSummary>,
    warnings: &mut Vec<String>,
) {
    if !config_path.is_file() {
        return;
    }
    let text = match fs::read_to_string(config_path) {
        Ok(text) => text,
        Err(error) => {
            warnings.push(format!(
                "Could not read Codex MCP config {}: {}",
                display_path(config_path),
                error
            ));
            return;
        }
    };
    let config = match text.parse::<Value>() {
        Ok(config) => config,
        Err(error) => {
            warnings.push(format!(
                "Could not parse Codex MCP config {}: {}",
                display_path(config_path),
                error
            ));
            return;
        }
    };
    let Some(mcp_servers) = config.get("mcp_servers").and_then(Value::as_table) else {
        return;
    };

    for (name, value) in mcp_servers {
        let Some(table) = value.as_table() else {
            warnings.push(format!(
                "mcp_servers.{} in {} is not a table.",
                name,
                display_path(config_path)
            ));
            continue;
        };
        let command = string_field(table, "command");
        let url = string_field(table, "url");
        let transport = if url.is_some() {
            CodexMcpTransport::StreamableHttp
        } else if command.is_some() {
            CodexMcpTransport::Stdio
        } else {
            CodexMcpTransport::Unknown
        };
        let mut server_warnings = Vec::new();
        if transport == CodexMcpTransport::Unknown {
            server_warnings.push("Server has neither command nor url.".into());
        }
        if table.contains_key("oauth") || table.contains_key("bearer_token_env_var") {
            server_warnings
                .push("Authentication may be required; OAuth tokens are not managed.".into());
        }

        servers.push(CodexMcpServerSummary {
            id: format!("codex-mcp:{}:{}", scope_wire(&scope), name),
            name: name.clone(),
            scope: scope.clone(),
            config_path: display_path(config_path),
            transport,
            command,
            args: string_array_field(table, "args"),
            url,
            enabled: table
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            enabled_tools: string_array_field(table, "enabled_tools"),
            disabled_tools: string_array_field(table, "disabled_tools"),
            approval_mode: string_field(table, "approval_mode")
                .or_else(|| string_field(table, "approval_policy"))
                .or_else(|| string_field(table, "approval")),
            warnings: server_warnings,
        });
    }
}

fn string_field(table: &toml::map::Map<String, Value>, key: &str) -> Option<String> {
    table.get(key).and_then(Value::as_str).map(str::to_string)
}

fn string_array_field(table: &toml::map::Map<String, Value>, key: &str) -> Vec<String> {
    table
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

struct SkillMetadata {
    name: String,
    description: String,
}

fn parse_skill_metadata(text: &str, fallback_name: &str) -> SkillMetadata {
    let mut name = None;
    let mut description = None;
    let mut lines = text.lines();
    if lines.next().map(str::trim) == Some("---") {
        for line in &mut lines {
            let line = line.trim();
            if line == "---" {
                break;
            }
            if let Some(value) = line.strip_prefix("name:") {
                name = Some(trim_yaml_scalar(value));
            } else if let Some(value) = line.strip_prefix("description:") {
                description = Some(trim_yaml_scalar(value));
            }
        }
    }
    let description = description
        .filter(|value| !value.is_empty())
        .or_else(|| first_content_paragraph(text))
        .unwrap_or_default();
    SkillMetadata {
        name: name
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| fallback_name.into()),
        description,
    }
}

fn trim_yaml_scalar(value: &str) -> String {
    value
        .trim()
        .trim_matches(|character| character == '"' || character == '\'')
        .to_string()
}

fn first_content_paragraph(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && *line != "---"
                && !line.starts_with('#')
                && !line.starts_with("name:")
                && !line.starts_with("description:")
        })
        .map(str::to_string)
}

fn scope_rank(scope: &CodexScope) -> u8 {
    match scope {
        CodexScope::Global => 0,
        CodexScope::Project => 1,
        CodexScope::System => 2,
    }
}

fn scope_wire(scope: &CodexScope) -> &'static str {
    match scope {
        CodexScope::Global => "global",
        CodexScope::Project => "project",
        CodexScope::System => "system",
    }
}
