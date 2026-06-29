use crate::path_safety::{guard_write_path, validate_single_path_component};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MOUNT_REGISTRY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingStatus {
    #[serde(rename = "mounted")]
    Mounted,
    #[serde(rename = "out_of_sync")]
    OutOfSync,
    #[serde(rename = "orphaned")]
    Orphaned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MountBinding {
    pub id: String,
    pub asset_id: String,
    pub target_id: String,
    pub status: BindingStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<String>,
}

impl MountBinding {
    pub fn new(
        asset_id: impl Into<String>,
        target_id: impl Into<String>,
        status: BindingStatus,
    ) -> Result<Self, MountRegistryError> {
        let asset_id = asset_id.into();
        let target_id = target_id.into();
        validate_asset_id(&asset_id)?;
        validate_single_path_component(&target_id, "target id")
            .map_err(MountRegistryError::InvalidRecord)?;
        Ok(Self {
            id: binding_id(&asset_id, &target_id),
            asset_id,
            target_id,
            status,
            last_synced_at: None,
        })
    }

    fn validate(&self) -> Result<(), MountRegistryError> {
        validate_asset_id(&self.asset_id)?;
        validate_single_path_component(&self.target_id, "target id")
            .map_err(MountRegistryError::InvalidRecord)?;
        let expected = binding_id(&self.asset_id, &self.target_id);
        if self.id != expected {
            return Err(MountRegistryError::InvalidRecord(format!(
                "binding id '{}' does not match '{expected}'",
                self.id
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MountRegistry {
    pub schema_version: u32,
    #[serde(default)]
    pub bindings: BTreeMap<String, MountBinding>,
}

impl Default for MountRegistry {
    fn default() -> Self {
        Self {
            schema_version: MOUNT_REGISTRY_SCHEMA_VERSION,
            bindings: BTreeMap::new(),
        }
    }
}

impl MountRegistry {
    pub fn validate(&self) -> Result<(), MountRegistryError> {
        if self.schema_version != MOUNT_REGISTRY_SCHEMA_VERSION {
            return Err(MountRegistryError::UnsupportedSchema {
                found: u64::from(self.schema_version),
                supported: MOUNT_REGISTRY_SCHEMA_VERSION,
            });
        }
        for (id, binding) in &self.bindings {
            binding.validate()?;
            if id != &binding.id {
                return Err(MountRegistryError::InvalidRecord(format!(
                    "binding map key '{id}' does not match record id '{}'",
                    binding.id
                )));
            }
        }
        Ok(())
    }

    pub fn upsert(&mut self, binding: MountBinding) -> Result<(), MountRegistryError> {
        binding.validate()?;
        self.bindings.insert(binding.id.clone(), binding);
        Ok(())
    }

    pub fn remove(&mut self, asset_id: &str, target_id: &str) -> Option<MountBinding> {
        self.bindings.remove(&binding_id(asset_id, target_id))
    }

    pub fn for_asset(&self, asset_id: &str) -> Vec<&MountBinding> {
        self.bindings
            .values()
            .filter(|binding| binding.asset_id == asset_id)
            .collect()
    }

    pub fn mark_asset_out_of_sync(&mut self, asset_id: &str) -> usize {
        let mut changed = 0;
        for binding in self
            .bindings
            .values_mut()
            .filter(|binding| binding.asset_id == asset_id)
        {
            if binding.status == BindingStatus::Mounted {
                binding.status = BindingStatus::OutOfSync;
                changed += 1;
            }
        }
        changed
    }
}

#[derive(Debug)]
pub enum MountRegistryError {
    Io { path: PathBuf, source: io::Error },
    InvalidYaml { path: PathBuf, message: String },
    UnsupportedSchema { found: u64, supported: u32 },
    InvalidRecord(String),
}

impl fmt::Display for MountRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(formatter, "failed to access {}: {source}", path.display())
            }
            Self::InvalidYaml { path, message } => {
                write!(
                    formatter,
                    "invalid mount registry {}: {message}",
                    path.display()
                )
            }
            Self::UnsupportedSchema { found, supported } => write!(
                formatter,
                "unsupported mount registry schemaVersion {found}; this build supports {supported}"
            ),
            Self::InvalidRecord(message) => write!(formatter, "invalid mount binding: {message}"),
        }
    }
}

impl std::error::Error for MountRegistryError {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemaHeader {
    schema_version: u64,
}

pub fn registry_path(home: &Path) -> PathBuf {
    home.join(".my-agent-assets/mounts.yaml")
}

pub fn load(home: &Path) -> Result<MountRegistry, MountRegistryError> {
    let path = registry_path(home);
    let text = fs::read_to_string(&path).map_err(|source| MountRegistryError::Io {
        path: path.clone(),
        source,
    })?;
    let header: SchemaHeader =
        serde_yaml::from_str(&text).map_err(|error| MountRegistryError::InvalidYaml {
            path: path.clone(),
            message: error.to_string(),
        })?;
    if header.schema_version != u64::from(MOUNT_REGISTRY_SCHEMA_VERSION) {
        return Err(MountRegistryError::UnsupportedSchema {
            found: header.schema_version,
            supported: MOUNT_REGISTRY_SCHEMA_VERSION,
        });
    }
    let registry: MountRegistry =
        serde_yaml::from_str(&text).map_err(|error| MountRegistryError::InvalidYaml {
            path: path.clone(),
            message: error.to_string(),
        })?;
    registry.validate()?;
    Ok(registry)
}

pub fn save(home: &Path, registry: &MountRegistry) -> Result<(), MountRegistryError> {
    registry.validate()?;
    let root = home.join(".my-agent-assets");
    let path = registry_path(home);
    let guarded = guard_write_path(&root, &path).map_err(|source| MountRegistryError::Io {
        path: path.clone(),
        source,
    })?;
    let content =
        serde_yaml::to_string(registry).map_err(|error| MountRegistryError::InvalidYaml {
            path: path.clone(),
            message: error.to_string(),
        })?;
    atomic_write(&guarded, content.as_bytes()).map_err(|source| MountRegistryError::Io {
        path: guarded,
        source,
    })
}

fn validate_asset_id(asset_id: &str) -> Result<(), MountRegistryError> {
    let (kind, name) = asset_id.split_once(':').ok_or_else(|| {
        MountRegistryError::InvalidRecord(format!("invalid asset id '{asset_id}'"))
    })?;
    if !matches!(kind, "skill" | "command" | "mcp") {
        return Err(MountRegistryError::InvalidRecord(format!(
            "invalid asset kind in '{asset_id}'"
        )));
    }
    validate_single_path_component(name, "asset name").map_err(MountRegistryError::InvalidRecord)
}

fn binding_id(asset_id: &str, target_id: &str) -> String {
    format!("{asset_id}@{target_id}")
}

fn atomic_write(path: &Path, content: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(".mounts.yaml.tmp-{}-{nonce}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    if let Err(error) = (|| {
        file.write_all(content)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        OpenOptions::new().read(true).open(parent)?.sync_all()
    })() {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_mark_out_of_sync_are_precise() {
        let home = test_home("round-trip");
        fs::create_dir_all(home.join(".my-agent-assets")).unwrap();
        let mut registry = MountRegistry::default();
        registry
            .upsert(
                MountBinding::new("mcp:postgres", "claude-user-mcp", BindingStatus::Mounted)
                    .unwrap(),
            )
            .unwrap();
        registry
            .upsert(
                MountBinding::new("skill:review", "claude-user-skills", BindingStatus::Mounted)
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(registry.mark_asset_out_of_sync("mcp:postgres"), 1);
        assert_eq!(
            registry.for_asset("mcp:postgres")[0].status,
            BindingStatus::OutOfSync
        );
        assert_eq!(
            registry.for_asset("skill:review")[0].status,
            BindingStatus::Mounted
        );
        save(&home, &registry).unwrap();
        assert_eq!(load(&home).unwrap(), registry);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn invalid_and_newer_registry_are_not_overwritten() {
        let home = test_home("invalid");
        fs::create_dir_all(home.join(".my-agent-assets")).unwrap();
        fs::write(registry_path(&home), "schemaVersion: 99\nbindings: {}\n").unwrap();
        assert!(matches!(
            load(&home),
            Err(MountRegistryError::UnsupportedSchema { found: 99, .. })
        ));
        assert!(MountBinding::new("bad", "target", BindingStatus::Mounted).is_err());
        let _ = fs::remove_dir_all(home);
    }

    fn test_home(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "maa-mount-registry-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&path);
        path
    }
}
