use super::apply::conflict_apply_for_home;
use super::contracts::{
    ApplyMode, ConflictApplyInput, ConflictResolution, ConflictResolutionChoice, ScanScope,
};
use super::preview::import_preview_id;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn conflict_apply_plan_only_does_not_write_for_any_resolution() {
    let home = TempHome::new("plan-only");
    home.write(".claude/commands/deploy.md", "# Incoming Deploy");
    home.write(".claude/skills/review.md", "# Incoming Review");
    home.write(
        ".my-agent-assets/assets/commands/deploy.md",
        "# Existing Deploy",
    );
    home.write(
        ".my-agent-assets/assets/skills/review.md",
        "# Existing Review",
    );
    let before = snapshot_tree(home.path());
    let input = conflict_input(
        ApplyMode::PlanOnly,
        vec!["command:deploy", "skill:review"],
        vec![
            choice("command:deploy", ConflictResolution::Overwrite, None),
            choice(
                "skill:review",
                ConflictResolution::Rename,
                Some("review-imported"),
            ),
        ],
    );

    let result = conflict_apply_for_home(home.path(), input);

    assert!(result.ok, "{:?}", result.errors);
    assert!(result.backup.is_none());
    assert_eq!(snapshot_tree(home.path()), before);
}

#[test]
fn conflict_apply_overwrite_creates_backup_and_replaces_asset() {
    let home = TempHome::new("overwrite");
    home.write(".claude/commands/deploy.md", "# Incoming Deploy");
    home.write(
        ".my-agent-assets/assets/commands/deploy.md",
        "# Existing Deploy",
    );
    let input = conflict_input(
        ApplyMode::Apply,
        vec!["command:deploy"],
        vec![choice(
            "command:deploy",
            ConflictResolution::Overwrite,
            None,
        )],
    );

    let result = conflict_apply_for_home(home.path(), input);

    assert!(result.ok, "{:?}", result.errors);
    let backup = result.backup.expect("overwrite should create backup");
    assert_eq!(backup.entry_count, 1);
    assert_eq!(
        home.read(".my-agent-assets/assets/commands/deploy.md"),
        "# Incoming Deploy"
    );
    assert_eq!(
        home.read(&format!(
            ".my-agent-assets/backups/{}/files/.my-agent-assets/assets/commands/deploy.md",
            backup.id
        )),
        "# Existing Deploy"
    );
}

#[test]
fn conflict_apply_rename_preserves_existing_asset_and_imports_new_name() {
    let home = TempHome::new("rename");
    home.write(".claude/skills/review.md", "# Incoming Review");
    home.write(
        ".my-agent-assets/assets/skills/review.md",
        "# Existing Review",
    );
    let input = conflict_input(
        ApplyMode::Apply,
        vec!["skill:review"],
        vec![choice(
            "conflict:skill:review",
            ConflictResolution::Rename,
            Some("review-imported"),
        )],
    );

    let result = conflict_apply_for_home(home.path(), input);

    assert!(result.ok, "{:?}", result.errors);
    assert!(result.backup.is_none());
    assert_eq!(
        home.read(".my-agent-assets/assets/skills/review.md"),
        "# Existing Review"
    );
    assert_eq!(
        home.read(".my-agent-assets/assets/skills/review-imported.md"),
        "# Incoming Review"
    );
}

#[test]
fn conflict_apply_mcp_rename_extracts_original_server_and_rejects_existing_target() {
    let home = TempHome::new("mcp-rename");
    home.write(
        ".claude.json",
        r#"{"mcpServers":{"PostgreSQL":{"command":"incoming"}}}"#,
    );
    home.write(
        ".my-agent-assets/assets/mcps/PostgreSQL.json",
        r#"{"command":"existing"}"#,
    );
    let input = conflict_input(
        ApplyMode::Apply,
        vec!["mcp:PostgreSQL"],
        vec![choice(
            "mcp:PostgreSQL",
            ConflictResolution::Rename,
            Some("PostgreSQL-imported"),
        )],
    );
    let result = conflict_apply_for_home(home.path(), input);
    assert!(result.ok, "{:?}", result.errors);
    assert!(home
        .read(".my-agent-assets/assets/mcps/PostgreSQL-imported.json")
        .contains("\"incoming\""));

    home.write(
        ".my-agent-assets/assets/mcps/PostgreSQL-second.json",
        r#"{"command":"must-not-change"}"#,
    );
    let before = snapshot_tree(home.path());
    let collision = conflict_apply_for_home(
        home.path(),
        conflict_input(
            ApplyMode::Apply,
            vec!["mcp:PostgreSQL"],
            vec![choice(
                "mcp:PostgreSQL",
                ConflictResolution::Rename,
                Some("PostgreSQL-second"),
            )],
        ),
    );
    assert!(!collision.ok);
    assert!(collision.errors[0].contains("Rename target already exists"));
    assert_eq!(snapshot_tree(home.path()), before);
}

#[test]
fn conflict_apply_skip_and_invalid_inputs_do_not_write() {
    let home = TempHome::new("invalid");
    home.write(".claude/skills/review.md", "# Incoming Review");
    home.write(
        ".my-agent-assets/assets/skills/review.md",
        "# Existing Review",
    );
    let before = snapshot_tree(home.path());

    let skip = conflict_apply_for_home(
        home.path(),
        conflict_input(
            ApplyMode::Apply,
            vec!["skill:review"],
            vec![choice("skill:review", ConflictResolution::Skip, None)],
        ),
    );
    assert!(skip.ok, "{:?}", skip.errors);
    assert_eq!(snapshot_tree(home.path()), before);

    let unsafe_rename = conflict_apply_for_home(
        home.path(),
        conflict_input(
            ApplyMode::Apply,
            vec!["skill:review"],
            vec![choice(
                "skill:review",
                ConflictResolution::Rename,
                Some("../outside"),
            )],
        ),
    );
    assert!(!unsafe_rename.ok);
    assert_eq!(snapshot_tree(home.path()), before);

    let mut mismatched = conflict_input(
        ApplyMode::Apply,
        vec!["skill:review"],
        vec![choice("skill:review", ConflictResolution::Overwrite, None)],
    );
    mismatched.preview_id = "preview:import:tampered".into();
    let mismatch = conflict_apply_for_home(home.path(), mismatched);
    assert!(!mismatch.ok);
    assert!(mismatch.errors[0].contains("Preview ID does not match"));
    assert_eq!(snapshot_tree(home.path()), before);
}

fn conflict_input(
    mode: ApplyMode,
    asset_ids: Vec<&str>,
    conflict_resolutions: Vec<ConflictResolutionChoice>,
) -> ConflictApplyInput {
    let scope = ScanScope::User;
    let asset_ids = asset_ids
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    ConflictApplyInput {
        preview_id: import_preview_id(&scope, &asset_ids, &conflict_resolutions),
        mode,
        scope,
        asset_ids,
        conflict_resolutions,
        backup_before_apply: true,
    }
}

fn choice(
    conflict_id: &str,
    resolution: ConflictResolution,
    rename_to: Option<&str>,
) -> ConflictResolutionChoice {
    ConflictResolutionChoice {
        conflict_id: conflict_id.into(),
        resolution,
        rename_to: rename_to.map(str::to_string),
    }
}

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
            "my-agent-assets-conflict-{}-{}-{}",
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

    fn write(&self, relative: &str, contents: &str) {
        let path = self.path.join(relative);
        fs::create_dir_all(path.parent().expect("fixture path should have parent"))
            .expect("fixture parent should be created");
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
                .expect("snapshot path should remain below root")
                .to_path_buf();
            let metadata =
                fs::symlink_metadata(&path).expect("snapshot metadata should be readable");
            if metadata.is_dir() {
                snapshot.push((relative, "directory".into(), vec![]));
                visit(root, &path, snapshot);
            } else if metadata.file_type().is_symlink() {
                snapshot.push((
                    relative,
                    "symlink".into(),
                    fs::read_link(&path)
                        .expect("symlink should be readable")
                        .as_os_str()
                        .as_encoded_bytes()
                        .to_vec(),
                ));
            } else {
                snapshot.push((
                    relative,
                    "file".into(),
                    fs::read(&path).expect("file should be readable"),
                ));
            }
        }
    }
    let mut snapshot = vec![];
    visit(root, root, &mut snapshot);
    snapshot
}
