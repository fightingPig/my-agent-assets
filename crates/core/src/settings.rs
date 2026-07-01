use crate::operation::{OperationJournal, OperationLock, RecoveryTarget};
use crate::path_safety::guard_write_path;
use crate::{MaaError, Result as CoreResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppearanceTheme {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "dark")]
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DensityPreference {
    #[serde(rename = "compact")]
    Compact,
    #[serde(rename = "comfortable")]
    Comfortable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "debug")]
    Debug,
}

/// Machine-local settings exposed to CLI and desktop adapters.
///
/// `asset_center_path` is derived from `home` by `load`/`save` and is never
/// persisted, so callers cannot relocate the V1/V2 asset center through this
/// settings API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub asset_center_path: String,
    pub scan_roots: Vec<String>,
    pub max_depth: u32,
    pub backup_before_apply: bool,
    pub plan_only_by_default: bool,
    pub git_default_branch: String,
    pub git_remote: String,
    pub appearance_theme: AppearanceTheme,
    pub density: DensityPreference,
    pub log_level: LogLevel,
    pub log_retention_days: u32,
    pub cli_path: String,
}

impl Settings {
    pub fn defaults_for_home(home: &Path) -> Self {
        Self {
            asset_center_path: display_path(&asset_center_path(home)),
            scan_roots: vec![
                display_path(&home.join(".claude")),
                display_path(&home.join("workspace")),
                display_path(&home.join("code")),
            ],
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
}

#[derive(Debug)]
pub enum SettingsError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    InvalidYaml {
        path: PathBuf,
        message: String,
    },
    UnsupportedSchema {
        path: PathBuf,
        found: u64,
        supported: u32,
    },
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "failed to access settings at {}: {source}",
                    display_path(path)
                )
            }
            Self::InvalidYaml { path, message } => write!(
                formatter,
                "invalid settings YAML at {}: {message}",
                display_path(path)
            ),
            Self::UnsupportedSchema {
                path,
                found,
                supported,
            } => write!(
                formatter,
                "unsupported settings schemaVersion {found} at {}; this build supports schemaVersion {supported}",
                display_path(path)
            ),
        }
    }
}

impl std::error::Error for SettingsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidYaml { .. } | Self::UnsupportedSchema { .. } => None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemaHeader {
    schema_version: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SettingsFile {
    schema_version: u32,
    scan_roots: Vec<String>,
    max_depth: u32,
    backup_before_apply: bool,
    plan_only_by_default: bool,
    git_default_branch: String,
    git_remote: String,
    appearance_theme: AppearanceTheme,
    density: DensityPreference,
    log_level: LogLevel,
    log_retention_days: u32,
    cli_path: String,
}

pub fn settings_path(home: &Path) -> PathBuf {
    asset_center_path(home).join("config.yaml")
}

pub fn load(home: &Path) -> Result<Settings, SettingsError> {
    let path = settings_path(home);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(Settings::defaults_for_home(home));
        }
        Err(source) => return Err(SettingsError::Io { path, source }),
    };

    let header: SchemaHeader =
        serde_yaml::from_str(&text).map_err(|error| SettingsError::InvalidYaml {
            path: path.clone(),
            message: error.to_string(),
        })?;
    if header.schema_version != u64::from(SETTINGS_SCHEMA_VERSION) {
        return Err(SettingsError::UnsupportedSchema {
            path,
            found: header.schema_version,
            supported: SETTINGS_SCHEMA_VERSION,
        });
    }

    let file: SettingsFile =
        serde_yaml::from_str(&text).map_err(|error| SettingsError::InvalidYaml {
            path: path.clone(),
            message: error.to_string(),
        })?;
    Ok(normalize_settings(home, file.into_settings(home)))
}

pub fn save(home: &Path, settings: &Settings) -> Result<Settings, SettingsError> {
    let asset_center = asset_center_path(home);
    let requested_path = settings_path(home);
    if requested_path.exists() {
        // Validate before replacing. Corrupt files and files from a newer
        // schema are preserved for diagnosis or an explicit migration.
        load(home)?;
    }

    let normalized = normalize_settings(home, settings.clone());
    let path =
        guard_write_path(&asset_center, &requested_path).map_err(|source| SettingsError::Io {
            path: requested_path,
            source,
        })?;
    let parent = path
        .parent()
        .expect("guarded settings path always has an asset-center parent");
    fs::create_dir_all(parent).map_err(|source| SettingsError::Io {
        path: parent.to_path_buf(),
        source,
    })?;

    let yaml = to_yaml(home, &normalized)?;
    write_atomic(&path, yaml.as_bytes()).map_err(|source| SettingsError::Io {
        path: path.clone(),
        source,
    })?;

    // Reload what reached disk. This verifies serialization and guarantees the
    // returned value matches the persisted, normalized representation.
    load(home)
}

pub fn to_yaml(home: &Path, settings: &Settings) -> Result<String, SettingsError> {
    let normalized = normalize_settings(home, settings.clone());
    serde_yaml::to_string(&SettingsFile::from_settings(&normalized)).map_err(|error| {
        SettingsError::InvalidYaml {
            path: settings_path(home),
            message: error.to_string(),
        }
    })
}

pub fn save_transactional(home: &Path, settings: &Settings) -> CoreResult<Settings> {
    let _lock = OperationLock::acquire(home)?;
    let operation_id = format!(
        "settings-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let mut journal = OperationJournal::start_recoverable(
        home,
        &operation_id,
        "settings_save",
        vec![RecoveryTarget::asset_center(settings_path(home))],
    )?;
    match save(home, settings).map_err(|error| MaaError::new(error.to_string())) {
        Ok(saved) => {
            journal.record_step("settings_saved")?;
            journal.complete()?;
            Ok(saved)
        }
        Err(error) => {
            let original = error.to_string();
            journal.rollback_now(home).map_err(|rollback| {
                MaaError::new(format!(
                    "{original}; persistent settings rollback failed: {rollback}"
                ))
            })?;
            Err(error)
        }
    }
}

impl SettingsFile {
    fn from_settings(settings: &Settings) -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            scan_roots: settings.scan_roots.clone(),
            max_depth: settings.max_depth,
            backup_before_apply: settings.backup_before_apply,
            plan_only_by_default: settings.plan_only_by_default,
            git_default_branch: settings.git_default_branch.clone(),
            git_remote: settings.git_remote.clone(),
            appearance_theme: settings.appearance_theme,
            density: settings.density,
            log_level: settings.log_level,
            log_retention_days: settings.log_retention_days,
            cli_path: settings.cli_path.clone(),
        }
    }

    fn into_settings(self, home: &Path) -> Settings {
        Settings {
            asset_center_path: display_path(&asset_center_path(home)),
            scan_roots: self.scan_roots,
            max_depth: self.max_depth,
            backup_before_apply: self.backup_before_apply,
            plan_only_by_default: self.plan_only_by_default,
            git_default_branch: self.git_default_branch,
            git_remote: self.git_remote,
            appearance_theme: self.appearance_theme,
            density: self.density,
            log_level: self.log_level,
            log_retention_days: self.log_retention_days,
            cli_path: self.cli_path,
        }
    }
}

fn normalize_settings(home: &Path, mut settings: Settings) -> Settings {
    let defaults = Settings::defaults_for_home(home);
    settings.asset_center_path = defaults.asset_center_path;
    settings.scan_roots = normalize_scan_roots(home, settings.scan_roots);
    if settings.scan_roots.is_empty() {
        settings.scan_roots = defaults.scan_roots;
    }
    settings.max_depth = settings.max_depth.clamp(1, 20);
    settings.log_retention_days = settings.log_retention_days.clamp(1, 365);
    settings.git_default_branch =
        non_empty_or_default(settings.git_default_branch, defaults.git_default_branch);
    settings.git_remote = non_empty_or_default(settings.git_remote, defaults.git_remote);
    settings.cli_path = non_empty_or_default(settings.cli_path, defaults.cli_path);
    settings
}

fn normalize_scan_roots(home: &Path, roots: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    roots
        .into_iter()
        .filter_map(|root| {
            let root = root.trim();
            if root.is_empty() {
                return None;
            }
            let expanded = expand_tilde(root, home);
            let absolute = if expanded.is_absolute() {
                expanded
            } else {
                home.join(expanded)
            };
            let normalized = normalize_lexically(&absolute);
            let display = display_path(&normalized);
            seen.insert(display.clone()).then_some(display)
        })
        .collect()
}

fn non_empty_or_default(value: String, default: String) -> String {
    let value = value.trim();
    if value.is_empty() {
        default
    } else {
        value.to_owned()
    }
}

fn asset_center_path(home: &Path) -> PathBuf {
    home.join(".my-agent-assets")
}

fn expand_tilde(value: &str, home: &Path) -> PathBuf {
    if value == "~" {
        home.to_path_buf()
    } else if let Some(rest) = value
        .strip_prefix("~/")
        .or_else(|| value.strip_prefix("~\\"))
    {
        home.join(rest)
    } else {
        PathBuf::from(value)
    }
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.yaml");
    let temporary =
        path.with_file_name(format!(".{file_name}.tmp-{}-{suffix}", std::process::id()));

    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);

        let written = fs::read(&temporary)?;
        if written != bytes {
            return Err(io::Error::other(
                "temporary settings verification failed before atomic replace",
            ));
        }
        fs::rename(&temporary, path)?;
        if let Some(parent) = path.parent() {
            sync_directory(parent)?;
        }
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> io::Result<()> {
    OpenOptions::new().read(true).open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

    fn fake_home(label: &str) -> PathBuf {
        let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
        let home = std::env::temp_dir().join(format!(
            "maa-core-settings-{label}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(&home).unwrap();
        home
    }

    #[test]
    fn missing_file_returns_defaults_without_writing() {
        let home = fake_home("defaults");

        let settings = load(&home).unwrap();

        assert_eq!(settings, Settings::defaults_for_home(&home));
        assert!(!settings_path(&home).exists());
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn save_normalizes_and_round_trips_yaml_without_asset_center_field() {
        let home = fake_home("round-trip");
        let outside = home.join("..").join(
            home.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("home"),
        );
        let mut settings = Settings::defaults_for_home(&home);
        settings.asset_center_path = "/tmp/ignored-asset-center".into();
        settings.scan_roots = vec![
            " ~/workspace/./app ".into(),
            "~/workspace/app".into(),
            outside.join("code").to_string_lossy().into_owned(),
            String::new(),
        ];
        settings.max_depth = 99;
        settings.log_retention_days = 0;
        settings.git_default_branch = " ".into();
        settings.git_remote = " upstream ".into();
        settings.cli_path = " ".into();

        let saved = save(&home, &settings).unwrap();
        let text = fs::read_to_string(settings_path(&home)).unwrap();

        assert_eq!(
            saved.asset_center_path,
            display_path(&home.join(".my-agent-assets"))
        );
        assert_eq!(saved.max_depth, 20);
        assert_eq!(saved.log_retention_days, 1);
        assert_eq!(saved.git_default_branch, "main");
        assert_eq!(saved.git_remote, "upstream");
        assert_eq!(saved.cli_path, "maa");
        assert_eq!(saved.scan_roots.len(), 2);
        assert!(text.contains("schemaVersion: 1"));
        assert!(!text.contains("assetCenterPath"));
        assert!(!text.contains("ignored-asset-center"));
        assert!(!text.contains(".tmp-"));
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn malformed_yaml_is_explicit_and_preserved() {
        let home = fake_home("malformed");
        let path = settings_path(&home);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = "schemaVersion: 1\nscanRoots: [unterminated\n";
        fs::write(&path, original).unwrap();

        let error = load(&home).unwrap_err();

        assert!(matches!(error, SettingsError::InvalidYaml { .. }));
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn newer_schema_is_explicit_and_save_does_not_overwrite_it() {
        let home = fake_home("new-schema");
        let path = settings_path(&home);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = "schemaVersion: 2\nfutureField: true\n";
        fs::write(&path, original).unwrap();

        let load_error = load(&home).unwrap_err();
        let save_error = save(&home, &Settings::defaults_for_home(&home)).unwrap_err();

        assert!(matches!(
            load_error,
            SettingsError::UnsupportedSchema { found: 2, .. }
        ));
        assert!(matches!(
            save_error,
            SettingsError::UnsupportedSchema { found: 2, .. }
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        fs::remove_dir_all(home).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn save_rejects_symlinked_asset_center() {
        use std::os::unix::fs::symlink;

        let home = fake_home("symlink");
        let real = home.join("real-center");
        fs::create_dir_all(&real).unwrap();
        symlink(&real, home.join(".my-agent-assets")).unwrap();

        let error = save(&home, &Settings::defaults_for_home(&home)).unwrap_err();

        assert!(matches!(error, SettingsError::Io { .. }));
        assert!(!real.join("config.yaml").exists());
        fs::remove_dir_all(home).unwrap();
    }
}
