#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub platform: &'static str,
    pub arch: &'static str,
    pub backend_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetType {
    #[serde(rename = "skill")]
    Skill,
    #[serde(rename = "command")]
    Command,
    #[serde(rename = "mcp")]
    Mcp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetStatus {
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "mounted")]
    Mounted,
    #[serde(rename = "unmounted")]
    Unmounted,
    #[serde(rename = "conflict")]
    Conflict,
    #[serde(rename = "invalid")]
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectStatus {
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "changed")]
    Changed,
    #[serde(rename = "needsSync")]
    NeedsSync,
    #[serde(rename = "invalid")]
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeScope {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "project")]
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    #[serde(rename = "skip")]
    Skip,
    #[serde(rename = "rename")]
    Rename,
    #[serde(rename = "overwrite")]
    Overwrite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStepKind {
    #[serde(rename = "check")]
    Check,
    #[serde(rename = "import")]
    Import,
    #[serde(rename = "mount")]
    Mount,
    #[serde(rename = "compileMcp")]
    CompileMcp,
    #[serde(rename = "backup")]
    Backup,
    #[serde(rename = "restore")]
    Restore,
    #[serde(rename = "git")]
    Git,
    #[serde(rename = "settings")]
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppearanceTheme {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "dark")]
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DensityPreference {
    #[serde(rename = "compact")]
    Compact,
    #[serde(rename = "comfortable")]
    Comfortable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "debug")]
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ScanScope {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "project")]
    Project {
        #[serde(rename = "projectPath")]
        project_path: String,
    },
    #[serde(rename = "custom")]
    Custom { path: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetCounts {
    pub total: u32,
    pub skills: u32,
    pub commands: u32,
    pub mcps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSummary {
    pub id: String,
    pub name: String,
    pub title: String,
    pub asset_type: AssetType,
    pub status: AssetStatus,
    pub category: String,
    pub description: String,
    pub source_path: String,
    pub scope: Option<RuntimeScope>,
    pub updated_at: Option<String>,
    pub mount_targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub title: String,
    pub path: String,
    pub status: ProjectStatus,
    pub description: String,
    pub updated_at: Option<String>,
    pub asset_counts: AssetCounts,
    pub mounts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanStep {
    pub id: String,
    pub kind: PlanStepKind,
    pub label: String,
    pub description: String,
    pub risk: RiskLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictResolutionChoice {
    pub conflict_id: String,
    pub resolution: ConflictResolution,
    pub rename_to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub scope: ScanScope,
    pub scanned_at: String,
    pub assets: Vec<AssetSummary>,
    pub counts: AssetCounts,
    pub conflict_count: u32,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictPreview {
    pub id: String,
    pub asset_id: String,
    pub asset_type: AssetType,
    pub name: String,
    pub reason: String,
    pub existing_content: String,
    pub incoming_content: String,
    pub allowed_resolutions: Vec<ConflictResolution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub scope: ScanScope,
    pub assets: Vec<AssetSummary>,
    pub conflicts: Vec<ConflictPreview>,
    pub steps: Vec<PlanStep>,
    pub warnings: Vec<String>,
    pub can_apply: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountTarget {
    pub scope: RuntimeScope,
    pub runtime_path: String,
    pub project_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountPreview {
    pub asset: AssetSummary,
    pub target: MountTarget,
    pub steps: Vec<PlanStep>,
    pub warnings: Vec<String>,
    pub backup_required: bool,
    pub can_apply: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub id: String,
    pub label: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub entry_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreview {
    pub backup: BackupSummary,
    pub affected_paths: Vec<String>,
    pub steps: Vec<PlanStep>,
    pub warnings: Vec<String>,
    pub backup_before_restore: bool,
    pub can_apply: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub repository_path: String,
    pub is_repository: bool,
    pub status_message: String,
    pub branch: String,
    pub remote: Option<String>,
    pub clean: bool,
    pub ahead: u32,
    pub behind: u32,
    pub changed_files: Vec<String>,
    pub conflicts: Vec<String>,
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSettings {
    pub asset_center_path: String,
    pub scan_roots: Vec<String>,
    pub max_depth: u32,
    pub backup_before_apply: bool,
    pub plan_only_by_default: bool,
    pub git_default_branch: String,
    pub git_remote: String,
    pub appearance_theme: AppearanceTheme,
    pub density: DensityPreference,
    pub log_level: LogLevel,
    pub log_retention_days: u32,
    pub cli_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanAssetsInput {
    pub scope: ScanScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewImportInput {
    pub scope: ScanScope,
    pub asset_ids: Vec<String>,
    pub conflict_resolutions: Vec<ConflictResolutionChoice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAssetsInput {
    pub asset_type: Option<AssetType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewMountInput {
    pub asset_id: String,
    pub target: MountTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewConflictsInput {
    pub scope: ScanScope,
    pub asset_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRestoreInput {
    pub backup_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSaveInput {
    pub settings: DesktopSettings,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use serde_json::{json, Value};

    fn wire_value<T: Serialize>(value: T) -> Value {
        serde_json::to_value(value).expect("contract value should serialize")
    }

    #[test]
    fn enum_wire_values_are_explicit_and_stable() {
        assert_eq!(wire_value(AssetType::Skill), json!("skill"));
        assert_eq!(wire_value(AssetType::Command), json!("command"));
        assert_eq!(wire_value(AssetType::Mcp), json!("mcp"));
        assert_eq!(wire_value(AssetStatus::Ready), json!("ready"));
        assert_eq!(wire_value(AssetStatus::Mounted), json!("mounted"));
        assert_eq!(wire_value(AssetStatus::Unmounted), json!("unmounted"));
        assert_eq!(wire_value(AssetStatus::Conflict), json!("conflict"));
        assert_eq!(wire_value(AssetStatus::Invalid), json!("invalid"));
        assert_eq!(wire_value(ProjectStatus::Ready), json!("ready"));
        assert_eq!(wire_value(ProjectStatus::Changed), json!("changed"));
        assert_eq!(wire_value(ProjectStatus::NeedsSync), json!("needsSync"));
        assert_eq!(wire_value(ProjectStatus::Invalid), json!("invalid"));
        assert_eq!(wire_value(RuntimeScope::User), json!("user"));
        assert_eq!(wire_value(RuntimeScope::Local), json!("local"));
        assert_eq!(wire_value(RuntimeScope::Project), json!("project"));
        assert_eq!(wire_value(ConflictResolution::Skip), json!("skip"));
        assert_eq!(wire_value(ConflictResolution::Rename), json!("rename"));
        assert_eq!(
            wire_value(ConflictResolution::Overwrite),
            json!("overwrite")
        );
        assert_eq!(wire_value(PlanStepKind::Check), json!("check"));
        assert_eq!(wire_value(PlanStepKind::Import), json!("import"));
        assert_eq!(wire_value(PlanStepKind::Mount), json!("mount"));
        assert_eq!(wire_value(PlanStepKind::CompileMcp), json!("compileMcp"));
        assert_eq!(wire_value(PlanStepKind::Backup), json!("backup"));
        assert_eq!(wire_value(PlanStepKind::Restore), json!("restore"));
        assert_eq!(wire_value(PlanStepKind::Git), json!("git"));
        assert_eq!(wire_value(PlanStepKind::Settings), json!("settings"));
        assert_eq!(wire_value(RiskLevel::None), json!("none"));
        assert_eq!(wire_value(RiskLevel::Low), json!("low"));
        assert_eq!(wire_value(RiskLevel::Medium), json!("medium"));
        assert_eq!(wire_value(RiskLevel::High), json!("high"));
        assert_eq!(wire_value(AppearanceTheme::System), json!("system"));
        assert_eq!(wire_value(AppearanceTheme::Light), json!("light"));
        assert_eq!(wire_value(AppearanceTheme::Dark), json!("dark"));
        assert_eq!(wire_value(DensityPreference::Compact), json!("compact"));
        assert_eq!(
            wire_value(DensityPreference::Comfortable),
            json!("comfortable")
        );
        assert_eq!(wire_value(LogLevel::Error), json!("error"));
        assert_eq!(wire_value(LogLevel::Warn), json!("warn"));
        assert_eq!(wire_value(LogLevel::Info), json!("info"));
        assert_eq!(wire_value(LogLevel::Debug), json!("debug"));
        assert_eq!(wire_value(ScanScope::User), json!({ "kind": "user" }));
        assert_eq!(
            wire_value(ScanScope::Project {
                project_path: "~/workspace/project-a".into()
            }),
            json!({ "kind": "project", "projectPath": "~/workspace/project-a" })
        );
        assert_eq!(
            wire_value(ScanScope::Custom {
                path: "~/code".into()
            }),
            json!({ "kind": "custom", "path": "~/code" })
        );
    }

    #[test]
    fn app_info_json_shape_is_unchanged() {
        let info = AppInfo {
            name: "My Agent Assets",
            version: "0.1.0",
            platform: "macos",
            arch: "arm64",
            backend_ready: true,
        };
        assert_eq!(
            wire_value(info),
            json!({
                "name": "My Agent Assets",
                "version": "0.1.0",
                "platform": "macos",
                "arch": "arm64",
                "backendReady": true
            })
        );
    }

    #[test]
    fn git_status_json_shape_includes_read_only_state() {
        let status = GitStatus {
            repository_path: "~/.my-agent-assets".into(),
            is_repository: false,
            status_message: "Asset center directory does not exist.".into(),
            branch: "".into(),
            remote: None,
            clean: true,
            ahead: 0,
            behind: 0,
            changed_files: vec![],
            conflicts: vec![],
            last_synced_at: None,
        };

        assert_eq!(
            wire_value(status),
            json!({
                "repositoryPath": "~/.my-agent-assets",
                "isRepository": false,
                "statusMessage": "Asset center directory does not exist.",
                "branch": "",
                "remote": null,
                "clean": true,
                "ahead": 0,
                "behind": 0,
                "changedFiles": [],
                "conflicts": [],
                "lastSyncedAt": null
            })
        );
    }

    #[test]
    fn preview_import_is_self_contained_and_round_trips() {
        let input = PreviewImportInput {
            scope: ScanScope::Project {
                project_path: "~/workspace/project-a".into(),
            },
            asset_ids: vec!["skill:review".into()],
            conflict_resolutions: vec![ConflictResolutionChoice {
                conflict_id: "mcp:PostgreSQL".into(),
                resolution: ConflictResolution::Rename,
                rename_to: Some("PostgreSQL-local".into()),
            }],
        };
        let value = wire_value(&input);
        assert_eq!(value["scope"]["kind"], json!("project"));
        assert_eq!(
            value["scope"]["projectPath"],
            json!("~/workspace/project-a")
        );
        assert_eq!(value["assetIds"], json!(["skill:review"]));
        assert!(value.get("scanId").is_none());
        assert!(value.get("sessionId").is_none());
        assert_eq!(
            serde_json::from_value::<PreviewImportInput>(value).unwrap(),
            input
        );
    }
}
