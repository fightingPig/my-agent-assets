use super::contracts::{
    AppearanceTheme, DensityPreference, DesktopSettings, LogLevel, SettingsSaveInput,
};
use super::settings::{settings_load_for_home, settings_save_for_home};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempHome {
    path: PathBuf,
}

impl TempHome {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "my-agent-assets-settings-{}-{}-{}",
            name,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&path).expect("temp home should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn config_path(&self) -> PathBuf {
        self.path.join(".my-agent-assets/config.yaml")
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn custom_settings(home: &Path) -> DesktopSettings {
    DesktopSettings {
        asset_center_path: home.join("custom-assets").to_string_lossy().into_owned(),
        scan_roots: vec![
            home.join("workspace").to_string_lossy().into_owned(),
            home.join("code").to_string_lossy().into_owned(),
        ],
        max_depth: 7,
        backup_before_apply: false,
        backup_warning_threshold_bytes: 2 * 1024 * 1024 * 1024,
        plan_only_by_default: false,
        git_default_branch: "trunk".into(),
        git_remote: "upstream".into(),
        appearance_theme: AppearanceTheme::Dark,
        density: DensityPreference::Comfortable,
        log_level: LogLevel::Debug,
        log_retention_days: 30,
        cli_path: "maa-dev".into(),
    }
}

#[test]
fn settings_load_missing_config_returns_defaults_without_creating_files() {
    let home = TempHome::new("load-defaults");

    let settings = settings_load_for_home(home.path()).expect("defaults should load");

    assert_eq!(
        settings.asset_center_path,
        home.path().join(".my-agent-assets").to_string_lossy()
    );
    assert_eq!(settings.max_depth, 5);
    assert!(!home.config_path().exists());
}

#[test]
fn settings_save_writes_config_and_settings_load_reads_it_back() {
    let home = TempHome::new("save-load");
    let input = custom_settings(home.path());

    let saved = settings_save_for_home(
        home.path(),
        SettingsSaveInput {
            settings: input.clone(),
        },
    )
    .expect("settings should save");
    let loaded = settings_load_for_home(home.path()).expect("saved settings should load");

    let expected_asset_center = home
        .path()
        .join(".my-agent-assets")
        .to_string_lossy()
        .into_owned();
    assert_eq!(saved.asset_center_path, expected_asset_center);
    assert_eq!(loaded, saved);
    assert_eq!(saved.scan_roots, input.scan_roots);
    assert!(home.config_path().exists());
    let raw = fs::read_to_string(home.config_path()).expect("config should be readable");
    assert!(!raw.contains("assetCenterPath"));
    assert!(raw.contains("appearanceTheme: dark"));
}

#[test]
fn settings_save_normalizes_empty_and_out_of_range_values() {
    let home = TempHome::new("normalize");
    let mut settings = custom_settings(home.path());
    settings.asset_center_path = " ".into();
    settings.scan_roots = vec![" ".into(), "".into()];
    settings.max_depth = 0;
    settings.log_retention_days = 999;
    settings.git_default_branch = "".into();
    settings.git_remote = " ".into();
    settings.cli_path = "".into();

    let saved = settings_save_for_home(home.path(), SettingsSaveInput { settings })
        .expect("settings should save");

    assert_eq!(
        saved.asset_center_path,
        home.path().join(".my-agent-assets").to_string_lossy()
    );
    assert_eq!(saved.scan_roots.len(), 3);
    assert_eq!(saved.max_depth, 1);
    assert_eq!(saved.log_retention_days, 365);
    assert_eq!(saved.git_default_branch, "main");
    assert_eq!(saved.git_remote, "origin");
    assert_eq!(saved.cli_path, "maa");
}

#[test]
fn settings_load_invalid_config_returns_error_without_overwriting() {
    let home = TempHome::new("invalid");
    fs::create_dir_all(home.config_path().parent().unwrap()).expect("config parent should exist");
    fs::write(home.config_path(), "{").expect("invalid config should be written");

    let error = settings_load_for_home(home.path()).expect_err("invalid YAML should fail");

    assert!(error.contains("invalid settings YAML"));
    assert_eq!(
        fs::read_to_string(home.config_path()).expect("invalid config should remain"),
        "{"
    );
}

#[test]
fn settings_save_rejects_symlinked_asset_center_without_writing_outside_home() {
    let home = TempHome::new("symlink-escape");
    let outside = TempHome::new("symlink-outside");
    let link = home.path().join(".my-agent-assets");
    create_test_directory_symlink(outside.path(), &link);

    let result = settings_save_for_home(
        home.path(),
        SettingsSaveInput {
            settings: custom_settings(home.path()),
        },
    );

    assert!(result.is_err());
    assert!(result
        .expect_err("symlink path must fail")
        .contains("Allowed root must not be a symlink"));
    assert!(!outside.config_path().exists());
}

#[test]
fn settings_save_ignores_inactive_asset_center_path_setting() {
    let home = TempHome::new("fixed-asset-center");
    let mut settings = custom_settings(home.path());
    settings.asset_center_path = home.path().join("ignored").to_string_lossy().into_owned();

    let saved = settings_save_for_home(home.path(), SettingsSaveInput { settings })
        .expect("settings should save");

    assert_eq!(
        saved.asset_center_path,
        home.path().join(".my-agent-assets").to_string_lossy()
    );
    assert!(!home.path().join("ignored").exists());
}

#[cfg(unix)]
fn create_test_directory_symlink(source: &Path, destination: &Path) {
    std::os::unix::fs::symlink(source, destination).expect("directory symlink should be created");
}

#[cfg(windows)]
fn create_test_directory_symlink(source: &Path, destination: &Path) {
    std::os::windows::fs::symlink_dir(source, destination)
        .expect("directory symlink should be created");
}
