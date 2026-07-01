pub mod adopt;
pub mod asset_registry;
pub mod backup_history;
pub mod batch_import;
pub mod delete;
pub mod diagnostics;
pub mod discovery;
pub mod fingerprint;
pub mod git_sync;
pub mod import;
pub mod initialization;
pub mod mcp;
pub mod mcp_management;
pub mod mount;
pub mod mount_registry;
pub mod operation;
pub mod path_safety;
pub mod query;
pub mod settings;
pub mod target_management;
pub mod targets;

use std::fmt::{self, Display};

pub type Result<T> = std::result::Result<T, MaaError>;

#[derive(Debug)]
pub struct MaaError {
    message: String,
}

impl MaaError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for MaaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for MaaError {}

impl From<std::io::Error> for MaaError {
    fn from(value: std::io::Error) -> Self {
        Self::new(value.to_string())
    }
}
