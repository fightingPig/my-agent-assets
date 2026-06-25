use std::env;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn home_dir() -> Option<PathBuf> {
    env::var_os("MY_AGENT_ASSETS_HOME")
        .or_else(|| env::var_os("HOME"))
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub fn expand_tilde(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        return home.to_path_buf();
    }

    if let Some(rest) = path.strip_prefix("~/") {
        return home.join(rest);
    }

    PathBuf::from(path)
}

pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub fn modified_time_iso(time: SystemTime) -> String {
    humantime::format_rfc3339_seconds(time).to_string()
}
