use super::apply::{import_apply_for_home, mount_apply_for_home};
use super::contracts::{
    ApplyMode, ApplyStepStatus, ConflictResolution, ConflictResolutionChoice, ImportApplyInput,
    MountApplyInput, MountTarget, RuntimeScope, ScanScope,
};
use super::preview::{import_preview_id, mount_preview_id};
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
    let asset_ids = asset_ids.into_iter().map(String::from).collect::<Vec<_>>();
    let preview_id = import_preview_id(&scope, &asset_ids, &[]);
    ImportApplyInput {
        preview_id,
        mode,
        scope,
        asset_ids,
        conflict_resolutions: vec![],
        backup_before_apply: true,
    }
}

fn overwrite_import_input(mode: ApplyMode, scope: ScanScope, asset_id: &str) -> ImportApplyInput {
    let asset_ids = vec![asset_id.to_string()];
    let conflict_resolutions = vec![ConflictResolutionChoice {
        conflict_id: asset_id.into(),
        resolution: ConflictResolution::Overwrite,
        rename_to: None,
    }];
    ImportApplyInput {
        preview_id: import_preview_id(&scope, &asset_ids, &conflict_resolutions),
        mode,
        scope,
        asset_ids,
        conflict_resolutions,
        backup_before_apply: true,
    }
}

#[test]
fn import_apply_rejects_unresolved_content_conflict_without_writing() {
    let home = TempHome::new("unresolved-conflict");
    home.write(".claude/skills/review.md", "# Incoming Review");
    home.write(
        ".my-agent-assets/assets/skills/review.md",
        "# Existing Review",
    );

    let result = import_apply_for_home(
        home.path(),
        import_input(ApplyMode::Apply, ScanScope::User, vec!["skill:review"]),
    );

    assert!(!result.ok);
    assert!(result
        .errors
        .iter()
        .any(|error| error.contains("explicit conflict resolution")));
    assert_eq!(
        home.read(".my-agent-assets/assets/skills/review.md"),
        "# Existing Review"
    );
}

fn mount_input(
    mode: ApplyMode,
    asset_id: &str,
    runtime_path: String,
    backup_before_apply: bool,
) -> MountApplyInput {
    let target = MountTarget {
        scope: RuntimeScope::Project,
        runtime_path,
        project_path: Some("~/workspace/project-a".into()),
    };
    let preview_id = mount_preview_id(asset_id, &target);
    MountApplyInput {
        preview_id,
        mode,
        asset_id: asset_id.into(),
        target,
        backup_before_apply,
    }
}

#[test]
fn import_apply_plan_only_does_not_create_asset_center_or_backup() {
    let home = TempHome::new("plan-only");
    home.write(".claude/skills/review.md", "# Review");
    let before = snapshot_tree(home.path());

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
    assert_eq!(snapshot_tree(home.path()), before);
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
        overwrite_import_input(ApplyMode::Apply, ScanScope::User, "command:deploy"),
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
fn import_apply_rejects_mismatched_preview_id_before_writing() {
    let home = TempHome::new("import-preview-mismatch");
    home.write(".claude/commands/deploy.md", "# Deploy");
    let mut input = import_input(ApplyMode::Apply, ScanScope::User, vec!["command:deploy"]);
    input.preview_id = "preview:import:tampered".into();

    let result = import_apply_for_home(home.path(), input);

    assert!(!result.ok);
    assert!(result.errors[0].contains("Preview ID does not match import input"));
    assert!(!home.exists(".my-agent-assets"));
}

#[test]
fn import_apply_rejects_unsafe_asset_ids_before_path_construction() {
    for asset_id in [
        "skill:../escape",
        "command:nested/name",
        r"mcp:nested\name",
        "skill:.",
        "command: name",
        "mcp:name:extra",
    ] {
        let home = TempHome::new("unsafe-asset-id");
        let result = import_apply_for_home(
            home.path(),
            import_input(ApplyMode::Apply, ScanScope::User, vec![asset_id]),
        );

        assert!(!result.ok, "{asset_id} should be rejected");
        assert!(result.errors[0].contains("safe path component"));
        assert!(!home.exists(".my-agent-assets"));
    }
}

#[test]
fn import_apply_rejects_scope_parentdir_and_symlink_escape() {
    let home = TempHome::new("import-scope-escape");
    let outside = TempHome::new("import-scope-outside");
    outside.write(".claude/commands/deploy.md", "# Outside");

    let parent_result = import_apply_for_home(
        home.path(),
        import_input(
            ApplyMode::Apply,
            ScanScope::Custom {
                path: home.path().join("../escape").to_string_lossy().into_owned(),
            },
            vec!["command:deploy"],
        ),
    );
    assert!(!parent_result.ok);
    assert!(parent_result.errors[0].contains("ParentDir"));

    let link = home.path().join("linked-runtime");
    create_test_directory_symlink(outside.path(), &link);
    let symlink_result = import_apply_for_home(
        home.path(),
        import_input(
            ApplyMode::Apply,
            ScanScope::Custom {
                path: link.to_string_lossy().into_owned(),
            },
            vec!["command:deploy"],
        ),
    );
    assert!(!symlink_result.ok);
    assert!(symlink_result.errors[0].contains("Symlink traversal"));
    assert!(!home.exists(".my-agent-assets"));
}

#[test]
fn import_apply_rejects_nested_skill_symlink_before_creating_asset_center() {
    let home = TempHome::new("import-nested-symlink");
    let outside = TempHome::new("import-nested-outside");
    home.write(".claude/skills/review/SKILL.md", "# Review");
    outside.write("secret.md", "secret");
    create_test_symlink(
        &outside.path().join("secret.md"),
        &home.path().join(".claude/skills/review/secret.md"),
    );

    let result = import_apply_for_home(
        home.path(),
        import_input(ApplyMode::Apply, ScanScope::User, vec!["skill:review"]),
    );

    assert!(!result.ok);
    assert!(result.errors[0].contains("Symlinks are forbidden"));
    assert!(!home.exists(".my-agent-assets"));
}

#[test]
fn mount_apply_plan_only_does_not_create_runtime_target() {
    let home = TempHome::new("mount-plan-only");
    home.write(".my-agent-assets/assets/skills/review.md", "# Review");
    let before = snapshot_tree(home.path());

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
    assert_eq!(snapshot_tree(home.path()), before);
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
    assert!(result.errors[0].contains("allowed root"));
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
    assert!(result.errors[0].contains("Path does not exist"));
    assert!(!home.exists("workspace/project-a/.claude/skills/missing.md"));
}

#[test]
fn mount_apply_rejects_parentdir_and_symlink_parent_before_writing() {
    let home = TempHome::new("mount-path-escape");
    let outside = TempHome::new("mount-path-outside");
    home.write(".my-agent-assets/assets/skills/review.md", "# Review");

    let parent_result = mount_apply_for_home(
        home.path(),
        mount_input(
            ApplyMode::Apply,
            "skill:review",
            home.path()
                .join("workspace/../escape.md")
                .to_string_lossy()
                .into_owned(),
            true,
        ),
    );
    assert!(!parent_result.ok);
    assert!(parent_result.errors[0].contains("ParentDir"));

    let linked_parent = home.path().join("workspace");
    create_test_directory_symlink(outside.path(), &linked_parent);
    let symlink_result = mount_apply_for_home(
        home.path(),
        mount_input(
            ApplyMode::Apply,
            "skill:review",
            linked_parent
                .join("project/.claude/skills/review.md")
                .to_string_lossy()
                .into_owned(),
            true,
        ),
    );
    assert!(!symlink_result.ok);
    assert!(symlink_result.errors[0].contains("Symlink traversal"));
    assert!(!outside.exists("project/.claude/skills/review.md"));
}

#[test]
fn mount_apply_rejects_mismatched_preview_id_before_writing() {
    let home = TempHome::new("mount-preview-mismatch");
    home.write(".my-agent-assets/assets/skills/review.md", "# Review");
    let mut input = mount_input(
        ApplyMode::Apply,
        "skill:review",
        "~/workspace/project-a/.claude/skills/review.md".into(),
        true,
    );
    input.preview_id = "preview:mount:tampered".into();

    let result = mount_apply_for_home(home.path(), input);

    assert!(!result.ok);
    assert!(result.errors[0].contains("Preview ID does not match mount input"));
    assert!(!home.exists("workspace/project-a/.claude/skills/review.md"));
}

#[test]
fn mount_apply_plan_only_does_not_compile_mcp_runtime_config() {
    let home = TempHome::new("mcp-plan-only");
    home.write(
        ".my-agent-assets/assets/mcps/PostgreSQL.json",
        r#"{"command":"postgres"}"#,
    );
    let before = snapshot_tree(home.path());

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
    assert_eq!(snapshot_tree(home.path()), before);
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

fn read_json(path: impl AsRef<Path>) -> serde_json::Value {
    let text = fs::read_to_string(path).expect("JSON file should be readable");
    serde_json::from_str(&text).expect("JSON should parse")
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
                .expect("snapshot path should stay under root")
                .to_path_buf();
            let metadata =
                fs::symlink_metadata(&path).expect("snapshot metadata should be readable");
            if metadata.file_type().is_symlink() {
                snapshot.push((
                    relative,
                    "symlink".into(),
                    fs::read_link(&path)
                        .expect("snapshot link should be readable")
                        .to_string_lossy()
                        .as_bytes()
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

#[cfg(unix)]
fn create_test_symlink(source: &Path, destination: &Path) {
    std::os::unix::fs::symlink(source, destination).expect("test symlink should be created");
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

#[cfg(windows)]
fn create_test_symlink(source: &Path, destination: &Path) {
    std::os::windows::fs::symlink_file(source, destination)
        .expect("test symlink should be created");
}
