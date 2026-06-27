use super::contracts::{ApplyMode, ApplyStepStatus, SyncApplyInput, SyncDirection};
use super::preview::sync_preview_id;
use super::read_only::git_status_for_home;
use super::sync_apply::sync_apply_for_home;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempGitHome {
    path: PathBuf,
}

impl TempGitHome {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "my-agent-assets-sync-{}-{}-{}",
            name,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&path).expect("temp home should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn repo(&self) -> PathBuf {
        self.path.join(".my-agent-assets")
    }
}

impl Drop for TempGitHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn sync_apply_plan_only_does_not_push_commits() {
    let home = setup_home_with_ahead_commit("plan-only");
    let status = git_status_for_home(home.path());
    assert_eq!(status.ahead, 1);
    let result = sync_apply_for_home(
        home.path(),
        SyncApplyInput {
            preview_id: sync_preview_id(&SyncDirection::Push, &status),
            mode: ApplyMode::PlanOnly,
            direction: SyncDirection::Push,
        },
    );

    assert!(result.ok, "{:?}", result.errors);
    assert_eq!(result.steps[0].status, ApplyStepStatus::Skipped);
    assert_eq!(git_status_for_home(home.path()).ahead, 1);
    assert_eq!(git_output(&home.repo(), &["status", "--porcelain"]), "");
}

#[test]
fn sync_apply_pushes_to_upstream_repository() {
    let home = setup_home_with_ahead_commit("push");
    let status = git_status_for_home(home.path());
    assert_eq!(status.ahead, 1);
    let result = sync_apply_for_home(
        home.path(),
        SyncApplyInput {
            preview_id: sync_preview_id(&SyncDirection::Push, &status),
            mode: ApplyMode::Apply,
            direction: SyncDirection::Push,
        },
    );

    assert!(result.ok, "{:?}", result.errors);
    assert_eq!(result.steps[0].status, ApplyStepStatus::Success);
    assert_eq!(git_status_for_home(home.path()).ahead, 0);
}

#[test]
fn sync_apply_rejects_mismatched_preview_id_before_running_git() {
    let home = setup_home_with_ahead_commit("mismatch");
    let result = sync_apply_for_home(
        home.path(),
        SyncApplyInput {
            preview_id: "preview:sync:tampered".into(),
            mode: ApplyMode::Apply,
            direction: SyncDirection::Push,
        },
    );

    assert!(!result.ok);
    assert!(result.errors[0].contains("Preview ID does not match"));
    assert_eq!(git_status_for_home(home.path()).ahead, 1);
}

#[test]
fn sync_apply_rejects_dirty_worktree() {
    let home = setup_home_with_ahead_commit("dirty");
    fs::write(home.repo().join("dirty.txt"), "dirty").expect("dirty file should be written");
    let status = git_status_for_home(home.path());
    assert!(!status.clean);

    let result = sync_apply_for_home(
        home.path(),
        SyncApplyInput {
            preview_id: sync_preview_id(&SyncDirection::Push, &status),
            mode: ApplyMode::Apply,
            direction: SyncDirection::Push,
        },
    );

    assert!(!result.ok);
    assert!(result.errors[0].contains("not applyable"));
}

#[test]
fn sync_apply_rejects_symlinked_repository_before_git_execution() {
    let home = TempGitHome::new("symlink-repository");
    let outside = setup_home_with_ahead_commit("symlink-repository-outside");
    create_test_directory_symlink(&outside.repo(), &home.repo());
    let status = git_status_for_home(home.path());

    let result = sync_apply_for_home(
        home.path(),
        SyncApplyInput {
            preview_id: sync_preview_id(&SyncDirection::Push, &status),
            mode: ApplyMode::PlanOnly,
            direction: SyncDirection::Push,
        },
    );

    assert!(!result.ok);
    assert!(result.errors[0].contains("Symlink traversal"));
    assert_eq!(git_status_for_home(outside.path()).ahead, 1);
}

fn setup_home_with_ahead_commit(name: &str) -> TempGitHome {
    let home = TempGitHome::new(name);
    let remote = home.path().join("remote.git");
    run_git(home.path(), &["init", "--bare", remote.to_str().unwrap()]);
    run_git(
        home.path(),
        &[
            "clone",
            remote.to_str().unwrap(),
            home.repo().to_str().unwrap(),
        ],
    );
    run_git(&home.repo(), &["config", "user.email", "test@example.com"]);
    run_git(
        &home.repo(),
        &["config", "user.name", "My Agent Assets Test"],
    );
    fs::write(home.repo().join("README.md"), "initial").expect("README should be written");
    run_git(&home.repo(), &["add", "README.md"]);
    run_git(&home.repo(), &["commit", "-m", "initial"]);
    run_git(&home.repo(), &["push", "-u", "origin", "HEAD"]);
    fs::write(home.repo().join("asset.md"), "asset").expect("asset should be written");
    run_git(&home.repo(), &["add", "asset.md"]);
    run_git(&home.repo(), &["commit", "-m", "asset"]);
    home
}

fn run_git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "git {} failed\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_output(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("git command should run");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[cfg(unix)]
fn create_test_directory_symlink(source: &Path, destination: &Path) {
    std::os::unix::fs::symlink(source, destination).expect("directory symlink should be created");
}

#[cfg(windows)]
fn create_test_directory_symlink(source: &Path, destination: &Path) {
    std::os::windows::fs::symlink_dir(source, destination)
        .expect("directory symlink should be created");
}
