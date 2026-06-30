use crate::mount::{copy_any, remove_path_if_present};
use crate::path_safety::{guard_existing_path, guard_write_path};
use crate::targets::load as load_targets;
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const JOURNAL_SCHEMA_VERSION: u32 = 2;

#[derive(Debug)]
pub struct OperationLock {
    path: PathBuf,
}

impl OperationLock {
    pub fn acquire(home: &Path) -> Result<Self> {
        Self::acquire_internal(home, false)
    }

    fn acquire_for_recovery(home: &Path) -> Result<Self> {
        Self::acquire_internal(home, true)
    }

    fn acquire_internal(home: &Path, allow_incomplete: bool) -> Result<Self> {
        let root = home.join(".my-agent-assets");
        let lock_dir = guard_write_path(&root, &root.join("locks"))?;
        fs::create_dir_all(&lock_dir)?;
        let path = guard_write_path(&root, &lock_dir.join("global.lock"))?;
        let mut file = match create_lock_file(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                reclaim_stale_lock(&path)?;
                create_lock_file(&path).map_err(|retry| {
                    MaaError::new(format!(
                        "another asset operation is active ({}): {retry}",
                        path.display()
                    ))
                })?
            }
            Err(error) => {
                return Err(MaaError::new(format!(
                    "cannot create operation lock {}: {error}",
                    path.display()
                )))
            }
        };
        writeln!(
            file,
            "pid={}\ncreatedAtEpochSeconds={}",
            std::process::id(),
            epoch_seconds()
        )?;
        file.sync_all()?;
        let lock = Self { path };
        if !allow_incomplete {
            let incomplete = incomplete_journals(home)?;
            if !incomplete.is_empty() {
                let ids = incomplete
                    .iter()
                    .map(|journal| journal.operation_id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(MaaError::new(format!(
                    "new writes are blocked by incomplete operation journal(s): {ids}; run recovery diagnostics before retrying"
                )));
            }
        }
        Ok(lock)
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
    #[serde(rename = "recovered")]
    Recovered,
    #[serde(rename = "completed")]
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecoveryAuthority {
    AssetCenter,
    RegisteredTarget { target_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryTarget {
    pub path: PathBuf,
    pub authority: RecoveryAuthority,
}

impl RecoveryTarget {
    pub fn asset_center(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            authority: RecoveryAuthority::AssetCenter,
        }
    }

    pub fn registered_target(target_id: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            authority: RecoveryAuthority::RegisteredTarget {
                target_id: target_id.into(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryEntry {
    pub target_path: PathBuf,
    pub authority: RecoveryAuthority,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryPayload {
    pub backup_root: PathBuf,
    pub entries: Vec<RecoveryEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub git_refs: Vec<GitRefRecovery>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRefRecovery {
    pub repository: PathBuf,
    pub reference: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_oid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_oid: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovered_at_epoch_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<RecoveryPayload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryStatus {
    pub writes_blocked: bool,
    pub journals: Vec<JournalFile>,
    pub recent_recoveries: Vec<JournalFile>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryAttempt {
    pub operation_id: String,
    pub operation_kind: String,
    pub recovered: bool,
    pub affected_paths: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryReport {
    pub attempted: bool,
    pub writes_blocked: bool,
    pub attempts: Vec<RecoveryAttempt>,
}

pub struct OperationJournal {
    path: PathBuf,
    state: JournalFile,
}

impl OperationJournal {
    pub fn start(home: &Path, operation_id: &str, operation_kind: &str) -> Result<Self> {
        Self::start_with_payload(home, operation_id, operation_kind, None)
    }

    pub fn start_recoverable(
        home: &Path,
        operation_id: &str,
        operation_kind: &str,
        targets: Vec<RecoveryTarget>,
    ) -> Result<Self> {
        validate_operation_id(operation_id)?;
        Self::start_recoverable_with_git(home, operation_id, operation_kind, targets, Vec::new())
    }

    pub fn start_recoverable_with_git(
        home: &Path,
        operation_id: &str,
        operation_kind: &str,
        targets: Vec<RecoveryTarget>,
        git_refs: Vec<GitRefRecovery>,
    ) -> Result<Self> {
        validate_operation_id(operation_id)?;
        let payload =
            create_recovery_payload(home, operation_id, operation_kind, targets, git_refs)?;
        match Self::start_with_payload(home, operation_id, operation_kind, Some(payload.clone())) {
            Ok(journal) => Ok(journal),
            Err(error) => {
                let _ = fs::remove_dir_all(&payload.backup_root);
                Err(error)
            }
        }
    }

    fn start_with_payload(
        home: &Path,
        operation_id: &str,
        operation_kind: &str,
        recovery: Option<RecoveryPayload>,
    ) -> Result<Self> {
        validate_operation_id(operation_id)?;
        let root = home.join(".my-agent-assets");
        let directory = guard_write_path(&root, &root.join("operations"))?;
        fs::create_dir_all(&directory)?;
        let path = guard_write_path(&root, &directory.join(format!("{operation_id}.yaml")))?;
        if path.exists() {
            return Err(MaaError::new(format!(
                "operation journal already exists: {}",
                path.display()
            )));
        }
        let mut journal = Self {
            path,
            state: JournalFile {
                schema_version: JOURNAL_SCHEMA_VERSION,
                operation_id: operation_id.to_string(),
                operation_kind: operation_kind.to_string(),
                status: JournalStatus::Started,
                created_at_epoch_seconds: epoch_seconds(),
                completed_steps: Vec::new(),
                recovery_message: None,
                recovered_at_epoch_seconds: None,
                recovery,
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

    pub fn rollback_now(&mut self, home: &Path) -> Result<Vec<PathBuf>> {
        let payload = self.state.recovery.clone().ok_or_else(|| {
            MaaError::new(format!(
                "operation '{}' has no persistent recovery payload",
                self.state.operation_id
            ))
        })?;
        let affected = restore_payload(home, &self.state.operation_id, &payload)?;
        self.state.status = JournalStatus::Recovered;
        self.state.recovery_message = Some("automatic transaction rollback succeeded".into());
        self.state.recovered_at_epoch_seconds = Some(epoch_seconds());
        self.persist()?;
        Ok(affected)
    }

    pub fn set_git_ref_expected(&mut self, reference: &str, expected_oid: &str) -> Result<()> {
        validate_git_reference(reference)?;
        validate_git_oid(expected_oid)?;
        let recovery = self
            .state
            .recovery
            .as_mut()
            .ok_or_else(|| MaaError::new("operation has no persistent recovery payload"))?;
        let git_ref = recovery
            .git_refs
            .iter_mut()
            .find(|entry| entry.reference == reference)
            .ok_or_else(|| {
                MaaError::new(format!(
                    "operation recovery payload has no Git ref '{reference}'"
                ))
            })?;
        git_ref.expected_oid = Some(expected_oid.to_string());
        self.persist()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn persist(&mut self) -> Result<()> {
        let content = serde_yaml::to_string(&self.state).map_err(|error| {
            MaaError::new(format!("cannot serialize operation journal: {error}"))
        })?;
        atomic_write(&self.path, content.as_bytes())
    }
}

pub fn incomplete_journals(home: &Path) -> Result<Vec<JournalFile>> {
    Ok(load_journals(home)?
        .into_iter()
        .filter(|journal| {
            matches!(
                journal.status,
                JournalStatus::Started | JournalStatus::RollbackRequired
            )
        })
        .collect())
}

pub fn recovery_status(home: &Path) -> Result<RecoveryStatus> {
    let all = load_journals(home)?;
    let journals = all
        .iter()
        .filter(|journal| {
            matches!(
                journal.status,
                JournalStatus::Started | JournalStatus::RollbackRequired
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    let recent_recoveries = all
        .into_iter()
        .filter(|journal| journal.status == JournalStatus::Recovered)
        .rev()
        .take(10)
        .collect::<Vec<_>>();
    let writes_blocked = !journals.is_empty();
    let message = if writes_blocked {
        format!(
            "检测到 {} 个未完成事务；新的写操作已阻止，等待安全恢复。",
            journals.len()
        )
    } else if let Some(journal) = recent_recoveries.first() {
        format!("事务 {} 已自动回滚。", journal.operation_id)
    } else {
        "没有未完成事务。".to_string()
    };
    Ok(RecoveryStatus {
        writes_blocked,
        journals,
        recent_recoveries,
        message,
    })
}

pub fn recover_incomplete(home: &Path) -> Result<RecoveryReport> {
    let incomplete = incomplete_journals(home)?;
    if incomplete.is_empty() {
        return Ok(RecoveryReport {
            attempted: false,
            writes_blocked: false,
            attempts: Vec::new(),
        });
    }

    let _lock = OperationLock::acquire_for_recovery(home)?;
    let mut attempts = Vec::new();
    for state in incomplete_journals(home)? {
        let mut journal = OperationJournal {
            path: journal_path(home, &state.operation_id)?,
            state,
        };
        let operation_id = journal.state.operation_id.clone();
        let operation_kind = journal.state.operation_kind.clone();
        match journal.rollback_now(home) {
            Ok(affected_paths) => attempts.push(RecoveryAttempt {
                operation_id,
                operation_kind,
                recovered: true,
                affected_paths,
                error: None,
            }),
            Err(error) => {
                let message = error.to_string();
                let _ = journal.mark_rollback_required(format!(
                    "automatic startup rollback failed: {message}"
                ));
                attempts.push(RecoveryAttempt {
                    operation_id,
                    operation_kind,
                    recovered: false,
                    affected_paths: Vec::new(),
                    error: Some(message),
                });
                break;
            }
        }
    }
    Ok(RecoveryReport {
        attempted: true,
        writes_blocked: !incomplete_journals(home)?.is_empty(),
        attempts,
    })
}

fn create_recovery_payload(
    home: &Path,
    operation_id: &str,
    operation_kind: &str,
    targets: Vec<RecoveryTarget>,
    git_refs: Vec<GitRefRecovery>,
) -> Result<RecoveryPayload> {
    let asset_center = home.join(".my-agent-assets");
    let backup_root = guard_write_path(
        &asset_center,
        &asset_center
            .join("backups/local")
            .join(format!("recovery-{operation_id}")),
    )?;
    if backup_root.exists() {
        return Err(MaaError::new(format!(
            "recovery backup already exists: {}",
            backup_root.display()
        )));
    }
    fs::create_dir_all(backup_root.join("items"))?;

    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    for (index, target) in targets.into_iter().enumerate() {
        validate_recovery_target(home, &target.path, &target.authority)?;
        if !seen.insert(target.path.clone()) {
            continue;
        }
        let backup_path = if path_exists_no_follow(&target.path) {
            let backup = backup_root.join("items").join(index.to_string());
            copy_any(&target.path, &backup)?;
            sync_path(&backup)?;
            Some(backup)
        } else {
            None
        };
        entries.push(RecoveryEntry {
            target_path: target.path,
            authority: target.authority,
            backup_path,
        });
    }
    for git_ref in &git_refs {
        validate_git_ref_recovery(home, git_ref)?;
    }
    let affected = entries
        .iter()
        .map(|entry| format!("  - {}", entry.target_path.display()))
        .collect::<Vec<_>>()
        .join("\n");
    atomic_write(
        &backup_root.join("manifest.yaml"),
        format!(
            "schemaVersion: 1\noperation: recovery-{operation_kind}\noperationId: {operation_id}\nruntimePaths:\n{affected}\n"
        )
        .as_bytes(),
    )?;
    sync_directory(&backup_root)?;
    Ok(RecoveryPayload {
        backup_root,
        entries,
        git_refs,
    })
}

fn restore_payload(
    home: &Path,
    operation_id: &str,
    payload: &RecoveryPayload,
) -> Result<Vec<PathBuf>> {
    let expected_root = home
        .join(".my-agent-assets/backups/local")
        .join(format!("recovery-{operation_id}"));
    if payload.backup_root != expected_root {
        return Err(MaaError::new(format!(
            "recovery backup root does not match operation '{}': {}",
            operation_id,
            payload.backup_root.display()
        )));
    }
    guard_existing_path(&home.join(".my-agent-assets"), &payload.backup_root)?;
    for entry in &payload.entries {
        validate_recovery_target(home, &entry.target_path, &entry.authority)?;
        if let Some(backup) = &entry.backup_path {
            guard_write_path(&payload.backup_root, backup)?;
            fs::symlink_metadata(backup)?;
        }
    }
    for git_ref in &payload.git_refs {
        validate_git_ref_recovery(home, git_ref)?;
    }

    let mut affected = Vec::new();
    for (index, entry) in payload.entries.iter().enumerate().rev() {
        if let Some(backup) = &entry.backup_path {
            let temporary = entry
                .target_path
                .with_extension(format!("maa-recovery-{operation_id}-{index}"));
            validate_recovery_target(home, &temporary, &entry.authority)?;
            remove_path_if_present(&temporary)?;
            copy_any(backup, &temporary)?;
            sync_path(&temporary)?;
            remove_path_if_present(&entry.target_path)?;
            if let Some(parent) = entry.target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(&temporary, &entry.target_path)?;
            sync_parent(&entry.target_path)?;
        } else {
            remove_path_if_present(&entry.target_path)?;
            sync_parent(&entry.target_path)?;
        }
        affected.push(entry.target_path.clone());
    }
    for git_ref in payload.git_refs.iter().rev() {
        restore_git_ref(git_ref)?;
        affected.push(git_ref.repository.clone());
    }
    Ok(affected)
}

fn validate_git_ref_recovery(home: &Path, recovery: &GitRefRecovery) -> Result<()> {
    let expected_repository = home.join(".my-agent-assets");
    if recovery.repository != expected_repository {
        return Err(MaaError::new(format!(
            "Git recovery repository is not the asset center: {}",
            recovery.repository.display()
        )));
    }
    guard_existing_path(home, &recovery.repository)?;
    if !recovery.repository.join(".git").is_dir() {
        return Err(MaaError::new("Git recovery repository is not initialized"));
    }
    validate_git_reference(&recovery.reference)?;
    if let Some(oid) = &recovery.old_oid {
        validate_git_oid(oid)?;
    }
    if let Some(oid) = &recovery.expected_oid {
        validate_git_oid(oid)?;
    }
    Ok(())
}

fn validate_git_reference(reference: &str) -> Result<()> {
    let suffix = reference
        .strip_prefix("refs/heads/")
        .ok_or_else(|| MaaError::new("Git recovery only supports local branch refs"))?;
    let valid = !suffix.is_empty()
        && !suffix.starts_with('-')
        && !suffix.contains("..")
        && !suffix.contains("@{")
        && !suffix.ends_with('.')
        && !suffix.ends_with('/')
        && suffix
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/'));
    if valid {
        Ok(())
    } else {
        Err(MaaError::new(format!(
            "unsafe Git recovery reference: {reference}"
        )))
    }
}

fn validate_git_oid(oid: &str) -> Result<()> {
    if (oid.len() == 40 || oid.len() == 64) && oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(MaaError::new("invalid Git recovery object id"))
    }
}

fn restore_git_ref(recovery: &GitRefRecovery) -> Result<()> {
    let current = git_output(
        &recovery.repository,
        &["rev-parse", "--verify", &recovery.reference],
    )
    .ok();
    if current == recovery.old_oid {
        return Ok(());
    }
    let expected = recovery.expected_oid.as_deref().ok_or_else(|| {
        MaaError::new(format!(
            "Git ref '{}' changed before an expected recovery value was persisted",
            recovery.reference
        ))
    })?;
    if current.as_deref() != Some(expected) {
        return Err(MaaError::new(format!(
            "Git ref '{}' changed outside the interrupted transaction",
            recovery.reference
        )));
    }
    match &recovery.old_oid {
        Some(old_oid) => run_git(
            &recovery.repository,
            &["update-ref", &recovery.reference, old_oid, expected],
        )?,
        None => run_git(
            &recovery.repository,
            &["update-ref", "-d", &recovery.reference, expected],
        )?,
    }
    match &recovery.old_oid {
        Some(old_oid) => run_git(&recovery.repository, &["read-tree", old_oid])?,
        None => run_git(&recovery.repository, &["read-tree", "--empty"])?,
    }
    Ok(())
}

fn git_output(repository: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repository)
        .args(args)
        .output()
        .map_err(|error| MaaError::new(format!("cannot run Git recovery command: {error}")))?;
    if !output.status.success() {
        return Err(MaaError::new("Git recovery command failed"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_git(repository: &Path, args: &[&str]) -> Result<()> {
    git_output(repository, args).map(|_| ())
}

fn validate_recovery_target(home: &Path, path: &Path, authority: &RecoveryAuthority) -> Result<()> {
    match authority {
        RecoveryAuthority::AssetCenter => {
            guard_write_path(&home.join(".my-agent-assets"), path)?;
        }
        RecoveryAuthority::RegisteredTarget { target_id } => {
            let targets = load_targets(home)?;
            let target = targets.resolve(target_id)?;
            if path == target.path {
                let parent = target.path.parent().ok_or_else(|| {
                    MaaError::new(format!("registered target '{}' has no parent", target.id))
                })?;
                guard_write_path(parent, path)?;
            } else {
                guard_write_path(&target.path, path)?;
            }
        }
    }
    Ok(())
}

fn load_journals(home: &Path) -> Result<Vec<JournalFile>> {
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
        if journal.schema_version == 0 || journal.schema_version > JOURNAL_SCHEMA_VERSION {
            return Err(MaaError::new(format!(
                "unsupported operation journal schemaVersion {}",
                journal.schema_version
            )));
        }
        journals.push(journal);
    }
    journals.sort_by(|left, right| {
        left.created_at_epoch_seconds
            .cmp(&right.created_at_epoch_seconds)
            .then_with(|| left.operation_id.cmp(&right.operation_id))
    });
    Ok(journals)
}

fn journal_path(home: &Path, operation_id: &str) -> Result<PathBuf> {
    validate_operation_id(operation_id)?;
    guard_write_path(
        &home.join(".my-agent-assets"),
        &home
            .join(".my-agent-assets/operations")
            .join(format!("{operation_id}.yaml")),
    )
    .map_err(Into::into)
}

fn validate_operation_id(operation_id: &str) -> Result<()> {
    let valid = !operation_id.is_empty()
        && operation_id.len() <= 160
        && operation_id != "."
        && operation_id != ".."
        && operation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if valid {
        Ok(())
    } else {
        Err(MaaError::new(format!(
            "unsafe operation id: {operation_id:?}"
        )))
    }
}

fn create_lock_file(path: &Path) -> std::io::Result<fs::File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

fn reclaim_stale_lock(path: &Path) -> Result<()> {
    let text = fs::read_to_string(path).map_err(|error| {
        MaaError::new(format!(
            "cannot inspect existing operation lock {}: {error}",
            path.display()
        ))
    })?;
    let pid = text
        .lines()
        .find_map(|line| line.strip_prefix("pid="))
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| {
            MaaError::new(format!(
                "existing operation lock is malformed and cannot be reclaimed: {}",
                path.display()
            ))
        })?;
    if process_is_alive(pid) {
        return Err(MaaError::new(format!(
            "another asset operation is active (pid {pid}, {})",
            path.display()
        )));
    }
    fs::remove_file(path).map_err(|error| {
        MaaError::new(format!(
            "cannot reclaim stale operation lock {}: {error}",
            path.display()
        ))
    })
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    if pid > i32::MAX as u32 {
        return false;
    }
    let result = unsafe { libc::kill(pid as i32, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(windows)]
fn process_is_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut exit_code = 0;
        let active = GetExitCodeProcess(handle, &mut exit_code) != 0 && exit_code == STILL_ACTIVE;
        CloseHandle(handle);
        active
    }
}

#[cfg(not(any(unix, windows)))]
fn process_is_alive(_pid: u32) -> bool {
    true
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| MaaError::new(format!("path has no parent: {}", path.display())))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".{}.tmp-{}-{}",
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("journal"),
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        sync_directory(parent)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
}

fn sync_path(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return sync_parent(path);
    }
    if metadata.is_file() {
        OpenOptions::new().read(true).open(path)?.sync_all()?;
        return sync_parent(path);
    }
    for entry in fs::read_dir(path)? {
        sync_path(&entry?.path())?;
    }
    sync_directory(path)?;
    sync_parent(path)
}

fn sync_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        sync_directory(parent)?;
    }
    Ok(())
}

fn sync_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        OpenOptions::new().read(true).open(path)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

fn path_exists_no_follow(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
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
    use crate::mount_registry::{save as save_mounts, MountRegistry};
    use crate::targets::{save as save_targets, MountAdapter, ProviderState, TargetRegistry};
    use std::process::Command;

    fn home(name: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!(
            "maa-operation-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(home.join(".my-agent-assets/backups/local")).unwrap();
        fs::create_dir_all(home.join(".my-agent-assets/operations")).unwrap();
        let targets = TargetRegistry::standard_user_targets(
            &home,
            ProviderState::Initialized,
            ProviderState::Initialized,
            if cfg!(windows) {
                MountAdapter::WindowsDirectoryJunction
            } else {
                MountAdapter::SymlinkDirectory
            },
        )
        .unwrap();
        save_targets(&home, &targets).unwrap();
        save_mounts(&home, &MountRegistry::default()).unwrap();
        home
    }

    fn test_git(repository: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(repository)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[test]
    fn lock_is_exclusive_and_journal_tracks_incomplete_state() {
        let home = home("lock");
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

    #[test]
    fn stale_lock_is_reclaimed_but_live_lock_is_not() {
        let home = home("stale-lock");
        let lock_path = home.join(".my-agent-assets/locks/global.lock");
        fs::create_dir_all(lock_path.parent().unwrap()).unwrap();
        fs::write(&lock_path, "pid=4294967295\ncreatedAtEpochSeconds=1").unwrap();
        let lock = OperationLock::acquire(&home).unwrap();
        drop(lock);

        fs::write(
            &lock_path,
            format!(
                "pid={}\ncreatedAtEpochSeconds={}",
                std::process::id(),
                epoch_seconds()
            ),
        )
        .unwrap();
        assert!(OperationLock::acquire(&home)
            .unwrap_err()
            .to_string()
            .contains("active"));
        let _ = fs::remove_file(lock_path);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn recoverable_journal_restores_file_directory_symlink_and_missing_path() {
        let home = home("recover");
        let root = home.join(".my-agent-assets");
        let file = root.join("assets.yaml");
        let directory = root.join("assets/skills/review");
        let missing = root.join("assets/commands/new.md");
        fs::create_dir_all(&directory).unwrap();
        fs::write(&file, "before").unwrap();
        fs::write(directory.join("SKILL.md"), "before skill").unwrap();

        let mut targets = vec![
            RecoveryTarget::asset_center(&file),
            RecoveryTarget::asset_center(&directory),
            RecoveryTarget::asset_center(&missing),
        ];
        #[cfg(unix)]
        {
            let link = root.join("assets/skills/review-link");
            std::os::unix::fs::symlink(&directory, &link).unwrap();
            targets.push(RecoveryTarget::asset_center(link));
        }

        let journal =
            OperationJournal::start_recoverable(&home, "recover-1", "test", targets).unwrap();
        fs::write(&file, "after").unwrap();
        fs::remove_dir_all(&directory).unwrap();
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("SKILL.md"), "after skill").unwrap();
        fs::create_dir_all(missing.parent().unwrap()).unwrap();
        fs::write(&missing, "created during operation").unwrap();
        drop(journal);

        let report = recover_incomplete(&home).unwrap();
        assert!(report.attempted);
        assert!(!report.writes_blocked, "{report:?}");
        assert_eq!(report.attempts.len(), 1);
        assert!(report.attempts[0].recovered);
        assert_eq!(fs::read_to_string(&file).unwrap(), "before");
        assert_eq!(
            fs::read_to_string(directory.join("SKILL.md")).unwrap(),
            "before skill"
        );
        assert!(!missing.exists());
        let status = recovery_status(&home).unwrap();
        assert!(!status.writes_blocked);
        assert_eq!(status.recent_recoveries.len(), 1);
        assert_eq!(status.recent_recoveries[0].status, JournalStatus::Recovered);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn recoverable_journal_restores_asset_center_git_ref_and_index() {
        let home = home("git-ref");
        let repository = home.join(".my-agent-assets");
        test_git(&repository, &["init", "-b", "main"]);
        test_git(
            &repository,
            &["config", "user.email", "test@example.invalid"],
        );
        test_git(&repository, &["config", "user.name", "Test"]);
        let asset = repository.join("assets.yaml");
        fs::write(&asset, "schemaVersion: 1\nassets: {}\n").unwrap();
        test_git(&repository, &["add", "assets.yaml"]);
        test_git(&repository, &["commit", "-m", "initial"]);
        let old_oid = test_git(&repository, &["rev-parse", "HEAD"]);

        let mut journal = OperationJournal::start_recoverable_with_git(
            &home,
            "git-crash",
            "git-sync",
            vec![RecoveryTarget::asset_center(asset.clone())],
            vec![GitRefRecovery {
                repository: repository.clone(),
                reference: "refs/heads/main".into(),
                old_oid: Some(old_oid.clone()),
                expected_oid: Some(old_oid.clone()),
            }],
        )
        .unwrap();
        fs::write(&asset, "schemaVersion: 1\nassets: changed\n").unwrap();
        test_git(&repository, &["add", "assets.yaml"]);
        test_git(&repository, &["commit", "-m", "changed"]);
        let changed_oid = test_git(&repository, &["rev-parse", "HEAD"]);
        journal
            .set_git_ref_expected("refs/heads/main", &changed_oid)
            .unwrap();
        drop(journal);

        let report = recover_incomplete(&home).unwrap();
        assert!(report.attempted);
        assert!(!report.writes_blocked);
        assert_eq!(test_git(&repository, &["rev-parse", "HEAD"]), old_oid);
        assert_eq!(
            fs::read_to_string(&asset).unwrap(),
            "schemaVersion: 1\nassets: {}\n"
        );
        assert!(test_git(&repository, &["diff", "--cached", "--quiet"]).is_empty());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn tampered_recovery_payload_stays_blocked_without_outside_write() {
        let home = home("tampered");
        let outside = home
            .parent()
            .unwrap()
            .join("maa-operation-outside-sentinel");
        fs::write(&outside, "safe").unwrap();
        let mut journal = OperationJournal::start_recoverable(
            &home,
            "tampered-1",
            "test",
            vec![RecoveryTarget::asset_center(
                home.join(".my-agent-assets/assets.yaml"),
            )],
        )
        .unwrap();
        journal.state.recovery.as_mut().unwrap().entries[0].target_path = outside.clone();
        journal.persist().unwrap();
        drop(journal);

        let report = recover_incomplete(&home).unwrap();
        assert!(report.writes_blocked);
        assert!(!report.attempts[0].recovered);
        assert_eq!(fs::read_to_string(&outside).unwrap(), "safe");
        assert!(OperationLock::acquire(&home).is_err());
        let _ = fs::remove_file(outside);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn legacy_incomplete_journal_cannot_be_falsely_marked_recovered() {
        let home = home("legacy");
        fs::write(
            home.join(".my-agent-assets/operations/legacy.yaml"),
            "schemaVersion: 1\noperationId: legacy\noperationKind: mount\nstatus: started\ncreatedAtEpochSeconds: 1\ncompletedSteps: []\n",
        )
        .unwrap();
        let report = recover_incomplete(&home).unwrap();
        assert!(report.writes_blocked);
        assert!(!report.attempts[0].recovered);
        assert!(report.attempts[0]
            .error
            .as_deref()
            .unwrap()
            .contains("no persistent recovery payload"));
        let _ = fs::remove_dir_all(home);
    }
}
