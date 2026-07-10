use crate::audit_log::{list_log_files, read_audit_entries, AuditLogEntry};
use crate::diagnostics::{doctor, DoctorCheckStatus, DoctorReport};
use crate::fingerprint::PreviewFingerprint;
use crate::operation::{OperationJournal, OperationLock, RecoveryTarget};
use crate::path_safety::guard_write_path;
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PREVIEW_TTL_SECONDS: u64 = 600;
const DIAGNOSTIC_SCHEMA_VERSION: u32 = 1;
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticExportPreview {
    pub preview_id: String,
    pub package_path: PathBuf,
    pub included_files: Vec<DiagnosticExportFile>,
    pub warnings: Vec<String>,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticExportFile {
    pub logical_path: String,
    pub kind: DiagnosticExportFileKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticExportFileKind {
    #[serde(rename = "audit_log")]
    AuditLog,
    #[serde(rename = "status_summary")]
    StatusSummary,
    #[serde(rename = "version_metadata")]
    VersionMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticExportApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticExportApplyResult {
    pub preview_id: String,
    pub package_path: PathBuf,
    pub journal_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticPackage {
    schema_version: u32,
    generated_at_epoch_seconds: u64,
    app_version: String,
    platform: String,
    arch: String,
    status: SanitizedDoctorSummary,
    audit_entries: Vec<AuditLogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SanitizedDoctorSummary {
    initialized: bool,
    checks: Vec<SanitizedCheck>,
    content_diagnostics: Vec<SanitizedContentDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SanitizedCheck {
    id: String,
    status: DoctorCheckStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SanitizedContentDiagnostic {
    asset_id: String,
    state: crate::asset_registry::ContentState,
}

pub fn preview_diagnostic_export(home: &Path) -> Result<DiagnosticExportPreview> {
    preview_diagnostic_export_at(home, epoch_seconds())
}

fn preview_diagnostic_export_at(
    home: &Path,
    generated_at_epoch_seconds: u64,
) -> Result<DiagnosticExportPreview> {
    let preview_id = export_fingerprint(home, generated_at_epoch_seconds)?;
    let package_path = package_path(home, generated_at_epoch_seconds);
    let mut included_files = vec![
        DiagnosticExportFile {
            logical_path: "status-summary.json".into(),
            kind: DiagnosticExportFileKind::StatusSummary,
        },
        DiagnosticExportFile {
            logical_path: "version-metadata.json".into(),
            kind: DiagnosticExportFileKind::VersionMetadata,
        },
    ];
    included_files.extend(list_log_files(home)?.into_iter().filter_map(|path| {
        path.file_name()
            .and_then(|value| value.to_str())
            .map(|name| DiagnosticExportFile {
                logical_path: format!("logs/{name}"),
                kind: DiagnosticExportFileKind::AuditLog,
            })
    }));
    Ok(DiagnosticExportPreview {
        preview_id,
        package_path,
        included_files,
        warnings: vec![
            "诊断包只包含脱敏状态摘要与 schema-valid 审计日志；不包含 canonical assets、live config、backup、settings 或用户配置。".into(),
        ],
        can_apply: true,
        generated_at_epoch_seconds,
        expires_at_epoch_seconds: generated_at_epoch_seconds.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

pub fn apply_diagnostic_export(
    home: &Path,
    request: &DiagnosticExportApplyRequest,
) -> Result<DiagnosticExportApplyResult> {
    let _lock = OperationLock::acquire(home)?;
    if epoch_seconds()
        > request
            .preview_generated_at_epoch_seconds
            .saturating_add(PREVIEW_TTL_SECONDS)
    {
        return Err(MaaError::new(
            "diagnostic export preview expired; generate a new preview before applying",
        ));
    }
    let preview = preview_diagnostic_export_at(home, request.preview_generated_at_epoch_seconds)?;
    if preview.preview_id != request.preview_id {
        return Err(MaaError::new(
            "diagnostic export preview is stale; generate a new preview before applying",
        ));
    }
    let root = home.join(".my-agent-assets");
    let output = guard_write_path(&root, &preview.package_path)?;
    let mut journal = OperationJournal::start_recoverable(
        home,
        &operation_id(),
        "diagnostic_export",
        vec![RecoveryTarget::asset_center(output.clone())],
    )?;
    let package = DiagnosticPackage {
        schema_version: DIAGNOSTIC_SCHEMA_VERSION,
        generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
        app_version: env!("CARGO_PKG_VERSION").into(),
        platform: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        status: sanitize_doctor(&doctor(home)),
        audit_entries: read_audit_entries(home)?,
    };
    let json =
        serde_json::to_vec_pretty(&package).map_err(|error| MaaError::new(error.to_string()))?;
    atomic_write(&output, &json)?;
    journal.record_step("diagnostic_package_written")?;
    journal.complete()?;
    Ok(DiagnosticExportApplyResult {
        preview_id: preview.preview_id,
        package_path: output,
        journal_path: journal.path().to_path_buf(),
    })
}

fn sanitize_doctor(report: &DoctorReport) -> SanitizedDoctorSummary {
    SanitizedDoctorSummary {
        initialized: report.initialized,
        checks: report
            .checks
            .iter()
            .map(|check| SanitizedCheck {
                id: check.id.clone(),
                status: check.status,
            })
            .collect(),
        content_diagnostics: report
            .content_diagnostics
            .iter()
            .map(|diagnostic| SanitizedContentDiagnostic {
                asset_id: diagnostic.asset_id.clone(),
                state: diagnostic.state,
            })
            .collect(),
    }
}

fn package_path(home: &Path, generated_at_epoch_seconds: u64) -> PathBuf {
    home.join(".my-agent-assets/logs/diagnostics")
        .join(format!("diagnostic-{generated_at_epoch_seconds}.json"))
}

fn export_fingerprint(home: &Path, generated_at_epoch_seconds: u64) -> Result<String> {
    let mut fingerprint = PreviewFingerprint::new("diagnostic-export");
    fingerprint.add_u64("generated-at", generated_at_epoch_seconds);
    for (label, path) in [
        ("asset-registry", home.join(".my-agent-assets/assets.yaml")),
        (
            "target-registry",
            home.join(".my-agent-assets/targets.yaml"),
        ),
        ("mount-registry", home.join(".my-agent-assets/mounts.yaml")),
        ("operations", home.join(".my-agent-assets/operations")),
        ("logs", home.join(".my-agent-assets/logs")),
    ] {
        fingerprint.add_path_if_present(label, &path)?;
    }
    Ok(fingerprint.finish("diagnostic-export"))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| MaaError::new("diagnostic package has no parent directory"))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".diagnostic-{}-{}.tmp",
        std::process::id(),
        epoch_seconds()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    if let Err(error) = (|| -> std::io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        OpenOptions::new().read(true).open(parent)?.sync_all()
    })() {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
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

    fn home(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "maa-diagnostic-export-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn export_is_preview_bound_and_contains_no_user_content_or_absolute_paths() {
        let home = home("safe");
        fs::create_dir_all(home.join(".my-agent-assets")).unwrap();
        crate::audit_log::append_operation(
            &home,
            "mcp_save",
            crate::audit_log::AuditOutcome::Completed,
        )
        .unwrap();
        let preview = preview_diagnostic_export(&home).unwrap();
        assert!(!preview.package_path.exists());
        let result = apply_diagnostic_export(
            &home,
            &DiagnosticExportApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
            },
        )
        .unwrap();
        let text = fs::read_to_string(result.package_path).unwrap();
        assert!(text.contains("mcp_save"));
        assert!(!text.contains(&home.to_string_lossy().to_string()));
        assert!(!text.contains("assetCenterPath"));
        assert!(!text.contains("assets.yaml"));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn changed_logs_invalidate_preview_without_writing_package() {
        let home = home("stale");
        fs::create_dir_all(home.join(".my-agent-assets")).unwrap();
        let preview = preview_diagnostic_export(&home).unwrap();
        crate::audit_log::append_operation(
            &home,
            "mount",
            crate::audit_log::AuditOutcome::Completed,
        )
        .unwrap();
        assert!(apply_diagnostic_export(
            &home,
            &DiagnosticExportApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
            }
        )
        .is_err());
        assert!(!preview.package_path.exists());
        let _ = fs::remove_dir_all(home);
    }
}
