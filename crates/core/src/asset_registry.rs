use crate::fs_sync::sync_directory;
use crate::mcp::CanonicalMcp;
use crate::path_safety::{guard_write_path, validate_single_path_component};
use crate::targets::AssetKind;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const ASSET_REGISTRY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetRecord {
    pub id: String,
    pub asset_type: AssetKind,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl AssetRecord {
    pub fn new(asset_type: AssetKind, name: impl Into<String>) -> Result<Self, RegistryError> {
        let name = name.into();
        validate_single_path_component(&name, "asset name")
            .map_err(RegistryError::InvalidRecord)?;
        Ok(Self {
            id: asset_id(asset_type, &name),
            asset_type,
            name,
            title: None,
            description: None,
        })
    }

    pub fn validate(&self) -> Result<(), RegistryError> {
        validate_single_path_component(&self.name, "asset name")
            .map_err(RegistryError::InvalidRecord)?;
        let expected = asset_id(self.asset_type, &self.name);
        if self.id != expected {
            return Err(RegistryError::InvalidRecord(format!(
                "asset id '{}' does not match expected '{expected}'",
                self.id
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetRegistry {
    pub schema_version: u32,
    #[serde(default)]
    pub assets: BTreeMap<String, AssetRecord>,
}

impl Default for AssetRegistry {
    fn default() -> Self {
        Self {
            schema_version: ASSET_REGISTRY_SCHEMA_VERSION,
            assets: BTreeMap::new(),
        }
    }
}

impl AssetRegistry {
    pub fn validate(&self) -> Result<(), RegistryError> {
        if self.schema_version != ASSET_REGISTRY_SCHEMA_VERSION {
            return Err(RegistryError::UnsupportedSchema {
                found: u64::from(self.schema_version),
                supported: ASSET_REGISTRY_SCHEMA_VERSION,
            });
        }
        let mut ids = BTreeSet::new();
        for (id, asset) in &self.assets {
            asset.validate()?;
            if id != &asset.id {
                return Err(RegistryError::InvalidRecord(format!(
                    "asset map key '{id}' does not match record id '{}'",
                    asset.id
                )));
            }
            if !ids.insert(asset.id.clone()) {
                return Err(RegistryError::InvalidRecord(format!(
                    "duplicate asset id '{}'",
                    asset.id
                )));
            }
        }
        Ok(())
    }

    pub fn get(&self, asset_type: AssetKind, name: &str) -> Option<&AssetRecord> {
        let id = asset_id(asset_type, name);
        self.assets.get(&id)
    }

    pub fn upsert(&mut self, record: AssetRecord) -> Result<(), RegistryError> {
        record.validate()?;
        self.assets.insert(record.id.clone(), record);
        Ok(())
    }

    pub fn remove(&mut self, asset_type: AssetKind, name: &str) -> Option<AssetRecord> {
        self.assets.remove(&asset_id(asset_type, name))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentState {
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "missing_content")]
    MissingContent,
    #[serde(rename = "unregistered")]
    Unregistered,
    #[serde(rename = "invalid_content")]
    InvalidContent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentDiagnostic {
    pub asset_id: String,
    pub asset_type: AssetKind,
    pub name: String,
    pub path: PathBuf,
    pub state: ContentState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug)]
pub enum RegistryError {
    Io { path: PathBuf, source: io::Error },
    InvalidYaml { path: PathBuf, message: String },
    UnsupportedSchema { found: u64, supported: u32 },
    InvalidRecord(String),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(formatter, "failed to access {}: {source}", path.display())
            }
            Self::InvalidYaml { path, message } => {
                write!(
                    formatter,
                    "invalid asset registry {}: {message}",
                    path.display()
                )
            }
            Self::UnsupportedSchema { found, supported } => write!(
                formatter,
                "unsupported asset registry schemaVersion {found}; this build supports {supported}"
            ),
            Self::InvalidRecord(message) => write!(formatter, "invalid asset record: {message}"),
        }
    }
}

impl std::error::Error for RegistryError {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemaHeader {
    schema_version: u64,
}

pub fn registry_path(home: &Path) -> PathBuf {
    home.join(".my-agent-assets/assets.yaml")
}

pub fn load(home: &Path) -> Result<AssetRegistry, RegistryError> {
    let path = registry_path(home);
    let text = fs::read_to_string(&path).map_err(|source| RegistryError::Io {
        path: path.clone(),
        source,
    })?;
    let header: SchemaHeader =
        serde_yaml::from_str(&text).map_err(|error| RegistryError::InvalidYaml {
            path: path.clone(),
            message: error.to_string(),
        })?;
    if header.schema_version != u64::from(ASSET_REGISTRY_SCHEMA_VERSION) {
        return Err(RegistryError::UnsupportedSchema {
            found: header.schema_version,
            supported: ASSET_REGISTRY_SCHEMA_VERSION,
        });
    }
    let registry: AssetRegistry =
        serde_yaml::from_str(&text).map_err(|error| RegistryError::InvalidYaml {
            path: path.clone(),
            message: error.to_string(),
        })?;
    registry.validate()?;
    Ok(registry)
}

pub fn save(home: &Path, registry: &AssetRegistry) -> Result<(), RegistryError> {
    registry.validate()?;
    let root = home.join(".my-agent-assets");
    let path = registry_path(home);
    let guarded = guard_write_path(&root, &path).map_err(|source| RegistryError::Io {
        path: path.clone(),
        source,
    })?;
    let content = serde_yaml::to_string(registry).map_err(|error| RegistryError::InvalidYaml {
        path: path.clone(),
        message: error.to_string(),
    })?;
    atomic_write(&guarded, content.as_bytes()).map_err(|source| RegistryError::Io {
        path: guarded,
        source,
    })
}

pub fn inspect_content(
    home: &Path,
    registry: &AssetRegistry,
) -> Result<Vec<ContentDiagnostic>, RegistryError> {
    registry.validate()?;
    let mut diagnostics = BTreeMap::<String, ContentDiagnostic>::new();
    for asset in registry.assets.values() {
        let path = canonical_path(home, asset.asset_type, &asset.name);
        let (state, message) = validate_content(asset.asset_type, &path);
        diagnostics.insert(
            asset.id.clone(),
            ContentDiagnostic {
                asset_id: asset.id.clone(),
                asset_type: asset.asset_type,
                name: asset.name.clone(),
                path,
                state,
                message,
            },
        );
    }

    for kind in [AssetKind::Skill, AssetKind::Command, AssetKind::Mcp] {
        let directory = canonical_directory(home, kind);
        if !directory.exists() {
            continue;
        }
        let entries = fs::read_dir(&directory).map_err(|source| RegistryError::Io {
            path: directory.clone(),
            source,
        })?;
        for entry in entries {
            let path = entry
                .map_err(|source| RegistryError::Io {
                    path: directory.clone(),
                    source,
                })?
                .path();
            let Some(name) = canonical_name(kind, &path) else {
                continue;
            };
            let id = asset_id(kind, &name);
            diagnostics.entry(id.clone()).or_insert(ContentDiagnostic {
                asset_id: id,
                asset_type: kind,
                name,
                path,
                state: ContentState::Unregistered,
                message: Some("canonical content exists without an assets.yaml record".into()),
            });
        }
    }
    Ok(diagnostics.into_values().collect())
}

pub fn asset_id(kind: AssetKind, name: &str) -> String {
    let prefix = match kind {
        AssetKind::Skill => "skill",
        AssetKind::Command => "command",
        AssetKind::Mcp => "mcp",
    };
    format!("{prefix}:{name}")
}

pub fn parse_asset_id(value: &str) -> Result<(AssetKind, String), RegistryError> {
    let (kind, name) = value
        .split_once(':')
        .ok_or_else(|| RegistryError::InvalidRecord(format!("invalid asset id '{value}'")))?;
    let kind = match kind {
        "skill" => AssetKind::Skill,
        "command" => AssetKind::Command,
        "mcp" => AssetKind::Mcp,
        _ => {
            return Err(RegistryError::InvalidRecord(format!(
                "invalid asset kind in '{value}'"
            )))
        }
    };
    validate_single_path_component(name, "asset name").map_err(RegistryError::InvalidRecord)?;
    Ok((kind, name.to_string()))
}

pub fn canonical_path(home: &Path, kind: AssetKind, name: &str) -> PathBuf {
    match kind {
        AssetKind::Skill => canonical_directory(home, kind).join(name),
        AssetKind::Command => canonical_directory(home, kind).join(format!("{name}.md")),
        AssetKind::Mcp => canonical_directory(home, kind).join(format!("{name}.json")),
    }
}

fn canonical_directory(home: &Path, kind: AssetKind) -> PathBuf {
    let child = match kind {
        AssetKind::Skill => "skills",
        AssetKind::Command => "commands",
        AssetKind::Mcp => "mcps",
    };
    home.join(".my-agent-assets/assets").join(child)
}

fn validate_content(kind: AssetKind, path: &Path) -> (ContentState, Option<String>) {
    if !path.exists() {
        return (
            ContentState::MissingContent,
            Some("assets.yaml record exists but canonical content is missing".into()),
        );
    }
    match kind {
        AssetKind::Skill if !path.is_dir() || !path.join("SKILL.md").is_file() => (
            ContentState::InvalidContent,
            Some("Skill must be a directory containing SKILL.md".into()),
        ),
        AssetKind::Command if !path.is_file() => (
            ContentState::InvalidContent,
            Some("Command must be a Markdown file".into()),
        ),
        AssetKind::Mcp => match fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<CanonicalMcp>(&text).ok())
            .and_then(|canonical| canonical.validate().ok())
        {
            Some(()) => (ContentState::Ready, None),
            None => (
                ContentState::InvalidContent,
                Some("MCP file is not a valid canonical MCP definition".into()),
            ),
        },
        _ => (ContentState::Ready, None),
    }
}

fn canonical_name(kind: AssetKind, path: &Path) -> Option<String> {
    match kind {
        AssetKind::Skill if path.is_dir() => path.file_name(),
        AssetKind::Command if path.extension().and_then(|value| value.to_str()) == Some("md") => {
            path.file_stem()
        }
        AssetKind::Mcp if path.extension().and_then(|value| value.to_str()) == Some("json") => {
            path.file_stem()
        }
        _ => None,
    }
    .and_then(|value| value.to_str())
    .map(str::to_string)
}

fn atomic_write(path: &Path, content: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(".assets.yaml.tmp-{}-{nonce}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    if let Err(error) = (|| {
        file.write_all(content)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        sync_directory(parent)
    })() {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn registry_round_trips_and_rejects_newer_schema() {
        let home = test_home("round-trip");
        fs::create_dir_all(home.join(".my-agent-assets")).unwrap();
        let mut registry = AssetRegistry::default();
        registry
            .upsert(AssetRecord::new(AssetKind::Skill, "review").unwrap())
            .unwrap();
        save(&home, &registry).unwrap();
        assert_eq!(load(&home).unwrap(), registry);

        fs::write(registry_path(&home), "schemaVersion: 99\nassets: {}\n").unwrap();
        assert!(matches!(
            load(&home),
            Err(RegistryError::UnsupportedSchema { found: 99, .. })
        ));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn consistency_reports_missing_unregistered_and_invalid_content() {
        let home = test_home("consistency");
        fs::create_dir_all(home.join(".my-agent-assets/assets/skills/orphan")).unwrap();
        fs::write(
            home.join(".my-agent-assets/assets/skills/orphan/SKILL.md"),
            "# Orphan",
        )
        .unwrap();
        fs::create_dir_all(home.join(".my-agent-assets/assets/mcps")).unwrap();
        fs::write(home.join(".my-agent-assets/assets/mcps/bad.json"), "{bad").unwrap();
        let mut registry = AssetRegistry::default();
        registry
            .upsert(AssetRecord::new(AssetKind::Command, "missing").unwrap())
            .unwrap();
        registry
            .upsert(AssetRecord::new(AssetKind::Mcp, "bad").unwrap())
            .unwrap();

        let diagnostics = inspect_content(&home, &registry).unwrap();
        assert!(diagnostics.iter().any(
            |item| item.asset_id == "skill:orphan" && item.state == ContentState::Unregistered
        ));
        assert!(diagnostics
            .iter()
            .any(|item| item.asset_id == "command:missing"
                && item.state == ContentState::MissingContent));
        assert!(diagnostics
            .iter()
            .any(|item| item.asset_id == "mcp:bad" && item.state == ContentState::InvalidContent));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn valid_canonical_mcp_is_ready() {
        let home = test_home("mcp-ready");
        let path = canonical_path(&home, AssetKind::Mcp, "filesystem");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            serde_json::to_vec_pretty(&json!({
                "schemaVersion": 1,
                "name": "filesystem",
                "spec": {"command": "npx"},
                "providerExtensions": {}
            }))
            .unwrap(),
        )
        .unwrap();
        let mut registry = AssetRegistry::default();
        registry
            .upsert(AssetRecord::new(AssetKind::Mcp, "filesystem").unwrap())
            .unwrap();
        assert_eq!(
            inspect_content(&home, &registry).unwrap()[0].state,
            ContentState::Ready
        );
        let _ = fs::remove_dir_all(home);
    }

    fn test_home(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "maa-registry-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&path);
        path
    }
}
