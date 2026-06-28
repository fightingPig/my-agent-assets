use super::contracts::{
    ConflictResolutionChoice, GitStatus, MountTarget, PreviewConflictsInput, PreviewImportInput,
    PreviewMountInput, PreviewRestoreInput, PreviewSyncInput, RuntimeScope, ScanScope,
    SyncDirection,
};
use super::preview::{
    import_preview_id, preview_conflicts, preview_conflicts_for_home, preview_import,
    preview_mount, preview_restore, preview_restore_for_home, preview_sync, restore_preview_id,
    sync_preview_id,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempProbe {
    path: PathBuf,
}

impl TempProbe {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "my-agent-assets-preview-{}-{}-{}",
            name,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&path).expect("temp probe should be created");
        Self { path }
    }

    fn snapshot(&self) -> Vec<String> {
        let mut entries = fs::read_dir(&self.path)
            .expect("temp probe should be readable")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        entries.sort();
        entries
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.path.join(relative);
        fs::create_dir_all(path.parent().expect("fixture path should have parent"))
            .expect("fixture parent should be created");
        fs::write(path, contents).expect("fixture should be written");
    }
}

impl Drop for TempProbe {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn snapshot_tree(root: &std::path::Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn visit(
        root: &std::path::Path,
        current: &std::path::Path,
        result: &mut Vec<(PathBuf, Vec<u8>)>,
    ) {
        let mut entries = fs::read_dir(current)
            .expect("snapshot directory should be readable")
            .map(|entry| entry.expect("snapshot entry should be readable"))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, result);
            } else {
                result.push((
                    path.strip_prefix(root)
                        .expect("snapshot path should stay below root")
                        .to_path_buf(),
                    fs::read(path).expect("snapshot file should be readable"),
                ));
            }
        }
    }
    let mut result = vec![];
    visit(root, root, &mut result);
    result
}

#[test]
fn preview_import_is_deterministic_and_does_not_write_files() {
    let probe = TempProbe::new("import");
    let before = probe.snapshot();
    let preview = preview_import(PreviewImportInput {
        scope: ScanScope::User,
        asset_ids: vec!["skill:review".into(), "mcp:PostgreSQL".into()],
        conflict_resolutions: vec![ConflictResolutionChoice {
            conflict_id: "mcp:PostgreSQL".into(),
            resolution: super::contracts::ConflictResolution::Rename,
            rename_to: Some("PostgreSQL-local".into()),
        }],
    });

    assert_eq!(probe.snapshot(), before);
    assert_eq!(preview.assets.len(), 2);
    assert_eq!(preview.conflicts.len(), 1);
    assert_eq!(preview.steps.len(), 3);
    assert!(preview.can_apply);
    assert!(preview.warnings[0].contains("Preview only"));
    assert_eq!(
        preview.preview_id,
        import_preview_id(
            &ScanScope::User,
            &["skill:review".into(), "mcp:PostgreSQL".into()],
            &[ConflictResolutionChoice {
                conflict_id: "mcp:PostgreSQL".into(),
                resolution: super::contracts::ConflictResolution::Rename,
                rename_to: Some("PostgreSQL-local".into()),
            }]
        )
    );
    assert!(preview.preview_id.starts_with("preview:import:"));
}

#[test]
fn preview_mount_returns_target_plan_without_writes() {
    let probe = TempProbe::new("mount");
    let before = probe.snapshot();
    let preview = preview_mount(PreviewMountInput {
        asset_id: "command:deploy-prod".into(),
        target: MountTarget {
            scope: RuntimeScope::Project,
            runtime_path: "~/workspace/project-a/.claude/commands/deploy-prod.md".into(),
            project_path: Some("~/workspace/project-a".into()),
        },
    });

    assert_eq!(probe.snapshot(), before);
    assert_eq!(preview.asset.id, "command:deploy-prod");
    assert_eq!(
        preview.target.runtime_path,
        "~/workspace/project-a/.claude/commands/deploy-prod.md"
    );
    assert!(preview.backup_required);
    assert!(preview.can_apply);
    assert_eq!(preview.steps.len(), 3);
    assert!(preview.preview_id.starts_with("preview:mount:"));
}

#[test]
fn previews_reject_invalid_asset_ids_without_misclassifying_them_as_skills() {
    let import = preview_import(PreviewImportInput {
        scope: ScanScope::User,
        asset_ids: vec![
            "unknown:review".into(),
            "skill:../escape".into(),
            "skill:".into(),
        ],
        conflict_resolutions: vec![],
    });
    assert!(!import.can_apply);
    assert!(import.assets.is_empty());
    assert!(import
        .warnings
        .iter()
        .any(|warning| warning.contains("unknown type")));

    let mount = preview_mount(PreviewMountInput {
        asset_id: "other:review".into(),
        target: MountTarget {
            scope: RuntimeScope::User,
            runtime_path: "~/.claude/skills/review".into(),
            project_path: None,
        },
    });
    assert!(!mount.can_apply);
    assert_eq!(mount.asset.status, super::contracts::AssetStatus::Invalid);

    let conflicts = preview_conflicts(PreviewConflictsInput {
        scope: ScanScope::User,
        asset_ids: vec![
            "skill:review".into(),
            "broken".into(),
            "mcp:../escape".into(),
        ],
    });
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].asset_id, "skill:review");
}

#[test]
fn preview_conflicts_synthesizes_allowed_resolutions_without_writes() {
    let probe = TempProbe::new("conflicts");
    let before = probe.snapshot();
    let conflicts = preview_conflicts(PreviewConflictsInput {
        scope: ScanScope::Project {
            project_path: "~/workspace/project-a".into(),
        },
        asset_ids: vec!["skill:review".into(), "mcp:PostgreSQL".into()],
    });

    assert_eq!(probe.snapshot(), before);
    assert_eq!(conflicts.len(), 2);
    assert_eq!(conflicts[0].name, "review");
    assert_eq!(conflicts[0].allowed_resolutions.len(), 3);
}

#[test]
fn preview_conflicts_reads_exact_mcp_json_and_omits_identical_content_without_writes() {
    let home = TempProbe::new("real-conflicts");
    home.write(
        ".claude.json",
        r#"{"mcpServers":{"PostgreSQL":{"command":"incoming","args":["--schema","public"]},"SQLite":{"command":"same"}}}"#,
    );
    home.write(
        ".my-agent-assets/assets/mcps/PostgreSQL.json",
        r#"{"command":"existing","args":["--read-only"]}"#,
    );
    home.write(
        ".my-agent-assets/assets/mcps/SQLite.json",
        r#"{"command":"same"}"#,
    );
    let before = snapshot_tree(&home.path);

    let conflicts = preview_conflicts_for_home(
        &home.path,
        PreviewConflictsInput {
            scope: ScanScope::User,
            asset_ids: vec!["mcp:PostgreSQL".into(), "mcp:SQLite".into()],
        },
    );

    assert_eq!(snapshot_tree(&home.path), before);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].name, "PostgreSQL");
    assert!(conflicts[0].existing_content.contains("\"existing\""));
    assert!(conflicts[0].incoming_content.contains("\"incoming\""));
    assert!(!conflicts[0].existing_content.contains("\"incoming\""));
}

#[test]
fn preview_restore_returns_impact_without_writes() {
    let probe = TempProbe::new("restore");
    let before = probe.snapshot();
    let preview = preview_restore(PreviewRestoreInput {
        backup_id: "backup-20260621-1842".into(),
    });

    assert_eq!(probe.snapshot(), before);
    assert_eq!(preview.backup.id, "backup-20260621-1842");
    assert_eq!(preview.affected_paths.len(), 3);
    assert!(preview.backup_before_restore);
    assert!(preview.can_apply);
    assert_eq!(preview.steps.len(), 3);
    assert_eq!(
        preview.preview_id,
        restore_preview_id("backup-20260621-1842")
    );
}

#[test]
fn preview_restore_reads_manifest_paths_without_restoring_files() {
    let home = TempProbe::new("restore-manifest");
    fs::create_dir_all(home.path.join(".my-agent-assets/backups/backup-real"))
        .expect("backup directory should be created");
    fs::write(
        home.path
            .join(".my-agent-assets/backups/backup-real/manifest.json"),
        r#"{
  "id": "backup-real",
  "label": "Real manifest backup",
  "createdAt": "2026-06-27T12:00:00Z",
  "runtimeRoot": "/tmp/fake-home",
  "entries": [
    { "originalPath": "/tmp/fake-home/.claude/skills/review", "backupPath": "files/review", "kind": "file", "sizeBytes": 128 },
    { "originalPath": "/tmp/fake-home/workspace/project-a/.mcp.json", "backupPath": "files/.mcp.json", "kind": "file", "sizeBytes": 64 }
  ]
}"#,
    )
    .expect("manifest should be written");
    let before = home.snapshot();

    let preview = preview_restore_for_home(
        &home.path,
        PreviewRestoreInput {
            backup_id: "backup-real".into(),
        },
    );

    assert_eq!(home.snapshot(), before);
    assert_eq!(preview.backup.id, "backup-real");
    assert_eq!(preview.backup.label, "Real manifest backup");
    assert_eq!(preview.backup.entry_count, 2);
    assert_eq!(preview.backup.size_bytes, 192);
    assert_eq!(
        preview.affected_paths,
        vec![
            "/tmp/fake-home/.claude/skills/review",
            "/tmp/fake-home/workspace/project-a/.mcp.json"
        ]
    );
    assert!(preview.can_apply);
    assert!(preview.warnings[0].contains("Preview only"));
    assert!(preview.preview_id.starts_with("preview:restore:"));
}

#[test]
fn preview_restore_rejects_unsafe_backup_id_without_path_traversal() {
    let home = TempProbe::new("restore-unsafe-id");
    let preview = preview_restore_for_home(
        &home.path,
        PreviewRestoreInput {
            backup_id: "../outside".into(),
        },
    );

    assert!(!preview.can_apply);
    assert!(preview
        .warnings
        .iter()
        .any(|warning| warning.contains("safe path component")));
    assert!(!home.path.join(".my-agent-assets").exists());
}

#[test]
fn preview_sync_returns_git_plan_without_writes() {
    let probe = TempProbe::new("sync");
    let before = probe.snapshot();
    let input = PreviewSyncInput {
        direction: SyncDirection::Push,
    };
    let status = GitStatus {
        repository_path: "~/.my-agent-assets".into(),
        is_repository: true,
        status_message: "Repository ready.".into(),
        branch: "main".into(),
        remote: Some("origin/main".into()),
        clean: true,
        ahead: 2,
        behind: 0,
        changed_files: vec![],
        conflicts: vec![],
        last_synced_at: None,
    };
    let preview = preview_sync(input.clone(), status.clone());

    assert_eq!(probe.snapshot(), before);
    assert_eq!(
        preview.preview_id,
        sync_preview_id(&input.direction, &status)
    );
    assert_eq!(preview.direction, SyncDirection::Push);
    assert_eq!(preview.repository_path, "~/.my-agent-assets");
    assert!(preview.can_apply);
    assert_eq!(preview.steps.len(), 3);
    assert!(preview.warnings[0].contains("Preview only"));
}

#[test]
fn preview_sync_blocks_when_repository_is_not_ready() {
    let preview = preview_sync(
        PreviewSyncInput {
            direction: SyncDirection::Pull,
        },
        GitStatus {
            repository_path: "~/.my-agent-assets".into(),
            is_repository: false,
            status_message: "Asset center directory does not exist.".into(),
            branch: "".into(),
            remote: None,
            clean: true,
            ahead: 0,
            behind: 4,
            changed_files: vec![],
            conflicts: vec![],
            last_synced_at: None,
        },
    );

    assert!(!preview.can_apply);
    assert!(preview
        .warnings
        .iter()
        .any(|warning| warning.contains("not a Git repository")));
    assert!(preview
        .warnings
        .iter()
        .any(|warning| warning.contains("No upstream remote")));
}
