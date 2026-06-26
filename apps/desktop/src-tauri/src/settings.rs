use crate::contracts::{DesktopSettings, SettingsSaveInput};
use crate::path_utils::home_dir;
use crate::read_only::settings_for_home;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn settings_load_command() -> DesktopSettings {
    match home_dir() {
        Some(home) => settings_load_for_home(&home),
        None => settings_for_home(None),
    }
}

#[tauri::command]
pub fn settings_save_command(input: SettingsSaveInput) -> DesktopSettings {
    match home_dir() {
        Some(home) => {
            settings_save_for_home(&home, input).unwrap_or_else(|_| settings_for_home(Some(&home)))
        }
        None => settings_for_home(None),
    }
}

pub fn settings_load_for_home(home: &Path) -> DesktopSettings {
    let path = settings_path(home);
    let Ok(text) = fs::read_to_string(&path) else {
        return settings_for_home(Some(home));
    };
    serde_json::from_str::<DesktopSettings>(&text)
        .map(|settings| normalize_settings(home, settings))
        .unwrap_or_else(|_| settings_for_home(Some(home)))
}

pub fn settings_save_for_home(
    home: &Path,
    input: SettingsSaveInput,
) -> io::Result<DesktopSettings> {
    let settings = normalize_settings(home, input.settings);
    let path = settings_path(home);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let bytes = serde_json::to_vec_pretty(&settings).map_err(io::Error::other)?;
    write_json_verified(&path, &bytes)?;
    Ok(settings)
}

fn settings_path(home: &Path) -> PathBuf {
    home.join(".my-agent-assets").join("config.json")
}

fn normalize_settings(home: &Path, mut settings: DesktopSettings) -> DesktopSettings {
    let defaults = settings_for_home(Some(home));

    if settings.asset_center_path.trim().is_empty() {
        settings.asset_center_path = defaults.asset_center_path;
    }
    settings.scan_roots = settings
        .scan_roots
        .into_iter()
        .map(|root| root.trim().to_string())
        .filter(|root| !root.is_empty())
        .collect();
    if settings.scan_roots.is_empty() {
        settings.scan_roots = defaults.scan_roots;
    }
    settings.max_depth = settings.max_depth.clamp(1, 20);
    settings.log_retention_days = settings.log_retention_days.clamp(1, 365);
    if settings.git_default_branch.trim().is_empty() {
        settings.git_default_branch = defaults.git_default_branch;
    }
    if settings.git_remote.trim().is_empty() {
        settings.git_remote = defaults.git_remote;
    }
    if settings.cli_path.trim().is_empty() {
        settings.cli_path = defaults.cli_path;
    }
    settings
}

fn write_json_verified(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp = path.with_file_name(format!(
        ".tmp-my-agent-assets-settings-{}",
        std::process::id()
    ));
    fs::write(&temp, bytes)?;
    let written = fs::read(&temp)?;
    if written != bytes {
        let _ = fs::remove_file(&temp);
        return Err(io::Error::other("Temporary settings verification failed."));
    }
    let parsed: DesktopSettings = serde_json::from_slice(&written).map_err(io::Error::other)?;
    if parsed.asset_center_path.trim().is_empty() {
        let _ = fs::remove_file(&temp);
        return Err(io::Error::other("Verified settings are invalid."));
    }
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&temp, path)?;
    let final_text = fs::read_to_string(path)?;
    serde_json::from_str::<DesktopSettings>(&final_text).map_err(io::Error::other)?;
    Ok(())
}
