use crate::path_safety::guard_write_path;
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct OperationLock {
    path: PathBuf,
}

impl OperationLock {
    pub fn acquire(home: &Path) -> Result<Self> {
        let root = home.join(".my-agent-assets");
        let lock_dir = guard_write_path(&root, &root.join("locks"))?;
        fs::create_dir_all(&lock_dir)?;
        let path = guard_write_path(&root, &lock_dir.join("global.lock"))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                MaaError::new(format!(
                    "another asset operation is active ({}): {error}",
                    path.display()
                ))
            })?;
        writeln!(
            file,
            "pid={}\ncreatedAtEpochSeconds={}",
            std::process::id(),
            epoch_seconds()
        )?;
        file.sync_all()?;
        Ok(Self { path })
    }
}

impl Drop for OperationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalStatus {
    #[serde(rename = "started")]
    Started,
    #[serde(rename = "rollback_required")]
    RollbackRequired,
    #[serde(rename = "completed")]
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalFile {
    pub schema_version: u32,
    pub operation_id: String,
    pub operation_kind: String,
    pub status: JournalStatus,
    pub created_at_epoch_seconds: u64,
    #[serde(default)]
    pub completed_steps: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_message: Option<String>,
}

pub struct OperationJournal {
    path: PathBuf,
    state: JournalFile,
}

impl OperationJournal {
    pub fn start(home: &Path, operation_id: &str, operation_kind: &str) -> Result<Self> {
        let root = home.join(".my-agent-assets");
        let directory = guard_write_path(&root, &root.join("operations"))?;
        fs::create_dir_all(&directory)?;
        let path = guard_write_path(&root, &directory.join(format!("{operation_id}.yaml")))?;
        let mut journal = Self {
            path,
            state: JournalFile {
                schema_version: 1,
                operation_id: operation_id.to_string(),
                operation_kind: operation_kind.to_string(),
                status: JournalStatus::Started,
                created_at_epoch_seconds: epoch_seconds(),
                completed_steps: Vec::new(),
                recovery_message: None,
            },
        };
        journal.persist()?;
        Ok(journal)
    }

    pub fn record_step(&mut self, step: impl Into<String>) -> Result<()> {
        self.state.completed_steps.push(step.into());
        self.persist()
    }

    pub fn mark_rollback_required(&mut self, message: impl Into<String>) -> Result<()> {
        self.state.status = JournalStatus::RollbackRequired;
        self.state.recovery_message = Some(message.into());
        self.persist()
    }

    pub fn complete(&mut self) -> Result<()> {
        self.state.status = JournalStatus::Completed;
        self.state.recovery_message = None;
        self.persist()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn persist(&mut self) -> Result<()> {
        let content = serde_yaml::to_string(&self.state).map_err(|error| {
            MaaError::new(format!("cannot serialize operation journal: {error}"))
        })?;
        fs::write(&self.path, content)?;
        Ok(())
    }
}

pub fn incomplete_journals(home: &Path) -> Result<Vec<JournalFile>> {
    let directory = home.join(".my-agent-assets/operations");
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut journals = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.path().extension().and_then(|value| value.to_str()) != Some("yaml") {
            continue;
        }
        let text = fs::read_to_string(entry.path())?;
        let journal: JournalFile = serde_yaml::from_str(&text)
            .map_err(|error| MaaError::new(format!("invalid operation journal: {error}")))?;
        if journal.status != JournalStatus::Completed {
            journals.push(journal);
        }
    }
    journals.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));
    Ok(journals)
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

    #[test]
    fn lock_is_exclusive_and_journal_tracks_incomplete_state() {
        let home = std::env::temp_dir().join(format!(
            "maa-operation-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(home.join(".my-agent-assets")).unwrap();
        let lock = OperationLock::acquire(&home).unwrap();
        assert!(OperationLock::acquire(&home).is_err());

        let mut journal = OperationJournal::start(&home, "op-1", "delete").unwrap();
        journal.record_step("backup").unwrap();
        journal
            .mark_rollback_required("runtime rollback failed")
            .unwrap();
        let incomplete = incomplete_journals(&home).unwrap();
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0].status, JournalStatus::RollbackRequired);
        journal.complete().unwrap();
        assert!(incomplete_journals(&home).unwrap().is_empty());

        drop(lock);
        assert!(OperationLock::acquire(&home).is_ok());
        let _ = fs::remove_dir_all(home);
    }
}
