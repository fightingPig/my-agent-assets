use crate::operation::OperationLock;
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const PREVIEW_TTL_SECONDS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRemotePreviewRequest {
    pub remote_name: String,
    pub remote_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRemotePreview {
    pub preview_id: String,
    pub remote_name: String,
    pub previous_url: Option<String>,
    pub remote_url: String,
    pub affected_paths: Vec<PathBuf>,
    pub warnings: Vec<String>,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRemoteApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: GitRemotePreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRemoteApplyResult {
    pub preview_id: String,
    pub remote_name: String,
    pub remote_url: String,
    pub backup_path: PathBuf,
    pub affected_paths: Vec<PathBuf>,
}

pub fn preview_git_remote(
    home: &Path,
    request: &GitRemotePreviewRequest,
) -> Result<GitRemotePreview> {
    preview_git_remote_at(home, request, epoch_seconds())
}

pub fn apply_git_remote(
    home: &Path,
    request: &GitRemoteApplyRequest,
) -> Result<GitRemoteApplyResult> {
    validate_preview_time(request.preview_generated_at_epoch_seconds)?;
    let _lock = OperationLock::acquire(home)?;
    let preview = preview_git_remote_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    if !preview.can_apply || preview.preview_id != request.preview_id {
        return Err(MaaError::new(
            "Git remote preview is invalid or stale; generate a new preview",
        ));
    }

    let repository = repository_path(home);
    let config_path = repository.join(".git/config");
    let backup_path = home
        .join(".my-agent-assets/backups/local")
        .join(format!("git-config-{}.bak", epoch_nanos()));
    fs::create_dir_all(
        backup_path
            .parent()
            .ok_or_else(|| MaaError::new("invalid Git config backup path"))?,
    )
    .map_err(|error| MaaError::new(format!("failed to create Git config backup: {error}")))?;
    fs::copy(&config_path, &backup_path)
        .map_err(|error| MaaError::new(format!("failed to back up Git config: {error}")))?;

    let args = if preview.previous_url.is_some() {
        vec![
            "remote",
            "set-url",
            request.request.remote_name.as_str(),
            request.request.remote_url.as_str(),
        ]
    } else {
        vec![
            "remote",
            "add",
            request.request.remote_name.as_str(),
            request.request.remote_url.as_str(),
        ]
    };
    let output = Command::new("git")
        .current_dir(&repository)
        .args(args)
        .output()
        .map_err(|_| MaaError::new("Git is unavailable"))?;
    if !output.status.success() {
        let _ = fs::copy(&backup_path, &config_path);
        return Err(MaaError::new(
            "failed to configure Git remote; the previous Git config was restored",
        ));
    }

    Ok(GitRemoteApplyResult {
        preview_id: preview.preview_id,
        remote_name: request.request.remote_name.clone(),
        remote_url: request.request.remote_url.clone(),
        backup_path,
        affected_paths: vec![config_path],
    })
}

fn preview_git_remote_at(
    home: &Path,
    request: &GitRemotePreviewRequest,
    generated_at: u64,
) -> Result<GitRemotePreview> {
    let repository = repository_path(home);
    let config_path = repository.join(".git/config");
    let mut warnings = Vec::new();
    if !repository.join(".git").is_dir() {
        warnings.push("资产中心尚未初始化为本地 Git 仓库。".into());
    }
    if !valid_remote_name(&request.remote_name) {
        warnings.push("远程名称只能包含字母、数字、点、下划线和连字符。".into());
    }
    if !valid_remote_url(&request.remote_url) {
        warnings.push("远程 URL 为空、包含凭据或不受支持。".into());
    }
    let previous_url = if warnings.is_empty() {
        git_stdout(
            &repository,
            &["remote", "get-url", request.remote_name.as_str()],
        )
        .ok()
    } else {
        None
    };
    let preview_id = fingerprint(request, previous_url.as_deref(), generated_at);
    Ok(GitRemotePreview {
        preview_id,
        remote_name: request.remote_name.clone(),
        previous_url,
        remote_url: request.remote_url.clone(),
        affected_paths: vec![config_path],
        can_apply: warnings.is_empty(),
        warnings,
        generated_at_epoch_seconds: generated_at,
        expires_at_epoch_seconds: generated_at + PREVIEW_TTL_SECONDS,
    })
}

fn repository_path(home: &Path) -> PathBuf {
    home.join(".my-agent-assets")
}

fn valid_remote_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn valid_remote_url(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() || value.chars().any(char::is_whitespace) {
        return false;
    }
    if let Some(rest) = value.strip_prefix("https://") {
        return !rest.contains('@') && rest.contains('/');
    }
    if let Some(rest) = value.strip_prefix("ssh://") {
        let authority = rest.split('/').next().unwrap_or_default();
        return !authority
            .split('@')
            .next()
            .unwrap_or_default()
            .contains(':')
            && rest.contains('/');
    }
    value.starts_with("git@") && value.contains(':')
}

fn git_stdout(repository: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repository)
        .args(args)
        .output()
        .map_err(|_| MaaError::new("Git is unavailable"))?;
    if !output.status.success() {
        return Err(MaaError::new("Git command failed"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn fingerprint(
    request: &GitRemotePreviewRequest,
    previous_url: Option<&str>,
    generated_at: u64,
) -> String {
    let mut hash = Sha256::new();
    hash.update(request.remote_name.as_bytes());
    hash.update([0]);
    hash.update(request.remote_url.as_bytes());
    hash.update([0]);
    hash.update(previous_url.unwrap_or_default().as_bytes());
    hash.update(generated_at.to_le_bytes());
    format!("git-remote:{:x}", hash.finalize())
}

fn validate_preview_time(generated_at: u64) -> Result<()> {
    let now = epoch_seconds();
    if generated_at > now.saturating_add(5)
        || now.saturating_sub(generated_at) > PREVIEW_TTL_SECONDS
    {
        return Err(MaaError::new(
            "Git remote preview expired; generate a new preview",
        ));
    }
    Ok(())
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn epoch_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository(label: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!("maa-git-remote-{label}-{}", epoch_nanos()));
        let repository = home.join(".my-agent-assets");
        fs::create_dir_all(&repository).unwrap();
        assert!(Command::new("git")
            .current_dir(&repository)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success());
        home
    }

    #[test]
    fn preview_is_read_only_and_apply_updates_only_after_confirmation() {
        let home = repository("apply");
        let request = GitRemotePreviewRequest {
            remote_name: "origin".into(),
            remote_url: "git@github.com:example/private-assets.git".into(),
        };
        let config_path = home.join(".my-agent-assets/.git/config");
        let before = fs::read(&config_path).unwrap();
        let preview = preview_git_remote(&home, &request).unwrap();
        assert!(preview.can_apply);
        assert_eq!(fs::read(&config_path).unwrap(), before);
        let result = apply_git_remote(
            &home,
            &GitRemoteApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap();
        assert!(result.backup_path.is_file());
        assert_eq!(
            git_stdout(
                &home.join(".my-agent-assets"),
                &["remote", "get-url", "origin"]
            )
            .unwrap(),
            "git@github.com:example/private-assets.git"
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn preview_rejects_credentials_and_missing_repository() {
        let home = std::env::temp_dir().join(format!("maa-git-remote-invalid-{}", epoch_nanos()));
        fs::create_dir_all(&home).unwrap();
        let preview = preview_git_remote(
            &home,
            &GitRemotePreviewRequest {
                remote_name: "origin".into(),
                remote_url: "https://token@example.test/repo.git".into(),
            },
        )
        .unwrap();
        assert!(!preview.can_apply);
        assert!(!preview.warnings.is_empty());
        let _ = fs::remove_dir_all(home);
    }
}
