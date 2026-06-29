//! Canonical MCP model and pure Claude/Codex live-config renderers.
//!
//! This module deliberately performs no filesystem access. Callers own target
//! authorization, backups, locking, atomic writes, and operation journals.

use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::BTreeMap;
use std::fmt;
use toml_edit::{Array, Document, InlineTable, Item, Table, Value as TomlValue};

const INTERNAL_FIELDS: &[&str] = &[
    "enabled",
    "source",
    "id",
    "name",
    "description",
    "tags",
    "homepage",
    "docs",
];

const CODEX_EXTENSION_FIELDS: &[&str] = &[
    "env_vars",
    "env_http_headers",
    "bearer_token_env_var",
    "startup_timeout_sec",
    "tool_timeout_sec",
    "enabled_tools",
    "disabled_tools",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalMcp {
    pub schema_version: u32,
    pub name: String,
    pub spec: McpSpec,
    #[serde(default)]
    pub provider_extensions: JsonMap<String, JsonValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpSpec {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<McpTransport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    #[serde(flatten)]
    pub extra: JsonMap<String, JsonValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpTransport {
    #[serde(rename = "stdio")]
    Stdio,
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "sse")]
    Sse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaudeScope<'a> {
    User,
    Local { project_path: &'a str },
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexScope {
    User,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered<T> {
    pub content: T,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedCommand {
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpError {
    InvalidCanonical(String),
    InvalidLiveConfig(String),
    IncompatibleTarget(String),
}

impl fmt::Display for McpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCanonical(message) => {
                write!(formatter, "invalid canonical MCP: {message}")
            }
            Self::InvalidLiveConfig(message) => write!(formatter, "invalid live config: {message}"),
            Self::IncompatibleTarget(message) => {
                write!(formatter, "incompatible MCP target: {message}")
            }
        }
    }
}

impl std::error::Error for McpError {}

impl CanonicalMcp {
    pub fn from_json(value: JsonValue) -> Result<Self, McpError> {
        let canonical: Self = serde_json::from_value(value)
            .map_err(|error| McpError::InvalidCanonical(error.to_string()))?;
        canonical.validate()?;
        Ok(canonical)
    }

    pub fn validate(&self) -> Result<(), McpError> {
        if self.schema_version != 1 {
            return Err(McpError::InvalidCanonical(format!(
                "unsupported schemaVersion {}; expected 1",
                self.schema_version
            )));
        }
        validate_server_name(&self.name)?;

        match self.spec.transport.unwrap_or(McpTransport::Stdio) {
            McpTransport::Stdio => {
                let command = self.spec.command.as_deref().unwrap_or_default().trim();
                if command.is_empty() {
                    return Err(McpError::InvalidCanonical(
                        "stdio requires a non-empty command".into(),
                    ));
                }
                if self.spec.url.is_some() {
                    return Err(McpError::InvalidCanonical(
                        "stdio must not define url".into(),
                    ));
                }
            }
            McpTransport::Http | McpTransport::Sse => {
                let url = self.spec.url.as_deref().unwrap_or_default().trim();
                if !is_http_url(url) {
                    return Err(McpError::InvalidCanonical(
                        "http/sse requires an http:// or https:// URL".into(),
                    ));
                }
                if self.spec.command.is_some() {
                    return Err(McpError::InvalidCanonical(
                        "http/sse must not define command".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn import_claude_server(name: &str, server: &JsonValue) -> Result<CanonicalMcp, McpError> {
    validate_server_name(name)?;
    let mut object = server.as_object().cloned().ok_or_else(|| {
        McpError::InvalidLiveConfig(format!("Claude MCP server '{name}' must be an object"))
    })?;
    for field in INTERNAL_FIELDS {
        object.remove(*field);
    }
    let spec: McpSpec = serde_json::from_value(JsonValue::Object(object))
        .map_err(|error| McpError::InvalidLiveConfig(error.to_string()))?;
    let canonical = CanonicalMcp {
        schema_version: 1,
        name: name.to_string(),
        spec,
        provider_extensions: JsonMap::new(),
    };
    canonical.validate()?;
    Ok(canonical)
}

pub fn import_codex_server(name: &str, server: &toml::Value) -> Result<CanonicalMcp, McpError> {
    validate_server_name(name)?;
    let mut object = serde_json::to_value(server)
        .map_err(|error| McpError::InvalidLiveConfig(error.to_string()))?
        .as_object()
        .cloned()
        .ok_or_else(|| {
            McpError::InvalidLiveConfig(format!("Codex MCP server '{name}' must be a table"))
        })?;

    if let Some(headers) = object.remove("http_headers") {
        object.insert("headers".into(), headers);
    }
    if !object.contains_key("type") {
        if object.contains_key("url") {
            object.insert("type".into(), JsonValue::String("http".into()));
        } else {
            object.insert("type".into(), JsonValue::String("stdio".into()));
        }
    }

    let mut codex_extensions = JsonMap::new();
    for field in CODEX_EXTENSION_FIELDS {
        if let Some(value) = object.remove(*field) {
            codex_extensions.insert((*field).to_string(), value);
        }
    }
    let spec: McpSpec = serde_json::from_value(JsonValue::Object(object))
        .map_err(|error| McpError::InvalidLiveConfig(error.to_string()))?;
    let mut provider_extensions = JsonMap::new();
    if !codex_extensions.is_empty() {
        provider_extensions.insert("codex".into(), JsonValue::Object(codex_extensions));
    }
    let canonical = CanonicalMcp {
        schema_version: 1,
        name: name.to_string(),
        spec,
        provider_extensions,
    };
    canonical.validate()?;
    Ok(canonical)
}

pub fn patch_claude_json(
    existing: JsonValue,
    canonical: &CanonicalMcp,
    scope: ClaudeScope<'_>,
    windows: bool,
) -> Result<Rendered<JsonValue>, McpError> {
    canonical.validate()?;
    let mut root = into_json_object(existing, "Claude config root")?;
    let server = claude_server_value(canonical, windows)?;
    let warnings = renderer_warnings(canonical, "Claude");
    claude_servers_mut(&mut root, scope)?.insert(canonical.name.clone(), server);
    Ok(Rendered {
        content: JsonValue::Object(root),
        warnings,
    })
}

pub fn remove_from_claude_json(
    existing: JsonValue,
    server_name: &str,
    scope: ClaudeScope<'_>,
) -> Result<Rendered<JsonValue>, McpError> {
    validate_server_name(server_name)?;
    let mut root = into_json_object(existing, "Claude config root")?;
    if let Some(servers) = find_claude_servers_mut(&mut root, scope)? {
        servers.remove(server_name);
    }
    Ok(Rendered {
        content: JsonValue::Object(root),
        warnings: Vec::new(),
    })
}

pub fn patch_codex_toml(
    existing: &str,
    canonical: &CanonicalMcp,
    _scope: CodexScope,
    windows: bool,
) -> Result<Rendered<String>, McpError> {
    canonical.validate()?;
    if canonical.spec.transport == Some(McpTransport::Sse) {
        return Err(McpError::IncompatibleTarget(
            "Codex does not support canonical SSE servers".into(),
        ));
    }

    let mut document = parse_toml_document(existing)?;
    ensure_table(document.as_table_mut(), "mcp_servers")?;
    let table = codex_server_table(canonical, windows)?;
    document["mcp_servers"][&canonical.name] = Item::Table(table);

    Ok(Rendered {
        content: document.to_string(),
        warnings: renderer_warnings(canonical, "Codex"),
    })
}

pub fn remove_from_codex_toml(
    existing: &str,
    server_name: &str,
    _scope: CodexScope,
) -> Result<Rendered<String>, McpError> {
    validate_server_name(server_name)?;
    let mut document = parse_toml_document(existing)?;

    remove_nested_table_entry(document.as_table_mut(), &["mcp_servers"], server_name)?;
    remove_nested_table_entry(document.as_table_mut(), &["mcp", "servers"], server_name)?;

    Ok(Rendered {
        content: document.to_string(),
        warnings: Vec::new(),
    })
}

pub fn wrap_command_for_windows(command: &str, args: &[String], windows: bool) -> WrappedCommand {
    let executable = command
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(command)
        .trim_end_matches(".cmd")
        .trim_end_matches(".exe")
        .to_ascii_lowercase();
    let needs_cmd = matches!(
        executable.as_str(),
        "npx" | "npm" | "yarn" | "pnpm" | "node" | "bun" | "deno"
    );

    if windows && needs_cmd {
        let mut wrapped_args = Vec::with_capacity(args.len() + 2);
        wrapped_args.push("/c".into());
        wrapped_args.push(command.into());
        wrapped_args.extend(args.iter().cloned());
        WrappedCommand {
            command: "cmd".into(),
            args: wrapped_args,
        }
    } else {
        WrappedCommand {
            command: command.into(),
            args: args.to_vec(),
        }
    }
}

fn validate_server_name(name: &str) -> Result<(), McpError> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
    {
        return Err(McpError::InvalidCanonical(
            "name must be a safe, non-empty single path component".into(),
        ));
    }
    Ok(())
}

fn is_http_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn into_json_object(value: JsonValue, label: &str) -> Result<JsonMap<String, JsonValue>, McpError> {
    match value {
        JsonValue::Object(object) => Ok(object),
        JsonValue::Null => Ok(JsonMap::new()),
        _ => Err(McpError::InvalidLiveConfig(format!(
            "{label} must be a JSON object"
        ))),
    }
}

fn claude_server_value(canonical: &CanonicalMcp, windows: bool) -> Result<JsonValue, McpError> {
    let transport = canonical.spec.transport.unwrap_or(McpTransport::Stdio);
    let mut output = JsonMap::new();
    output.insert(
        "type".into(),
        JsonValue::String(
            match transport {
                McpTransport::Stdio => "stdio",
                McpTransport::Http => "http",
                McpTransport::Sse => "sse",
            }
            .into(),
        ),
    );

    match transport {
        McpTransport::Stdio => {
            let wrapped = wrap_command_for_windows(
                canonical.spec.command.as_deref().unwrap_or_default(),
                &canonical.spec.args,
                windows,
            );
            output.insert("command".into(), JsonValue::String(wrapped.command));
            if !wrapped.args.is_empty() {
                output.insert(
                    "args".into(),
                    JsonValue::Array(wrapped.args.into_iter().map(JsonValue::String).collect()),
                );
            }
            insert_json_string_map(&mut output, "env", &canonical.spec.env);
            if let Some(cwd) = &canonical.spec.cwd {
                output.insert("cwd".into(), JsonValue::String(cwd.clone()));
            }
        }
        McpTransport::Http | McpTransport::Sse => {
            output.insert(
                "url".into(),
                JsonValue::String(canonical.spec.url.clone().unwrap_or_default()),
            );
            insert_json_string_map(&mut output, "headers", &canonical.spec.headers);
        }
    }

    for field in INTERNAL_FIELDS {
        output.remove(*field);
    }
    Ok(JsonValue::Object(output))
}

fn insert_json_string_map(
    output: &mut JsonMap<String, JsonValue>,
    key: &str,
    values: &BTreeMap<String, String>,
) {
    if values.is_empty() {
        return;
    }
    output.insert(
        key.into(),
        JsonValue::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), JsonValue::String(value.clone())))
                .collect(),
        ),
    );
}

fn claude_servers_mut<'a>(
    root: &'a mut JsonMap<String, JsonValue>,
    scope: ClaudeScope<'_>,
) -> Result<&'a mut JsonMap<String, JsonValue>, McpError> {
    match scope {
        ClaudeScope::User | ClaudeScope::Project => object_field_mut(root, "mcpServers"),
        ClaudeScope::Local { project_path } => {
            if project_path.trim().is_empty() {
                return Err(McpError::InvalidCanonical(
                    "Claude local scope requires a canonical project path".into(),
                ));
            }
            let projects = object_field_mut(root, "projects")?;
            let project = object_field_mut(projects, project_path)?;
            object_field_mut(project, "mcpServers")
        }
    }
}

fn find_claude_servers_mut<'a>(
    root: &'a mut JsonMap<String, JsonValue>,
    scope: ClaudeScope<'_>,
) -> Result<Option<&'a mut JsonMap<String, JsonValue>>, McpError> {
    match scope {
        ClaudeScope::User | ClaudeScope::Project => optional_object_field_mut(root, "mcpServers"),
        ClaudeScope::Local { project_path } => {
            let Some(projects) = optional_object_field_mut(root, "projects")? else {
                return Ok(None);
            };
            let Some(project) = optional_object_field_mut(projects, project_path)? else {
                return Ok(None);
            };
            optional_object_field_mut(project, "mcpServers")
        }
    }
}

fn object_field_mut<'a>(
    object: &'a mut JsonMap<String, JsonValue>,
    key: &str,
) -> Result<&'a mut JsonMap<String, JsonValue>, McpError> {
    if !object.contains_key(key) {
        object.insert(key.into(), JsonValue::Object(JsonMap::new()));
    }
    object
        .get_mut(key)
        .and_then(JsonValue::as_object_mut)
        .ok_or_else(|| {
            McpError::InvalidLiveConfig(format!("existing {key} field must be a JSON object"))
        })
}

fn optional_object_field_mut<'a>(
    object: &'a mut JsonMap<String, JsonValue>,
    key: &str,
) -> Result<Option<&'a mut JsonMap<String, JsonValue>>, McpError> {
    match object.get_mut(key) {
        Some(JsonValue::Object(value)) => Ok(Some(value)),
        Some(_) => Err(McpError::InvalidLiveConfig(format!(
            "existing {key} field must be a JSON object"
        ))),
        None => Ok(None),
    }
}

fn parse_toml_document(existing: &str) -> Result<Document, McpError> {
    existing
        .parse::<Document>()
        .map_err(|error| McpError::InvalidLiveConfig(error.to_string()))
}

fn codex_server_table(canonical: &CanonicalMcp, windows: bool) -> Result<Table, McpError> {
    let mut table = Table::new();
    match canonical.spec.transport.unwrap_or(McpTransport::Stdio) {
        McpTransport::Stdio => {
            let wrapped = wrap_command_for_windows(
                canonical.spec.command.as_deref().unwrap_or_default(),
                &canonical.spec.args,
                windows,
            );
            table["command"] = toml_edit::value(wrapped.command);
            if !wrapped.args.is_empty() {
                table["args"] = Item::Value(TomlValue::Array(string_array(&wrapped.args)));
            }
            insert_toml_string_map(&mut table, "env", &canonical.spec.env);
            if let Some(cwd) = &canonical.spec.cwd {
                table["cwd"] = toml_edit::value(cwd);
            }
        }
        McpTransport::Http => {
            table["url"] = toml_edit::value(canonical.spec.url.as_deref().unwrap_or_default());
            insert_toml_string_map(&mut table, "http_headers", &canonical.spec.headers);
        }
        McpTransport::Sse => {
            return Err(McpError::IncompatibleTarget(
                "Codex does not support canonical SSE servers".into(),
            ));
        }
    }

    if let Some(JsonValue::Object(extension)) = canonical.provider_extensions.get("codex") {
        for key in CODEX_EXTENSION_FIELDS {
            if let Some(value) = extension.get(*key) {
                table[*key] = json_to_toml_value(value).ok_or_else(|| {
                    McpError::IncompatibleTarget(format!(
                        "Codex extension {key} has an unsupported value shape"
                    ))
                })?;
            }
        }
    }
    Ok(table)
}

fn string_array(values: &[String]) -> Array {
    let mut output = Array::new();
    for value in values {
        output.push(value.as_str());
    }
    output
}

fn insert_toml_string_map(table: &mut Table, key: &str, values: &BTreeMap<String, String>) {
    if values.is_empty() {
        return;
    }
    let mut output = InlineTable::new();
    for (name, value) in values {
        output.insert(name, TomlValue::from(value.as_str()));
    }
    table[key] = Item::Value(TomlValue::InlineTable(output));
}

fn json_to_toml_value(value: &JsonValue) -> Option<Item> {
    match value {
        JsonValue::String(value) => Some(toml_edit::value(value)),
        JsonValue::Bool(value) => Some(toml_edit::value(*value)),
        JsonValue::Number(value) => value
            .as_i64()
            .map(toml_edit::value)
            .or_else(|| value.as_f64().map(toml_edit::value)),
        JsonValue::Array(values) => {
            let strings = values
                .iter()
                .map(JsonValue::as_str)
                .collect::<Option<Vec<_>>>()?;
            let mut array = Array::new();
            for value in strings {
                array.push(value);
            }
            Some(Item::Value(TomlValue::Array(array)))
        }
        JsonValue::Object(values) => {
            let mut table = InlineTable::new();
            for (key, value) in values {
                table.insert(key, TomlValue::from(value.as_str()?));
            }
            Some(Item::Value(TomlValue::InlineTable(table)))
        }
        JsonValue::Null => None,
    }
}

fn ensure_table(parent: &mut Table, key: &str) -> Result<(), McpError> {
    match parent.get(key) {
        Some(Item::Table(_)) => Ok(()),
        Some(_) => Err(McpError::InvalidLiveConfig(format!(
            "existing {key} field must be a TOML table"
        ))),
        None => {
            parent.insert(key, Item::Table(Table::new()));
            Ok(())
        }
    }
}

fn remove_nested_table_entry(
    root: &mut Table,
    path: &[&str],
    server_name: &str,
) -> Result<(), McpError> {
    let Some((first, rest)) = path.split_first() else {
        return Ok(());
    };
    let Some(item) = root.get_mut(first) else {
        return Ok(());
    };
    let Some(table) = item.as_table_mut() else {
        return Err(McpError::InvalidLiveConfig(format!(
            "existing {} field must be a TOML table",
            path.join(".")
        )));
    };

    if rest.is_empty() {
        table.remove(server_name);
        return Ok(());
    }
    remove_nested_table_entry(table, rest, server_name)
}

fn renderer_warnings(canonical: &CanonicalMcp, provider: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    for key in canonical.spec.extra.keys() {
        if !INTERNAL_FIELDS.contains(&key.as_str()) {
            warnings.push(format!(
                "{provider} renderer omitted unsupported canonical field spec.{key}"
            ));
        }
    }
    for key in canonical.provider_extensions.keys() {
        let applies =
            (provider == "Codex" && key == "codex") || (provider == "Claude" && key == "claude");
        if !applies {
            warnings.push(format!(
                "{provider} renderer ignored providerExtensions.{key}"
            ));
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn stdio_server() -> CanonicalMcp {
        CanonicalMcp::from_json(json!({
            "schemaVersion": 1,
            "name": "filesystem",
            "spec": {
                "type": "stdio",
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
                "env": {"MODE": "readonly"}
            },
            "providerExtensions": {}
        }))
        .unwrap()
    }

    #[test]
    fn claude_import_removes_internal_fields_and_builds_canonical_spec() {
        let canonical = import_claude_server(
            "filesystem",
            &json!({
                "command": "npx",
                "args": ["-y", "server"],
                "env": {"MODE": "readonly"},
                "enabled": true,
                "description": "internal metadata"
            }),
        )
        .unwrap();

        assert_eq!(canonical.name, "filesystem");
        assert_eq!(canonical.spec.command.as_deref(), Some("npx"));
        assert_eq!(canonical.spec.args, ["-y", "server"]);
        assert!(!canonical.spec.extra.contains_key("enabled"));
        assert!(!canonical.spec.extra.contains_key("description"));
    }

    #[test]
    fn codex_import_maps_headers_and_preserves_extensions() {
        let document: toml::Value = toml::from_str(
            r#"
command = "npx"
args = ["-y", "server"]
http_headers = { X-Test = "value" }
startup_timeout_sec = 30
enabled_tools = ["read"]
"#,
        )
        .unwrap();
        let canonical = import_codex_server("filesystem", &document).unwrap();

        assert_eq!(canonical.spec.transport, Some(McpTransport::Stdio));
        assert_eq!(canonical.spec.headers.get("X-Test").unwrap(), "value");
        assert_eq!(
            canonical.provider_extensions["codex"]["startup_timeout_sec"],
            30
        );
        assert_eq!(
            canonical.provider_extensions["codex"]["enabled_tools"],
            json!(["read"])
        );
    }

    #[test]
    fn claude_patch_preserves_unrelated_fields_and_servers() {
        let existing = json!({
            "theme": "dark",
            "mcpServers": {
                "existing": {"type": "stdio", "command": "existing-command"}
            }
        });
        let rendered =
            patch_claude_json(existing, &stdio_server(), ClaudeScope::User, false).unwrap();

        assert_eq!(rendered.content["theme"], "dark");
        assert_eq!(
            rendered.content["mcpServers"]["existing"]["command"],
            "existing-command"
        );
        assert_eq!(
            rendered.content["mcpServers"]["filesystem"]["command"],
            "npx"
        );
    }

    #[test]
    fn claude_local_patch_only_changes_selected_project() {
        let existing = json!({
            "mcpServers": {"root": {"command": "keep-root"}},
            "projects": {
                "/work/other": {
                    "mcpServers": {"other": {"command": "keep-other"}}
                }
            }
        });
        let rendered = patch_claude_json(
            existing,
            &stdio_server(),
            ClaudeScope::Local {
                project_path: "/work/current",
            },
            false,
        )
        .unwrap();

        assert_eq!(
            rendered.content["mcpServers"]["root"]["command"],
            "keep-root"
        );
        assert_eq!(
            rendered.content["projects"]["/work/other"]["mcpServers"]["other"]["command"],
            "keep-other"
        );
        assert_eq!(
            rendered.content["projects"]["/work/current"]["mcpServers"]["filesystem"]["command"],
            "npx"
        );
    }

    #[test]
    fn claude_remove_is_precise() {
        let existing = json!({
            "other": 7,
            "mcpServers": {
                "filesystem": {"command": "remove-me"},
                "keep": {"command": "keep-me"}
            }
        });
        let rendered = remove_from_claude_json(existing, "filesystem", ClaudeScope::User).unwrap();
        assert!(rendered.content["mcpServers"].get("filesystem").is_none());
        assert_eq!(rendered.content["mcpServers"]["keep"]["command"], "keep-me");
        assert_eq!(rendered.content["other"], 7);
    }

    #[test]
    fn codex_patch_preserves_comments_and_unrelated_configuration() {
        let existing = r#"# top-level comment
model = "gpt-5"

[mcp_servers.existing]
# existing server comment
command = "keep-me"
"#;
        let rendered =
            patch_codex_toml(existing, &stdio_server(), CodexScope::User, false).unwrap();

        assert!(rendered.content.contains("# top-level comment"));
        assert!(rendered.content.contains("# existing server comment"));
        assert!(rendered.content.contains("model = \"gpt-5\""));
        assert!(rendered.content.contains("[mcp_servers.existing]"));
        assert!(rendered.content.contains("[mcp_servers.filesystem]"));
    }

    #[test]
    fn codex_maps_headers_and_remove_cleans_current_and_legacy_tables() {
        let canonical = CanonicalMcp::from_json(json!({
            "schemaVersion": 1,
            "name": "remote",
            "spec": {
                "type": "http",
                "url": "https://example.test/mcp",
                "headers": {"X-Test": "value"}
            },
            "providerExtensions": {}
        }))
        .unwrap();
        let patched = patch_codex_toml(
            "# keep unrelated\nmodel = \"gpt-5\"\n\n[mcp.servers.remote]\nurl = \"https://legacy\"\n",
            &canonical,
            CodexScope::Project,
            false,
        )
        .unwrap();
        assert!(patched.content.contains("http_headers"));

        let removed =
            remove_from_codex_toml(&patched.content, "remote", CodexScope::Project).unwrap();
        assert!(removed.content.contains("# keep unrelated"));
        assert!(removed.content.contains("model = \"gpt-5\""));
        assert!(!removed.content.contains("[mcp_servers.remote]"));
        assert!(!removed.content.contains("[mcp.servers.remote]"));
    }

    #[test]
    fn codex_blocks_sse() {
        let canonical = CanonicalMcp::from_json(json!({
            "schemaVersion": 1,
            "name": "events",
            "spec": {
                "type": "sse",
                "url": "https://example.test/sse"
            },
            "providerExtensions": {}
        }))
        .unwrap();
        assert!(matches!(
            patch_codex_toml("", &canonical, CodexScope::User, false),
            Err(McpError::IncompatibleTarget(_))
        ));
    }

    #[test]
    fn windows_wrapper_only_wraps_known_runtime_commands() {
        let args = vec!["-y".into(), "server-package".into()];
        assert_eq!(
            wrap_command_for_windows("npx", &args, true),
            WrappedCommand {
                command: "cmd".into(),
                args: vec![
                    "/c".into(),
                    "npx".into(),
                    "-y".into(),
                    "server-package".into()
                ]
            }
        );
        assert_eq!(
            wrap_command_for_windows("python", &args, true),
            WrappedCommand {
                command: "python".into(),
                args
            }
        );
    }

    #[test]
    fn unsupported_fields_are_reported_without_leaking_values() {
        let canonical = CanonicalMcp::from_json(json!({
            "schemaVersion": 1,
            "name": "custom",
            "spec": {
                "command": "server",
                "secretBehavior": "do-not-log-this"
            },
            "providerExtensions": {"other": {"token": "also-secret"}}
        }))
        .unwrap();
        let rendered =
            patch_claude_json(json!({}), &canonical, ClaudeScope::Project, false).unwrap();
        let joined = rendered.warnings.join(" ");
        assert!(joined.contains("spec.secretBehavior"));
        assert!(joined.contains("providerExtensions.other"));
        assert!(!joined.contains("do-not-log-this"));
        assert!(!joined.contains("also-secret"));
    }
}
