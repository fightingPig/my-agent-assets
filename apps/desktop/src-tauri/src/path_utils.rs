use std::env;
use std::path::PathBuf;
use std::time::SystemTime;

pub use my_agent_assets_core::path_safety::{
    display_path, expand_tilde, guard_existing_path, guard_write_path,
    validate_single_path_component,
};

pub fn home_dir() -> Option<PathBuf> {
    env::var_os("MY_AGENT_ASSETS_HOME")
        .or_else(|| env::var_os("HOME"))
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub fn modified_time_iso(time: SystemTime) -> String {
    humantime::format_rfc3339_seconds(time).to_string()
}
