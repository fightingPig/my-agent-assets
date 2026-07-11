//! Local, intentionally low-detail audit logging.
//!
//! Entries never contain paths, MCP payloads, environment values, Git URLs,
//! credentials, backup contents, or backend error text. The log is useful for
//! support timelines without becoming another copy of user configuration.

use crate::path_safety::{guard_write_path, is_link_or_junction};
use crate::settings;
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_SCHEMA_VERSION: u32 = 1;
const SECONDS_PER_DAY: u64 = 86_400;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditLogEntry {
    pub schema_version: u32,
    pub occurred_at_epoch_seconds: u64,
    pub operation_type: String,
    pub outcome: AuditOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditOutcome {
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "rollback_required")]
    RollbackRequired,
    #[serde(rename = "recovered")]
    Recovered,
}

pub fn append_operation(home: &Path, operation_type: &str, outcome: AuditOutcome) -> Result<()> {
    if !valid_operation_type(operation_type) {
        return Err(MaaError::new("audit log operation type is invalid"));
    }
    let root = home.join(".my-agent-assets");
    let now = epoch_seconds();
    let directory = guard_write_path(&root, &root.join("logs"))?;
    fs::create_dir_all(&directory)?;
    let retention_days = settings::load(home)
        .map(|value| value.log_retention_days)
        .unwrap_or_else(|_| settings::Settings::defaults_for_home(home).log_retention_days);
    prune_old_logs(&directory, now, retention_days)?;

    let path = guard_write_path(&root, &directory.join(log_file_name(now)))?;
    let entry = AuditLogEntry {
        schema_version: LOG_SCHEMA_VERSION,
        occurred_at_epoch_seconds: now,
        operation_type: operation_type.to_string(),
        outcome,
    };
    let encoded = serde_json::to_vec(&entry).map_err(|error| MaaError::new(error.to_string()))?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(&encoded)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

pub fn list_log_files(home: &Path) -> Result<Vec<PathBuf>> {
    let directory = home.join(".my-agent-assets/logs");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        let path = entry.path();
        if is_link_or_junction(&fs::symlink_metadata(&path)?) {
            continue;
        }
        if log_epoch_day(&path).is_some() {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// Reads only schema-valid, redacted audit entries. Tampered or malformed lines
/// are deliberately ignored rather than copied into a diagnostics package.
pub fn read_audit_entries(home: &Path) -> Result<Vec<AuditLogEntry>> {
    let mut entries = Vec::new();
    for path in list_log_files(home)? {
        let text = fs::read_to_string(path)?;
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            if let Ok(entry) = serde_json::from_str::<AuditLogEntry>(line) {
                entries.push(entry);
            }
        }
    }
    entries.sort_by(|left, right| {
        left.occurred_at_epoch_seconds
            .cmp(&right.occurred_at_epoch_seconds)
            .then_with(|| left.operation_type.cmp(&right.operation_type))
    });
    Ok(entries)
}

fn prune_old_logs(directory: &Path, now: u64, retention_days: u32) -> Result<()> {
    let today = now / SECONDS_PER_DAY;
    let oldest = today.saturating_sub(u64::from(retention_days.max(1).saturating_sub(1)));
    for path in list_directory_logs(directory)? {
        let Some(day) = log_epoch_day(&path) else {
            continue;
        };
        if day < oldest {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn list_directory_logs(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if is_link_or_junction(&fs::symlink_metadata(&path)?) {
            continue;
        }
        if log_epoch_day(&path).is_some() {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn log_file_name(epoch_seconds: u64) -> String {
    format!("operations-{}.jsonl", epoch_seconds / SECONDS_PER_DAY)
}

fn log_epoch_day(path: &Path) -> Option<u64> {
    let name = path.file_name()?.to_str()?;
    name.strip_prefix("operations-")?
        .strip_suffix(".jsonl")?
        .parse()
        .ok()
}

fn valid_operation_type(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn home(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "maa-audit-log-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn writes_only_redacted_operation_metadata_and_ignores_symlinks() {
        let home = home("redacted");
        fs::create_dir_all(home.join(".my-agent-assets")).unwrap();
        append_operation(&home, "mcp_save", AuditOutcome::Completed).unwrap();
        let logs = list_log_files(&home).unwrap();
        assert_eq!(logs.len(), 1);
        let text = fs::read_to_string(&logs[0]).unwrap();
        assert!(text.contains("mcp_save"));
        assert!(text.contains("completed"));
        assert!(!text.contains("command"));
        assert!(!text.contains("token"));
        assert!(!text.contains("path"));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn rejects_unsafe_operation_type() {
        let home = home("unsafe");
        fs::create_dir_all(home.join(".my-agent-assets")).unwrap();
        assert!(append_operation(&home, "mcp save: secret", AuditOutcome::Completed).is_err());
        assert!(!home.join(".my-agent-assets/logs").exists());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn prunes_only_old_regular_log_files() {
        let home = home("retention");
        let logs = home.join(".my-agent-assets/logs");
        fs::create_dir_all(&logs).unwrap();
        fs::write(logs.join("operations-0.jsonl"), "old\n").unwrap();
        fs::write(logs.join("unrelated.txt"), "keep\n").unwrap();
        append_operation(&home, "mount", AuditOutcome::Completed).unwrap();
        assert!(!logs.join("operations-0.jsonl").exists());
        assert!(logs.join("unrelated.txt").is_file());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn ignores_malformed_or_unexpected_log_content_when_reading_entries() {
        let home = home("read-redacted");
        let logs = home.join(".my-agent-assets/logs");
        fs::create_dir_all(&logs).unwrap();
        fs::write(
            logs.join("operations-1.jsonl"),
            concat!(
                "{\"schemaVersion\":1,\"occurredAtEpochSeconds\":1,\"operationType\":\"mount\",\"outcome\":\"completed\"}\n",
                "{\"schemaVersion\":1,\"occurredAtEpochSeconds\":2,\"operationType\":\"mount\",\"outcome\":\"completed\",\"secret\":\"nope\"}\n"
            ),
        )
        .unwrap();
        let entries = read_audit_entries(&home).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].operation_type, "mount");
        let _ = fs::remove_dir_all(home);
    }
}
