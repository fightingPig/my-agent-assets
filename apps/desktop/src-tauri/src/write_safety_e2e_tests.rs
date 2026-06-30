use super::apply::{import_apply_for_home, mount_apply_for_home};
use super::contracts::{
    ApplyMode, ConflictResolution, ConflictResolutionChoice, ImportApplyInput, MountApplyInput,
    MountTarget, RuntimeScope, ScanScope, SettingsSaveInput, SyncApplyInput, SyncDirection,
};
use super::preview::{import_preview_id, mount_preview_id, sync_preview_id};
use super::read_only::{git_status_for_home, settings_for_home};
use super::settings::{settings_load_for_home, settings_save_for_home};
use super::sync_apply::sync_apply_for_home;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_HOME_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn fake_home_write_workflow_stays_isolated_and_sync_plan_only_does_not_mutate_remote() {
    let home = TempHome::new();
    let non_target_home = TempHome::new();
    non_target_home.write("real-home-sentinel.txt", "must remain unchanged");
    let non_target_before = snapshot_tree(non_target_home.path());

    home.write(".claude/commands/deploy.md", "# New Deploy");
    home.write(".claude/skills/review.md", "# Review");
    home.write(".my-agent-assets/assets/commands/deploy.md", "# Old Deploy");

    let scope = ScanScope::User;
    let asset_ids = vec!["command:deploy".to_string(), "skill:review".to_string()];
    let conflict_resolutions = vec![ConflictResolutionChoice {
        conflict_id: "command:deploy".into(),
        resolution: ConflictResolution::Overwrite,
        rename_to: None,
    }];
    let import = import_apply_for_home(
        home.path(),
        ImportApplyInput {
            preview_id: import_preview_id(&scope, &asset_ids, &conflict_resolutions),
            mode: ApplyMode::Apply,
            scope,
            asset_ids,
            conflict_resolutions,
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

    assert_eq!(
        home.read(".my-agent-assets/assets/commands/deploy.md"),
        "# New Deploy"
    );
    assert!(home
        .asset_center()
        .join("backups")
        .join(backup_id)
        .join("manifest.json")
        .is_file());

    let mut settings = settings_for_home(Some(home.path()));
    settings.max_depth = 9;
    settings_save_for_home(
        home.path(),
        SettingsSaveInput {
            settings: settings.clone(),
        },
    )
    .expect("settings should save inside fake HOME");
    assert_eq!(
        settings_load_for_home(home.path()).expect("saved settings should load"),
        settings
    );

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
    assert_eq!(snapshot_tree(non_target_home.path()), non_target_before);
}

#[test]
fn fake_home_plan_only_workflow_leaves_entire_home_and_non_target_home_unchanged() {
    let home = TempHome::new();
    let non_target_home = TempHome::new();
    non_target_home.write("real-home-sentinel.txt", "must remain unchanged");

    home.write(".claude/commands/deploy.md", "# Deploy");
    home.write(".my-agent-assets/assets/skills/review.md", "# Review");
    home.write(
        ".my-agent-assets/assets/mcps/PostgreSQL.json",
        r#"{"command":"postgres"}"#,
    );
    setup_git_with_ahead_commit(home.path());

    let home_before = snapshot_tree(home.path());
    let non_target_before = snapshot_tree(non_target_home.path());

    let import_scope = ScanScope::User;
    let import_asset_ids = vec!["command:deploy".to_string()];
    let import = import_apply_for_home(
        home.path(),
        ImportApplyInput {
            preview_id: import_preview_id(&import_scope, &import_asset_ids, &[]),
            mode: ApplyMode::PlanOnly,
            scope: import_scope,
            asset_ids: import_asset_ids,
            conflict_resolutions: vec![],
            backup_before_apply: true,
        },
    );
    assert!(import.ok, "{:?}", import.errors);

    let skill_target = MountTarget {
        scope: RuntimeScope::Project,
        runtime_path: "~/workspace/project-a/.claude/skills/review.md".into(),
        project_path: Some("~/workspace/project-a".into()),
    };
    let mount = mount_apply_for_home(
        home.path(),
        MountApplyInput {
            preview_id: mount_preview_id("skill:review", &skill_target),
            mode: ApplyMode::PlanOnly,
            asset_id: "skill:review".into(),
            target: skill_target,
            backup_before_apply: true,
        },
    );
    assert!(mount.ok, "{:?}", mount.errors);

    let mcp_target = MountTarget {
        scope: RuntimeScope::Project,
        runtime_path: "~/workspace/project-a/.mcp.json".into(),
        project_path: Some("~/workspace/project-a".into()),
    };
    let mcp_mount = mount_apply_for_home(
        home.path(),
        MountApplyInput {
            preview_id: mount_preview_id("mcp:PostgreSQL", &mcp_target),
            mode: ApplyMode::PlanOnly,
            asset_id: "mcp:PostgreSQL".into(),
            target: mcp_target,
            backup_before_apply: true,
        },
    );
    assert!(mcp_mount.ok, "{:?}", mcp_mount.errors);

    let status = git_status_for_home(home.path());
    let sync = sync_apply_for_home(
        home.path(),
        SyncApplyInput {
            preview_id: sync_preview_id(&SyncDirection::Push, &status),
            mode: ApplyMode::PlanOnly,
            direction: SyncDirection::Push,
        },
    );
    assert!(sync.ok, "{:?}", sync.errors);

    assert_eq!(snapshot_tree(home.path()), home_before);
    assert_eq!(snapshot_tree(non_target_home.path()), non_target_before);
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
            "my-agent-assets-write-e2e-{}-{}-{}",
            std::process::id(),
            nanos,
            TEMP_HOME_SEQUENCE.fetch_add(1, Ordering::Relaxed)
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

fn snapshot_tree(root: &Path) -> Vec<(PathBuf, String, Vec<u8>)> {
    fn visit(root: &Path, current: &Path, snapshot: &mut Vec<(PathBuf, String, Vec<u8>)>) {
        let mut entries = fs::read_dir(current)
            .expect("snapshot directory should be readable")
            .map(|entry| entry.expect("snapshot entry should be readable"))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .expect("snapshot path should stay below root")
                .to_path_buf();
            let metadata =
                fs::symlink_metadata(&path).expect("snapshot metadata should be readable");
            if metadata.file_type().is_symlink() {
                snapshot.push((
                    relative,
                    "symlink".into(),
                    fs::read_link(&path)
                        .expect("snapshot symlink should be readable")
                        .as_os_str()
                        .as_encoded_bytes()
                        .to_vec(),
                ));
            } else if metadata.is_dir() {
                snapshot.push((relative, "directory".into(), vec![]));
                visit(root, &path, snapshot);
            } else {
                snapshot.push((
                    relative,
                    "file".into(),
                    fs::read(&path).expect("snapshot file should be readable"),
                ));
            }
        }
    }

    let mut snapshot = vec![];
    visit(root, root, &mut snapshot);
    snapshot
}
