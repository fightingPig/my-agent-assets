use super::contracts::{AssetStatus, AssetType, ListAssetsInput};
use super::read_only::{
    git_status_for_home, list_assets_for_home, list_backups_for_home, list_projects_for_home,
    settings_for_home,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempHome {
    path: PathBuf,
}

impl TempHome {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "my-agent-assets-desktop-{}-{}-{}",
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

    fn write(&self, relative: &str, content: &str) {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directory should be created");
        }
        fs::write(path, content).expect("file should be written");
    }

    fn mkdir(&self, relative: &str) {
        fs::create_dir_all(self.path.join(relative)).expect("directory should be created");
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn all_assets_input() -> ListAssetsInput {
    ListAssetsInput { asset_type: None }
}

#[test]
fn settings_load_returns_defaults_without_creating_files() {
    let home = TempHome::new("settings");
    let settings = settings_for_home(Some(home.path()));

    assert_eq!(
        settings.asset_center_path,
        home.path().join(".my-agent-assets").to_string_lossy()
    );
    assert_eq!(settings.scan_roots.len(), 3);
    assert_eq!(settings.max_depth, 5);
    assert!(settings.backup_before_apply);
    assert!(settings.plan_only_by_default);
    assert_eq!(settings.git_default_branch, "main");
    assert_eq!(settings.git_remote, "origin");
    assert!(!home.path().join(".my-agent-assets").exists());
}

#[test]
fn list_assets_returns_empty_when_asset_center_is_missing() {
    let home = TempHome::new("missing-assets");
    assert!(list_assets_for_home(home.path(), all_assets_input()).is_empty());
    assert!(!home.path().join(".my-agent-assets").exists());
}

#[test]
fn list_backups_returns_empty_when_backup_root_is_missing() {
    let home = TempHome::new("missing-backups");
    assert!(list_backups_for_home(home.path()).is_empty());
    assert!(!home.path().join(".my-agent-assets").exists());
}

#[test]
fn list_backups_reads_manifest_summaries_without_backup_contents() {
    let home = TempHome::new("backups");
    home.write(
        ".my-agent-assets/backups/import-20260627/manifest.json",
        r#"{
  "id": "import-20260627",
  "label": "Import apply backup",
  "createdAt": "2026-06-27T10:00:00Z",
  "runtimeRoot": "/tmp/fake-home",
  "entries": [
    { "originalPath": "~/.claude/skills/review", "backupPath": "files/review", "kind": "file", "sizeBytes": 120 },
    { "originalPath": "~/.claude/commands/deploy.md", "backupPath": "files/deploy.md", "kind": "file", "sizeBytes": 80 }
  ]
}"#,
    );
    home.write(
        ".my-agent-assets/backups/mount-20260628/manifest.json",
        r#"{
  "id": "mount-20260628",
  "label": "Mount apply backup",
  "createdAt": "2026-06-28T09:00:00Z",
  "runtimeRoot": "/tmp/fake-home",
  "entries": [
    { "originalPath": "~/workspace/project-a/.mcp.json", "backupPath": "files/.mcp.json", "kind": "file", "sizeBytes": 42 }
  ]
}"#,
    );
    home.write(".my-agent-assets/backups/broken/manifest.json", "{");

    let backups = list_backups_for_home(home.path());

    assert_eq!(backups.len(), 2);
    assert_eq!(backups[0].id, "mount-20260628");
    assert_eq!(backups[0].entry_count, 1);
    assert_eq!(backups[0].size_bytes, 42);
    assert_eq!(backups[1].id, "import-20260627");
    assert_eq!(backups[1].entry_count, 2);
    assert_eq!(backups[1].size_bytes, 200);
}

#[test]
fn list_assets_reads_skills_commands_and_mcp_json_safely() {
    let home = TempHome::new("assets");
    home.write(
        ".my-agent-assets/assets/skills/review/SKILL.md",
        "# Review\n\nCheck code changes.",
    );
    home.write(
        ".my-agent-assets/assets/skills/api-design.md",
        "# API Design",
    );
    home.write(
        ".my-agent-assets/assets/commands/deploy-prod.md",
        "# Deploy",
    );
    home.write(
        ".my-agent-assets/assets/mcps/postgres.json",
        r#"{"command":"postgres-mcp"}"#,
    );
    home.write(".my-agent-assets/assets/mcps/broken.json", "{");

    let assets = list_assets_for_home(home.path(), all_assets_input());
    assert_eq!(assets.len(), 5);
    assert!(assets.iter().any(|asset| asset.id == "skill:review"));
    assert!(assets.iter().any(|asset| asset.id == "skill:api-design"));
    assert!(assets.iter().any(|asset| asset.id == "command:deploy-prod"));
    assert!(assets
        .iter()
        .any(|asset| asset.id == "mcp:postgres" && asset.status == AssetStatus::Ready));
    assert!(assets
        .iter()
        .any(|asset| asset.id == "mcp:broken" && asset.status == AssetStatus::Invalid));
}

#[test]
fn list_assets_derives_symlink_and_mcp_mount_targets() {
    let home = TempHome::new("asset-mounts");
    home.write(".my-agent-assets/assets/skills/review.md", "# Review");
    home.write(
        ".my-agent-assets/assets/mcps/PostgreSQL.json",
        r#"{"command":"postgres"}"#,
    );
    let skill_source = home.path().join(".my-agent-assets/assets/skills/review.md");
    let skill_target = home.path().join(".claude/skills/review.md");
    fs::create_dir_all(skill_target.parent().unwrap()).unwrap();
    create_test_file_symlink(&skill_source, &skill_target);
    home.write(
        "workspace/project-a/.mcp.json",
        r#"{"mcpServers":{"PostgreSQL":{"command":"postgres"}}}"#,
    );
    home.write("workspace/project-a/package.json", "{}");

    let assets = list_assets_for_home(home.path(), all_assets_input());
    let skill = assets
        .iter()
        .find(|asset| asset.id == "skill:review")
        .unwrap();
    let mcp = assets
        .iter()
        .find(|asset| asset.id == "mcp:PostgreSQL")
        .unwrap();
    assert_eq!(skill.status, AssetStatus::Mounted);
    assert_eq!(skill.mount_targets, vec![skill_target.to_string_lossy()]);
    assert_eq!(
        mcp.mount_targets,
        vec![home
            .path()
            .join("workspace/project-a/.mcp.json")
            .to_string_lossy()]
    );
}

#[test]
fn list_projects_detects_first_level_project_markers() {
    let home = TempHome::new("projects");
    home.write("workspace/web-app/package.json", "{}");
    home.write(
        "code/rust-app/Cargo.toml",
        "[package]\nname = \"rust-app\"\n",
    );
    home.mkdir("workspace/claude-app/.claude");
    home.write("workspace/claude-app/.claude/skills/review.md", "# Review");
    home.write(
        "workspace/claude-app/.mcp.json",
        r#"{"mcpServers":{"Filesystem":{"command":"fs"}}}"#,
    );
    home.write("workspace/nested/child/package.json", "{}");

    let projects = list_projects_for_home(home.path());

    assert!(projects.iter().any(|project| project.name == "web-app"));
    assert!(projects.iter().any(|project| project.name == "rust-app"));
    assert!(projects.iter().any(|project| project.name == "claude-app"));
    let claude_project = projects
        .iter()
        .find(|project| project.name == "claude-app")
        .unwrap();
    assert_eq!(claude_project.asset_counts.total, 2);
    assert_eq!(claude_project.mounts, vec!["review", "Filesystem"]);
    assert!(!projects.iter().any(|project| project.name == "child"));
}

#[cfg(unix)]
fn create_test_file_symlink(source: &Path, target: &Path) {
    std::os::unix::fs::symlink(source, target).expect("test symlink should be created");
}

#[cfg(windows)]
fn create_test_file_symlink(source: &Path, target: &Path) {
    std::os::windows::fs::symlink_file(source, target).expect("test symlink should be created");
}

#[test]
fn list_projects_marks_git_projects_with_changes_when_git_is_available() {
    let home = TempHome::new("dirty-project");
    let project = home.path().join("workspace/git-app");
    fs::create_dir_all(&project).expect("project directory should be created");
    if !git_ok(&project, &["init"]) {
        return;
    }
    home.write("workspace/git-app/untracked.txt", "dirty");

    let projects = list_projects_for_home(home.path());
    let git_project = projects
        .iter()
        .find(|project| project.name == "git-app")
        .expect("git project should be detected");
    assert_eq!(git_project.status, super::contracts::ProjectStatus::Changed);
}

#[test]
fn git_status_is_safe_for_missing_and_non_git_asset_centers() {
    let home = TempHome::new("git-status-safe");

    let missing = git_status_for_home(home.path());
    assert!(!missing.is_repository);
    assert!(missing.status_message.contains("does not exist"));

    home.mkdir(".my-agent-assets");
    let non_git = git_status_for_home(home.path());
    assert!(!non_git.is_repository);
    assert!(non_git.status_message.contains("not a Git repository"));
}

#[test]
fn git_status_reads_repository_without_upstream_when_git_is_available() {
    let home = TempHome::new("git-status-repo");
    let repo = home.path().join(".my-agent-assets");
    fs::create_dir_all(&repo).expect("repo should be created");
    if !git_ok(&repo, &["init"]) {
        return;
    }
    fs::write(repo.join("asset.txt"), "changed").expect("asset should be written");

    let status = git_status_for_home(home.path());
    assert!(status.is_repository);
    assert!(status.status_message.contains("no upstream"));
    assert!(!status.clean);
    assert!(status.changed_files.iter().any(|file| file == "asset.txt"));
}

#[test]
fn list_assets_can_filter_by_type() {
    let home = TempHome::new("asset-filter");
    home.write(".my-agent-assets/assets/skills/review/SKILL.md", "# Review");
    home.write(".my-agent-assets/assets/commands/deploy.md", "# Deploy");

    let commands = list_assets_for_home(
        home.path(),
        ListAssetsInput {
            asset_type: Some(AssetType::Command),
        },
    );

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, "command:deploy");
}

fn git_ok(cwd: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
