use super::contracts::{
    ConflictResolutionChoice, MountTarget, PreviewConflictsInput, PreviewImportInput,
    PreviewMountInput, PreviewRestoreInput, RuntimeScope, ScanScope,
};
use super::preview::{preview_conflicts, preview_import, preview_mount, preview_restore};
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
}

impl Drop for TempProbe {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
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
}
