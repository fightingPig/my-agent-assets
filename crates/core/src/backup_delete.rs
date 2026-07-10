use crate::backup_history::{resolve_backup_directory, resolve_backup_entry, BackupClass};
use crate::fingerprint::PreviewFingerprint;
use crate::operation::{incomplete_journals, OperationJournal, OperationLock, RecoveryTarget};
use crate::path_safety::guard_write_path;
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PREVIEW_TTL_SECONDS: u64 = 600;
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupDeletePreviewRequest {
    pub entry_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupDeletePreview {
    pub preview_id: String,
    pub entry_id: String,
    pub backup_id: String,
    pub class: BackupClass,
    pub backup_path: PathBuf,
    pub size_bytes: u64,
    pub entry_count: u32,
    pub sensitive_config_risk: bool,
    pub planned_effects: Vec<String>,
    pub warnings: Vec<String>,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupDeleteApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: BackupDeletePreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupDeleteApplyResult {
    pub preview_id: String,
    pub entry_id: String,
    pub deleted: bool,
    pub affected_paths: Vec<PathBuf>,
    pub warnings: Vec<String>,
    pub journal_path: PathBuf,
}

pub fn preview_backup_delete(
    home: &Path,
    request: &BackupDeletePreviewRequest,
) -> Result<BackupDeletePreview> {
    preview_backup_delete_at(home, request, epoch_seconds())
}

fn preview_backup_delete_at(
    home: &Path,
    request: &BackupDeletePreviewRequest,
    generated_at_epoch_seconds: u64,
) -> Result<BackupDeletePreview> {
    let entry = resolve_backup_entry(home, &request.entry_id)?;
    let backup_path = resolve_backup_directory(home, &request.entry_id)?;
    let mut warnings =
        vec!["删除后将失去这份备份的手动恢复材料；该操作不能通过应用内 Restore 撤销。".into()];
    if entry.sensitive_config_risk {
        warnings.push("该备份可能包含敏感 MCP 配置，请确认不再需要其中的值。".into());
    }

    let referenced_by = journals_referencing_backup(home, &backup_path)?;
    if !referenced_by.is_empty() {
        warnings.push(format!(
            "该备份正被未完成事务 {} 引用；必须先完成启动恢复，不能删除。",
            referenced_by.join(", ")
        ));
    }

    let preview_id =
        backup_delete_fingerprint(home, request, &backup_path, generated_at_epoch_seconds)?;
    Ok(BackupDeletePreview {
        preview_id,
        entry_id: entry.id,
        backup_id: entry.backup_id,
        class: entry.class,
        backup_path,
        size_bytes: entry.size_bytes,
        entry_count: entry.entry_count,
        sensitive_config_risk: entry.sensitive_config_risk,
        planned_effects: vec![format!(
            "permanently delete backup directory {}",
            entry
                .manifest_path
                .parent()
                .unwrap_or(Path::new("."))
                .display()
        )],
        warnings,
        can_apply: referenced_by.is_empty(),
        generated_at_epoch_seconds,
        expires_at_epoch_seconds: generated_at_epoch_seconds.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

pub fn apply_backup_delete(
    home: &Path,
    request: &BackupDeleteApplyRequest,
) -> Result<BackupDeleteApplyResult> {
    let _lock = OperationLock::acquire(home)?;
    if epoch_seconds()
        > request
            .preview_generated_at_epoch_seconds
            .saturating_add(PREVIEW_TTL_SECONDS)
    {
        return Err(MaaError::new(
            "backup deletion preview expired; generate a new preview before applying",
        ));
    }
    let preview = preview_backup_delete_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    if preview.preview_id != request.preview_id {
        return Err(MaaError::new(
            "backup deletion preview is stale; generate a new preview before applying",
        ));
    }
    if !preview.can_apply {
        return Err(MaaError::new(
            preview
                .warnings
                .last()
                .cloned()
                .unwrap_or_else(|| "backup deletion is blocked".into()),
        ));
    }

    let root = home.join(".my-agent-assets");
    let backup_path = guard_write_path(&root, &preview.backup_path)?;
    let operation_id = operation_id();
    let mut journal = OperationJournal::start_recoverable(
        home,
        &operation_id,
        "backup_delete",
        vec![RecoveryTarget::asset_center(backup_path.clone())],
    )?;
    fs::remove_dir_all(&backup_path)?;
    journal.record_step("backup_removed")?;
    journal.complete()?;

    let recovery_snapshot = root
        .join("backups/local")
        .join(format!("recovery-{operation_id}"));
    let mut warnings = Vec::new();
    if let Err(error) = fs::remove_dir_all(&recovery_snapshot) {
        warnings.push(format!(
            "deleted backup, but the completed transaction snapshot could not be cleaned: {error}"
        ));
    }
    Ok(BackupDeleteApplyResult {
        preview_id: preview.preview_id,
        entry_id: preview.entry_id,
        deleted: true,
        affected_paths: vec![backup_path],
        warnings,
        journal_path: journal.path().to_path_buf(),
    })
}

fn journals_referencing_backup(home: &Path, backup_path: &Path) -> Result<Vec<String>> {
    let mut references = Vec::new();
    for journal in incomplete_journals(home)? {
        let Some(recovery) = journal.recovery else {
            continue;
        };
        let referenced = recovery.backup_root == backup_path
            || recovery.backup_root.starts_with(backup_path)
            || recovery.entries.iter().any(|entry| {
                entry.target_path == backup_path
                    || entry.target_path.starts_with(backup_path)
                    || entry
                        .backup_path
                        .as_ref()
                        .is_some_and(|path| path == backup_path || path.starts_with(backup_path))
            });
        if referenced {
            references.push(journal.operation_id);
        }
    }
    references.sort();
    references.dedup();
    Ok(references)
}

fn backup_delete_fingerprint(
    home: &Path,
    request: &BackupDeletePreviewRequest,
    backup_path: &Path,
    generated_at_epoch_seconds: u64,
) -> Result<String> {
    let mut fingerprint = PreviewFingerprint::new("backup-delete");
    fingerprint.add_bytes(
        "request",
        &serde_json::to_vec(request).map_err(|error| MaaError::new(error.to_string()))?,
    );
    fingerprint.add_u64("generated-at", generated_at_epoch_seconds);
    fingerprint.add_path("backup", backup_path)?;
    fingerprint.add_path_if_present("operations", &home.join(".my-agent-assets/operations"))?;
    Ok(fingerprint.finish("backup-delete"))
}

fn operation_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos}-{}-{counter}", std::process::id())
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup_history::list_backups;
    use crate::operation::{crash_test, recover_incomplete};
    use std::panic::{catch_unwind, AssertUnwindSafe};

    fn home(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "maa-backup-delete-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn write_backup(home: &Path, id: &str) -> PathBuf {
        let backup = home.join(".my-agent-assets/backups/local").join(id);
        fs::create_dir_all(backup.join("content")).unwrap();
        fs::write(
            backup.join("manifest.yaml"),
            "schemaVersion: 1\noperation: mount\ntarget: /tmp/runtime/.claude.json\n",
        )
        .unwrap();
        fs::write(backup.join("content/runtime.json"), "{}").unwrap();
        backup
    }

    #[test]
    fn preview_and_apply_delete_only_the_selected_backup() {
        let home = home("success");
        let selected = write_backup(&home, "selected");
        let retained = write_backup(&home, "retained");
        let request = BackupDeletePreviewRequest {
            entry_id: "local:selected".into(),
        };
        let preview = preview_backup_delete(&home, &request).unwrap();
        crate::fingerprint::assert_sha256_preview_id(&preview.preview_id, "backup-delete-");
        assert!(preview.can_apply);
        assert!(preview
            .warnings
            .iter()
            .any(|warning| warning.contains("手动恢复材料")));

        let result = apply_backup_delete(
            &home,
            &BackupDeleteApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap();
        assert!(result.deleted);
        assert!(!selected.exists());
        assert!(retained.exists());
        assert!(list_backups(&home)
            .iter()
            .all(|entry| entry.id != "local:selected"));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn stale_preview_and_incomplete_journal_block_backup_deletion() {
        let home = home("blocked");
        let selected = write_backup(&home, "selected");
        let request = BackupDeletePreviewRequest {
            entry_id: "local:selected".into(),
        };
        let preview = preview_backup_delete(&home, &request).unwrap();
        fs::write(selected.join("content/runtime.json"), "changed").unwrap();
        assert!(apply_backup_delete(
            &home,
            &BackupDeleteApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request: request.clone(),
            },
        )
        .is_err());

        let _journal = OperationJournal::start_recoverable(
            &home,
            "backup-reference",
            "test",
            vec![RecoveryTarget::asset_center(selected.clone())],
        )
        .unwrap();
        let blocked = preview_backup_delete(&home, &request).unwrap();
        assert!(!blocked.can_apply);
        assert!(blocked
            .warnings
            .iter()
            .any(|warning| warning.contains("未完成事务")));
        drop(_journal);
        recover_incomplete(&home).unwrap();
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn persisted_delete_step_recovers_after_process_interruption() {
        let home = home("crash");
        let selected = write_backup(&home, "selected");
        let request = BackupDeletePreviewRequest {
            entry_id: "local:selected".into(),
        };
        let preview = preview_backup_delete(&home, &request).unwrap();
        let _crash = crash_test::crash_after_step("backup_delete", "backup_removed");
        let crashed = catch_unwind(AssertUnwindSafe(|| {
            let _ = apply_backup_delete(
                &home,
                &BackupDeleteApplyRequest {
                    preview_id: preview.preview_id,
                    preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                    request,
                },
            );
        }));
        assert!(crashed.is_err());
        assert!(!selected.exists());
        let report = recover_incomplete(&home).unwrap();
        assert!(report.attempts.iter().any(|attempt| attempt.recovered));
        assert!(selected.exists());
        let _ = fs::remove_dir_all(home);
    }
}
