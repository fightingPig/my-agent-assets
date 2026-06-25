use super::apply::import_apply_for_home;
use super::contracts::{ApplyMode, ApplyStepStatus, ImportApplyInput, ScanScope};
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
