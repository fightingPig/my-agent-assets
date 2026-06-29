use my_agent_assets_core::{
    doctor, init_apply, init_plan, list_assets, mount_apply, mount_plan, remove_apply, remove_plan,
    scan_apply, scan_plan, status, sync_command, unmount_apply, AssetType, ConflictStrategy,
    Context, MaaError, McpScope, Result,
};
use std::env;
use std::path::PathBuf;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args: Vec<String> = env::args().skip(1).collect();
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
    let ctx = Context::new(home);
    let apply = take_flag(&mut args, "--apply");
    let command = args.remove(0);
    if take_flag(&mut args, "--help") || take_flag(&mut args, "-h") {
        print_command_help(&command);
        return Ok(());
    }

    match command.as_str() {
        "init" => {
            let plan = if apply {
                init_apply(&ctx)?
            } else {
                init_plan(&ctx)
            };
            if !apply {
                println!("{}", plan.render());
                println!("Run with --apply to create the asset center.");
            } else {
                println!("{}", plan.render());
            }
        }
        "scan" => {
            let conflict_strategy = conflict_strategy(&mut args)?;
            let plan = if apply {
                scan_apply(&ctx, conflict_strategy)?
            } else {
                scan_plan(&ctx)?
            };
            println!("{}", plan.render());
            if !apply {
                println!("Run with --apply to execute this plan.");
            }
        }
        "list" => print!("{}", list_assets(&ctx)?),
        "status" => print!("{}", status(&ctx)?),
        "doctor" => print!("{}", doctor(&ctx)?),
        "mount" => {
            let name = next_arg(&mut args, "asset name")?;
            let kind = required_type(&mut args)?;
            let scope = take_option(&mut args, "--scope")
                .map(|v| McpScope::parse(&v))
                .transpose()?;
            let project = take_option(&mut args, "--project").map(PathBuf::from);
            let target = target_for(&ctx, &kind, &scope, project);
            let plan = if apply {
                mount_apply(&ctx, &name, kind, target, scope)?
            } else {
                mount_plan(&ctx, &name, kind, target, scope)?
            };
            println!("{}", plan.render());
            if !apply {
                println!("Run with --apply to execute this plan.");
            }
        }
        "unmount" => {
            let name = next_arg(&mut args, "asset name")?;
            let kind = required_type(&mut args)?;
            if apply {
                println!("{}", unmount_apply(&ctx, &name, kind)?.render());
            } else {
                println!(
                    "Unmount plan\n1. [remove-mount] remove mounts for {}:{} risk=medium",
                    kind.singular(),
                    name
                );
                println!("Run with --apply to execute this plan.");
            }
        }
        "remove" => {
            let name = next_arg(&mut args, "asset name")?;
            let kind = required_type(&mut args)?;
            let plan = if apply {
                remove_apply(&ctx, &name, kind)?
            } else {
                remove_plan(&ctx, &name, kind)?
            };
            println!("{}", plan.render());
            if !apply {
                println!("Run with --apply to execute this plan.");
            }
        }
        "sync" => {
            let op = next_arg(&mut args, "pull or push")?;
            print!("{}", sync_command(&ctx, &op)?);
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

fn print_help() {
    println!(
        "My Agent Assets CLI\n\n\
Usage:\n  maa [--home <home>] <command> [options]\n\n\
Commands:\n  init [--apply]\n  scan [--apply]\n  list\n  status\n  doctor\n  mount <name> --type skill|command|mcp [--scope user|local|project] [--project <path>] [--apply]\n  unmount <name> --type skill|command|mcp [--apply]\n  remove <name> --type skill|command|mcp [--apply]\n  sync pull|push\n\n\
Scan conflict options:\n  --on-conflict skip|overwrite|rename [--rename-to <new-name>]\n\n\
Environment:\n  MY_AGENT_ASSETS_HOME overrides the runtime home. Defaults to HOME/USERPROFILE.\n"
    );
}

fn print_command_help(command: &str) {
    match command {
        "scan" => println!(
            "Usage:\n  maa scan [--apply] [--on-conflict skip|overwrite|rename] [--rename-to <new-name>]\n\n\
Default behavior prints a plan only. --apply executes. MCP JSON conflicts require an explicit --on-conflict decision."
        ),
        "mount" => println!(
            "Usage:\n  maa mount <name> --type skill|command|mcp [--scope user|local|project] [--project <path>] [--apply]"
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

fn conflict_strategy(args: &mut Vec<String>) -> Result<ConflictStrategy> {
    let Some(value) = take_option(args, "--on-conflict") else {
        return Ok(ConflictStrategy::Prompt);
    };
    match value.as_str() {
        "skip" => Ok(ConflictStrategy::Skip),
        "overwrite" => Ok(ConflictStrategy::Overwrite),
        "rename" => {
            let rename_to = take_option(args, "--rename-to").ok_or_else(|| {
                MaaError::new("--rename-to is required with --on-conflict rename")
            })?;
            Ok(ConflictStrategy::Rename(rename_to))
        }
        other => Err(MaaError::new(format!(
            "unknown conflict strategy: {other}; expected skip, overwrite, or rename"
        ))),
    }
}

fn target_for(
    ctx: &Context,
    kind: &AssetType,
    scope: &Option<McpScope>,
    project: Option<PathBuf>,
) -> PathBuf {
    if *kind == AssetType::Mcp && matches!(scope, Some(McpScope::User) | None) {
        ctx.home.clone()
    } else {
        project.unwrap_or_else(|| ctx.home.clone())
    }
}

fn required_type(args: &mut Vec<String>) -> Result<AssetType> {
    let value = take_option(args, "--type").ok_or_else(|| MaaError::new("--type is required"))?;
    AssetType::parse(&value)
}

fn next_arg(args: &mut Vec<String>, label: &str) -> Result<String> {
    if args.is_empty() {
        Err(MaaError::new(format!("missing {label}")))
    } else {
        Ok(args.remove(0))
    }
}

fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
    if let Some(pos) = args.iter().position(|arg| arg == flag) {
        args.remove(pos);
        true
    } else {
        false
    }
}

fn take_option(args: &mut Vec<String>, flag: &str) -> Option<String> {
    let pos = args.iter().position(|arg| arg == flag)?;
    args.remove(pos);
    if pos >= args.len() {
        return None;
    }
    Some(args.remove(pos))
}
