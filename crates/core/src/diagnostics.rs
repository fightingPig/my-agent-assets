use crate::asset_registry::{inspect_content, load as load_assets, ContentState};
use crate::initialization::preview_initialization;
use crate::mount_registry::load as load_mounts;
use crate::operation::incomplete_journals;
use crate::targets::{load as load_targets, ProviderState, TargetStatus};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoctorCheckStatus {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorCheck {
    pub id: String,
    pub label: String,
    pub status: DoctorCheckStatus,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorReport {
    pub asset_center_path: String,
    pub initialized: bool,
    pub checks: Vec<DoctorCheck>,
}

pub fn doctor(home: &Path) -> DoctorReport {
    let root = home.join(".my-agent-assets");
    let mut checks = Vec::new();
    checks.push(git_check());

    let initialized = match preview_initialization(home) {
        Ok(preview) if preview.already_initialized => {
            checks.push(check(
                "asset_center",
                "资产中心",
                DoctorCheckStatus::Ok,
                "资产中心结构、schema 和 Git repository 有效。",
            ));
            true
        }
        Ok(preview) if preview.can_apply => {
            checks.push(check(
                "asset_center",
                "资产中心",
                DoctorCheckStatus::Warning,
                "资产中心尚未初始化；请先执行初始化 Preview/Apply。",
            ));
            false
        }
        Ok(preview) => {
            checks.push(check(
                "asset_center",
                "资产中心",
                DoctorCheckStatus::Error,
                preview
                    .warnings
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "资产中心状态无效。".into()),
            ));
            false
        }
        Err(error) => {
            checks.push(check(
                "asset_center",
                "资产中心",
                DoctorCheckStatus::Error,
                error.to_string(),
            ));
            false
        }
    };

    if initialized {
        checks.push(asset_registry_check(home));
        checks.push(target_registry_check(home));
        checks.push(mount_registry_check(home));
        checks.push(operation_check(home));
    }
    checks.push(runtime_check(
        "claude_runtime",
        "Claude Code Runtime",
        home.join(".claude").exists() || home.join(".claude.json").exists(),
    ));
    checks.push(runtime_check(
        "codex_runtime",
        "Codex Runtime",
        home.join(".codex").exists(),
    ));
    checks.push(platform_mount_check());

    DoctorReport {
        asset_center_path: root.to_string_lossy().into_owned(),
        initialized,
        checks,
    }
}

fn git_check() -> DoctorCheck {
    match Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => check(
            "git",
            "Git",
            DoctorCheckStatus::Ok,
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ),
        Ok(_) => check("git", "Git", DoctorCheckStatus::Error, "Git 命令执行失败。"),
        Err(error) => check(
            "git",
            "Git",
            DoctorCheckStatus::Error,
            format!("Git 不可用：{error}"),
        ),
    }
}

fn asset_registry_check(home: &Path) -> DoctorCheck {
    match load_assets(home) {
        Ok(registry) => match inspect_content(home, &registry) {
            Ok(entries) => {
                let invalid = entries
                    .iter()
                    .filter(|entry| entry.state != ContentState::Ready)
                    .count();
                if invalid == 0 {
                    check(
                        "asset_registry",
                        "资产索引",
                        DoctorCheckStatus::Ok,
                        format!("{} 个 canonical assets 一致。", entries.len()),
                    )
                } else {
                    check(
                        "asset_registry",
                        "资产索引",
                        DoctorCheckStatus::Warning,
                        format!("{invalid} 个资产存在 missing、invalid 或 unregistered 状态。"),
                    )
                }
            }
            Err(error) => check(
                "asset_registry",
                "资产索引",
                DoctorCheckStatus::Error,
                error.to_string(),
            ),
        },
        Err(error) => check(
            "asset_registry",
            "资产索引",
            DoctorCheckStatus::Error,
            error.to_string(),
        ),
    }
}

fn target_registry_check(home: &Path) -> DoctorCheck {
    match load_targets(home) {
        Ok(registry) => {
            let blocked = registry
                .targets
                .iter()
                .filter(|target| {
                    target.status != TargetStatus::Ready
                        || target.provider_state != ProviderState::Initialized
                })
                .count();
            let status = if blocked == 0 {
                DoctorCheckStatus::Ok
            } else {
                DoctorCheckStatus::Warning
            };
            check(
                "target_registry",
                "Target Registry",
                status,
                format!(
                    "{} 个 targets，{blocked} 个因 provider 未初始化或配置无效而阻止。",
                    registry.targets.len()
                ),
            )
        }
        Err(error) => check(
            "target_registry",
            "Target Registry",
            DoctorCheckStatus::Error,
            error.to_string(),
        ),
    }
}

fn mount_registry_check(home: &Path) -> DoctorCheck {
    match load_mounts(home) {
        Ok(registry) => check(
            "mount_registry",
            "挂载索引",
            DoctorCheckStatus::Ok,
            format!("{} 个本机挂载关系。", registry.bindings.len()),
        ),
        Err(error) => check(
            "mount_registry",
            "挂载索引",
            DoctorCheckStatus::Error,
            error.to_string(),
        ),
    }
}

fn operation_check(home: &Path) -> DoctorCheck {
    match incomplete_journals(home) {
        Ok(journals) if journals.is_empty() => check(
            "operations",
            "事务恢复",
            DoctorCheckStatus::Ok,
            "没有未完成事务。",
        ),
        Ok(journals) => check(
            "operations",
            "事务恢复",
            DoctorCheckStatus::Error,
            format!("检测到 {} 个未完成事务，写入必须保持阻止。", journals.len()),
        ),
        Err(error) => check(
            "operations",
            "事务恢复",
            DoctorCheckStatus::Error,
            error.to_string(),
        ),
    }
}

fn runtime_check(id: &str, label: &str, initialized: bool) -> DoctorCheck {
    if initialized {
        check(id, label, DoctorCheckStatus::Ok, "已检测到本机配置。")
    } else {
        check(
            id,
            label,
            DoctorCheckStatus::Warning,
            "未检测到本机配置；不会自动创建。",
        )
    }
}

fn platform_mount_check() -> DoctorCheck {
    #[cfg(windows)]
    {
        check(
            "mount_mechanism",
            "Windows 挂载机制",
            DoctorCheckStatus::Warning,
            "Skill 使用 directory junction；Command 使用 file symlink，Apply 时会再次检查 Developer Mode/权限，禁止 copy 或 hardlink fallback。",
        )
    }
    #[cfg(not(windows))]
    {
        check(
            "mount_mechanism",
            "挂载机制",
            DoctorCheckStatus::Ok,
            "Skill directory 与 Command file 使用符号链接；MCP 使用 JSON/TOML renderer。",
        )
    }
}

fn check(
    id: impl Into<String>,
    label: impl Into<String>,
    status: DoctorCheckStatus,
    message: impl Into<String>,
) -> DoctorCheck {
    DoctorCheck {
        id: id.into(),
        label: label.into(),
        status,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::initialization::{apply_initialization, InitializationApplyRequest};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn home(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "maa-doctor-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn reports_uninitialized_without_writing() {
        let home = home("empty");
        let report = doctor(&home);
        assert!(!report.initialized);
        assert!(report
            .checks
            .iter()
            .any(|entry| entry.id == "asset_center" && entry.status == DoctorCheckStatus::Warning));
        assert_eq!(fs::read_dir(&home).unwrap().count(), 0);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn reports_initialized_registries_and_mount_mechanism() {
        let home = home("initialized");
        let preview = preview_initialization(&home).unwrap();
        apply_initialization(
            &home,
            &InitializationApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
            },
        )
        .unwrap();
        let report = doctor(&home);
        assert!(report.initialized);
        for id in [
            "asset_center",
            "asset_registry",
            "target_registry",
            "mount_registry",
            "operations",
            "mount_mechanism",
        ] {
            assert!(report.checks.iter().any(|entry| entry.id == id), "{id}");
        }
        let _ = fs::remove_dir_all(home);
    }
}
