use my_agent_assets_core::adopt::{
    apply_adopt, preview_adopt, AdoptApplyRequest, AdoptPreviewRequest, AdoptSelection,
};
use my_agent_assets_core::asset_registry::{inspect_content, load as load_assets};
use my_agent_assets_core::backup_delete::{
    apply_backup_delete, preview_backup_delete, BackupDeleteApplyRequest,
    BackupDeletePreviewRequest,
};
use my_agent_assets_core::backup_history::list_backups;
use my_agent_assets_core::consistency_repair::{
    apply_consistency_repair, preview_consistency_repair, ConsistencyRepairAction,
    ConsistencyRepairApplyRequest, ConsistencyRepairPreviewRequest,
};
use my_agent_assets_core::delete::{
    apply_delete, preview_delete, DeleteApplyRequest, DeleteMode, DeletePreviewRequest,
};
use my_agent_assets_core::diagnostic_export::{
    apply_diagnostic_export, preview_diagnostic_export, DiagnosticExportApplyRequest,
};
use my_agent_assets_core::diagnostics::doctor;
use my_agent_assets_core::discovery::{discover, DiscoveryScope, SourceFormat};
use my_agent_assets_core::git_sync::{
    apply_sync, preview_sync, SyncApplyRequest, SyncDirection, SyncPreviewRequest,
};
use my_agent_assets_core::import::{
    apply_import, preview_import, ImportApplyRequest, ImportPreviewRequest, ImportResolution,
};
use my_agent_assets_core::initialization::{
    apply_initialization, preview_initialization, InitializationApplyRequest,
};
use my_agent_assets_core::mount::{
    apply_mount, apply_unmount, preview_mount, preview_unmount, MountApplyRequest,
    MountPreviewRequest, UnmountApplyRequest, UnmountPreviewRequest,
};
use my_agent_assets_core::mount_registry::load as load_mounts;
use my_agent_assets_core::operation::{incomplete_journals, recover_incomplete};
use my_agent_assets_core::query::{list_assets as query_assets, AssetQueryRequest};
use my_agent_assets_core::target_management::{
    apply_register_target, apply_remove_target, preview_register_target, preview_remove_target,
    TargetRegistrationApplyRequest, TargetRegistrationPreviewRequest, TargetRemoveApplyRequest,
    TargetRemovePreviewRequest,
};
use my_agent_assets_core::targets::{load as load_targets, AssetKind, MountTargetKind};
use my_agent_assets_core::{MaaError, Result};
use serde::Serialize;
use serde_json::json;
use std::env;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    run_args(env::args().skip(1).collect())
}

fn run_args(mut args: Vec<String>) -> Result<()> {
    if args.is_empty() || matches!(args.first().map(String::as_str), Some("--help" | "-h")) {
        print_help();
        return Ok(());
    }

    let home = take_option(&mut args, "--home")
        .map(PathBuf::from)
        .or_else(|| env::var("MY_AGENT_ASSETS_HOME").ok().map(PathBuf::from))
        .or_else(default_home)
        .ok_or_else(|| {
            MaaError::new("could not determine home directory; pass --home explicitly")
        })?;
    match recover_incomplete(&home) {
        Ok(report) if report.attempted => {
            let recovered = report
                .attempts
                .iter()
                .filter(|attempt| attempt.recovered)
                .count();
            eprintln!("已自动检查未完成事务：成功回滚 {recovered} 个。");
            if report.writes_blocked {
                eprintln!("仍有事务未恢复；新的写操作将保持阻止。");
            }
        }
        Ok(_) => {}
        Err(error) => {
            eprintln!("启动恢复失败：{error}；只读命令仍可使用，写操作将保持阻止。");
        }
    }
    let apply = take_flag(&mut args, "--apply");
    let command = next_arg(&mut args, "command")?;
    if take_flag(&mut args, "--help") || take_flag(&mut args, "-h") {
        print_command_help(&command);
        return Ok(());
    }

    match command.as_str() {
        "init" => {
            let preview = preview_initialization(&home)?;
            if apply {
                ensure_can_apply(preview.can_apply, &preview.warnings)?;
                print_json(&apply_initialization(
                    &home,
                    &InitializationApplyRequest {
                        preview_id: preview.preview_id.clone(),
                        preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                    },
                )?)?;
            } else {
                print_json(&preview)?;
                println!("Run with --apply to create the asset center.");
            }
        }
        "scan" => {
            reject_apply(apply, "scan is read-only; use `maa import` or `maa adopt` to write")?;
            let scope = parse_discovery_scope(&home, &mut args)?;
            print_json(&discover(&home, scope))?;
        }
        "import" => {
            let source_id = next_arg(&mut args, "source ID")?;
            let scope = parse_discovery_scope(&home, &mut args)?;
            let request = ImportPreviewRequest {
                scope,
                source_id,
                resolution: parse_resolution(&mut args)?,
            };
            let preview = preview_import(&home, &request)?;
            if apply {
                ensure_can_apply(preview.can_apply, &preview.warnings)?;
                print_json(&apply_import(
                    &home,
                    &ImportApplyRequest {
                        preview_id: preview.preview_id.clone(),
                        preview_generated_at_epoch_seconds: preview
                            .generated_at_epoch_seconds,
                        request,
                    },
                )?)?;
            } else {
                print_json(&preview)?;
                println!("Run the same command with --apply to execute this preview.");
            }
        }
        "adopt" => {
            let source_id = next_arg(&mut args, "source ID")?;
            let scope = parse_discovery_scope(&home, &mut args)?;
            let request = AdoptPreviewRequest {
                scope,
                selections: vec![AdoptSelection {
                    source_id,
                    resolution: parse_resolution(&mut args)?,
                }],
            };
            let preview = preview_adopt(&home, &request)?;
            if apply {
                ensure_can_apply(preview.can_apply, &preview.warnings)?;
                print_json(&apply_adopt(
                    &home,
                    &AdoptApplyRequest {
                        preview_id: preview.preview_id.clone(),
                        preview_generated_at_epoch_seconds: preview
                            .generated_at_epoch_seconds,
                        request,
                    },
                )?)?;
            } else {
                print_json(&preview)?;
                println!("Run the same command with --apply to execute this preview.");
            }
        }
        "target" => handle_target(&home, &mut args, apply)?,
        "backup" => handle_backup(&home, &mut args, apply)?,
        "mount" => {
            let request = MountPreviewRequest {
                asset_id: next_arg(&mut args, "asset ID")?,
                target_id: required_option(&mut args, "--target")?,
            };
            let preview = preview_mount(&home, &request)?;
            if apply {
                ensure_can_apply(preview.can_apply, &preview.warnings)?;
                print_json(&apply_mount(
                    &home,
                    &MountApplyRequest {
                        preview_id: preview.preview_id.clone(),
                        preview_generated_at_epoch_seconds: preview
                            .generated_at_epoch_seconds,
                        request,
                    },
                )?)?;
            } else {
                print_json(&preview)?;
                println!("Run the same command with --apply to execute this preview.");
            }
        }
        "unmount" => {
            let request = UnmountPreviewRequest {
                asset_id: next_arg(&mut args, "asset ID")?,
                target_id: required_option(&mut args, "--target")?,
            };
            let preview = preview_unmount(&home, &request)?;
            if apply {
                ensure_can_apply(preview.can_apply, &preview.warnings)?;
                print_json(&apply_unmount(
                    &home,
                    &UnmountApplyRequest {
                        preview_id: preview.preview_id.clone(),
                        preview_generated_at_epoch_seconds: preview
                            .generated_at_epoch_seconds,
                        request,
                    },
                )?)?;
            } else {
                print_json(&preview)?;
                println!("Run the same command with --apply to execute this preview.");
            }
        }
        "remove" => {
            let request = DeletePreviewRequest {
                asset_id: next_arg(&mut args, "asset ID")?,
                mode: if take_flag(&mut args, "--unmount-all") {
                    DeleteMode::UnmountAll
                } else {
                    DeleteMode::RequireUnmounted
                },
            };
            let preview = preview_delete(&home, &request)?;
            if apply {
                ensure_can_apply(preview.can_apply, &preview.warnings)?;
                print_json(&apply_delete(
                    &home,
                    &DeleteApplyRequest {
                        preview_id: preview.preview_id.clone(),
                        preview_generated_at_epoch_seconds: preview
                            .generated_at_epoch_seconds,
                        request,
                    },
                )?)?;
            } else {
                print_json(&preview)?;
                println!("Run the same command with --apply to execute this preview.");
            }
        }
        "list" => {
            reject_apply(apply, "list is read-only")?;
            print_json(&query_assets(
                &home,
                &AssetQueryRequest { asset_type: None },
            )?)?;
        }
        "status" => {
            reject_apply(apply, "status is read-only")?;
            let assets =
                load_assets(&home).map_err(|error| MaaError::new(error.to_string()))?;
            let mounts =
                load_mounts(&home).map_err(|error| MaaError::new(error.to_string()))?;
            let targets = load_targets(&home)?;
            let diagnostics = inspect_content(&home, &assets)
                .map_err(|error| MaaError::new(error.to_string()))?;
            print_json(&json!({
                "assetCount": assets.assets.len(),
                "bindingCount": mounts.bindings.len(),
                "targetCount": targets.targets.len(),
                "diagnostics": diagnostics,
                "incompleteOperations": incomplete_journals(&home)?,
            }))?;
        }
        "doctor" => handle_doctor(&home, &mut args, apply)?,
        "sync" => {
            let direction = match next_arg(&mut args, "pull or push")?.as_str() {
                "pull" => SyncDirection::Pull,
                "push" => SyncDirection::Push,
                value => {
                    return Err(MaaError::new(format!(
                        "unknown sync direction: {value}; expected pull or push"
                    )))
                }
            };
            let request = SyncPreviewRequest { direction };
            let preview = preview_sync(&home, &request)?;
            if apply {
                ensure_can_apply(preview.can_apply, &preview.warnings)?;
                print_json(&apply_sync(
                    &home,
                    &SyncApplyRequest {
                        preview_id: preview.preview_id.clone(),
                        preview_generated_at_epoch_seconds: preview
                            .generated_at_epoch_seconds,
                        request,
                    },
                )?)?;
            } else {
                print_json(&preview)?;
                println!("Run the same command with --apply to execute this sync preview.");
            }
        }
        "restore" => {
            return Err(MaaError::new(
                "automatic historical Restore is not supported; use Backup History and the manual restore guide",
            ))
        }
        other => return Err(MaaError::new(format!("unknown command: {other}"))),
    }

    if !args.is_empty() {
        return Err(MaaError::new(format!(
            "unexpected arguments: {}",
            args.join(" ")
        )));
    }
    Ok(())
}

fn handle_target(home: &Path, args: &mut Vec<String>, apply: bool) -> Result<()> {
    match next_arg(args, "target operation")?.as_str() {
        "list" => {
            reject_apply(apply, "target list is read-only")?;
            print_json(&load_targets(home)?)
        }
        "add" => {
            let kind = parse_target_kind(&next_arg(args, "target kind")?)?;
            let id = next_arg(args, "target ID")?;
            let location = if is_project_kind(kind) {
                PathBuf::from(required_option(args, "--project")?)
            } else {
                PathBuf::from(required_option(args, "--path")?)
            };
            let request = TargetRegistrationPreviewRequest { id, kind, location };
            let preview = preview_register_target(home, &request)?;
            if apply {
                ensure_can_apply(preview.can_apply, &preview.warnings)?;
                print_json(&apply_register_target(
                    home,
                    &TargetRegistrationApplyRequest {
                        preview_id: preview.preview_id.clone(),
                        preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                        request,
                    },
                )?)
            } else {
                print_json(&preview)?;
                println!("Run the same command with --apply to register this target.");
                Ok(())
            }
        }
        "remove" => {
            let request = TargetRemovePreviewRequest {
                target_id: next_arg(args, "target ID")?,
            };
            let preview = preview_remove_target(home, &request)?;
            if apply {
                ensure_can_apply(preview.can_apply, &preview.warnings)?;
                print_json(&apply_remove_target(
                    home,
                    &TargetRemoveApplyRequest {
                        preview_id: preview.preview_id.clone(),
                        preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                        request,
                    },
                )?)
            } else {
                print_json(&preview)?;
                println!("Run the same command with --apply to remove this target.");
                Ok(())
            }
        }
        operation => Err(MaaError::new(format!(
            "unknown target operation: {operation}"
        ))),
    }
}

fn handle_backup(home: &Path, args: &mut Vec<String>, apply: bool) -> Result<()> {
    match next_arg(args, "backup operation")?.as_str() {
        "list" => {
            reject_apply(apply, "backup list is read-only")?;
            print_json(&list_backups(home))
        }
        "delete" => {
            let request = BackupDeletePreviewRequest {
                entry_id: next_arg(args, "backup entry ID")?,
            };
            let preview = preview_backup_delete(home, &request)?;
            if apply {
                ensure_can_apply(preview.can_apply, &preview.warnings)?;
                print_json(&apply_backup_delete(
                    home,
                    &BackupDeleteApplyRequest {
                        preview_id: preview.preview_id.clone(),
                        preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                        request,
                    },
                )?)
            } else {
                print_json(&preview)?;
                println!("Run the same command with --apply to permanently delete this backup.");
                Ok(())
            }
        }
        operation => Err(MaaError::new(format!(
            "unknown backup operation: {operation}; expected list or delete"
        ))),
    }
}

fn handle_doctor(home: &Path, args: &mut Vec<String>, apply: bool) -> Result<()> {
    if args.is_empty() {
        reject_apply(apply, "doctor report is read-only")?;
        return print_json(&doctor(home));
    }
    match next_arg(args, "doctor operation")?.as_str() {
        "export" => {
            let preview = preview_diagnostic_export(home)?;
            if apply {
                print_json(&apply_diagnostic_export(
                    home,
                    &DiagnosticExportApplyRequest {
                        preview_id: preview.preview_id.clone(),
                        preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                    },
                )?)
            } else {
                print_json(&preview)?;
                println!(
                    "Run the same command with --apply to export the reviewed diagnostic package."
                );
                Ok(())
            }
        }
        "repair" => {
            let action = match next_arg(args, "repair action")?.as_str() {
                "remove-missing" => ConsistencyRepairAction::RemoveMissingRegistryRecord,
                "register-unregistered" => ConsistencyRepairAction::RegisterUnregisteredContent,
                "delete-unregistered" => ConsistencyRepairAction::DeleteUnregisteredContent,
                value => {
                    return Err(MaaError::new(format!(
                        "unknown doctor repair action: {value}; expected remove-missing, register-unregistered, or delete-unregistered"
                    )))
                }
            };
            let request = ConsistencyRepairPreviewRequest {
                asset_id: next_arg(args, "asset ID")?,
                action,
            };
            let preview = preview_consistency_repair(home, &request)?;
            if apply {
                ensure_can_apply(preview.can_apply, &preview.warnings)?;
                print_json(&apply_consistency_repair(
                    home,
                    &ConsistencyRepairApplyRequest {
                        preview_id: preview.preview_id.clone(),
                        preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                        request,
                    },
                )?)
            } else {
                print_json(&preview)?;
                println!("Run the same command with --apply to execute this high-risk repair.");
                Ok(())
            }
        }
        operation => Err(MaaError::new(format!(
            "unknown doctor operation: {operation}; expected export or repair"
        ))),
    }
}

fn parse_discovery_scope(home: &Path, args: &mut Vec<String>) -> Result<DiscoveryScope> {
    let scope = take_option(args, "--scope").unwrap_or_else(|| "user".into());
    match scope.as_str() {
        "user" => Ok(DiscoveryScope::User),
        "project" => Ok(DiscoveryScope::Project {
            project_path: canonical_existing_directory(
                home,
                PathBuf::from(required_option(args, "--project")?),
            )?,
        }),
        "custom" => Ok(DiscoveryScope::Custom {
            path: expand_home(home, &required_option(args, "--path")?),
            asset_kind: parse_asset_kind(&required_option(args, "--type")?)?,
            source_format: parse_source_format(&required_option(args, "--format")?)?,
        }),
        value => Err(MaaError::new(format!(
            "unknown scope: {value}; expected user, project, or custom"
        ))),
    }
}

fn parse_resolution(args: &mut Vec<String>) -> Result<ImportResolution> {
    match take_option(args, "--resolution")
        .unwrap_or_else(|| "unresolved".into())
        .as_str()
    {
        "unresolved" => Ok(ImportResolution::Unresolved),
        "skip" => Ok(ImportResolution::Skip),
        "overwrite" => Ok(ImportResolution::Overwrite),
        "rename" => Ok(ImportResolution::Rename {
            new_name: required_option(args, "--rename-to")?,
        }),
        value => Err(MaaError::new(format!(
            "unknown resolution: {value}; expected unresolved, skip, overwrite, or rename"
        ))),
    }
}

fn parse_asset_kind(value: &str) -> Result<AssetKind> {
    match value {
        "skill" => Ok(AssetKind::Skill),
        "command" => Ok(AssetKind::Command),
        "mcp" => Ok(AssetKind::Mcp),
        _ => Err(MaaError::new(format!("unknown asset type: {value}"))),
    }
}

fn parse_source_format(value: &str) -> Result<SourceFormat> {
    match value {
        "skill-directory" => Ok(SourceFormat::SkillDirectory),
        "markdown" => Ok(SourceFormat::Markdown),
        "claude-mcp-json" => Ok(SourceFormat::ClaudeMcpJson),
        "codex-mcp-toml" => Ok(SourceFormat::CodexMcpToml),
        _ => Err(MaaError::new(format!("unknown source format: {value}"))),
    }
}

fn parse_target_kind(value: &str) -> Result<MountTargetKind> {
    match value {
        "claude-project-skills" => Ok(MountTargetKind::ClaudeProjectSkills),
        "codex-project-skills" => Ok(MountTargetKind::CodexProjectSkills),
        "claude-project-commands" => Ok(MountTargetKind::ClaudeProjectCommands),
        "claude-project-mcp" => Ok(MountTargetKind::ClaudeProjectMcpJson),
        "codex-project-mcp" => Ok(MountTargetKind::CodexProjectMcpToml),
        "custom-skills" => Ok(MountTargetKind::CustomSkillDirectory),
        "custom-commands" => Ok(MountTargetKind::CustomCommandDirectory),
        "custom-claude-mcp" => Ok(MountTargetKind::CustomClaudeMcpJson),
        "custom-codex-mcp" => Ok(MountTargetKind::CustomCodexMcpToml),
        _ => Err(MaaError::new(format!("unknown target kind: {value}"))),
    }
}

fn is_project_kind(kind: MountTargetKind) -> bool {
    matches!(
        kind,
        MountTargetKind::ClaudeProjectSkills
            | MountTargetKind::CodexProjectSkills
            | MountTargetKind::ClaudeProjectCommands
            | MountTargetKind::ClaudeProjectMcpJson
            | MountTargetKind::CodexProjectMcpToml
    )
}

fn canonical_existing_directory(home: &Path, path: PathBuf) -> Result<PathBuf> {
    let path = if path == Path::new("~") {
        home.to_path_buf()
    } else if let Ok(relative) = path.strip_prefix("~") {
        home.join(relative)
    } else {
        path
    };
    let canonical = std::fs::canonicalize(&path).map_err(|error| {
        MaaError::new(format!(
            "project path must be an existing directory ({}): {error}",
            path.display()
        ))
    })?;
    if !canonical.is_dir() {
        return Err(MaaError::new(format!(
            "project path is not a directory: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

fn expand_home(home: &Path, value: &str) -> PathBuf {
    if value == "~" {
        home.to_path_buf()
    } else if let Some(relative) = value.strip_prefix("~/") {
        home.join(relative)
    } else {
        PathBuf::from(value)
    }
}

fn ensure_can_apply(can_apply: bool, warnings: &[String]) -> Result<()> {
    if can_apply {
        Ok(())
    } else {
        Err(MaaError::new(warnings.first().cloned().unwrap_or_else(
            || "preview is blocked and cannot be applied".into(),
        )))
    }
}

fn reject_apply(apply: bool, message: &str) -> Result<()> {
    if apply {
        Err(MaaError::new(message))
    } else {
        Ok(())
    }
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    let json =
        serde_json::to_string_pretty(value).map_err(|error| MaaError::new(error.to_string()))?;
    println!("{json}");
    Ok(())
}

fn print_help() {
    println!(
        "My Agent Assets CLI\n\n\
Usage:\n  maa [--home <home>] <command> [options]\n\n\
Commands:\n  init [--apply]\n  scan [--scope user|project|custom] [scope options]\n  import <source-id> [scope options] [--resolution ...] [--apply]\n  adopt <source-id> [scope options] [--resolution ...] [--apply]\n  target list\n  target add <target-kind> <target-id> --project <path>|--path <path> [--apply]\n  target remove <target-id> [--apply]\n  mount <asset-id> --target <target-id> [--apply]\n  unmount <asset-id> --target <target-id> [--apply]\n  remove <asset-id> [--unmount-all] [--apply]\n  backup list\n  backup delete <entry-id> [--apply]\n  list\n  status\n  doctor\n  sync pull|push [--apply]\n\n\
Scope options:\n  --scope user\n  --scope project --project <path>\n  --scope custom --path <path> --type skill|command|mcp \\\n    --format skill-directory|markdown|claude-mcp-json|codex-mcp-toml\n\n\
Conflict resolution:\n  --resolution unresolved|skip|overwrite|rename [--rename-to <name>]\n\n\
Writes always show a preview unless --apply is explicitly supplied. Push requires a live GitHub Private visibility check. Automatic historical Restore is disabled.\n"
    );
}

fn print_command_help(command: &str) {
    match command {
        "scan" => println!(
            "Usage: maa scan [--scope user|project|custom] [scope options]\nScan only discovers sources; it never imports or mounts."
        ),
        "import" => println!(
            "Usage: maa import <source-id> [scope options] [--resolution unresolved|skip|overwrite|rename] [--rename-to <name>] [--apply]"
        ),
        "adopt" => println!(
            "Usage: maa adopt <source-id> [scope options] [--resolution unresolved|skip|overwrite] [--apply]"
        ),
        "mount" => println!("Usage: maa mount <asset-id> --target <target-id> [--apply]"),
        "target" => println!(
            "Usage:\n  maa target list\n  maa target add <target-kind> <target-id> --project <path>|--path <path> [--apply]\n  maa target remove <target-id> [--apply]"
        ),
        "backup" => println!(
            "Usage:\n  maa backup list\n  maa backup delete <entry-id> [--apply]\nBackup deletion is preview-bound and does not restore historical files."
        ),
        "doctor" => println!(
            "Usage:\n  maa doctor\n  maa doctor export [--apply]\n  maa doctor repair remove-missing|register-unregistered|delete-unregistered <asset-id> [--apply]\nDiagnostic export and consistency repairs are preview-bound writes."
        ),
        _ => print_help(),
    }
}

fn default_home() -> Option<PathBuf> {
    env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| env::var("USERPROFILE").ok().map(PathBuf::from))
}

fn required_option(args: &mut Vec<String>, flag: &str) -> Result<String> {
    take_option(args, flag).ok_or_else(|| MaaError::new(format!("{flag} is required")))
}

fn next_arg(args: &mut Vec<String>, label: &str) -> Result<String> {
    if args.is_empty() {
        Err(MaaError::new(format!("missing {label}")))
    } else {
        Ok(args.remove(0))
    }
}

fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
    if let Some(position) = args.iter().position(|argument| argument == flag) {
        args.remove(position);
        true
    } else {
        false
    }
}

fn take_option(args: &mut Vec<String>, flag: &str) -> Option<String> {
    let position = args.iter().position(|argument| argument == flag)?;
    args.remove(position);
    if position >= args.len() {
        return None;
    }
    Some(args.remove(position))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_supported_target_kinds() {
        for kind in [
            "claude-project-skills",
            "codex-project-skills",
            "claude-project-commands",
            "claude-project-mcp",
            "codex-project-mcp",
            "custom-skills",
            "custom-commands",
            "custom-claude-mcp",
            "custom-codex-mcp",
        ] {
            parse_target_kind(kind).unwrap();
        }
        assert!(parse_target_kind("codex-project-commands").is_err());
    }

    #[test]
    fn unresolved_is_the_default_conflict_resolution() {
        assert_eq!(
            parse_resolution(&mut Vec::new()).unwrap(),
            ImportResolution::Unresolved
        );
    }

    #[test]
    fn custom_scope_requires_explicit_type_and_format() {
        let home = Path::new("/tmp/fake-home");
        let mut args = vec![
            "--scope".into(),
            "custom".into(),
            "--path".into(),
            "/tmp/assets".into(),
        ];
        assert!(parse_discovery_scope(home, &mut args).is_err());
    }
}
