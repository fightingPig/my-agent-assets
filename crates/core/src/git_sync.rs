use crate::mount::copy_any;
use crate::operation::{GitRefRecovery, OperationJournal, OperationLock, RecoveryTarget};
use crate::path_safety::guard_existing_path;
use crate::settings;
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PREVIEW_TTL_SECONDS: u64 = 600;
const SYNC_PATHS: &[&str] = &[".gitignore", "assets", "assets.yaml", "backups/portable"];
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncDirection {
    #[serde(rename = "pull")]
    Pull,
    #[serde(rename = "push")]
    Push,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepositoryVisibility {
    #[serde(rename = "private")]
    Private,
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "internal")]
    Internal,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub repository_path: PathBuf,
    pub is_repository: bool,
    pub status_message: String,
    pub branch: String,
    pub remote_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_identity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    pub clean: bool,
    pub ahead: u32,
    pub behind: u32,
    pub changed_files: Vec<String>,
    pub conflicts: Vec<String>,
    pub syncable_changes: Vec<String>,
    pub blocked_changes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPreviewRequest {
    pub direction: SyncDirection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPreview {
    pub preview_id: String,
    pub direction: SyncDirection,
    pub status: GitStatus,
    pub repository_visibility: RepositoryVisibility,
    pub planned_effects: Vec<String>,
    pub warnings: Vec<String>,
    pub backup_required: bool,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: SyncPreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncApplyResult {
    pub preview_id: String,
    pub direction: SyncDirection,
    pub affected_paths: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,
    pub committed: bool,
    pub pushed: bool,
    pub pulled: bool,
    pub warnings: Vec<String>,
    pub journal_path: PathBuf,
}

pub trait VisibilityVerifier {
    fn verify(&self, remote_url: &str) -> Result<(RepositoryVisibility, String)>;
}

pub struct GhCliVisibilityVerifier;

impl VisibilityVerifier for GhCliVisibilityVerifier {
    fn verify(&self, remote_url: &str) -> Result<(RepositoryVisibility, String)> {
        let repository = parse_github_repository(remote_url)?;
        let output = Command::new("gh")
            .args(["api", &format!("repos/{repository}"), "--jq", ".visibility"])
            .output()
            .map_err(|_| {
                MaaError::new(
                    "GitHub Private visibility could not be verified because `gh` is unavailable",
                )
            })?;
        if !output.status.success() {
            return Err(MaaError::new(
                "GitHub Private visibility could not be verified with the local `gh` authentication",
            ));
        }
        let visibility = match String::from_utf8_lossy(&output.stdout).trim() {
            "private" | "PRIVATE" => RepositoryVisibility::Private,
            "public" | "PUBLIC" => RepositoryVisibility::Public,
            "internal" | "INTERNAL" => RepositoryVisibility::Internal,
            _ => RepositoryVisibility::Unknown,
        };
        Ok((visibility, format!("github.com/{repository}")))
    }
}

pub fn status(home: &Path) -> GitStatus {
    let repository = home.join(".my-agent-assets");
    let settings =
        settings::load(home).unwrap_or_else(|_| settings::Settings::defaults_for_home(home));
    status_for_repository(&repository, &settings.git_remote)
}

pub fn preview_sync(home: &Path, request: &SyncPreviewRequest) -> Result<SyncPreview> {
    preview_sync_with(home, request, &GhCliVisibilityVerifier)
}

pub fn apply_sync(home: &Path, request: &SyncApplyRequest) -> Result<SyncApplyResult> {
    apply_sync_with(home, request, &GhCliVisibilityVerifier)
}

fn preview_sync_with(
    home: &Path,
    request: &SyncPreviewRequest,
    verifier: &dyn VisibilityVerifier,
) -> Result<SyncPreview> {
    preview_sync_at(home, request, verifier, epoch_seconds())
}

fn preview_sync_at(
    home: &Path,
    request: &SyncPreviewRequest,
    verifier: &dyn VisibilityVerifier,
    generated_at: u64,
) -> Result<SyncPreview> {
    let repository = guard_repository(home)?;
    let settings = settings::load(home).map_err(|error| MaaError::new(error.to_string()))?;
    let mut status = status_for_repository(&repository, &settings.git_remote);
    let mut warnings = Vec::new();
    let mut planned_effects = Vec::new();
    let mut visibility = RepositoryVisibility::Unknown;
    let mut can_apply = status.is_repository;
    let remote_name_valid = valid_remote_name(&settings.git_remote);
    if !remote_name_valid {
        warnings.push(
            "Git remote name must use only letters, numbers, dot, underscore, or hyphen and must not start with a hyphen"
                .into(),
        );
        can_apply = false;
    }
    if !status.branch.is_empty() && !valid_branch_name(&status.branch) {
        warnings.push("Current Git branch name is unsafe for sync".into());
        can_apply = false;
    }
    let remote_url = if remote_name_valid {
        git_stdout(&repository, &["remote", "get-url", &settings.git_remote])
    } else {
        Err(MaaError::new("invalid Git remote name"))
    };
    let remote_url = match remote_url {
        Ok(value) if !value.is_empty() => value,
        _ => {
            warnings.push(format!(
                "Git remote '{}' is not configured",
                settings.git_remote
            ));
            can_apply = false;
            String::new()
        }
    };

    if !status.conflicts.is_empty() {
        warnings.push("Git conflicts must be resolved manually before sync".into());
        can_apply = false;
    }

    match request.direction {
        SyncDirection::Pull => {
            if !status.clean {
                warnings.push("Pull requires a completely clean worktree".into());
                can_apply = false;
            }
            if status.upstream.is_none() {
                warnings.push("Pull requires an upstream branch".into());
                can_apply = false;
            }
            let remote_head = if remote_url.is_empty() {
                None
            } else {
                remote_head(&repository, &settings.git_remote, &status.branch)
            };
            if let Some(remote_head) = remote_head {
                let local_head =
                    git_stdout(&repository, &["rev-parse", "HEAD"]).unwrap_or_default();
                if remote_head == local_head {
                    warnings.push("Local branch already matches the remote branch".into());
                    can_apply = false;
                }
            }
            planned_effects.push(format!(
                "back up {} before updating canonical data",
                SYNC_PATHS.join(", ")
            ));
            planned_effects.push(format!(
                "run git pull --ff-only {} {}",
                settings.git_remote, status.branch
            ));
        }
        SyncDirection::Push => {
            match verifier.verify(&remote_url) {
                Ok((verified, identity)) => {
                    visibility = verified;
                    status.remote_identity = Some(identity);
                    if visibility != RepositoryVisibility::Private {
                        warnings.push(format!(
                            "Push requires GitHub visibility PRIVATE; received {:?}",
                            visibility
                        ));
                        can_apply = false;
                    }
                }
                Err(error) => {
                    warnings.push(error.to_string());
                    can_apply = false;
                }
            }
            if !status.blocked_changes.is_empty() {
                warnings.push(format!(
                    "Non-syncable worktree changes must be removed or committed manually: {}",
                    status.blocked_changes.join(", ")
                ));
                can_apply = false;
            }
            if !git_success(&repository, &["diff", "--cached", "--quiet"]) {
                warnings.push(
                    "The Git index already contains staged changes; commit or unstage them before Push"
                        .into(),
                );
                can_apply = false;
            }
            if status.behind > 0 {
                warnings.push("Remote branch is ahead; Pull before Push".into());
                can_apply = false;
            }
            if status.ahead > 0 && status.behind > 0 {
                warnings.push("Local and remote branches have diverged".into());
                can_apply = false;
            }
            if status.upstream.is_none() && !status.branch.is_empty() && !remote_url.is_empty() {
                if remote_head(&repository, &settings.git_remote, &status.branch).is_some() {
                    warnings.push(
                        "Remote branch already exists without a local upstream; configure or Pull it before Push"
                            .into(),
                    );
                    can_apply = false;
                }
            } else if let Some(remote_head) =
                remote_head(&repository, &settings.git_remote, &status.branch)
            {
                let tracked_head =
                    git_stdout(&repository, &["rev-parse", "@{upstream}"]).unwrap_or_default();
                if !tracked_head.is_empty() && tracked_head != remote_head {
                    warnings.push(
                        "Remote branch changed after the local tracking ref; Pull before Push"
                            .into(),
                    );
                    can_apply = false;
                }
            }
            if status.syncable_changes.is_empty() && status.ahead == 0 {
                warnings.push("There are no canonical changes or commits to Push".into());
                can_apply = false;
            }
            planned_effects.push(format!(
                "stage only the sync whitelist: {}",
                SYNC_PATHS.join(", ")
            ));
            if !status.syncable_changes.is_empty() {
                planned_effects.push("create one local sync commit".into());
            }
            planned_effects.push(format!(
                "run git push {} {} without force",
                settings.git_remote, status.branch
            ));
        }
    }

    let preview_id = fingerprint_preview(&repository, request, &status, visibility, generated_at)?;
    Ok(SyncPreview {
        preview_id,
        direction: request.direction,
        status,
        repository_visibility: visibility,
        planned_effects,
        warnings,
        backup_required: request.direction == SyncDirection::Pull,
        can_apply,
        generated_at_epoch_seconds: generated_at,
        expires_at_epoch_seconds: generated_at.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

fn apply_sync_with(
    home: &Path,
    request: &SyncApplyRequest,
    verifier: &dyn VisibilityVerifier,
) -> Result<SyncApplyResult> {
    validate_preview_time(request.preview_generated_at_epoch_seconds)?;
    let _lock = OperationLock::acquire(home)?;
    let preview = preview_sync_at(
        home,
        &request.request,
        verifier,
        request.preview_generated_at_epoch_seconds,
    )?;
    if preview.preview_id != request.preview_id {
        return Err(MaaError::new(
            "Git repository or remote identity changed after preview; generate a new preview",
        ));
    }
    if !preview.can_apply {
        return Err(MaaError::new(
            preview
                .warnings
                .first()
                .cloned()
                .unwrap_or_else(|| "Git sync is blocked".into()),
        ));
    }

    let repository = guard_repository(home)?;
    let settings = settings::load(home).map_err(|error| MaaError::new(error.to_string()))?;
    let operation_id = format!(
        "sync-{}-{}-{}",
        direction_name(request.request.direction),
        epoch_nanos(),
        OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let old_head = git_stdout(&repository, &["rev-parse", "HEAD"])
        .ok()
        .filter(|value| !value.is_empty());
    let reference = format!("refs/heads/{}", preview.status.branch);
    let expected_oid = match request.request.direction {
        SyncDirection::Pull => {
            remote_head(&repository, &settings.git_remote, &preview.status.branch)
        }
        SyncDirection::Push => old_head.clone(),
    };
    let recovery_targets = if request.request.direction == SyncDirection::Pull {
        sync_paths(&repository)
            .into_iter()
            .map(RecoveryTarget::asset_center)
            .collect()
    } else {
        Vec::new()
    };
    let mut journal = OperationJournal::start_recoverable_with_git(
        home,
        &operation_id,
        "git-sync",
        recovery_targets,
        vec![GitRefRecovery {
            repository: repository.clone(),
            reference: reference.clone(),
            old_oid: old_head.clone(),
            expected_oid,
        }],
    )?;
    let mut affected_paths = vec![repository.clone()];
    let mut backup_id = None;
    let mut committed = false;
    let mut pushed = false;
    let mut pulled = false;
    let mut warnings = Vec::new();

    match request.request.direction {
        SyncDirection::Pull => {
            let id = create_pull_backup(home, &operation_id)?;
            backup_id = Some(id);
            journal.record_step("backup_created")?;
            run_git_checked(
                &repository,
                &[
                    "pull",
                    "--ff-only",
                    &settings.git_remote,
                    &preview.status.branch,
                ],
                "git pull --ff-only failed",
            )?;
            journal.record_step("pull_completed")?;
            pulled = true;
            affected_paths.extend(sync_paths(&repository));
        }
        SyncDirection::Push => {
            let old_head = old_head.unwrap_or_default();
            let temporary_index = repository
                .join("cache")
                .join(format!(".sync-index-{operation_id}"));
            if let Some(parent) = temporary_index.parent() {
                fs::create_dir_all(parent)?;
            }
            let new_head = if preview.status.syncable_changes.is_empty() {
                old_head.clone()
            } else {
                let commit = create_sync_commit(&repository, &temporary_index, &old_head)?;
                journal.set_git_ref_expected(&reference, &commit)?;
                if old_head.is_empty() {
                    run_git_checked(
                        &repository,
                        &["update-ref", "HEAD", &commit],
                        "failed to create the initial local sync commit",
                    )?;
                } else {
                    run_git_checked(
                        &repository,
                        &["update-ref", "HEAD", &commit, &old_head],
                        "failed to atomically attach the local sync commit",
                    )?;
                }
                committed = true;
                journal.record_step("sync_commit_created")?;
                commit
            };
            let push = run_git(
                &repository,
                &[
                    "push",
                    if preview.status.upstream.is_none() {
                        "--set-upstream"
                    } else {
                        "--porcelain"
                    },
                    &settings.git_remote,
                    &preview.status.branch,
                ],
            );
            if !push.status.success() {
                let rollback_result = if committed {
                    if old_head.is_empty() {
                        run_git_checked(
                            &repository,
                            &["update-ref", "-d", "HEAD", &new_head],
                            "failed to remove the initial local sync commit after Push failure",
                        )
                    } else {
                        run_git_checked(
                            &repository,
                            &["update-ref", "HEAD", &old_head, &new_head],
                            "failed to restore the local branch ref after Push failure",
                        )
                    }
                } else {
                    Ok(())
                };
                let _ = fs::remove_file(&temporary_index);
                let message =
                    "Git Push failed; canonical files remain local and no force operation was attempted";
                if let Err(rollback_error) = rollback_result {
                    let recovery = format!("{message}; {rollback_error}");
                    journal.mark_rollback_required(&recovery)?;
                    return Err(MaaError::new(recovery));
                }
                journal.complete()?;
                return Err(MaaError::new(message));
            }
            if committed {
                run_git_checked(
                    &repository,
                    &["read-tree", &new_head],
                    "Push succeeded but the local Git index could not be refreshed",
                )?;
            }
            let _ = fs::remove_file(&temporary_index);
            journal.record_step("push_completed")?;
            pushed = true;
            affected_paths.extend(sync_paths(&repository));
        }
    }

    journal.complete()?;
    Ok(SyncApplyResult {
        preview_id: preview.preview_id,
        direction: request.request.direction,
        affected_paths,
        backup_id,
        committed,
        pushed,
        pulled,
        warnings: std::mem::take(&mut warnings),
        journal_path: journal.path().to_path_buf(),
    })
}

fn status_for_repository(repository: &Path, remote_name: &str) -> GitStatus {
    let mut status = GitStatus {
        repository_path: repository.to_path_buf(),
        is_repository: false,
        status_message: "Asset center is not a Git repository".into(),
        branch: String::new(),
        remote_name: remote_name.to_string(),
        remote_identity: None,
        upstream: None,
        clean: true,
        ahead: 0,
        behind: 0,
        changed_files: Vec::new(),
        conflicts: Vec::new(),
        syncable_changes: Vec::new(),
        blocked_changes: Vec::new(),
    };
    if !repository.is_dir() || !git_success(repository, &["rev-parse", "--is-inside-work-tree"]) {
        return status;
    }
    status.is_repository = true;
    status.branch = git_stdout(repository, &["branch", "--show-current"]).unwrap_or_default();
    status.upstream = git_stdout(
        repository,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    )
    .ok()
    .filter(|value| !value.is_empty());
    if status.upstream.is_some() {
        if let Ok(counts) = git_stdout(
            repository,
            &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
        ) {
            let mut parts = counts.split_whitespace();
            status.ahead = parts
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            status.behind = parts
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
        }
    }
    let porcelain =
        git_stdout(repository, &["status", "--porcelain=v1", "-uall"]).unwrap_or_default();
    for line in porcelain.lines().filter(|line| line.len() >= 3) {
        let code = &line[..2];
        let path = normalize_status_path(line.get(2..).unwrap_or_default().trim_start());
        status.changed_files.push(path.clone());
        if code.contains('U') || matches!(code, "AA" | "DD") {
            status.conflicts.push(path.clone());
        }
        if is_sync_path(&path) {
            status.syncable_changes.push(path);
        } else {
            status.blocked_changes.push(path);
        }
    }
    for values in [
        &mut status.changed_files,
        &mut status.conflicts,
        &mut status.syncable_changes,
        &mut status.blocked_changes,
    ] {
        values.sort();
        values.dedup();
    }
    status.clean = status.changed_files.is_empty() && status.conflicts.is_empty();
    status.status_message = if status.clean {
        "Git worktree is clean".into()
    } else if !status.blocked_changes.is_empty() {
        "Git worktree contains non-syncable changes".into()
    } else {
        "Canonical changes are ready for preview".into()
    };
    status
}

fn guard_repository(home: &Path) -> Result<PathBuf> {
    let repository = home.join(".my-agent-assets");
    guard_existing_path(home, &repository).map_err(|error| MaaError::new(error.to_string()))
}

fn is_sync_path(path: &str) -> bool {
    path == "assets.yaml"
        || path == ".gitignore"
        || path == "assets"
        || path.starts_with("assets/")
        || path == "backups/portable"
        || path.starts_with("backups/portable/")
}

fn normalize_status_path(value: &str) -> String {
    let value = value.rsplit(" -> ").next().unwrap_or(value);
    value.trim_matches('"').replace('\\', "/")
}

fn remote_head(repository: &Path, remote: &str, branch: &str) -> Option<String> {
    if branch.is_empty() {
        return None;
    }
    git_stdout(
        repository,
        &[
            "ls-remote",
            "--heads",
            remote,
            &format!("refs/heads/{branch}"),
        ],
    )
    .ok()
    .and_then(|output| output.split_whitespace().next().map(ToOwned::to_owned))
}

fn fingerprint_preview(
    repository: &Path,
    request: &SyncPreviewRequest,
    status: &GitStatus,
    visibility: RepositoryVisibility,
    generated_at: u64,
) -> Result<String> {
    let mut hash = Sha256::new();
    hash.update(
        serde_json::to_vec(&(request, status, visibility, generated_at))
            .map_err(|error| MaaError::new(error.to_string()))?,
    );
    for path in sync_paths(repository) {
        fingerprint_path(&path, &mut hash)?;
    }
    Ok(format!("sync-{}", hex_digest(hash.finalize().as_slice())))
}

fn fingerprint_path(path: &Path, hash: &mut Sha256) -> Result<()> {
    hash.update(path.to_string_lossy().as_bytes());
    let Ok(metadata) = fs::symlink_metadata(path) else {
        hash.update(b"missing");
        return Ok(());
    };
    if metadata.file_type().is_symlink() {
        hash.update(b"symlink");
        hash.update(fs::read_link(path)?.to_string_lossy().as_bytes());
    } else if metadata.is_file() {
        hash.update(b"file");
        hash.update(fs::read(path)?);
    } else if metadata.is_dir() {
        hash.update(b"directory");
        let mut entries = fs::read_dir(path)?.flatten().collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            fingerprint_path(&entry.path(), hash)?;
        }
    }
    Ok(())
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn create_pull_backup(home: &Path, operation_id: &str) -> Result<String> {
    let root = home.join(".my-agent-assets");
    let id = format!("sync-pull-{operation_id}");
    let backup = root.join("backups/local").join(&id);
    fs::create_dir_all(&backup)?;
    let mut paths = Vec::new();
    for relative in SYNC_PATHS {
        let source = root.join(relative);
        if fs::symlink_metadata(&source).is_ok() {
            let destination = backup.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            copy_any(&source, &destination)?;
            paths.push(relative.to_string());
        }
    }
    fs::write(
        backup.join("manifest.yaml"),
        format!(
            "schemaVersion: 1\noperation: sync-pull\nruntimePaths:\n{}\n",
            paths
                .iter()
                .map(|path| format!("  - {path}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    )?;
    Ok(id)
}

fn sync_paths(repository: &Path) -> Vec<PathBuf> {
    SYNC_PATHS
        .iter()
        .map(|relative| repository.join(relative))
        .collect()
}

fn create_sync_commit(repository: &Path, index: &Path, parent: &str) -> Result<String> {
    let index_value = index.to_string_lossy().to_string();
    if parent.is_empty() {
        run_git_with_index_checked(
            repository,
            &index_value,
            &["read-tree", "--empty"],
            "failed to initialize the empty temporary sync index",
        )?;
    } else {
        run_git_with_index_checked(
            repository,
            &index_value,
            &["read-tree", parent],
            "failed to initialize the temporary sync index",
        )?;
    }
    for relative in SYNC_PATHS {
        if repository.join(relative).exists() {
            run_git_with_index_checked(
                repository,
                &index_value,
                &["add", "--", relative],
                "failed to stage the canonical sync whitelist",
            )?;
        }
    }
    let tree = git_stdout_with_index(repository, &index_value, &["write-tree"])?;
    let mut args = vec![
        "-c",
        "user.name=My Agent Assets",
        "-c",
        "user.email=my-agent-assets@localhost",
        "commit-tree",
        tree.as_str(),
    ];
    if !parent.is_empty() {
        args.extend(["-p", parent]);
    }
    args.extend(["-m", "chore: sync agent assets"]);
    git_stdout_with_index(repository, &index_value, &args)
}

fn run_git_with_index_checked(
    repository: &Path,
    index: &str,
    args: &[&str],
    message: &str,
) -> Result<()> {
    let output = Command::new("git")
        .current_dir(repository)
        .env("GIT_INDEX_FILE", index)
        .args(args)
        .output()
        .map_err(|_| MaaError::new(message))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(MaaError::new(message))
    }
}

fn git_stdout_with_index(repository: &Path, index: &str, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repository)
        .env("GIT_INDEX_FILE", index)
        .args(args)
        .output()
        .map_err(|_| MaaError::new("Git is unavailable"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(MaaError::new("Git command failed"))
    }
}

fn validate_preview_time(generated_at: u64) -> Result<()> {
    let now = epoch_seconds();
    if generated_at > now.saturating_add(5)
        || now.saturating_sub(generated_at) > PREVIEW_TTL_SECONDS
    {
        return Err(MaaError::new(
            "Git sync preview expired; generate a new preview",
        ));
    }
    Ok(())
}

fn run_git(repository: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .current_dir(repository)
        .args(args)
        .output()
        .unwrap_or_else(|_| failed_output())
}

fn run_git_checked(repository: &Path, args: &[&str], message: &str) -> Result<()> {
    if run_git(repository, args).status.success() {
        Ok(())
    } else {
        Err(MaaError::new(message))
    }
}

fn git_stdout(repository: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repository)
        .args(args)
        .output()
        .map_err(|_| MaaError::new("Git is unavailable"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(MaaError::new("Git command failed"))
    }
}

fn git_success(repository: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .current_dir(repository)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(unix)]
fn failed_output() -> Output {
    use std::os::unix::process::ExitStatusExt;
    Output {
        status: std::process::ExitStatus::from_raw(1),
        stdout: Vec::new(),
        stderr: Vec::new(),
    }
}

#[cfg(windows)]
fn failed_output() -> Output {
    use std::os::windows::process::ExitStatusExt;
    Output {
        status: std::process::ExitStatus::from_raw(1),
        stdout: Vec::new(),
        stderr: Vec::new(),
    }
}

fn parse_github_repository(remote_url: &str) -> Result<String> {
    let normalized = if let Some(value) = remote_url.strip_prefix("git@github.com:") {
        value
    } else if let Some(value) = remote_url.strip_prefix("ssh://git@github.com/") {
        value
    } else if let Some(value) = remote_url.strip_prefix("https://github.com/") {
        if value.contains('@') {
            return Err(MaaError::new(
                "credential-bearing Git remote URLs are not accepted",
            ));
        }
        value
    } else {
        return Err(MaaError::new(
            "Push is supported only for a verifiable GitHub repository",
        ));
    };
    let repository = normalized.trim_end_matches('/').trim_end_matches(".git");
    let mut parts = repository.split('/');
    let owner = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || !valid_repository_component(owner)
        || !valid_repository_component(name)
    {
        return Err(MaaError::new("invalid GitHub repository identity"));
    }
    Ok(format!("{owner}/{name}"))
}

fn valid_repository_component(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_remote_name(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_branch_name(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with(['/', '.'])
        && !value.contains("..")
        && !value.contains("@{")
        && !value.contains('\\')
        && !value.contains("//")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/'))
}

fn direction_name(direction: SyncDirection) -> &'static str {
    match direction {
        SyncDirection::Pull => "pull",
        SyncDirection::Push => "push",
    }
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
    use crate::operation::{crash_test, recover_incomplete};
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct PrivateVerifier;
    impl VisibilityVerifier for PrivateVerifier {
        fn verify(&self, _remote_url: &str) -> Result<(RepositoryVisibility, String)> {
            Ok((RepositoryVisibility::Private, "test/private".into()))
        }
    }

    struct PublicVerifier;
    impl VisibilityVerifier for PublicVerifier {
        fn verify(&self, _remote_url: &str) -> Result<(RepositoryVisibility, String)> {
            Ok((RepositoryVisibility::Public, "test/public".into()))
        }
    }

    fn test_home(label: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!(
            "maa-git-sync-{label}-{}-{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&home).unwrap();
        home
    }

    fn run(cwd: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(cwd)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn setup(label: &str) -> (PathBuf, PathBuf) {
        let home = test_home(label);
        let remote = home.join("remote.git");
        let repository = home.join(".my-agent-assets");
        run(&home, &["init", "--bare", remote.to_str().unwrap()]);
        run(
            &home,
            &[
                "clone",
                remote.to_str().unwrap(),
                repository.to_str().unwrap(),
            ],
        );
        run(
            &repository,
            &["config", "user.email", "test@example.invalid"],
        );
        run(&repository, &["config", "user.name", "Test"]);
        fs::create_dir_all(repository.join("assets/skills/review")).unwrap();
        fs::write(repository.join("assets/skills/review/SKILL.md"), "# Review").unwrap();
        fs::write(
            repository.join("assets.yaml"),
            "schemaVersion: 1\nassets: {}\n",
        )
        .unwrap();
        fs::write(
            repository.join(".gitignore"),
            "config.yaml\ntargets.yaml\nmounts.yaml\nbackups/local/\noperations/\nlocks/\n",
        )
        .unwrap();
        run(&repository, &["add", "assets", "assets.yaml", ".gitignore"]);
        run(&repository, &["commit", "-m", "initial"]);
        run(&repository, &["push", "-u", "origin", "HEAD"]);
        (home, remote)
    }

    #[test]
    fn github_remote_parser_accepts_supported_urls_and_rejects_credentials() {
        for (url, expected) in [
            (
                "git@github.com:fightingPig/assets.git",
                "fightingPig/assets",
            ),
            (
                "ssh://git@github.com/fightingPig/assets.git",
                "fightingPig/assets",
            ),
            (
                "https://github.com/fightingPig/assets.git",
                "fightingPig/assets",
            ),
        ] {
            assert_eq!(parse_github_repository(url).unwrap(), expected);
        }
        assert!(parse_github_repository("https://token@github.com/a/b.git").is_err());
        assert!(parse_github_repository("/tmp/remote.git").is_err());
        assert!(valid_remote_name("origin"));
        assert!(!valid_remote_name("--upload-pack"));
        assert!(valid_branch_name("feature/assets"));
        assert!(!valid_branch_name("-unsafe"));
        assert!(!valid_branch_name("feature..unsafe"));
    }

    #[test]
    fn push_stages_only_whitelist_and_rejects_public_visibility() {
        let (home, _) = setup("whitelist");
        let repository = home.join(".my-agent-assets");
        fs::write(
            repository.join("assets/skills/review/SKILL.md"),
            "# Updated",
        )
        .unwrap();
        fs::write(repository.join("notes.txt"), "blocked").unwrap();
        let blocked = preview_sync_with(
            &home,
            &SyncPreviewRequest {
                direction: SyncDirection::Push,
            },
            &PrivateVerifier,
        )
        .unwrap();
        assert!(!blocked.can_apply);
        assert_eq!(blocked.status.blocked_changes, vec!["notes.txt"]);
        fs::remove_file(repository.join("notes.txt")).unwrap();

        let public = preview_sync_with(
            &home,
            &SyncPreviewRequest {
                direction: SyncDirection::Push,
            },
            &PublicVerifier,
        )
        .unwrap();
        assert!(!public.can_apply);
        assert_eq!(public.repository_visibility, RepositoryVisibility::Public);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn private_push_commits_whitelist_and_pushes_without_force() {
        let (home, remote) = setup("push");
        let repository = home.join(".my-agent-assets");
        fs::write(
            repository.join("assets/skills/review/SKILL.md"),
            "# Updated",
        )
        .unwrap();
        let request = SyncPreviewRequest {
            direction: SyncDirection::Push,
        };
        let preview = preview_sync_with(&home, &request, &PrivateVerifier).unwrap();
        assert!(preview.can_apply, "{:?}", preview.warnings);
        let result = apply_sync_with(
            &home,
            &SyncApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
            &PrivateVerifier,
        )
        .unwrap();
        assert!(result.committed);
        assert!(result.pushed);
        let remote_text =
            git_stdout(&remote, &["show", "HEAD:assets/skills/review/SKILL.md"]).unwrap();
        assert_eq!(remote_text, "# Updated");
        assert_eq!(
            git_stdout(&repository, &["status", "--porcelain"]).unwrap(),
            ""
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn initial_private_push_supports_an_unborn_main_branch() {
        let home = test_home("initial-push");
        let remote = home.join("remote.git");
        let repository = home.join(".my-agent-assets");
        run(&home, &["init", "--bare", remote.to_str().unwrap()]);
        fs::create_dir_all(repository.join("assets/skills/review")).unwrap();
        fs::create_dir_all(repository.join("backups/portable")).unwrap();
        run(&home, &["init", "-b", "main", repository.to_str().unwrap()]);
        run(
            &repository,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        fs::write(repository.join("assets/skills/review/SKILL.md"), "# Review").unwrap();
        fs::write(
            repository.join("assets.yaml"),
            "schemaVersion: 1\nassets: {}\n",
        )
        .unwrap();
        fs::write(
            repository.join(".gitignore"),
            "backups/local/\ncache/\nlocks/\noperations/\n",
        )
        .unwrap();

        let request = SyncPreviewRequest {
            direction: SyncDirection::Push,
        };
        let preview = preview_sync_with(&home, &request, &PrivateVerifier).unwrap();
        assert!(preview.can_apply, "{:?}", preview.warnings);
        let result = apply_sync_with(
            &home,
            &SyncApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
            &PrivateVerifier,
        )
        .unwrap();
        assert!(result.committed && result.pushed);
        assert_eq!(
            git_stdout(&repository, &["status", "--porcelain"]).unwrap(),
            ""
        );
        assert_eq!(
            git_stdout(&remote, &["show", "main:assets/skills/review/SKILL.md"]).unwrap(),
            "# Review"
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn push_recovers_when_process_crashes_after_sync_commit_step() {
        let (home, remote) = setup("push-crash-recovery");
        let repository = home.join(".my-agent-assets");
        fs::write(
            repository.join("assets/skills/review/SKILL.md"),
            "# Updated",
        )
        .unwrap();
        let old_head = git_stdout(&repository, &["rev-parse", "HEAD"]).unwrap();
        let request = SyncPreviewRequest {
            direction: SyncDirection::Push,
        };
        let preview = preview_sync_with(&home, &request, &PrivateVerifier).unwrap();
        let remote_ref = format!("{}:assets/skills/review/SKILL.md", preview.status.branch);
        let crash = crash_test::crash_after_step("git-sync", "sync_commit_created");

        let result = catch_unwind(AssertUnwindSafe(|| {
            apply_sync_with(
                &home,
                &SyncApplyRequest {
                    preview_id: preview.preview_id,
                    preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                    request,
                },
                &PrivateVerifier,
            )
        }));
        assert!(result.is_err());
        drop(crash);

        let crash_head = git_stdout(&repository, &["rev-parse", "HEAD"]).unwrap();
        assert_ne!(crash_head, old_head);
        assert_eq!(
            git_stdout(&remote, &["show", &remote_ref]).unwrap(),
            "# Review"
        );

        let report = recover_incomplete(&home).unwrap();
        assert!(report.attempted);
        assert!(!report.writes_blocked);
        assert!(report.attempts[0].recovered);
        assert_eq!(
            git_stdout(&repository, &["rev-parse", "HEAD"]).unwrap(),
            old_head
        );
        assert!(git_stdout(&repository, &["status", "--porcelain"])
            .unwrap()
            .contains("assets/skills/review/SKILL.md"));
        assert_eq!(
            git_stdout(&remote, &["show", &remote_ref]).unwrap(),
            "# Review"
        );
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(unix)]
    #[test]
    fn failed_push_restores_branch_ref_without_resetting_worktree() {
        use std::os::unix::fs::PermissionsExt;

        let (home, remote) = setup("push-failure");
        let repository = home.join(".my-agent-assets");
        fs::write(
            repository.join("assets/skills/review/SKILL.md"),
            "# Updated",
        )
        .unwrap();
        let old_head = git_stdout(&repository, &["rev-parse", "HEAD"]).unwrap();
        let request = SyncPreviewRequest {
            direction: SyncDirection::Push,
        };
        let preview = preview_sync_with(&home, &request, &PrivateVerifier).unwrap();
        let hook = remote.join("hooks/pre-receive");
        fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();

        let error = apply_sync_with(
            &home,
            &SyncApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
            &PrivateVerifier,
        )
        .unwrap_err();
        assert!(error.to_string().contains("Push failed"));
        assert_eq!(
            git_stdout(&repository, &["rev-parse", "HEAD"]).unwrap(),
            old_head
        );
        assert!(git_stdout(&repository, &["status", "--porcelain"])
            .unwrap()
            .contains("assets/skills/review/SKILL.md"));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn pull_requires_clean_worktree_and_creates_local_backup() {
        let (home, remote) = setup("pull");
        let repository = home.join(".my-agent-assets");
        let other = home.join("other");
        run(
            &home,
            &["clone", remote.to_str().unwrap(), other.to_str().unwrap()],
        );
        run(&other, &["config", "user.email", "test@example.invalid"]);
        run(&other, &["config", "user.name", "Test"]);
        fs::write(other.join("assets/skills/review/SKILL.md"), "# Remote").unwrap();
        run(&other, &["add", "assets"]);
        run(&other, &["commit", "-m", "remote"]);
        run(&other, &["push"]);

        fs::write(repository.join("dirty.txt"), "dirty").unwrap();
        let blocked = preview_sync_with(
            &home,
            &SyncPreviewRequest {
                direction: SyncDirection::Pull,
            },
            &PrivateVerifier,
        )
        .unwrap();
        assert!(!blocked.can_apply);
        fs::remove_file(repository.join("dirty.txt")).unwrap();

        let request = SyncPreviewRequest {
            direction: SyncDirection::Pull,
        };
        let preview = preview_sync_with(&home, &request, &PrivateVerifier).unwrap();
        assert!(preview.can_apply, "{:?}", preview.warnings);
        let result = apply_sync_with(
            &home,
            &SyncApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
            &PrivateVerifier,
        )
        .unwrap();
        assert!(result.pulled);
        assert!(result.backup_id.is_some());
        assert_eq!(
            fs::read_to_string(repository.join("assets/skills/review/SKILL.md")).unwrap(),
            "# Remote"
        );
        let _ = fs::remove_dir_all(home);
    }
}
