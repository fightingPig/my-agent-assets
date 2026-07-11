use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCommandError {
    pub code: DesktopCommandErrorCode,
    pub message: &'static str,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DesktopCommandErrorCode {
    #[serde(rename = "environmentUnavailable")]
    EnvironmentUnavailable,
    #[serde(rename = "stalePreview")]
    StalePreview,
    #[serde(rename = "validationFailed")]
    ValidationFailed,
    #[serde(rename = "notInitialized")]
    NotInitialized,
    #[serde(rename = "operationBlocked")]
    OperationBlocked,
    #[serde(rename = "notFound")]
    NotFound,
    #[serde(rename = "operationFailed")]
    OperationFailed,
}

impl DesktopCommandError {
    pub fn from_core_message(message: String) -> Self {
        let lowercase = message.to_ascii_lowercase();
        let code = if lowercase.contains("home is unavailable") {
            DesktopCommandErrorCode::EnvironmentUnavailable
        } else if lowercase.contains("stale")
            || lowercase.contains("preview") && lowercase.contains("expired")
        {
            DesktopCommandErrorCode::StalePreview
        } else if lowercase.contains("not initialized") || lowercase.contains("uninitialized") {
            DesktopCommandErrorCode::NotInitialized
        } else if lowercase.contains("not found") || lowercase.contains("does not exist") {
            DesktopCommandErrorCode::NotFound
        } else if lowercase.contains("blocked")
            || lowercase.contains("incompatible")
            || lowercase.contains("incomplete operation")
        {
            DesktopCommandErrorCode::OperationBlocked
        } else if lowercase.contains("invalid")
            || lowercase.contains("malformed")
            || lowercase.contains("must ")
            || lowercase.contains("required")
        {
            DesktopCommandErrorCode::ValidationFailed
        } else {
            DesktopCommandErrorCode::OperationFailed
        };

        let message = match code {
            DesktopCommandErrorCode::EnvironmentUnavailable => {
                "无法读取本机环境。请检查系统账户目录后重试。"
            }
            DesktopCommandErrorCode::StalePreview => "预览已过期或相关内容已变化。请重新生成预览。",
            DesktopCommandErrorCode::ValidationFailed => {
                "输入或本地配置不符合要求。请检查后重新预览。"
            }
            DesktopCommandErrorCode::NotInitialized => {
                "目标客户端尚未完成初始化。请先启动对应客户端后重试。"
            }
            DesktopCommandErrorCode::OperationBlocked => {
                "当前操作被安全检查阻止。请查看预览或诊断信息后重试。"
            }
            DesktopCommandErrorCode::NotFound => "未找到所需的本地资产或配置。请刷新后重试。",
            DesktopCommandErrorCode::OperationFailed => {
                "本地操作未完成。请查看系统状态或导出诊断包后重试。"
            }
        };

        Self {
            code,
            message,
            parameters: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DesktopCommandError, DesktopCommandErrorCode};

    #[test]
    fn serializes_a_stable_safe_error_shape() {
        let error = DesktopCommandError::from_core_message(
            "stale preview /tmp/private token=secret".to_string(),
        );
        let value = serde_json::to_value(error).unwrap();

        assert_eq!(value["code"], "stalePreview");
        assert_eq!(
            value["message"],
            "预览已过期或相关内容已变化。请重新生成预览。"
        );
        assert_eq!(value["parameters"], serde_json::json!({}));
        assert!(!value.to_string().contains("/tmp/private"));
        assert!(!value.to_string().contains("secret"));
    }

    #[test]
    fn classifies_stable_error_codes() {
        assert_eq!(
            DesktopCommandError::from_core_message("HOME is unavailable".to_string()).code,
            DesktopCommandErrorCode::EnvironmentUnavailable
        );
        assert_eq!(
            DesktopCommandError::from_core_message("invalid schema".to_string()).code,
            DesktopCommandErrorCode::ValidationFailed
        );
        assert_eq!(
            DesktopCommandError::from_core_message("target is blocked".to_string()).code,
            DesktopCommandErrorCode::OperationBlocked
        );
    }
}
