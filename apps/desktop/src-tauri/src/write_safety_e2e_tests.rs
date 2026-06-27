use super::apply::{import_apply_for_home, mount_apply_for_home, restore_apply_for_home};
use super::contracts::{
    ApplyMode, ImportApplyInput, MountApplyInput, MountTarget, RestoreApplyInput, RuntimeScope,
    ScanScope, SettingsSaveInput, SyncApplyInput, SyncDirection,
};
use super::preview::{import_preview_id, mount_preview_id, restore_preview_id, sync_preview_id};
use super::read_only::{git_status_for_home, settings_for_home};
use super::settings::{settings_load_for_home, settings_save_for_home};
use super::sync_apply::sync_apply_for_home;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn fake_home_write_workflow_stays_isolated_and_sync_plan_only_does_not_mutate_remote() {
    let home = TempHome::new();
    home.write(".claude/commands/deploy.md", "# New Deploy");
    home.write(".claude/skills/review.md", "# Review");
    home.write(".my-agent-assets/assets/commands/deploy.md", "# Old Deploy");

    let scope = ScanScope::User;
    let asset_ids = vec!["command:deploy".to_string(), "skill:review".to_string()];
    let import = import_apply_for_home(
        home.path(),
        ImportApplyInput {
            preview_id: import_preview_id(&scope, &asset_ids, &[]),
            mode: ApplyMode::Apply,
            scope,
            asset_ids,
            conflict_resolutions: vec![],
            backup_before_apply: true,
        },
    );
    assert!(import.ok, "{:?}", import.errors);
    let backup_id = import.backup.expect("replacement import should back up").id;

    let target = MountTarget {
        scope: RuntimeScope::Project,
        runtime_path: "~/workspace/project-a/.claude/skills/review.md".into(),
        project_path: Some("~/workspace/project-a".into()),
    };
    let mount = mount_apply_for_home(
        home.path(),
        MountApplyInput {
            preview_id: mount_preview_id("skill:review", &target),
            mode: ApplyMode::Apply,
            asset_id: "skill:review".into(),
            target,
            backup_before_apply: true,
        },
    );
    assert!(mount.ok, "{:?}", mount.errors);

    let restore = restore_apply_for_home(
        home.path(),
        RestoreApplyInput {
            preview_id: restore_preview_id(&backup_id),
            mode: ApplyMode::Apply,
            backup_id,
            backup_before_restore: true,
        },
    );
    assert!(restore.ok, "{:?}", restore.errors);
    assert_eq!(
        home.read(".my-agent-assets/assets/commands/deploy.md"),
        "# Old Deploy"
    );

    let mut settings = settings_for_home(Some(home.path()));
    settings.max_depth = 9;
    settings_save_for_home(
        home.path(),
        SettingsSaveInput {
            settings: settings.clone(),
        },
    )
    .expect("settings should save inside fake HOME");
    assert_eq!(settings_load_for_home(home.path()), settings);

    setup_git_with_ahead_commit(home.path());
    let status = git_status_for_home(home.path());
    assert_eq!(status.ahead, 1);
    let remote_before = git_output(&home.asset_center(), &["ls-remote", "origin", "HEAD"]);
    let sync = sync_apply_for_home(
        home.path(),
        SyncApplyInput {
            preview_id: sync_preview_id(&SyncDirection::Push, &status),
            mode: ApplyMode::PlanOnly,
            direction: SyncDirection::Push,
        },
    );
    assert!(sync.ok, "{:?}", sync.errors);
    assert_eq!(
        git_output(&home.asset_center(), &["ls-remote", "origin", "HEAD"]),
        remote_before
    );
    assert_eq!(git_status_for_home(home.path()).ahead, 1);
}

struct TempHome {
    path: PathBuf,
}

impl TempHome {
    fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "my-agent-assets-write-e2e-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&path).expect("fake HOME should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn asset_center(&self) -> PathBuf {
        self.path.join(".my-agent-assets")
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.path.join(relative);
        fs::create_dir_all(path.parent().expect("path should have parent"))
            .expect("parent should be created");
        fs::write(path, contents).expect("fixture should be written");
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path.join(relative)).expect("fixture should be readable")
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn setup_git_with_ahead_commit(home: &Path) {
    let repository = home.join(".my-agent-assets");
    let remote = home.join("remote.git");
    run_git(home, &["init", "--bare", remote.to_str().unwrap()]);
    run_git(&repository, &["init"]);
    run_git(&repository, &["config", "user.email", "test@example.com"]);
    run_git(&repository, &["config", "user.name", "Safety Test"]);
    run_git(&repository, &["add", "."]);
    run_git(&repository, &["commit", "-m", "initial"]);
    run_git(
        &repository,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    run_git(&repository, &["push", "-u", "origin", "HEAD"]);
    fs::write(repository.join("ahead.txt"), "ahead").expect("ahead file should be written");
    run_git(&repository, &["add", "ahead.txt"]);
    run_git(&repository, &["commit", "-m", "ahead"]);
}

fn run_git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("git should run");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_output(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("git should run");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).into_owned()
}
