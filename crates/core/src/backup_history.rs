use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupClass {
    #[serde(rename = "portable")]
    Portable,
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "legacy")]
    Legacy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupHistoryEntry {
    pub id: String,
    pub backup_id: String,
    pub label: String,
    pub class: BackupClass,
    pub operation: String,
    pub created_at_epoch_seconds: u64,
    pub size_bytes: u64,
    pub entry_count: u32,
    pub manifest_path: PathBuf,
    pub affected_paths: Vec<PathBuf>,
    pub sensitive_config_risk: bool,
    pub manual_restore_only: bool,
    pub warnings: Vec<String>,
}

pub fn list_backups(home: &Path) -> Vec<BackupHistoryEntry> {
    let root = home.join(".my-agent-assets/backups");
    let mut entries = Vec::new();
    scan_class(&root.join("portable"), BackupClass::Portable, &mut entries);
    scan_class(&root.join("local"), BackupClass::Local, &mut entries);
    scan_legacy_root(&root, &mut entries);
    entries.sort_by(|left, right| {
        right
            .created_at_epoch_seconds
            .cmp(&left.created_at_epoch_seconds)
            .then_with(|| left.id.cmp(&right.id))
    });
    entries
}

pub fn resolve_backup_manifest(home: &Path, entry_id: &str) -> Result<PathBuf> {
    if entry_id.is_empty()
        || entry_id.contains('/')
        || entry_id.contains('\\')
        || entry_id.contains('\0')
        || entry_id
            .split(':')
            .any(|part| part.is_empty() || part == "..")
    {
        return Err(MaaError::new("invalid backup history entry id"));
    }
    let entry = list_backups(home)
        .into_iter()
        .find(|entry| entry.id == entry_id)
        .ok_or_else(|| MaaError::new(format!("backup history entry not found: {entry_id}")))?;
    let metadata = fs::symlink_metadata(&entry.manifest_path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(MaaError::new("backup manifest must be a real file"));
    }
    let backup_root = home.join(".my-agent-assets/backups").canonicalize()?;
    let manifest = entry.manifest_path.canonicalize()?;
    if !manifest.starts_with(&backup_root) {
        return Err(MaaError::new("backup manifest escapes the backup root"));
    }
    Ok(manifest)
}

fn scan_class(root: &Path, class: BackupClass, output: &mut Vec<BackupHistoryEntry>) {
    let Ok(children) = fs::read_dir(root) else {
        return;
    };
    for child in children.flatten() {
        let path = child.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            continue;
        }
        let manifest = path.join("manifest.yaml");
        if is_real_file(&manifest) {
            output.push(read_yaml_entry(&path, &manifest, class));
        }
    }
}

fn scan_legacy_root(root: &Path, output: &mut Vec<BackupHistoryEntry>) {
    let Ok(children) = fs::read_dir(root) else {
        return;
    };
    for child in children.flatten() {
        let path = child.path();
        let name = child.file_name();
        if matches!(name.to_str(), Some("portable" | "local")) {
            continue;
        }
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            continue;
        }
        let manifest = path.join("manifest.json");
        if is_real_file(&manifest) {
            output.push(read_legacy_entry(&path, &manifest));
        }
    }
}

fn is_real_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
}

fn read_yaml_entry(backup_root: &Path, manifest: &Path, class: BackupClass) -> BackupHistoryEntry {
    let backup_id = backup_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut warnings = Vec::new();
    let parsed = fs::read_to_string(manifest)
        .map_err(|error| error.to_string())
        .and_then(|text| {
            serde_yaml::from_str::<YamlValue>(&text).map_err(|error| error.to_string())
        });
    let (operation, affected_paths) = match parsed {
        Ok(value) => (
            yaml_string(&value, "operation").unwrap_or_else(|| "unknown".into()),
            yaml_paths(&value),
        ),
        Err(error) => {
            warnings.push(format!("Cannot parse backup manifest: {error}"));
            ("invalid".into(), Vec::new())
        }
    };
    let size_bytes = directory_size(backup_root);
    let created_at_epoch_seconds = modified_epoch_seconds(manifest);
    BackupHistoryEntry {
        id: format!("{}:{backup_id}", class_name(class)),
        backup_id,
        label: label_for(class, &operation),
        class,
        operation,
        created_at_epoch_seconds,
        size_bytes,
        entry_count: affected_paths.len().max(content_entry_count(backup_root)) as u32,
        manifest_path: manifest.to_path_buf(),
        sensitive_config_risk: contains_sensitive_config(&affected_paths),
        affected_paths,
        manual_restore_only: true,
        warnings,
    }
}

fn read_legacy_entry(backup_root: &Path, manifest: &Path) -> BackupHistoryEntry {
    let backup_id = backup_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut warnings = Vec::new();
    let parsed = fs::read_to_string(manifest)
        .map_err(|error| error.to_string())
        .and_then(|text| {
            serde_json::from_str::<serde_json::Value>(&text).map_err(|error| error.to_string())
        });
    let (label, operation, affected_paths, created_at) = match parsed {
        Ok(value) => {
            let paths = value
                .get("entries")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|entry| {
                    entry
                        .get("originalPath")
                        .and_then(serde_json::Value::as_str)
                })
                .map(PathBuf::from)
                .collect::<Vec<_>>();
            (
                value
                    .get("label")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Legacy backup")
                    .to_string(),
                value
                    .get("operation")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("legacy")
                    .to_string(),
                paths,
                value
                    .get("createdAtEpochSeconds")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_else(|| modified_epoch_seconds(manifest)),
            )
        }
        Err(error) => {
            warnings.push(format!("Cannot parse legacy backup manifest: {error}"));
            (
                "Invalid legacy backup".into(),
                "invalid".into(),
                Vec::new(),
                modified_epoch_seconds(manifest),
            )
        }
    };
    BackupHistoryEntry {
        id: format!("legacy:{backup_id}"),
        backup_id,
        label,
        class: BackupClass::Legacy,
        operation,
        created_at_epoch_seconds: created_at,
        size_bytes: directory_size(backup_root),
        entry_count: affected_paths.len().max(content_entry_count(backup_root)) as u32,
        manifest_path: manifest.to_path_buf(),
        sensitive_config_risk: contains_sensitive_config(&affected_paths),
        affected_paths,
        manual_restore_only: true,
        warnings,
    }
}

fn yaml_string(value: &YamlValue, key: &str) -> Option<String> {
    value
        .as_mapping()?
        .get(YamlValue::String(key.into()))?
        .as_str()
        .map(ToOwned::to_owned)
}

fn yaml_paths(value: &YamlValue) -> Vec<PathBuf> {
    let Some(mapping) = value.as_mapping() else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    for key in ["destination", "target", "canonicalPath"] {
        if let Some(path) = mapping
            .get(YamlValue::String(key.into()))
            .and_then(YamlValue::as_str)
        {
            paths.push(PathBuf::from(path));
        }
    }
    if let Some(runtime_paths) = mapping
        .get(YamlValue::String("runtimePaths".into()))
        .and_then(YamlValue::as_sequence)
    {
        paths.extend(
            runtime_paths
                .iter()
                .filter_map(YamlValue::as_str)
                .map(PathBuf::from),
        );
    }
    paths.sort();
    paths.dedup();
    paths
}

fn content_entry_count(root: &Path) -> usize {
    fs::read_dir(root)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| {
                    !matches!(
                        entry.file_name().to_str(),
                        Some("manifest.yaml" | "manifest.json")
                    )
                })
                .count()
        })
        .unwrap_or(0)
}

fn directory_size(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.file_type().is_symlink() {
        return 0;
    }
    if metadata.is_file() {
        return metadata.len();
    }
    fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| directory_size(&entry.path()))
                .sum()
        })
        .unwrap_or(0)
}

fn modified_epoch_seconds(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn contains_sensitive_config(paths: &[PathBuf]) -> bool {
    paths.iter().any(|path| {
        let value = path.to_string_lossy().replace('\\', "/");
        value.ends_with(".claude.json")
            || value.ends_with(".mcp.json")
            || value.ends_with(".codex/config.toml")
            || value.contains("/mcps/")
    })
}

fn class_name(class: BackupClass) -> &'static str {
    match class {
        BackupClass::Portable => "portable",
        BackupClass::Local => "local",
        BackupClass::Legacy => "legacy",
    }
}

fn label_for(class: BackupClass, operation: &str) -> String {
    let class = match class {
        BackupClass::Portable => "Portable",
        BackupClass::Local => "Local",
        BackupClass::Legacy => "Legacy",
    };
    format!("{class} {operation} backup")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn home() -> PathBuf {
        std::env::temp_dir().join(format!(
            "maa-backup-history-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn lists_portable_local_and_legacy_backups_without_restoring() {
        let home = home();
        let root = home.join(".my-agent-assets/backups");
        let portable = root.join("portable/import-1");
        let local = root.join("local/mount-1");
        let legacy = root.join("legacy-1");
        fs::create_dir_all(portable.join("content")).unwrap();
        fs::create_dir_all(&local).unwrap();
        fs::create_dir_all(&legacy).unwrap();
        fs::write(
            portable.join("manifest.yaml"),
            "schemaVersion: 1\noperation: import-overwrite\ndestination: assets/mcps/postgres.json\n",
        )
        .unwrap();
        fs::write(portable.join("content/server.json"), "{}").unwrap();
        fs::write(
            local.join("manifest.yaml"),
            "schemaVersion: 1\noperation: mount\ntarget: /tmp/home/.claude.json\n",
        )
        .unwrap();
        fs::write(
            legacy.join("manifest.json"),
            r#"{"label":"Old backup","entries":[{"originalPath":"/tmp/old"}]}"#,
        )
        .unwrap();

        let entries = list_backups(&home);
        assert_eq!(entries.len(), 3);
        let portable = entries
            .iter()
            .find(|entry| entry.class == BackupClass::Portable)
            .unwrap();
        assert_eq!(portable.operation, "import-overwrite");
        assert!(portable.sensitive_config_risk);
        assert!(portable.manual_restore_only);
        assert!(entries
            .iter()
            .any(|entry| entry.class == BackupClass::Local));
        assert!(entries
            .iter()
            .any(|entry| entry.class == BackupClass::Legacy));
        assert_eq!(
            resolve_backup_manifest(&home, &portable.id).unwrap(),
            portable.manifest_path.canonicalize().unwrap()
        );
        assert!(resolve_backup_manifest(&home, "../escape").is_err());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn missing_backup_root_returns_empty_history() {
        assert!(list_backups(&home()).is_empty());
    }

    #[test]
    fn symlinked_backup_directories_are_not_followed() {
        let home = home();
        let outside = home.join("outside");
        let portable = home.join(".my-agent-assets/backups/portable");
        fs::create_dir_all(&outside).unwrap();
        fs::create_dir_all(&portable).unwrap();
        fs::write(outside.join("manifest.yaml"), "operation: delete\n").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, portable.join("escape")).unwrap();
        assert!(list_backups(&home).is_empty());
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_manifest_is_not_listed_or_revealed() {
        let home = home();
        let outside = home.join("outside.yaml");
        let backup = home.join(".my-agent-assets/backups/local/linked");
        fs::create_dir_all(&backup).unwrap();
        fs::write(&outside, "operation: mount\n").unwrap();
        std::os::unix::fs::symlink(&outside, backup.join("manifest.yaml")).unwrap();
        assert!(list_backups(&home).is_empty());
        assert!(resolve_backup_manifest(&home, "local:linked").is_err());
        let _ = fs::remove_dir_all(home);
    }
}
