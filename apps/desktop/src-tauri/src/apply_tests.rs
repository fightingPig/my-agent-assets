use super::apply::{import_apply_for_home, mount_apply_for_home, restore_apply_for_home};
use super::contracts::{
    ApplyMode, ApplyStepStatus, ImportApplyInput, MountApplyInput, MountTarget, RestoreApplyInput,
    RuntimeScope, ScanScope,
};
use std::fs;
use std::path::{Path, PathBuf};
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
            "my-agent-assets-apply-{}-{}-{}",
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

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path.join(relative)).expect("file should be readable")
    }

    fn exists(&self, relative: &str) -> bool {
        self.path.join(relative).exists()
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn import_input(mode: ApplyMode, scope: ScanScope, asset_ids: Vec<&str>) -> ImportApplyInput {
    ImportApplyInput {
        preview_id: "preview-import-test".into(),
        mode,
        scope,
        asset_ids: asset_ids.into_iter().map(String::from).collect(),
        conflict_resolutions: vec![],
        backup_before_apply: true,
    }
}

fn mount_input(
    mode: ApplyMode,
    asset_id: &str,
    runtime_path: String,
    backup_before_apply: bool,
) -> MountApplyInput {
    MountApplyInput {
        preview_id: "preview-mount-test".into(),
        mode,
        asset_id: asset_id.into(),
        target: MountTarget {
            scope: RuntimeScope::Project,
            runtime_path,
            project_path: Some("~/workspace/project-a".into()),
        },
        backup_before_apply,
    }
}

fn restore_input(
    mode: ApplyMode,
    backup_id: String,
    backup_before_restore: bool,
) -> RestoreApplyInput {
    RestoreApplyInput {
        preview_id: "preview-restore-test".into(),
        mode,
        backup_id,
        backup_before_restore,
    }
}

#[test]
fn import_apply_plan_only_does_not_create_asset_center_or_backup() {
    let home = TempHome::new("plan-only");
    home.write(".claude/skills/review.md", "# Review");

    let result = import_apply_for_home(
        home.path(),
        import_input(ApplyMode::PlanOnly, ScanScope::User, vec!["skill:review"]),
    );

    assert!(result.ok);
    assert_eq!(result.mode, ApplyMode::PlanOnly);
    assert_eq!(result.steps.len(), 1);
    assert_eq!(result.steps[0].status, ApplyStepStatus::Skipped);
    assert!(!home.exists(".my-agent-assets"));
    assert!(result.backup.is_none());
}

#[test]
fn import_apply_copies_user_skill_markdown_into_asset_center() {
    let home = TempHome::new("user-skill");
    home.write(".claude/skills/review.md", "# Review\n\nCheck code.");

    let result = import_apply_for_home(
        home.path(),
        import_input(ApplyMode::Apply, ScanScope::User, vec!["skill:review"]),
    );

    assert!(result.ok, "{:?}", result.errors);
    assert_eq!(
        home.read(".my-agent-assets/assets/skills/review.md"),
        "# Review\n\nCheck code."
    );
    assert_eq!(result.steps[0].status, ApplyStepStatus::Success);
    assert!(result.backup.is_none());
}

#[test]
fn import_apply_copies_skill_directory_into_asset_center() {
    let home = TempHome::new("skill-dir");
    home.write(".claude/skills/review/SKILL.md", "# Review");
    home.write(".claude/skills/review/examples/basic.md", "Example");

    let result = import_apply_for_home(
        home.path(),
        import_input(ApplyMode::Apply, ScanScope::User, vec!["skill:review"]),
    );

    assert!(result.ok, "{:?}", result.errors);
    assert_eq!(
        home.read(".my-agent-assets/assets/skills/review/SKILL.md"),
        "# Review"
    );
    assert_eq!(
        home.read(".my-agent-assets/assets/skills/review/examples/basic.md"),
        "Example"
    );
}

#[test]
fn import_apply_copies_project_command_into_asset_center() {
    let home = TempHome::new("project-command");
    home.write("workspace/project-a/.claude/commands/deploy.md", "# Deploy");

    let result = import_apply_for_home(
        home.path(),
        import_input(
            ApplyMode::Apply,
            ScanScope::Project {
                project_path: "~/workspace/project-a".into(),
            },
            vec!["command:deploy"],
        ),
    );

    assert!(result.ok, "{:?}", result.errors);
    assert_eq!(
        home.read(".my-agent-assets/assets/commands/deploy.md"),
        "# Deploy"
    );
}

#[test]
fn import_apply_extracts_mcp_server_json_without_changing_runtime_config() {
    let home = TempHome::new("mcp");
    let runtime_config = r#"{"mcpServers":{"PostgreSQL":{"command":"postgres","args":["--stdio"]},"Redis":{"command":"redis"}}}"#;
    home.write(".claude.json", runtime_config);

    let result = import_apply_for_home(
        home.path(),
        import_input(ApplyMode::Apply, ScanScope::User, vec!["mcp:PostgreSQL"]),
    );

    assert!(result.ok, "{:?}", result.errors);
    let imported = home.read(".my-agent-assets/assets/mcps/PostgreSQL.json");
    assert!(imported.contains("\"command\": \"postgres\""));
    assert!(imported.contains("\"args\""));
    assert_eq!(home.read(".claude.json"), runtime_config);
}

#[test]
fn import_apply_backs_up_existing_destination_before_replacement() {
    let home = TempHome::new("backup");
    home.write(".claude/commands/deploy.md", "# New Deploy");
    home.write(".my-agent-assets/assets/commands/deploy.md", "# Old Deploy");

    let result = import_apply_for_home(
        home.path(),
        import_input(ApplyMode::Apply, ScanScope::User, vec!["command:deploy"]),
    );

    assert!(result.ok, "{:?}", result.errors);
    assert_eq!(
        home.read(".my-agent-assets/assets/commands/deploy.md"),
        "# New Deploy"
    );
    let backup = result.backup.expect("backup should be created");
    assert_eq!(backup.entry_count, 1);
    assert!(Path::new(&backup.manifest_path).exists());
    let backup_file = Path::new(&backup.manifest_path)
        .parent()
        .unwrap()
        .join("files/.my-agent-assets/assets/commands/deploy.md");
    assert_eq!(
        fs::read_to_string(backup_file).expect("backup file should be readable"),
        "# Old Deploy"
    );
}

#[test]
fn import_apply_reports_missing_source_without_creating_destination() {
    let home = TempHome::new("missing");

    let result = import_apply_for_home(
        home.path(),
        import_input(ApplyMode::Apply, ScanScope::User, vec!["command:missing"]),
    );

    assert!(!result.ok);
    assert_eq!(result.steps[0].status, ApplyStepStatus::Failed);
    assert!(!home.exists(".my-agent-assets/assets/commands/missing.md"));
}

#[test]
fn mount_apply_plan_only_does_not_create_runtime_target() {
    let home = TempHome::new("mount-plan-only");
    home.write(".my-agent-assets/assets/skills/review.md", "# Review");

    let result = mount_apply_for_home(
        home.path(),
        mount_input(
            ApplyMode::PlanOnly,
            "skill:review",
            "~/workspace/project-a/.claude/skills/review.md".into(),
            true,
        ),
    );

    assert!(result.ok, "{:?}", result.errors);
    assert_eq!(result.steps.len(), 1);
    assert_eq!(result.steps[0].status, ApplyStepStatus::Skipped);
    assert!(!home.exists("workspace/project-a/.claude/skills/review.md"));
    assert!(result.backup.is_none());
}

#[test]
fn mount_apply_creates_skill_symlink_from_asset_center() {
    let home = TempHome::new("mount-skill");
    home.write(".my-agent-assets/assets/skills/review.md", "# Review");
    let target = home
        .path()
        .join("workspace/project-a/.claude/skills/review.md");

    let result = mount_apply_for_home(
        home.path(),
        mount_input(
            ApplyMode::Apply,
            "skill:review",
            "~/workspace/project-a/.claude/skills/review.md".into(),
            true,
        ),
    );

    assert!(result.ok, "{:?}", result.errors);
    assert_eq!(result.steps[0].status, ApplyStepStatus::Success);
    assert_eq!(
        fs::read_link(&target).expect("target should be a symlink"),
        home.path().join(".my-agent-assets/assets/skills/review.md")
    );
    assert!(result.backup.is_none());
}

#[test]
fn mount_apply_creates_command_symlink_and_backs_up_existing_target() {
    let home = TempHome::new("mount-command-backup");
    home.write(".my-agent-assets/assets/commands/deploy.md", "# Deploy");
    home.write(
        "workspace/project-a/.claude/commands/deploy.md",
        "# Existing deploy",
    );
    let target = home
        .path()
        .join("workspace/project-a/.claude/commands/deploy.md");

    let result = mount_apply_for_home(
        home.path(),
        mount_input(
            ApplyMode::Apply,
            "command:deploy",
            "~/workspace/project-a/.claude/commands/deploy.md".into(),
            true,
        ),
    );

    assert!(result.ok, "{:?}", result.errors);
    assert_eq!(
        fs::read_link(&target).expect("target should be a symlink"),
        home.path()
            .join(".my-agent-assets/assets/commands/deploy.md")
    );
    let backup = result.backup.expect("backup should be created");
    assert_eq!(backup.entry_count, 1);
    let backup_file = Path::new(&backup.manifest_path)
        .parent()
        .unwrap()
        .join("files/workspace/project-a/.claude/commands/deploy.md");
    assert_eq!(
        fs::read_to_string(backup_file).expect("backup file should be readable"),
        "# Existing deploy"
    );
}

#[test]
fn mount_apply_rejects_target_outside_home() {
    let home = TempHome::new("mount-outside-home");
    home.write(".my-agent-assets/assets/skills/review.md", "# Review");
    let outside = std::env::temp_dir().join(format!(
        "my-agent-assets-outside-target-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&outside);

    let result = mount_apply_for_home(
        home.path(),
        mount_input(
            ApplyMode::Apply,
            "skill:review",
            outside.to_string_lossy().into_owned(),
            true,
        ),
    );

    assert!(!result.ok);
    assert!(result.errors[0].contains("must stay under resolved HOME"));
    assert!(!outside.exists());
}

#[test]
fn mount_apply_reports_missing_asset_without_creating_target() {
    let home = TempHome::new("mount-missing");

    let result = mount_apply_for_home(
        home.path(),
        mount_input(
            ApplyMode::Apply,
            "skill:missing",
            "~/workspace/project-a/.claude/skills/missing.md".into(),
            true,
        ),
    );

    assert!(!result.ok);
    assert!(result.errors[0].contains("Asset center path does not exist"));
    assert!(!home.exists("workspace/project-a/.claude/skills/missing.md"));
}

#[test]
fn mount_apply_plan_only_does_not_compile_mcp_runtime_config() {
    let home = TempHome::new("mcp-plan-only");
    home.write(
        ".my-agent-assets/assets/mcps/PostgreSQL.json",
        r#"{"command":"postgres"}"#,
    );

    let result = mount_apply_for_home(
        home.path(),
        mount_input(
            ApplyMode::PlanOnly,
            "mcp:PostgreSQL",
            "~/workspace/project-a/.mcp.json".into(),
            true,
        ),
    );

    assert!(result.ok, "{:?}", result.errors);
    assert_eq!(result.steps[0].status, ApplyStepStatus::Skipped);
    assert!(!home.exists("workspace/project-a/.mcp.json"));
}

#[test]
fn mount_apply_compiles_mcp_server_into_new_runtime_config() {
    let home = TempHome::new("mcp-new");
    home.write(
        ".my-agent-assets/assets/mcps/PostgreSQL.json",
        r#"{"command":"postgres","args":["--stdio"]}"#,
    );

    let result = mount_apply_for_home(
        home.path(),
        mount_input(
            ApplyMode::Apply,
            "mcp:PostgreSQL",
            "~/workspace/project-a/.mcp.json".into(),
            true,
        ),
    );

    assert!(result.ok, "{:?}", result.errors);
    assert!(result.backup.is_none());
    assert_eq!(result.steps[0].status, ApplyStepStatus::Success);
    let config = read_json(home.path().join("workspace/project-a/.mcp.json"));
    assert_eq!(config["mcpServers"]["PostgreSQL"]["command"], "postgres");
    assert_eq!(config["mcpServers"]["PostgreSQL"]["args"][0], "--stdio");
}

#[test]
fn mount_apply_merges_mcp_server_and_backs_up_existing_runtime_config() {
    let home = TempHome::new("mcp-merge");
    home.write(
        ".my-agent-assets/assets/mcps/PostgreSQL.json",
        r#"{"command":"postgres"}"#,
    );
    home.write(
        "workspace/project-a/.mcp.json",
        r#"{"comment":"keep","mcpServers":{"Redis":{"command":"redis"}}}"#,
    );

    let result = mount_apply_for_home(
        home.path(),
        mount_input(
            ApplyMode::Apply,
            "mcp:PostgreSQL",
            "~/workspace/project-a/.mcp.json".into(),
            true,
        ),
    );

    assert!(result.ok, "{:?}", result.errors);
    let config = read_json(home.path().join("workspace/project-a/.mcp.json"));
    assert_eq!(config["comment"], "keep");
    assert_eq!(config["mcpServers"]["Redis"]["command"], "redis");
    assert_eq!(config["mcpServers"]["PostgreSQL"]["command"], "postgres");

    let backup = result.backup.expect("backup should be created");
    assert_eq!(backup.entry_count, 1);
    let backup_file = Path::new(&backup.manifest_path)
        .parent()
        .unwrap()
        .join("files/workspace/project-a/.mcp.json");
    let backup_config = read_json(backup_file);
    assert_eq!(backup_config["mcpServers"]["Redis"]["command"], "redis");
    assert!(backup_config["mcpServers"].get("PostgreSQL").is_none());
}

#[test]
fn mount_apply_rejects_invalid_existing_mcp_runtime_config_without_overwrite() {
    let home = TempHome::new("mcp-invalid-runtime");
    home.write(
        ".my-agent-assets/assets/mcps/PostgreSQL.json",
        r#"{"command":"postgres"}"#,
    );
    home.write("workspace/project-a/.mcp.json", "{");

    let result = mount_apply_for_home(
        home.path(),
        mount_input(
            ApplyMode::Apply,
            "mcp:PostgreSQL",
            "~/workspace/project-a/.mcp.json".into(),
            true,
        ),
    );

    assert!(!result.ok);
    assert!(result.errors[0].contains("Could not parse JSON file"));
    assert_eq!(home.read("workspace/project-a/.mcp.json"), "{");
}

#[test]
fn restore_apply_plan_only_reads_manifest_without_changing_files() {
    let home = TempHome::new("restore-plan-only");
    home.write(".claude/commands/deploy.md", "# New Deploy");
    home.write(".my-agent-assets/assets/commands/deploy.md", "# Old Deploy");
    let import_result = import_apply_for_home(
        home.path(),
        import_input(ApplyMode::Apply, ScanScope::User, vec!["command:deploy"]),
    );
    let backup_id = import_result.backup.expect("backup should exist").id;
    home.write(".my-agent-assets/assets/commands/deploy.md", "# Mutated");

    let restore_result = restore_apply_for_home(
        home.path(),
        restore_input(ApplyMode::PlanOnly, backup_id, true),
    );

    assert!(restore_result.ok, "{:?}", restore_result.errors);
    assert_eq!(restore_result.steps[0].status, ApplyStepStatus::Skipped);
    assert_eq!(
        home.read(".my-agent-assets/assets/commands/deploy.md"),
        "# Mutated"
    );
    assert!(restore_result.backup.is_none());
}

#[test]
fn restore_apply_restores_file_from_import_backup_and_backs_up_current() {
    let home = TempHome::new("restore-file");
    home.write(".claude/commands/deploy.md", "# New Deploy");
    home.write(".my-agent-assets/assets/commands/deploy.md", "# Old Deploy");
    let import_result = import_apply_for_home(
        home.path(),
        import_input(ApplyMode::Apply, ScanScope::User, vec!["command:deploy"]),
    );
    let backup_id = import_result.backup.expect("backup should exist").id;
    assert_eq!(
        home.read(".my-agent-assets/assets/commands/deploy.md"),
        "# New Deploy"
    );

    let restore_result = restore_apply_for_home(
        home.path(),
        restore_input(ApplyMode::Apply, backup_id, true),
    );

    assert!(restore_result.ok, "{:?}", restore_result.errors);
    assert_eq!(
        home.read(".my-agent-assets/assets/commands/deploy.md"),
        "# Old Deploy"
    );
    let current_backup = restore_result
        .backup
        .expect("restore should back up current file");
    let backup_file = Path::new(&current_backup.manifest_path)
        .parent()
        .unwrap()
        .join("files/.my-agent-assets/assets/commands/deploy.md");
    assert_eq!(
        fs::read_to_string(backup_file).expect("current backup should be readable"),
        "# New Deploy"
    );
}

#[test]
fn restore_apply_restores_directory_from_import_backup() {
    let home = TempHome::new("restore-directory");
    home.write(".claude/skills/review/SKILL.md", "# New Review");
    home.write(
        ".my-agent-assets/assets/skills/review/SKILL.md",
        "# Old Review",
    );
    let import_result = import_apply_for_home(
        home.path(),
        import_input(ApplyMode::Apply, ScanScope::User, vec!["skill:review"]),
    );
    let backup_id = import_result.backup.expect("backup should exist").id;
    home.write(
        ".my-agent-assets/assets/skills/review/SKILL.md",
        "# Mutated",
    );

    let restore_result = restore_apply_for_home(
        home.path(),
        restore_input(ApplyMode::Apply, backup_id, false),
    );

    assert!(restore_result.ok, "{:?}", restore_result.errors);
    assert_eq!(
        home.read(".my-agent-assets/assets/skills/review/SKILL.md"),
        "# Old Review"
    );
    assert!(restore_result.backup.is_none());
    assert!(restore_result
        .warnings
        .iter()
        .any(|warning| warning.contains("without backup")));
}

#[test]
fn restore_apply_restores_symlink_from_mount_backup() {
    let home = TempHome::new("restore-symlink");
    home.write(".my-agent-assets/assets/commands/deploy.md", "# Deploy");
    home.write(
        ".my-agent-assets/assets/commands/old-deploy.md",
        "# Old Deploy",
    );
    let target = home
        .path()
        .join("workspace/project-a/.claude/commands/deploy.md");
    fs::create_dir_all(target.parent().unwrap()).expect("target parent should be created");
    create_test_symlink(
        &home
            .path()
            .join(".my-agent-assets/assets/commands/old-deploy.md"),
        &target,
    );

    let mount_result = mount_apply_for_home(
        home.path(),
        mount_input(
            ApplyMode::Apply,
            "command:deploy",
            "~/workspace/project-a/.claude/commands/deploy.md".into(),
            true,
        ),
    );
    let backup_id = mount_result.backup.expect("backup should exist").id;
    assert_eq!(
        fs::read_link(&target).expect("target should be a symlink"),
        home.path()
            .join(".my-agent-assets/assets/commands/deploy.md")
    );

    let restore_result = restore_apply_for_home(
        home.path(),
        restore_input(ApplyMode::Apply, backup_id, false),
    );

    assert!(restore_result.ok, "{:?}", restore_result.errors);
    assert_eq!(
        fs::read_link(&target).expect("restored target should be a symlink"),
        home.path()
            .join(".my-agent-assets/assets/commands/old-deploy.md")
    );
}

#[test]
fn restore_apply_rejects_tampered_manifest_outside_home() {
    let home = TempHome::new("restore-tampered");
    let backup_root = home.path().join(".my-agent-assets/backups/tampered");
    fs::create_dir_all(backup_root.join("files")).expect("backup root should be created");
    let outside = std::env::temp_dir().join(format!(
        "my-agent-assets-restore-outside-{}",
        std::process::id()
    ));
    let backup_file = backup_root.join("files/value.md");
    fs::write(&backup_file, "backup").expect("backup file should be written");
    fs::write(
        backup_root.join("manifest.json"),
        format!(
            r#"{{
  "id": "tampered",
  "label": "Tampered",
  "createdAt": "test",
  "runtimeRoot": "{}",
  "entries": [
    {{
      "originalPath": "{}",
      "backupPath": "{}",
      "kind": "file",
      "sizeBytes": 6
    }}
  ]
}}"#,
            home.path().to_string_lossy(),
            outside.to_string_lossy(),
            backup_file.to_string_lossy()
        ),
    )
    .expect("manifest should be written");

    let result = restore_apply_for_home(
        home.path(),
        restore_input(ApplyMode::Apply, "tampered".into(), true),
    );

    assert!(!result.ok);
    assert!(result.errors[0].contains("must stay under resolved HOME"));
    assert!(!outside.exists());
}

fn read_json(path: impl AsRef<Path>) -> serde_json::Value {
    let text = fs::read_to_string(path).expect("JSON file should be readable");
    serde_json::from_str(&text).expect("JSON should parse")
}

#[cfg(unix)]
fn create_test_symlink(source: &Path, destination: &Path) {
    std::os::unix::fs::symlink(source, destination).expect("test symlink should be created");
}

#[cfg(windows)]
fn create_test_symlink(source: &Path, destination: &Path) {
    std::os::windows::fs::symlink_file(source, destination)
        .expect("test symlink should be created");
}
