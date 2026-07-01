use crate::asset_registry::{self, AssetRegistry};
use crate::fingerprint::PreviewFingerprint;
use crate::mount_registry::{self, MountRegistry};
use crate::settings::{self, Settings};
use crate::targets::{self, MountAdapter, ProviderState, TargetRegistry};
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const PREVIEW_TTL_SECONDS: u64 = 600;
const ROOT_NAME: &str = ".my-agent-assets";
const REQUIRED_DIRECTORIES: &[&str] = &[
    "assets/skills",
    "assets/commands",
    "assets/mcps",
    "backups/portable",
    "backups/local",
];
const REQUIRED_FILES: &[&str] = &[
    "config.yaml",
    "assets.yaml",
    "targets.yaml",
    "mounts.yaml",
    ".gitignore",
];
const GITIGNORE: &str = "config.yaml\ntargets.yaml\nmounts.yaml\nbackups/local/\noperations/\nlocks/\ncache/\nlogs/\nsecrets/\n";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializationPreview {
    pub preview_id: String,
    pub asset_center_path: PathBuf,
    pub planned_paths: Vec<PathBuf>,
    pub warnings: Vec<String>,
    pub already_initialized: bool,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializationApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializationApplyResult {
    pub preview_id: String,
    pub asset_center_path: PathBuf,
    pub created: bool,
    pub created_paths: Vec<PathBuf>,
}

pub fn preview_initialization(home: &Path) -> Result<InitializationPreview> {
    preview_initialization_at(home, epoch_seconds())
}

fn preview_initialization_at(
    home: &Path,
    generated_at_epoch_seconds: u64,
) -> Result<InitializationPreview> {
    validate_home(home)?;
    let root = home.join(ROOT_NAME);
    let mut warnings = Vec::new();
    let mut planned_paths = Vec::new();
    let already_initialized = if fs::symlink_metadata(&root).is_ok() {
        match validate_existing(home, &root) {
            Ok(()) => true,
            Err(error) => {
                warnings.push(format!(
                    "资产中心目录已存在但结构无效，初始化已阻止：{error}"
                ));
                false
            }
        }
    } else {
        planned_paths.push(root.clone());
        planned_paths.extend(
            REQUIRED_DIRECTORIES
                .iter()
                .map(|relative| root.join(relative)),
        );
        planned_paths.extend(REQUIRED_FILES.iter().map(|relative| root.join(relative)));
        planned_paths.push(root.join(".git"));
        false
    };
    if !git_available() {
        warnings.push("未找到 Git；初始化资产中心需要本机 Git。".into());
    }
    let root_exists = fs::symlink_metadata(&root).is_ok();
    let can_apply = already_initialized || (!root_exists && warnings.is_empty());
    let mut fingerprint = PreviewFingerprint::new("initialization");
    fingerprint.add_bytes("home", home.to_string_lossy().as_bytes());
    fingerprint.add_u64("generated-at", generated_at_epoch_seconds);
    fingerprint.add_path_if_present("asset-center", &root)?;
    fingerprint.add_bytes(
        "planned-paths",
        &serde_json::to_vec(&planned_paths).map_err(|error| MaaError::new(error.to_string()))?,
    );
    Ok(InitializationPreview {
        preview_id: fingerprint.finish("init"),
        asset_center_path: root,
        planned_paths,
        warnings,
        already_initialized,
        can_apply,
        generated_at_epoch_seconds,
        expires_at_epoch_seconds: generated_at_epoch_seconds.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

pub fn apply_initialization(
    home: &Path,
    request: &InitializationApplyRequest,
) -> Result<InitializationApplyResult> {
    let _lock = InitializationLock::acquire(home)?;
    if epoch_seconds()
        > request
            .preview_generated_at_epoch_seconds
            .saturating_add(PREVIEW_TTL_SECONDS)
    {
        return Err(MaaError::new(
            "initialization preview expired; generate a new preview before applying",
        ));
    }
    let preview = preview_initialization_at(home, request.preview_generated_at_epoch_seconds)?;
    if preview.preview_id != request.preview_id {
        return Err(MaaError::new(
            "initialization preview is stale; generate a new preview before applying",
        ));
    }
    if !preview.can_apply {
        return Err(MaaError::new(
            preview
                .warnings
                .first()
                .cloned()
                .unwrap_or_else(|| "initialization is blocked".into()),
        ));
    }
    if preview.already_initialized {
        return Ok(InitializationApplyResult {
            preview_id: preview.preview_id,
            asset_center_path: preview.asset_center_path,
            created: false,
            created_paths: Vec::new(),
        });
    }

    let staging = home.join(format!(
        ".my-agent-assets.init-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    if fs::symlink_metadata(&staging).is_ok() {
        return Err(MaaError::new(format!(
            "initialization staging path already exists: {}",
            staging.display()
        )));
    }
    let result = build_staging(home, &staging).and_then(|_| {
        if fs::symlink_metadata(&preview.asset_center_path).is_ok() {
            return Err(MaaError::new(
                "asset center appeared during initialization; generate a new preview",
            ));
        }
        fs::rename(&staging, &preview.asset_center_path)?;
        sync_directory(home)?;
        validate_existing(home, &preview.asset_center_path)
    });
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    Ok(InitializationApplyResult {
        preview_id: preview.preview_id,
        asset_center_path: preview.asset_center_path,
        created: true,
        created_paths: preview.planned_paths,
    })
}

fn build_staging(home: &Path, staging: &Path) -> Result<()> {
    for relative in REQUIRED_DIRECTORIES {
        fs::create_dir_all(staging.join(relative))?;
    }
    write_synced(
        &staging.join("config.yaml"),
        settings::to_yaml(home, &Settings::defaults_for_home(home))
            .map_err(|error| MaaError::new(error.to_string()))?
            .as_bytes(),
    )?;
    write_synced(
        &staging.join("assets.yaml"),
        serde_yaml::to_string(&AssetRegistry::default())
            .map_err(|error| MaaError::new(error.to_string()))?
            .as_bytes(),
    )?;
    let targets = TargetRegistry::standard_user_targets(
        home,
        provider_state(home.join(".claude").exists() || home.join(".claude.json").exists()),
        provider_state(home.join(".codex").exists()),
        directory_mount_adapter(),
    )?;
    write_synced(&staging.join("targets.yaml"), targets.to_yaml()?.as_bytes())?;
    write_synced(
        &staging.join("mounts.yaml"),
        serde_yaml::to_string(&MountRegistry::default())
            .map_err(|error| MaaError::new(error.to_string()))?
            .as_bytes(),
    )?;
    write_synced(&staging.join(".gitignore"), GITIGNORE.as_bytes())?;
    initialize_git(staging)?;
    sync_tree(staging)?;
    Ok(())
}

fn validate_existing(home: &Path, root: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(root)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(MaaError::new("asset center path must be a real directory"));
    }
    for relative in REQUIRED_DIRECTORIES {
        let path = root.join(relative);
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            MaaError::new(format!(
                "missing required directory {}: {error}",
                path.display()
            ))
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(MaaError::new(format!(
                "required path is not a real directory: {}",
                path.display()
            )));
        }
    }
    for relative in REQUIRED_FILES {
        let path = root.join(relative);
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            MaaError::new(format!("missing required file {}: {error}", path.display()))
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(MaaError::new(format!(
                "required path is not a real file: {}",
                path.display()
            )));
        }
    }
    settings::load(home).map_err(|error| MaaError::new(error.to_string()))?;
    asset_registry::load(home).map_err(|error| MaaError::new(error.to_string()))?;
    mount_registry::load(home).map_err(|error| MaaError::new(error.to_string()))?;
    targets::load(home)?;
    let output = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(root)
        .output()
        .map_err(git_error)?;
    if !output.status.success() || String::from_utf8_lossy(&output.stdout).trim() != "true" {
        return Err(MaaError::new("asset center is not a valid Git repository"));
    }
    Ok(())
}

fn initialize_git(staging: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(staging)
        .output()
        .map_err(git_error)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(MaaError::new(
            "failed to initialize asset center Git repository",
        ))
    }
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn git_error(error: std::io::Error) -> MaaError {
    if error.kind() == std::io::ErrorKind::NotFound {
        MaaError::new("Git is not installed or not available in PATH")
    } else {
        MaaError::new(error.to_string())
    }
}

fn validate_home(home: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(home)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(MaaError::new("HOME must be an existing real directory"));
    }
    Ok(())
}

fn provider_state(initialized: bool) -> ProviderState {
    if initialized {
        ProviderState::Initialized
    } else {
        ProviderState::NotInstalled
    }
}

fn directory_mount_adapter() -> MountAdapter {
    #[cfg(windows)]
    {
        MountAdapter::WindowsDirectoryJunction
    }
    #[cfg(not(windows))]
    {
        MountAdapter::SymlinkDirectory
    }
}

fn write_synced(path: &Path, content: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(content)?;
    file.sync_all()?;
    Ok(())
}

fn sync_tree(path: &Path) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() {
            sync_tree(&path)?;
        } else if metadata.is_file() {
            OpenOptions::new().read(true).open(&path)?.sync_all()?;
        }
    }
    sync_directory(path)
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<()> {
    OpenOptions::new().read(true).open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<()> {
    Ok(())
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

struct InitializationLock {
    path: PathBuf,
}

impl InitializationLock {
    fn acquire(home: &Path) -> Result<Self> {
        let path = home.join(".my-agent-assets.init.lock");
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    MaaError::new("another asset center initialization is already running")
                } else {
                    MaaError::new(error.to_string())
                }
            })?;
        Ok(Self { path })
    }
}

impl Drop for InitializationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(name: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!(
            "maa-init-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&home).unwrap();
        home
    }

    #[test]
    fn preview_is_read_only_and_apply_is_atomic_and_idempotent() {
        let home = home("happy");
        let preview = preview_initialization(&home).unwrap();
        crate::fingerprint::assert_sha256_preview_id(&preview.preview_id, "init-");
        assert!(preview.can_apply);
        assert!(!preview.already_initialized);
        assert!(!home.join(ROOT_NAME).exists());

        let applied = apply_initialization(
            &home,
            &InitializationApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
            },
        )
        .unwrap();
        assert!(applied.created);
        validate_existing(&home, &home.join(ROOT_NAME)).unwrap();
        assert!(!home.join(".my-agent-assets.init.lock").exists());

        let preview = preview_initialization(&home).unwrap();
        assert!(preview.already_initialized);
        assert!(preview.planned_paths.is_empty());
        let applied = apply_initialization(
            &home,
            &InitializationApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
            },
        )
        .unwrap();
        assert!(!applied.created);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn first_start_recovery_check_and_preview_do_not_create_asset_center() {
        let home = home("first-start");
        let recovery = crate::operation::recover_incomplete(&home).unwrap();
        assert!(!recovery.attempted);
        let preview = preview_initialization(&home).unwrap();
        assert!(preview.can_apply);
        assert!(!home.join(ROOT_NAME).exists());
        assert_eq!(
            fs::read_dir(&home).unwrap().count(),
            0,
            "first-start checks must not write into HOME"
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn damaged_existing_root_is_blocked_without_overwrite() {
        let home = home("damaged");
        let root = home.join(ROOT_NAME);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("keep.txt"), "user data").unwrap();
        let preview = preview_initialization(&home).unwrap();
        assert!(!preview.can_apply);
        assert!(!preview.already_initialized);
        assert_eq!(
            fs::read_to_string(root.join("keep.txt")).unwrap(),
            "user data"
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn stale_preview_does_not_modify_existing_path() {
        let home = home("stale");
        let preview = preview_initialization(&home).unwrap();
        let root = home.join(ROOT_NAME);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("keep.txt"), "external").unwrap();
        let error = apply_initialization(
            &home,
            &InitializationApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("stale"));
        assert_eq!(
            fs::read_to_string(root.join("keep.txt")).unwrap(),
            "external"
        );
        let _ = fs::remove_dir_all(home);
    }
}
