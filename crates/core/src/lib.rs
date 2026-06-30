pub mod adopt;
pub mod asset_registry;
pub mod backup_history;
pub mod batch_import;
pub mod delete;
pub mod discovery;
pub mod git_sync;
pub mod import;
pub mod mcp;
pub mod mount;
pub mod mount_registry;
pub mod operation;
pub mod path_safety;
pub mod settings;
pub mod target_management;
pub mod targets;

use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for MaaError {}

impl From<std::io::Error> for MaaError {
    fn from(value: std::io::Error) -> Self {
        Self::new(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssetType {
    Skill,
    Command,
    Mcp,
}

impl AssetType {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "skill" | "skills" => Ok(Self::Skill),
            "command" | "commands" => Ok(Self::Command),
            "mcp" | "mcps" => Ok(Self::Mcp),
            other => Err(MaaError::new(format!("unknown asset type: {other}"))),
        }
    }

    pub fn singular(&self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::Command => "command",
            Self::Mcp => "mcp",
        }
    }

    pub fn plural(&self) -> &'static str {
        match self {
            Self::Skill => "skills",
            Self::Command => "commands",
            Self::Mcp => "mcps",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssetId {
    pub kind: AssetType,
    pub name: String,
}

impl AssetId {
    pub fn new(kind: AssetType, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
        }
    }
}

impl Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind.singular(), self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpScope {
    User,
    Local,
    Project,
}

impl McpScope {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "user" => Ok(Self::User),
            "local" => Ok(Self::Local),
            "project" => Ok(Self::Project),
            other => Err(MaaError::new(format!("unknown MCP scope: {other}"))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Local => "local",
            Self::Project => "project",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Context {
    pub home: PathBuf,
    pub asset_center: PathBuf,
}

impl Context {
    pub fn new(home: PathBuf) -> Self {
        let asset_center = home.join(".my-agent-assets");
        Self { home, asset_center }
    }
}

#[derive(Debug, Clone)]
pub enum ActionKind {
    CreateDir,
    CreateFile,
    ImportAsset,
    ReplaceWithSymlink,
    CompileMcp,
    RegisterMount,
    RemoveMount,
    RemoveAsset,
    Check,
}

impl ActionKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::CreateDir => "create-dir",
            Self::CreateFile => "create-file",
            Self::ImportAsset => "import-asset",
            Self::ReplaceWithSymlink => "replace-with-symlink",
            Self::CompileMcp => "compile-mcp",
            Self::RegisterMount => "register-mount",
            Self::RemoveMount => "remove-mount",
            Self::RemoveAsset => "remove-asset",
            Self::Check => "check",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanItem {
    pub kind: ActionKind,
    pub asset: Option<AssetId>,
    pub source: Option<PathBuf>,
    pub target: Option<PathBuf>,
    pub message: String,
    pub risk: &'static str,
    pub details: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub title: String,
    pub items: Vec<PlanItem>,
}

impl Plan {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
        }
    }

    pub fn push(&mut self, item: PlanItem) {
        self.items.push(item);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("{}\n", self.title));
        if self.items.is_empty() {
            out.push_str("No changes.\n");
            return out;
        }
        for (idx, item) in self.items.iter().enumerate() {
            let asset = item
                .asset
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "-".to_string());
            out.push_str(&format!(
                "{}. [{}] {} asset={} risk={}\n",
                idx + 1,
                item.kind.as_str(),
                item.message,
                asset,
                item.risk
            ));
            if let Some(source) = &item.source {
                out.push_str(&format!("   source: {}\n", source.display()));
            }
            if let Some(target) = &item.target {
                out.push_str(&format!("   target: {}\n", target.display()));
            }
            for detail in &item.details {
                out.push_str("   ");
                out.push_str(&detail.replace('\n', "\n   "));
                out.push('\n');
            }
        }
        out
    }
}

#[derive(Debug, Clone)]
pub enum ConflictStrategy {
    Prompt,
    Skip,
    Overwrite,
    Rename(String),
}

impl ConflictStrategy {
    pub fn skip() -> Self {
        Self::Skip
    }
}

#[derive(Debug, Clone)]
struct DiscoveredAsset {
    id: AssetId,
    source: PathBuf,
    runtime_root: PathBuf,
    mcp_scope: Option<McpScope>,
    mcp_config: Option<JsonValue>,
}

#[derive(Debug, Clone, Default)]
pub struct Registry {
    pub assets: BTreeMap<AssetId, AssetRecord>,
    pub mounts: BTreeMap<AssetId, Vec<MountRecord>>,
}

#[derive(Debug, Clone)]
pub struct AssetRecord {
    pub path: PathBuf,
    pub file_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MountRecord {
    pub target: PathBuf,
    pub scope: Option<McpScope>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DiskAssetsFile {
    #[serde(default)]
    assets: DiskAssetGroups,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DiskAssetGroups {
    #[serde(default)]
    skills: BTreeMap<String, DiskAssetRecord>,
    #[serde(default)]
    commands: BTreeMap<String, DiskAssetRecord>,
    #[serde(default)]
    mcps: BTreeMap<String, DiskAssetRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DiskAssetRecord {
    path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    file_name: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DiskMountsFile {
    #[serde(default)]
    mounts: DiskMountGroups,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DiskMountGroups {
    #[serde(default)]
    skills: BTreeMap<String, Vec<DiskMountRecord>>,
    #[serde(default)]
    commands: BTreeMap<String, Vec<DiskMountRecord>>,
    #[serde(default)]
    mcps: BTreeMap<String, Vec<DiskMountRecord>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DiskMountRecord {
    target: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    #[serde(default)]
    scan_roots: Vec<String>,
    #[serde(default)]
    max_depth: Option<usize>,
}

pub fn init_plan(ctx: &Context) -> Plan {
    let mut plan = Plan::new("Init plan");
    for path in [
        ctx.asset_center.clone(),
        ctx.asset_center.join("assets"),
        ctx.asset_center.join("assets/skills"),
        ctx.asset_center.join("assets/commands"),
        ctx.asset_center.join("assets/mcps"),
        ctx.asset_center.join("backups/portable"),
        ctx.asset_center.join("backups/local"),
    ] {
        if !path.exists() {
            plan.push(PlanItem {
                kind: ActionKind::CreateDir,
                asset: None,
                source: None,
                target: Some(path),
                message: "create asset center directory".to_string(),
                risk: "low",
                details: Vec::new(),
            });
        }
    }
    for file in [
        "config.yaml",
        "assets.yaml",
        "targets.yaml",
        "mounts.yaml",
        ".gitignore",
    ] {
        let path = ctx.asset_center.join(file);
        if !path.exists() {
            plan.push(PlanItem {
                kind: ActionKind::CreateFile,
                asset: None,
                source: None,
                target: Some(path),
                message: format!("create {file}"),
                risk: "low",
                details: Vec::new(),
            });
        }
    }
    plan
}

pub fn init_apply(ctx: &Context) -> Result<Plan> {
    let plan = init_plan(ctx);
    fs::create_dir_all(ctx.asset_center.join("assets/skills"))?;
    fs::create_dir_all(ctx.asset_center.join("assets/commands"))?;
    fs::create_dir_all(ctx.asset_center.join("assets/mcps"))?;
    fs::create_dir_all(ctx.asset_center.join("backups/portable"))?;
    fs::create_dir_all(ctx.asset_center.join("backups/local"))?;

    if !ctx.asset_center.join("config.yaml").exists() {
        settings::save(&ctx.home, &settings::Settings::defaults_for_home(&ctx.home))
            .map_err(|error| MaaError::new(error.to_string()))?;
    }
    write_if_missing(
        &ctx.asset_center.join("assets.yaml"),
        "schemaVersion: 1\nassets: {}\n",
    )?;
    if !ctx.asset_center.join("targets.yaml").exists() {
        let targets = targets::TargetRegistry::standard_user_targets(
            &ctx.home,
            provider_state(
                ctx.home.join(".claude").exists() || ctx.home.join(".claude.json").exists(),
            ),
            provider_state(ctx.home.join(".codex").exists()),
            directory_mount_adapter(),
        )?;
        write_if_missing(&ctx.asset_center.join("targets.yaml"), &targets.to_yaml()?)?;
    }
    write_if_missing(
        &ctx.asset_center.join("mounts.yaml"),
        "schemaVersion: 1\nbindings: {}\n",
    )?;
    write_if_missing(
        &ctx.asset_center.join(".gitignore"),
        "config.yaml\ntargets.yaml\nmounts.yaml\nbackups/local/\noperations/\nlocks/\ncache/\nlogs/\nsecrets/\n",
    )?;
    init_asset_center_git(ctx)?;
    Ok(plan)
}

fn provider_state(initialized: bool) -> targets::ProviderState {
    if initialized {
        targets::ProviderState::Initialized
    } else {
        targets::ProviderState::NotInstalled
    }
}

fn directory_mount_adapter() -> targets::MountAdapter {
    #[cfg(windows)]
    {
        targets::MountAdapter::WindowsDirectoryJunction
    }
    #[cfg(not(windows))]
    {
        targets::MountAdapter::SymlinkDirectory
    }
}

pub fn scan_plan(ctx: &Context) -> Result<Plan> {
    ensure_initialized(ctx)?;
    let registry = load_registry(ctx)?;
    let discovered = discover(ctx)?;
    let mut plan = Plan::new("Scan plan");
    for asset in discovered {
        if let Some(existing) = registry.assets.get(&asset.id) {
            let details = if asset.id.kind == AssetType::Mcp {
                mcp_conflict_details(ctx, existing, &asset)?
            } else {
                Vec::new()
            };
            plan.push(PlanItem {
                kind: ActionKind::Check,
                asset: Some(asset.id),
                source: Some(asset.source),
                target: None,
                message: if details.is_empty() {
                    "asset already exists; identical MCP assets will only create mount records during apply".to_string()
                } else {
                    "MCP name conflict with different JSON; choose skip, overwrite, or rename".to_string()
                },
                risk: "medium",
                details,
            });
            continue;
        }
        let target = asset_center_path(ctx, &asset.id);
        plan.push(PlanItem {
            kind: ActionKind::ImportAsset,
            asset: Some(asset.id.clone()),
            source: Some(asset.source.clone()),
            target: Some(target),
            message: "import runtime asset into asset center".to_string(),
            risk: "medium",
            details: Vec::new(),
        });
        if asset.id.kind != AssetType::Mcp {
            plan.push(PlanItem {
                kind: ActionKind::ReplaceWithSymlink,
                asset: Some(asset.id),
                source: None,
                target: Some(asset.source),
                message: "replace runtime path with symlink".to_string(),
                risk: "high",
                details: Vec::new(),
            });
        } else {
            plan.push(PlanItem {
                kind: ActionKind::RegisterMount,
                asset: Some(asset.id),
                source: None,
                target: Some(asset.runtime_root),
                message: "register MCP mount; runtime JSON source is not rewritten during scan"
                    .to_string(),
                risk: "medium",
                details: Vec::new(),
            });
        }
    }
    Ok(plan)
}

pub fn scan_apply(ctx: &Context, conflict_strategy: ConflictStrategy) -> Result<Plan> {
    ensure_initialized(ctx)?;
    let mut registry = load_registry(ctx)?;
    let discovered = discover(ctx)?;
    if let ConflictStrategy::Rename(_) = &conflict_strategy {
        let conflict_count = discovered
            .iter()
            .filter(|asset| mcp_has_different_existing(ctx, &registry, asset).unwrap_or(false))
            .count();
        if conflict_count > 1 {
            return Err(MaaError::new(
                "rename conflict strategy can only be used when exactly one MCP conflict is present",
            ));
        }
    }
    let backup_id = backup_id();
    let backup_root = ctx.asset_center.join("backups").join(&backup_id);
    let mut manifest = BackupManifest::new(backup_id.clone());
    let mut backed_up_sources = BTreeSet::<PathBuf>::new();
    let mut plan = Plan::new("Scan apply");

    for asset in discovered {
        if let Some(existing) = registry.assets.get(&asset.id).cloned() {
            if asset.id.kind == AssetType::Mcp {
                let existing_json = parse_json_file(&ctx.asset_center.join(&existing.path))?;
                let incoming_json = asset
                    .mcp_config
                    .clone()
                    .ok_or_else(|| MaaError::new("MCP asset missing scanned config"))?;
                if existing_json == incoming_json {
                    let scope = asset.mcp_scope.clone().unwrap_or(McpScope::User);
                    add_mount_if_missing(
                        &mut registry,
                        asset.id.clone(),
                        asset.runtime_root.clone(),
                        Some(scope),
                    );
                    plan.push(PlanItem {
                        kind: ActionKind::RegisterMount,
                        asset: Some(asset.id),
                        source: Some(asset.source),
                        target: Some(asset.runtime_root),
                        message:
                            "MCP asset already exists with identical JSON; registered mount only"
                                .to_string(),
                        risk: "low",
                        details: Vec::new(),
                    });
                    continue;
                }

                match &conflict_strategy {
                    ConflictStrategy::Prompt => {
                        return Err(MaaError::new(format!(
                            "unresolved MCP conflict for {}. Run scan first to inspect JSON, then use --on-conflict skip|overwrite|rename",
                            asset.id
                        )));
                    }
                    ConflictStrategy::Skip => {
                        plan.push(PlanItem {
                            kind: ActionKind::Check,
                            asset: Some(asset.id.clone()),
                            source: Some(asset.source.clone()),
                            target: None,
                            message: "skipped conflicting MCP asset".to_string(),
                            risk: "medium",
                            details: mcp_conflict_details(ctx, &existing, &asset)?,
                        });
                        continue;
                    }
                    ConflictStrategy::Overwrite => {
                        let asset_path = ctx.asset_center.join(&existing.path);
                        write_text_atomic(&asset_path, &json_to_pretty(&incoming_json)?)?;
                        let scope = asset.mcp_scope.clone().unwrap_or(McpScope::User);
                        add_mount_if_missing(
                            &mut registry,
                            asset.id.clone(),
                            asset.runtime_root.clone(),
                            Some(scope.clone()),
                        );
                        plan.push(PlanItem {
                            kind: ActionKind::ImportAsset,
                            asset: Some(asset.id),
                            source: Some(asset.source),
                            target: Some(asset_path),
                            message: "overwrote asset center MCP JSON and registered mount"
                                .to_string(),
                            risk: "high",
                            details: Vec::new(),
                        });
                        continue;
                    }
                    ConflictStrategy::Rename(new_name) => {
                        let renamed_id = AssetId::new(AssetType::Mcp, new_name);
                        if registry.assets.contains_key(&renamed_id) {
                            return Err(MaaError::new(format!(
                                "rename target already exists: {renamed_id}"
                            )));
                        }
                        import_new_mcp_asset(ctx, &mut registry, &asset, renamed_id, &mut plan)?;
                        continue;
                    }
                }
            }
            plan.push(PlanItem {
                kind: ActionKind::Check,
                asset: Some(asset.id),
                source: Some(asset.source),
                target: None,
                message: "skipped existing non-MCP asset; conflict decisions are not automatic"
                    .to_string(),
                risk: "medium",
                details: vec![
                    "resolve manually: remove the existing asset or rename the runtime asset before scanning again".to_string(),
                ],
            });
            continue;
        }

        let asset_path = asset_center_path(ctx, &asset.id);
        match asset.id.kind {
            AssetType::Skill => {
                backup_path(&asset.source, &backup_root, &mut manifest)?;
                copy_dir_all(&asset.source, &asset_path)?;
                remove_path(&asset.source)?;
                create_symlink(&asset_path, &asset.source)?;
                registry.assets.insert(
                    asset.id.clone(),
                    AssetRecord {
                        path: relative_to(&asset_path, &ctx.asset_center)?,
                        file_name: None,
                    },
                );
                registry
                    .mounts
                    .entry(asset.id.clone())
                    .or_default()
                    .push(MountRecord {
                        target: asset.runtime_root.clone(),
                        scope: None,
                    });
                plan.push(imported_item(&asset.id, &asset.source, &asset_path));
            }
            AssetType::Command => {
                backup_path(&asset.source, &backup_root, &mut manifest)?;
                if let Some(parent) = asset_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&asset.source, &asset_path)?;
                remove_path(&asset.source)?;
                create_symlink(&asset_path, &asset.source)?;
                registry.assets.insert(
                    asset.id.clone(),
                    AssetRecord {
                        path: relative_to(&asset_path, &ctx.asset_center)?,
                        file_name: Some(format!("{}.md", asset.id.name)),
                    },
                );
                registry
                    .mounts
                    .entry(asset.id.clone())
                    .or_default()
                    .push(MountRecord {
                        target: asset.runtime_root.clone(),
                        scope: None,
                    });
                plan.push(imported_item(&asset.id, &asset.source, &asset_path));
            }
            AssetType::Mcp => {
                if asset.source.exists() && backed_up_sources.insert(asset.source.clone()) {
                    backup_path(&asset.source, &backup_root, &mut manifest)?;
                }
                import_new_mcp_asset(ctx, &mut registry, &asset, asset.id.clone(), &mut plan)?;
            }
        }
    }

    save_registry(ctx, &registry)?;
    if !manifest.entries.is_empty() {
        fs::create_dir_all(&backup_root)?;
        fs::write(backup_root.join("manifest.txt"), manifest.render())?;
    }
    Ok(plan)
}

pub fn list_assets(ctx: &Context) -> Result<String> {
    ensure_initialized(ctx)?;
    let registry = load_registry(ctx)?;
    if registry.assets.is_empty() {
        return Ok("No assets.\n".to_string());
    }
    let mut out = String::new();
    for (id, record) in registry.assets {
        out.push_str(&format!("{} -> {}\n", id, record.path.display()));
    }
    Ok(out)
}

pub fn status(ctx: &Context) -> Result<String> {
    let mut out = String::new();
    out.push_str(&format!("home: {}\n", ctx.home.display()));
    out.push_str(&format!("asset_center: {}\n", ctx.asset_center.display()));
    out.push_str(&format!("initialized: {}\n", ctx.asset_center.exists()));
    if ctx.asset_center.exists() {
        let registry = load_registry(ctx)?;
        out.push_str(&format!("assets: {}\n", registry.assets.len()));
        out.push_str(&format!("mount groups: {}\n", registry.mounts.len()));
    }
    Ok(out)
}

pub fn doctor(ctx: &Context) -> Result<String> {
    let mut out = String::new();
    if ctx.asset_center.exists() {
        out.push_str("asset center: ok\n");
    } else {
        out.push_str("asset center: missing\n");
    }
    Ok(out)
}

pub fn mount_plan(
    ctx: &Context,
    name: &str,
    kind: AssetType,
    target: PathBuf,
    scope: Option<McpScope>,
) -> Result<Plan> {
    ensure_initialized(ctx)?;
    let registry = load_registry(ctx)?;
    let id = AssetId::new(kind, name);
    if !registry.assets.contains_key(&id) {
        return Err(MaaError::new(format!("asset not found: {id}")));
    }
    let mut plan = Plan::new("Mount plan");
    plan.push(PlanItem {
        kind: if id.kind == AssetType::Mcp {
            ActionKind::CompileMcp
        } else {
            ActionKind::ReplaceWithSymlink
        },
        asset: Some(id),
        source: None,
        target: Some(target),
        message: format!(
            "mount asset{}",
            scope
                .map(|s| format!(" with {} scope", s.as_str()))
                .unwrap_or_default()
        ),
        risk: "medium",
        details: Vec::new(),
    });
    Ok(plan)
}

pub fn mount_apply(
    ctx: &Context,
    name: &str,
    kind: AssetType,
    target: PathBuf,
    scope: Option<McpScope>,
) -> Result<Plan> {
    let plan = mount_plan(ctx, name, kind.clone(), target.clone(), scope.clone())?;
    let mut registry = load_registry(ctx)?;
    let id = AssetId::new(kind, name);
    let record = registry
        .assets
        .get(&id)
        .cloned()
        .ok_or_else(|| MaaError::new(format!("asset not found: {id}")))?;
    let asset_path = ctx.asset_center.join(record.path);
    match id.kind {
        AssetType::Skill => {
            let runtime_path = target.join(".claude/skills").join(&id.name);
            if runtime_path.exists() || is_symlink(&runtime_path) {
                if !is_symlink(&runtime_path) {
                    return Err(MaaError::new(format!(
                        "refusing to overwrite non-symlink runtime path without backup: {}",
                        runtime_path.display()
                    )));
                }
                remove_path(&runtime_path)?;
            }
            if let Some(parent) = runtime_path.parent() {
                fs::create_dir_all(parent)?;
            }
            create_symlink(&asset_path, &runtime_path)?;
            registry.mounts.entry(id).or_default().push(MountRecord {
                target,
                scope: None,
            });
        }
        AssetType::Command => {
            let file_name = record
                .file_name
                .unwrap_or_else(|| format!("{}.md", id.name));
            let runtime_path = target.join(".claude/commands").join(file_name);
            if runtime_path.exists() || is_symlink(&runtime_path) {
                if !is_symlink(&runtime_path) {
                    return Err(MaaError::new(format!(
                        "refusing to overwrite non-symlink runtime path without backup: {}",
                        runtime_path.display()
                    )));
                }
                remove_path(&runtime_path)?;
            }
            if let Some(parent) = runtime_path.parent() {
                fs::create_dir_all(parent)?;
            }
            create_symlink(&asset_path, &runtime_path)?;
            registry.mounts.entry(id).or_default().push(MountRecord {
                target,
                scope: None,
            });
        }
        AssetType::Mcp => {
            let mcp_scope = scope.unwrap_or(McpScope::User);
            registry
                .mounts
                .entry(id.clone())
                .or_default()
                .push(MountRecord {
                    target: target.clone(),
                    scope: Some(mcp_scope.clone()),
                });
            compile_mcp_for_target(ctx, &registry, &mcp_scope, &target, &[])?;
            save_registry(ctx, &registry)?;
            return Ok(plan);
        }
    }
    save_registry(ctx, &registry)?;
    Ok(plan)
}

pub fn unmount_apply(ctx: &Context, name: &str, kind: AssetType) -> Result<Plan> {
    ensure_initialized(ctx)?;
    let mut registry = load_registry(ctx)?;
    let id = AssetId::new(kind, name);
    let mounts = registry.mounts.remove(&id).unwrap_or_default();
    for mount in &mounts {
        match id.kind {
            AssetType::Skill => {
                remove_path_if_symlink(&mount.target.join(".claude/skills").join(&id.name))?
            }
            AssetType::Command => remove_path_if_symlink(
                &mount
                    .target
                    .join(".claude/commands")
                    .join(format!("{}.md", id.name)),
            )?,
            AssetType::Mcp => {
                if let Some(scope) = &mount.scope {
                    compile_mcp_for_target(
                        ctx,
                        &registry,
                        scope,
                        &mount.target,
                        std::slice::from_ref(&id.name),
                    )?;
                }
            }
        }
    }
    save_registry(ctx, &registry)?;
    let mut plan = Plan::new("Unmount apply");
    plan.push(PlanItem {
        kind: ActionKind::RemoveMount,
        asset: Some(id),
        source: None,
        target: None,
        message: "removed mount records and runtime materialization".to_string(),
        risk: "medium",
        details: Vec::new(),
    });
    Ok(plan)
}

pub fn remove_plan(ctx: &Context, name: &str, kind: AssetType) -> Result<Plan> {
    ensure_initialized(ctx)?;
    let registry = load_registry(ctx)?;
    let id = AssetId::new(kind, name);
    let record = registry
        .assets
        .get(&id)
        .ok_or_else(|| MaaError::new(format!("asset not found: {id}")))?;
    let mut plan = Plan::new("Remove plan");
    plan.push(PlanItem {
        kind: ActionKind::RemoveAsset,
        asset: Some(id),
        source: Some(ctx.asset_center.join(&record.path)),
        target: None,
        message: "remove asset and all mounts".to_string(),
        risk: "high",
        details: Vec::new(),
    });
    Ok(plan)
}

pub fn remove_apply(ctx: &Context, name: &str, kind: AssetType) -> Result<Plan> {
    let plan = remove_plan(ctx, name, kind.clone())?;
    unmount_apply(ctx, name, kind.clone())?;
    let mut registry = load_registry(ctx)?;
    let id = AssetId::new(kind, name);
    if let Some(record) = registry.assets.remove(&id) {
        remove_path(&ctx.asset_center.join(record.path))?;
    }
    registry.mounts.remove(&id);
    save_registry(ctx, &registry)?;
    Ok(plan)
}

pub fn sync_command(ctx: &Context, op: &str) -> Result<String> {
    ensure_initialized(ctx)?;
    let args: &[&str] = match op {
        "pull" => &["--no-pager", "-c", "pull.ff=only", "pull", "--ff-only"],
        "push" => &["--no-pager", "-c", "push.default=simple", "push"],
        other => return Err(MaaError::new(format!("unknown sync operation: {other}"))),
    };
    let output = Command::new("git")
        .args(args)
        .current_dir(&ctx.asset_center)
        .output()
        .map_err(git_command_error)?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    let text = sanitize_command_output(&text);
    if !output.status.success() {
        return Err(MaaError::new(text));
    }
    Ok(text)
}

fn sanitize_command_output(text: &str) -> String {
    text.lines()
        .map(sanitize_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn sanitize_line(line: &str) -> String {
    line.split_whitespace()
        .map(sanitize_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn sanitize_token(token: &str) -> String {
    for scheme in ["https://", "http://", "ssh://"] {
        if let Some(rest) = token.strip_prefix(scheme) {
            if let Some(at) = rest.find('@') {
                return format!("{scheme}***@{}", &rest[at + 1..]);
            }
        }
    }
    for marker in ["gho_", "ghp_", "github_pat_", "glpat-"] {
        if let Some(pos) = token.find(marker) {
            let prefix = &token[..pos + marker.len()];
            return format!("{prefix}***");
        }
    }
    token.to_string()
}

fn git_command_error(err: std::io::Error) -> MaaError {
    if err.kind() == ErrorKind::NotFound {
        MaaError::new("Git is not installed or not in PATH. Git is required for init and sync.")
    } else {
        MaaError::new(err.to_string())
    }
}

fn imported_item(id: &AssetId, source: &Path, target: &Path) -> PlanItem {
    PlanItem {
        kind: ActionKind::ImportAsset,
        asset: Some(id.clone()),
        source: Some(source.to_path_buf()),
        target: Some(target.to_path_buf()),
        message: "imported asset".to_string(),
        risk: "medium",
        details: Vec::new(),
    }
}

fn import_new_mcp_asset(
    ctx: &Context,
    registry: &mut Registry,
    asset: &DiscoveredAsset,
    id: AssetId,
    plan: &mut Plan,
) -> Result<()> {
    let asset_path = asset_center_path(ctx, &id);
    let config = asset
        .mcp_config
        .clone()
        .ok_or_else(|| MaaError::new("MCP asset missing scanned config"))?;
    if let Some(parent) = asset_path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_text_atomic(&asset_path, &json_to_pretty(&config)?)?;
    registry.assets.insert(
        id.clone(),
        AssetRecord {
            path: relative_to(&asset_path, &ctx.asset_center)?,
            file_name: None,
        },
    );
    let scope = asset.mcp_scope.clone().unwrap_or(McpScope::User);
    add_mount_if_missing(
        registry,
        id.clone(),
        asset.runtime_root.clone(),
        Some(scope.clone()),
    );
    plan.push(imported_item(&id, &asset.source, &asset_path));
    Ok(())
}

fn add_mount_if_missing(
    registry: &mut Registry,
    id: AssetId,
    target: PathBuf,
    scope: Option<McpScope>,
) {
    let mounts = registry.mounts.entry(id).or_default();
    if mounts
        .iter()
        .any(|mount| mount.target == target && mount.scope == scope)
    {
        return;
    }
    mounts.push(MountRecord { target, scope });
}

fn mcp_has_different_existing(
    ctx: &Context,
    registry: &Registry,
    asset: &DiscoveredAsset,
) -> Result<bool> {
    if asset.id.kind != AssetType::Mcp {
        return Ok(false);
    }
    let Some(record) = registry.assets.get(&asset.id) else {
        return Ok(false);
    };
    let existing_json = parse_json_file(&ctx.asset_center.join(&record.path))?;
    let incoming_json = asset
        .mcp_config
        .clone()
        .ok_or_else(|| MaaError::new("MCP asset missing scanned config"))?;
    Ok(existing_json != incoming_json)
}

fn mcp_conflict_details(
    ctx: &Context,
    existing: &AssetRecord,
    asset: &DiscoveredAsset,
) -> Result<Vec<String>> {
    if asset.id.kind != AssetType::Mcp {
        return Ok(Vec::new());
    }
    let existing_json = parse_json_file(&ctx.asset_center.join(&existing.path))?;
    let incoming_json = asset
        .mcp_config
        .clone()
        .ok_or_else(|| MaaError::new("MCP asset missing scanned config"))?;
    if existing_json == incoming_json {
        return Ok(Vec::new());
    }
    Ok(vec![
        format!(
            "asset-center-json:\n{}",
            json_to_pretty(&existing_json)?.trim_end()
        ),
        format!(
            "scanned-runtime-json:\n{}",
            json_to_pretty(&incoming_json)?.trim_end()
        ),
        "resolve with: --on-conflict skip | --on-conflict overwrite | --on-conflict rename --rename-to <new-name>".to_string(),
    ])
}

fn ensure_initialized(ctx: &Context) -> Result<()> {
    if !ctx.asset_center.exists() {
        return Err(MaaError::new(format!(
            "asset center is not initialized: {}",
            ctx.asset_center.display()
        )));
    }
    Ok(())
}

fn write_if_missing(path: &Path, content: &str) -> Result<()> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
    }
    Ok(())
}

fn init_asset_center_git(ctx: &Context) -> Result<()> {
    if ctx.asset_center.join(".git").exists() {
        return Ok(());
    }
    let output = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(&ctx.asset_center)
        .output()
        .map_err(git_command_error)?;
    if !output.status.success() {
        let mut message = String::from("failed to initialize asset center git repository\n");
        message.push_str(&String::from_utf8_lossy(&output.stdout));
        message.push_str(&String::from_utf8_lossy(&output.stderr));
        return Err(MaaError::new(message));
    }
    Ok(())
}

fn asset_center_path(ctx: &Context, id: &AssetId) -> PathBuf {
    match id.kind {
        AssetType::Skill => ctx.asset_center.join("assets/skills").join(&id.name),
        AssetType::Command => ctx
            .asset_center
            .join("assets/commands")
            .join(format!("{}.md", id.name)),
        AssetType::Mcp => ctx
            .asset_center
            .join("assets/mcps")
            .join(format!("{}.json", id.name)),
    }
}

fn discover(ctx: &Context) -> Result<Vec<DiscoveredAsset>> {
    let mut out = Vec::new();
    let user_claude = ctx.home.join(".claude");
    discover_claude_dir(&user_claude, &ctx.home, &mut out)?;
    discover_user_mcp(ctx, &mut out)?;
    let max_depth = scan_max_depth(ctx)?;
    for root in scan_roots(ctx)? {
        discover_projects(&root, 0, max_depth, &mut out)?;
        discover_project_mcp(&root, 0, max_depth, &mut out)?;
    }
    Ok(out)
}

fn scan_max_depth(ctx: &Context) -> Result<usize> {
    Ok(load_config(ctx)?.max_depth.unwrap_or(5))
}

fn scan_roots(ctx: &Context) -> Result<Vec<PathBuf>> {
    let roots = load_config(ctx)?
        .scan_roots
        .iter()
        .map(|root| expand_home(ctx, root))
        .collect::<Vec<_>>();
    Ok(roots.into_iter().filter(|p| p.exists()).collect())
}

fn load_config(ctx: &Context) -> Result<ConfigFile> {
    let path = ctx.asset_center.join("config.yaml");
    if !path.exists() {
        return Ok(ConfigFile::default());
    }
    serde_yaml::from_str(&fs::read_to_string(path)?)
        .map_err(|err| MaaError::new(format!("failed to parse config.yaml: {err}")))
}

fn expand_home(ctx: &Context, value: &str) -> PathBuf {
    if value == "~" {
        ctx.home.clone()
    } else if let Some(rest) = value.strip_prefix("~/") {
        ctx.home.join(rest)
    } else {
        PathBuf::from(value)
    }
}

fn discover_projects(
    root: &Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<DiscoveredAsset>,
) -> Result<()> {
    if depth > max_depth || should_skip_dir(root) {
        return Ok(());
    }
    let claude = root.join(".claude");
    if claude.is_dir() {
        discover_claude_dir(&claude, root, out)?;
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && !is_symlink(&path) {
            discover_projects(&path, depth + 1, max_depth, out)?;
        }
    }
    Ok(())
}

fn discover_claude_dir(
    claude: &Path,
    runtime_root: &Path,
    out: &mut Vec<DiscoveredAsset>,
) -> Result<()> {
    let skills = claude.join("skills");
    if skills.is_dir() {
        for entry in fs::read_dir(skills)? {
            let path = entry?.path();
            if path.is_dir() && !is_symlink(&path) {
                if let Some(name) = path.file_name().and_then(|v| v.to_str()) {
                    out.push(DiscoveredAsset {
                        id: AssetId::new(AssetType::Skill, name),
                        source: path,
                        runtime_root: runtime_root.to_path_buf(),
                        mcp_scope: None,
                        mcp_config: None,
                    });
                }
            }
        }
    }
    let commands = claude.join("commands");
    if commands.is_dir() {
        for entry in fs::read_dir(commands)? {
            let path = entry?.path();
            if path.is_file() && path.extension().and_then(|v| v.to_str()) == Some("md") {
                if let Some(stem) = path.file_stem().and_then(|v| v.to_str()) {
                    out.push(DiscoveredAsset {
                        id: AssetId::new(AssetType::Command, stem),
                        source: path,
                        runtime_root: runtime_root.to_path_buf(),
                        mcp_scope: None,
                        mcp_config: None,
                    });
                }
            }
        }
    }
    Ok(())
}

fn discover_user_mcp(ctx: &Context, out: &mut Vec<DiscoveredAsset>) -> Result<()> {
    let path = ctx.home.join(".claude.json");
    if !path.exists() {
        return Ok(());
    }
    let json = parse_json_file(&path)?;
    if let Some(servers) = json.get("mcpServers").and_then(|value| value.as_object()) {
        for (name, config) in servers {
            out.push(DiscoveredAsset {
                id: AssetId::new(AssetType::Mcp, name.clone()),
                source: path.clone(),
                runtime_root: ctx.home.clone(),
                mcp_scope: Some(McpScope::User),
                mcp_config: Some(config.clone()),
            });
        }
    }
    if let Some(projects) = json.get("projects").and_then(|value| value.as_object()) {
        for (project_path, project_value) in projects {
            if let Some(servers) = project_value
                .get("mcpServers")
                .and_then(|value| value.as_object())
            {
                for (name, config) in servers {
                    out.push(DiscoveredAsset {
                        id: AssetId::new(AssetType::Mcp, name.clone()),
                        source: path.clone(),
                        runtime_root: PathBuf::from(project_path.clone()),
                        mcp_scope: Some(McpScope::Local),
                        mcp_config: Some(config.clone()),
                    });
                }
            }
        }
    }
    Ok(())
}

fn discover_project_mcp(
    root: &Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<DiscoveredAsset>,
) -> Result<()> {
    if depth > max_depth || should_skip_dir(root) {
        return Ok(());
    }
    let path = root.join(".mcp.json");
    if path.exists() {
        let json = parse_json_file(&path)?;
        if let Some(servers) = json.get("mcpServers").and_then(|value| value.as_object()) {
            for (name, config) in servers {
                out.push(DiscoveredAsset {
                    id: AssetId::new(AssetType::Mcp, name.clone()),
                    source: path.clone(),
                    runtime_root: root.to_path_buf(),
                    mcp_scope: Some(McpScope::Project),
                    mcp_config: Some(config.clone()),
                });
            }
        }
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() && !should_skip_dir(&path) && !is_symlink(&path) {
            discover_project_mcp(&path, depth + 1, max_depth, out)?;
        }
    }
    Ok(())
}

fn should_skip_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|v| v.to_str()),
        Some(".git" | "node_modules" | "dist" | "build" | "target" | ".venv" | "__pycache__")
    )
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

fn load_registry(ctx: &Context) -> Result<Registry> {
    let mut registry = Registry::default();
    let assets = ctx.asset_center.join("assets.yaml");
    if assets.exists() {
        let disk: DiskAssetsFile = serde_yaml::from_str(&fs::read_to_string(assets)?)
            .map_err(|err| MaaError::new(format!("failed to parse assets.yaml: {err}")))?;
        load_asset_group(&mut registry, AssetType::Skill, disk.assets.skills);
        load_asset_group(&mut registry, AssetType::Command, disk.assets.commands);
        load_asset_group(&mut registry, AssetType::Mcp, disk.assets.mcps);
    }
    let mounts = ctx.asset_center.join("mounts.yaml");
    if mounts.exists() {
        let disk: DiskMountsFile = serde_yaml::from_str(&fs::read_to_string(mounts)?)
            .map_err(|err| MaaError::new(format!("failed to parse mounts.yaml: {err}")))?;
        load_mount_group(&mut registry, AssetType::Skill, disk.mounts.skills)?;
        load_mount_group(&mut registry, AssetType::Command, disk.mounts.commands)?;
        load_mount_group(&mut registry, AssetType::Mcp, disk.mounts.mcps)?;
    }
    Ok(registry)
}

fn save_registry(ctx: &Context, registry: &Registry) -> Result<()> {
    let mut assets = DiskAssetsFile::default();
    for (id, record) in &registry.assets {
        let disk = DiskAssetRecord {
            path: path_to_portable(&record.path),
            file_name: record.file_name.clone(),
            aliases: Vec::new(),
        };
        match id.kind {
            AssetType::Skill => assets.assets.skills.insert(id.name.clone(), disk),
            AssetType::Command => assets.assets.commands.insert(id.name.clone(), disk),
            AssetType::Mcp => assets.assets.mcps.insert(id.name.clone(), disk),
        };
    }
    write_text_atomic(
        &ctx.asset_center.join("assets.yaml"),
        &serde_yaml::to_string(&assets)
            .map_err(|err| MaaError::new(format!("failed to write assets.yaml: {err}")))?,
    )?;

    let mut mounts = DiskMountsFile::default();
    for (id, records) in &registry.mounts {
        for record in records {
            let disk = DiskMountRecord {
                target: record.target.clone(),
                scope: record
                    .scope
                    .as_ref()
                    .map(|scope| scope.as_str().to_string()),
            };
            match id.kind {
                AssetType::Skill => mounts
                    .mounts
                    .skills
                    .entry(id.name.clone())
                    .or_default()
                    .push(disk),
                AssetType::Command => mounts
                    .mounts
                    .commands
                    .entry(id.name.clone())
                    .or_default()
                    .push(disk),
                AssetType::Mcp => mounts
                    .mounts
                    .mcps
                    .entry(id.name.clone())
                    .or_default()
                    .push(disk),
            }
        }
    }
    write_text_atomic(
        &ctx.asset_center.join("mounts.yaml"),
        &serde_yaml::to_string(&mounts)
            .map_err(|err| MaaError::new(format!("failed to write mounts.yaml: {err}")))?,
    )?;
    Ok(())
}

fn write_text_atomic(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn load_asset_group(
    registry: &mut Registry,
    kind: AssetType,
    group: BTreeMap<String, DiskAssetRecord>,
) {
    for (name, record) in group {
        registry.assets.insert(
            AssetId::new(kind.clone(), name),
            AssetRecord {
                path: PathBuf::from(record.path),
                file_name: record.file_name,
            },
        );
    }
}

fn load_mount_group(
    registry: &mut Registry,
    kind: AssetType,
    group: BTreeMap<String, Vec<DiskMountRecord>>,
) -> Result<()> {
    for (name, records) in group {
        let id = AssetId::new(kind.clone(), name);
        for record in records {
            registry
                .mounts
                .entry(id.clone())
                .or_default()
                .push(MountRecord {
                    target: record.target,
                    scope: record.scope.as_deref().map(McpScope::parse).transpose()?,
                });
        }
    }
    Ok(())
}

fn compile_mcp_for_target(
    ctx: &Context,
    registry: &Registry,
    scope: &McpScope,
    target: &Path,
    extra_remove: &[String],
) -> Result<()> {
    let mut managed = BTreeMap::<String, JsonValue>::new();
    let mut names_to_replace = BTreeSet::<String>::new();
    for (id, mounts) in &registry.mounts {
        if id.kind != AssetType::Mcp {
            continue;
        }
        let mounted = mounts
            .iter()
            .any(|m| m.target == target && m.scope.as_ref() == Some(scope));
        if mounted {
            names_to_replace.insert(id.name.clone());
            if let Some(record) = registry.assets.get(id) {
                let path = ctx.asset_center.join(&record.path);
                managed.insert(id.name.clone(), parse_json_file(&path)?);
            }
        }
    }
    for name in extra_remove {
        names_to_replace.insert(name.clone());
    }

    match scope {
        McpScope::User => {
            let path = ctx.home.join(".claude.json");
            let mut json = read_json_or_object(&path)?;
            merge_mcp_servers(&mut json, &names_to_replace, managed);
            write_text_atomic(&path, &json_to_pretty(&json)?)?;
        }
        McpScope::Local => {
            let path = ctx.home.join(".claude.json");
            let mut json = read_json_or_object(&path)?;
            let obj = ensure_json_object(&mut json);
            let projects_value = obj
                .entry("projects".to_string())
                .or_insert_with(json_object);
            let projects = ensure_json_object(projects_value);
            let project = projects
                .entry(target.display().to_string())
                .or_insert_with(json_object);
            merge_mcp_servers(project, &names_to_replace, managed);
            write_text_atomic(&path, &json_to_pretty(&json)?)?;
        }
        McpScope::Project => {
            let path = target.join(".mcp.json");
            let mut json = read_json_or_object(&path)?;
            merge_mcp_servers(&mut json, &names_to_replace, managed);
            write_text_atomic(&path, &json_to_pretty(&json)?)?;
        }
    }
    Ok(())
}

fn read_json_or_object(path: &Path) -> Result<JsonValue> {
    if path.exists() {
        parse_json_file(path)
    } else {
        Ok(json_object())
    }
}

fn merge_mcp_servers(
    json: &mut JsonValue,
    all_managed_names: &BTreeSet<String>,
    managed: BTreeMap<String, JsonValue>,
) {
    let obj = ensure_json_object(json);
    let servers_value = obj
        .entry("mcpServers".to_string())
        .or_insert_with(json_object);
    let servers = ensure_json_object(servers_value);
    for name in all_managed_names {
        servers.remove(name);
    }
    for (name, value) in managed {
        servers.insert(name, value);
    }
}

fn backup_id() -> String {
    static BACKUP_COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let seq = BACKUP_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("backup-{nanos}-{}-{seq}", std::process::id())
}

#[derive(Debug)]
struct BackupManifest {
    id: String,
    entries: Vec<BackupEntry>,
}

#[derive(Debug)]
struct BackupEntry {
    original: PathBuf,
    backup: PathBuf,
    kind: String,
}

impl BackupManifest {
    fn new(id: String) -> Self {
        Self {
            id,
            entries: Vec::new(),
        }
    }

    fn render(&self) -> String {
        let mut out = format!("id|{}\n", self.id);
        for entry in &self.entries {
            out.push_str(&format!(
                "entry|{}|{}|{}\n",
                entry.kind,
                entry.original.display(),
                entry.backup.display()
            ));
        }
        out
    }
}

fn backup_path(path: &Path, backup_root: &Path, manifest: &mut BackupManifest) -> Result<()> {
    fs::create_dir_all(backup_root)?;
    let backup = backup_root.join(format!("item-{}", manifest.entries.len()));
    let kind = if path.is_dir() { "dir" } else { "file" };
    if path.is_dir() {
        copy_dir_all(path, &backup)?;
    } else {
        fs::copy(path, &backup)?;
    }
    manifest.entries.push(BackupEntry {
        original: path.to_path_buf(),
        backup,
        kind: kind.to_string(),
    });
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<()> {
    if !path.exists() && !is_symlink(path) {
        return Ok(());
    }
    if path.is_dir() && !is_symlink(path) {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn remove_path_if_symlink(path: &Path) -> Result<()> {
    if is_symlink(path) {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn relative_to(path: &Path, base: &Path) -> Result<PathBuf> {
    path.strip_prefix(base).map(Path::to_path_buf).map_err(|_| {
        MaaError::new(format!(
            "{} is not inside {}",
            path.display(),
            base.display()
        ))
    })
}

fn path_to_portable(path: &Path) -> String {
    path.iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(unix)]
fn create_symlink(src: &Path, dst: &Path) -> Result<()> {
    std::os::unix::fs::symlink(src, dst).map_err(|err| {
        MaaError::new(format!(
            "failed to create symlink {} -> {}: {err}",
            dst.display(),
            src.display()
        ))
    })
}

#[cfg(windows)]
fn create_symlink(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        std::os::windows::fs::symlink_dir(src, dst).map_err(|err| {
            MaaError::new(format!(
                "failed to create directory symlink {} -> {}: {err}. Enable Windows Developer Mode or run as administrator.",
                dst.display(),
                src.display()
            ))
        })
    } else {
        std::os::windows::fs::symlink_file(src, dst).map_err(|err| {
            MaaError::new(format!(
                "failed to create file symlink {} -> {}: {err}. Enable Windows Developer Mode or run as administrator.",
                dst.display(),
                src.display()
            ))
        })
    }
}

fn parse_json_file(path: &Path) -> Result<JsonValue> {
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text)
        .map_err(|err| MaaError::new(format!("failed to parse JSON {}: {err}", path.display())))
}

fn json_to_pretty(value: &JsonValue) -> Result<String> {
    let mut text = serde_json::to_string_pretty(value)
        .map_err(|err| MaaError::new(format!("failed to serialize JSON: {err}")))?;
    text.push('\n');
    Ok(text)
}

fn json_object() -> JsonValue {
    JsonValue::Object(JsonMap::new())
}

fn ensure_json_object(value: &mut JsonValue) -> &mut JsonMap<String, JsonValue> {
    if !value.is_object() {
        *value = json_object();
    }
    value
        .as_object_mut()
        .expect("value was just forced to object")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_asset_removes_md_extension() {
        let id = AssetId::new(AssetType::Command, "commit");
        assert_eq!(id.to_string(), "command:commit");
    }

    #[test]
    fn parses_and_preserves_json_objects() {
        let json: JsonValue =
            serde_json::from_str(r#"{"mcpServers":{"github":{"command":"npx","args":["x"]}}}"#)
                .unwrap();
        assert!(json.get("mcpServers").is_some());
        assert!(json_to_pretty(&json).unwrap().contains("\"github\""));
    }

    #[test]
    fn serde_json_handles_unicode_surrogate_pairs() {
        let json: JsonValue = serde_json::from_str(r#"{"emoji":"\uD83D\uDE00"}"#).unwrap();
        assert_eq!(json["emoji"], "😀");
    }

    #[test]
    fn config_yaml_supports_comments_and_max_depth() {
        let root = test_dir("config-yaml");
        let ctx = Context::new(root.clone());
        fs::create_dir_all(&ctx.asset_center).unwrap();
        fs::create_dir_all(root.join("workspace with spaces")).unwrap();
        fs::write(
            ctx.asset_center.join("config.yaml"),
            "scan_roots:\n  - \"~/workspace with spaces\" # comment\nmax_depth: 2\n",
        )
        .unwrap();
        assert_eq!(scan_max_depth(&ctx).unwrap(), 2);
        assert_eq!(
            scan_roots(&ctx).unwrap(),
            vec![root.join("workspace with spaces")]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn init_creates_versioned_asset_center_and_is_idempotent() {
        let root = test_dir("init-versioned");
        let ctx = Context::new(root.clone());
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::create_dir_all(root.join(".codex")).unwrap();

        init_apply(&ctx).unwrap();

        for relative in [
            "assets/skills",
            "assets/commands",
            "assets/mcps",
            "backups/portable",
            "backups/local",
        ] {
            assert!(ctx.asset_center.join(relative).is_dir(), "{relative}");
        }

        let config = fs::read_to_string(ctx.asset_center.join("config.yaml")).unwrap();
        assert!(config.contains("schemaVersion: 1"));
        assert!(!config.contains("assetCenterPath"));

        let targets = fs::read_to_string(ctx.asset_center.join("targets.yaml")).unwrap();
        assert!(targets.contains("schemaVersion: 1"));
        assert!(targets.contains("claude-user"));
        assert!(targets.contains("codex-user"));

        let gitignore = fs::read_to_string(ctx.asset_center.join(".gitignore")).unwrap();
        assert!(gitignore.contains("backups/local/"));
        assert!(gitignore.contains("operations/"));

        if ctx.asset_center.join(".git").is_dir() {
            let branch = Command::new("git")
                .args(["branch", "--show-current"])
                .current_dir(&ctx.asset_center)
                .output()
                .unwrap();
            assert!(branch.status.success());
            assert_eq!(String::from_utf8_lossy(&branch.stdout).trim(), "main");
        }

        fs::write(
            ctx.asset_center.join("assets.yaml"),
            "schemaVersion: 1\nassets:\n  preserved: true\n",
        )
        .unwrap();
        init_apply(&ctx).unwrap();
        assert_eq!(
            fs::read_to_string(ctx.asset_center.join("assets.yaml")).unwrap(),
            "schemaVersion: 1\nassets:\n  preserved: true\n"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn registry_round_trips_as_yaml() {
        let root = test_dir("registry-yaml");
        let ctx = Context::new(root.clone());
        init_apply(&ctx).unwrap();
        let mut registry = Registry::default();
        registry.assets.insert(
            AssetId::new(AssetType::Command, "commit"),
            AssetRecord {
                path: PathBuf::from("assets/commands/commit.md"),
                file_name: Some("commit.md".to_string()),
            },
        );
        registry.mounts.insert(
            AssetId::new(AssetType::Command, "commit"),
            vec![MountRecord {
                target: root.clone(),
                scope: None,
            }],
        );
        save_registry(&ctx, &registry).unwrap();
        let assets = fs::read_to_string(ctx.asset_center.join("assets.yaml")).unwrap();
        assert!(assets.contains("commands:"));
        let loaded = load_registry(&ctx).unwrap();
        assert!(loaded
            .assets
            .contains_key(&AssetId::new(AssetType::Command, "commit")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn backup_ids_are_not_equal_for_quick_successive_calls() {
        let first = backup_id();
        let second = backup_id();
        assert_ne!(first, second);
    }

    #[test]
    fn relative_to_rejects_paths_outside_base() {
        let err = relative_to(Path::new("/tmp/outside"), Path::new("/var/base")).unwrap_err();
        assert!(err.to_string().contains("is not inside"));
    }

    #[test]
    fn sanitizes_git_output_without_collapsing_lines() {
        let output = "remote https://ghp_secret@example.com/repo.git\nfatal token glpat-secret";
        let sanitized = sanitize_command_output(output);
        assert!(sanitized.contains("https://***@example.com/repo.git"));
        assert!(sanitized.contains("glpat-***"));
        assert!(sanitized.contains('\n'));
        assert!(!sanitized.contains("ghp_secret"));
        assert!(!sanitized.contains("glpat-secret"));
    }

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "my-agent-assets-test-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&path);
        path
    }
}
