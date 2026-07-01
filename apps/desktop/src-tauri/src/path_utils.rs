use std::env;
use std::path::PathBuf;

pub fn home_dir() -> Option<PathBuf> {
    env::var_os("MY_AGENT_ASSETS_HOME")
        .or_else(|| env::var_os("HOME"))
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}
