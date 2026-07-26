use crate::contracts::{
    AppearanceTheme, DensityPreference, DesktopSettings, LogLevel, SettingsApplyInput,
    SettingsApplyResult, SettingsPreview, SettingsPreviewInput,
};
use crate::path_utils::home_dir;
use my_agent_assets_core::settings::{
    self as core_settings, AppearanceTheme as CoreAppearanceTheme,
    DensityPreference as CoreDensityPreference, LogLevel as CoreLogLevel, Settings as CoreSettings,
};
use std::path::Path;

pub fn settings_load_command() -> Result<DesktopSettings, String> {
    let home = home_dir().ok_or_else(|| "Could not resolve HOME for settings load.".to_string())?;
    settings_load_for_home(&home)
}

pub fn settings_preview_command(input: SettingsPreviewInput) -> Result<SettingsPreview, String> {
    let home =
        home_dir().ok_or_else(|| "Could not resolve HOME for settings preview.".to_string())?;
    settings_preview_for_home(&home, input)
}

pub fn settings_apply_command(input: SettingsApplyInput) -> Result<SettingsApplyResult, String> {
    let home =
        home_dir().ok_or_else(|| "Could not resolve HOME for settings apply.".to_string())?;
    settings_apply_for_home(&home, input)
}

pub fn settings_load_for_home(home: &Path) -> Result<DesktopSettings, String> {
    core_settings::load(home)
        .map(DesktopSettings::from)
        .map_err(|error| error.to_string())
}

pub fn settings_preview_for_home(
    home: &Path,
    input: SettingsPreviewInput,
) -> Result<SettingsPreview, String> {
    core_settings::preview_settings(
        home,
        &core_settings::SettingsPreviewRequest {
            settings: CoreSettings::from(input.settings),
        },
    )
    .map(SettingsPreview::from)
    .map_err(|error| error.to_string())
}

pub fn settings_apply_for_home(
    home: &Path,
    input: SettingsApplyInput,
) -> Result<SettingsApplyResult, String> {
    core_settings::apply_settings(
        home,
        &core_settings::SettingsApplyRequest {
            preview_id: input.preview_id,
            preview_generated_at_epoch_seconds: input.preview_generated_at_epoch_seconds,
            request: core_settings::SettingsPreviewRequest {
                settings: CoreSettings::from(input.request.settings),
            },
        },
    )
    .map(SettingsApplyResult::from)
    .map_err(|error| error.to_string())
}

impl From<CoreSettings> for DesktopSettings {
    fn from(settings: CoreSettings) -> Self {
        Self {
            asset_center_path: settings.asset_center_path,
            scan_roots: settings.scan_roots,
            max_depth: settings.max_depth,
            backup_before_apply: settings.backup_before_apply,
            backup_warning_threshold_bytes: settings.backup_warning_threshold_bytes,
            plan_only_by_default: settings.plan_only_by_default,
            git_default_branch: settings.git_default_branch,
            git_remote: settings.git_remote,
            allow_public_remote_push: settings.allow_public_remote_push,
            appearance_theme: match settings.appearance_theme {
                CoreAppearanceTheme::System => AppearanceTheme::System,
                CoreAppearanceTheme::Light => AppearanceTheme::Light,
                CoreAppearanceTheme::Dark => AppearanceTheme::Dark,
            },
            density: match settings.density {
                CoreDensityPreference::Compact => DensityPreference::Compact,
                CoreDensityPreference::Comfortable => DensityPreference::Comfortable,
            },
            log_level: match settings.log_level {
                CoreLogLevel::Error => LogLevel::Error,
                CoreLogLevel::Warn => LogLevel::Warn,
                CoreLogLevel::Info => LogLevel::Info,
                CoreLogLevel::Debug => LogLevel::Debug,
            },
            log_retention_days: settings.log_retention_days,
            cli_path: settings.cli_path,
        }
    }
}

impl From<DesktopSettings> for CoreSettings {
    fn from(settings: DesktopSettings) -> Self {
        Self {
            asset_center_path: settings.asset_center_path,
            scan_roots: settings.scan_roots,
            max_depth: settings.max_depth,
            backup_before_apply: settings.backup_before_apply,
            backup_warning_threshold_bytes: settings.backup_warning_threshold_bytes,
            plan_only_by_default: settings.plan_only_by_default,
            git_default_branch: settings.git_default_branch,
            git_remote: settings.git_remote,
            allow_public_remote_push: settings.allow_public_remote_push,
            appearance_theme: match settings.appearance_theme {
                AppearanceTheme::System => CoreAppearanceTheme::System,
                AppearanceTheme::Light => CoreAppearanceTheme::Light,
                AppearanceTheme::Dark => CoreAppearanceTheme::Dark,
            },
            density: match settings.density {
                DensityPreference::Compact => CoreDensityPreference::Compact,
                DensityPreference::Comfortable => CoreDensityPreference::Comfortable,
            },
            log_level: match settings.log_level {
                LogLevel::Error => CoreLogLevel::Error,
                LogLevel::Warn => CoreLogLevel::Warn,
                LogLevel::Info => CoreLogLevel::Info,
                LogLevel::Debug => CoreLogLevel::Debug,
            },
            log_retention_days: settings.log_retention_days,
            cli_path: settings.cli_path,
        }
    }
}

impl From<core_settings::SettingsPreview> for SettingsPreview {
    fn from(preview: core_settings::SettingsPreview) -> Self {
        Self {
            preview_id: preview.preview_id,
            settings: DesktopSettings::from(preview.settings),
            affected_paths: preview.affected_paths,
            planned_effects: preview.planned_effects,
            warnings: preview.warnings,
            can_apply: preview.can_apply,
            generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
            expires_at_epoch_seconds: preview.expires_at_epoch_seconds,
        }
    }
}

impl From<core_settings::SettingsApplyResult> for SettingsApplyResult {
    fn from(result: core_settings::SettingsApplyResult) -> Self {
        Self {
            preview_id: result.preview_id,
            settings: DesktopSettings::from(result.settings),
            affected_paths: result.affected_paths,
        }
    }
}
