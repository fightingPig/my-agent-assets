use crate::asset_registry::{
    canonical_path, load as load_assets, parse_asset_id, registry_path as asset_registry_path,
};
use crate::fingerprint::PreviewFingerprint;
use crate::fs_sync::sync_directory;
use crate::mcp::{
    patch_claude_json, patch_codex_toml, remove_from_claude_json, remove_from_codex_toml,
    CanonicalMcp, ClaudeScope, CodexScope,
};
use crate::mount_registry::{
    load as load_mounts, registry_path as mount_registry_path, save as save_mounts, BindingStatus,
    MountBinding,
};
use crate::operation::{OperationJournal, OperationLock, RecoveryTarget};
use crate::path_safety::{guard_write_path, is_link_or_junction};
use crate::targets::{load as load_targets, MountAdapter, MountTarget, MountTargetKind};
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PREVIEW_TTL_SECONDS: u64 = 300;
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountPreviewRequest {
    pub asset_id: String,
    pub target_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MountDisposition {
    #[serde(rename = "create_link")]
    CreateLink,
    #[serde(rename = "replace_runtime_path")]
    ReplaceRuntimePath,
    #[serde(rename = "compile_mcp")]
    CompileMcp,
    #[serde(rename = "already_mounted")]
    AlreadyMounted,
    #[serde(rename = "blocked")]
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountPreview {
    pub preview_id: String,
    pub asset_id: String,
    pub target_id: String,
    pub canonical_path: PathBuf,
    pub affected_target_path: PathBuf,
    pub compatible: bool,
    pub adapter: MountAdapter,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unsupported_reason: Option<String>,
    pub disposition: MountDisposition,
    pub planned_effects: Vec<String>,
    pub warnings: Vec<String>,
    pub backup_required: bool,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: MountPreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountApplyResult {
    pub preview_id: String,
    pub asset_id: String,
    pub target_id: String,
    pub mounted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,
    pub affected_paths: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnmountPreviewRequest {
    pub asset_id: String,
    pub target_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnmountPreview {
    pub preview_id: String,
    pub asset_id: String,
    pub target_id: String,
    pub affected_target_path: PathBuf,
    pub planned_effects: Vec<String>,
    pub warnings: Vec<String>,
    pub backup_required: bool,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnmountApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: UnmountPreviewRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnmountApplyResult {
    pub preview_id: String,
    pub asset_id: String,
    pub target_id: String,
    pub unmounted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,
    pub affected_paths: Vec<PathBuf>,
}

pub fn preview_mount(home: &Path, request: &MountPreviewRequest) -> Result<MountPreview> {
    preview_mount_at(home, request, epoch_seconds())
}

pub(crate) fn preview_mount_at(
    home: &Path,
    request: &MountPreviewRequest,
    generated_at_epoch_seconds: u64,
) -> Result<MountPreview> {
    let (kind, name) =
        parse_asset_id(&request.asset_id).map_err(|error| MaaError::new(error.to_string()))?;
    let assets = load_assets(home).map_err(|error| MaaError::new(error.to_string()))?;
    if assets.get(kind, &name).is_none() {
        return Err(MaaError::new(format!(
            "asset '{}' is not registered",
            request.asset_id
        )));
    }
    let canonical = canonical_path(home, kind, &name);
    if !canonical.exists() {
        return Err(MaaError::new(format!(
            "canonical content is missing: {}",
            canonical.display()
        )));
    }

    let targets = load_targets(home)?;
    let target = targets.resolve(&request.target_id)?;
    let compatibility = target.compatibility(kind);
    let affected_target_path = target_asset_path(target, kind, &name);
    guard_target_path(target, &affected_target_path)?;
    let mut warnings = compatibility.reason.clone().into_iter().collect::<Vec<_>>();
    let (disposition, backup_required, renderer_ok) = if !compatibility.compatible {
        (MountDisposition::Blocked, false, false)
    } else {
        preview_disposition(&canonical, &affected_target_path, target, &mut warnings)?
    };
    let can_apply = compatibility.compatible && renderer_ok;
    let planned_effects = planned_effects(
        disposition,
        &canonical,
        &affected_target_path,
        target.adapter,
    );
    let preview_id = mount_fingerprint(
        home,
        request,
        &canonical,
        &affected_target_path,
        generated_at_epoch_seconds,
    )?;
    Ok(MountPreview {
        preview_id,
        asset_id: request.asset_id.clone(),
        target_id: request.target_id.clone(),
        canonical_path: canonical,
        affected_target_path,
        compatible: compatibility.compatible,
        adapter: target.adapter,
        unsupported_reason: compatibility.reason,
        disposition,
        planned_effects,
        warnings,
        backup_required,
        can_apply,
        generated_at_epoch_seconds,
        expires_at_epoch_seconds: generated_at_epoch_seconds.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

fn planned_effects(
    disposition: MountDisposition,
    canonical: &Path,
    target: &Path,
    adapter: MountAdapter,
) -> Vec<String> {
    match disposition {
        MountDisposition::CreateLink | MountDisposition::ReplaceRuntimePath => vec![format!(
            "link {} to {} using {:?}",
            target.display(),
            canonical.display(),
            adapter
        )],
        MountDisposition::CompileMcp => vec![format!(
            "patch only the selected MCP server entry in {}",
            target.display()
        )],
        MountDisposition::AlreadyMounted => vec![
            "runtime target already resolves to the canonical asset; refresh binding state".into(),
        ],
        MountDisposition::Blocked => Vec::new(),
    }
}

pub fn apply_mount(home: &Path, request: &MountApplyRequest) -> Result<MountApplyResult> {
    let _operation_lock = OperationLock::acquire(home)?;
    let preview = preview_mount_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    if preview.preview_id != request.preview_id || !preview.can_apply {
        return apply_mount_locked(home, request);
    }
    let operation_id = operation_id();
    let mut journal = OperationJournal::start_recoverable(
        home,
        &operation_id,
        "mount",
        vec![
            RecoveryTarget::asset_center(mount_registry_path(home)),
            RecoveryTarget::registered_target(
                request.request.target_id.clone(),
                preview.affected_target_path.clone(),
            ),
        ],
    )?;
    match apply_mount_locked(home, request) {
        Ok(result) => {
            journal.record_step("mount_applied")?;
            journal.complete()?;
            Ok(result)
        }
        Err(error) => {
            let original = error.to_string();
            journal.rollback_now(home).map_err(|rollback| {
                MaaError::new(format!(
                    "{original}; persistent mount rollback failed: {rollback}"
                ))
            })?;
            Err(error)
        }
    }
}

pub(crate) fn apply_mount_locked(
    home: &Path,
    request: &MountApplyRequest,
) -> Result<MountApplyResult> {
    if epoch_seconds()
        > request
            .preview_generated_at_epoch_seconds
            .saturating_add(PREVIEW_TTL_SECONDS)
    {
        return Err(MaaError::new(
            "mount preview expired; generate a new preview before applying",
        ));
    }
    let preview = preview_mount_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    if preview.preview_id != request.preview_id {
        return Err(MaaError::new(
            "mount preview is stale; generate a new preview before applying",
        ));
    }
    if !preview.can_apply {
        return Err(MaaError::new(
            preview
                .warnings
                .first()
                .cloned()
                .unwrap_or_else(|| "mount target is blocked".into()),
        ));
    }

    let (kind, name) = parse_asset_id(&request.request.asset_id)
        .map_err(|error| MaaError::new(error.to_string()))?;
    let targets = load_targets(home)?;
    let target = targets.resolve_for_apply(&request.request.target_id, kind)?;
    guard_target_path(target, &preview.affected_target_path)?;
    let mut mounts = load_mounts(home).map_err(|error| MaaError::new(error.to_string()))?;
    let original_mounts = fs::read(mount_registry_path(home))?;
    let backup_id = if preview.backup_required {
        Some(create_local_backup(
            home,
            &preview.affected_target_path,
            &original_mounts,
        )?)
    } else {
        None
    };
    let mut runtime_snapshot = Some(snapshot_runtime_path(&preview.affected_target_path)?);

    let warnings = match apply_runtime(
        &preview.canonical_path,
        &preview.affected_target_path,
        target,
        kind,
        &name,
    ) {
        Ok(warnings) => warnings,
        Err(error) => {
            if let Some(snapshot) = runtime_snapshot.take() {
                let _ = restore_runtime_snapshot(&preview.affected_target_path, snapshot);
            }
            let _ = fs::write(mount_registry_path(home), &original_mounts);
            return Err(error);
        }
    };
    let result = (|| -> Result<()> {
        let mut binding = MountBinding::new(
            request.request.asset_id.clone(),
            request.request.target_id.clone(),
            BindingStatus::Mounted,
        )
        .map_err(|error| MaaError::new(error.to_string()))?;
        binding.last_synced_at = Some(humantime::format_rfc3339(SystemTime::now()).to_string());
        mounts
            .upsert(binding)
            .map_err(|error| MaaError::new(error.to_string()))?;
        save_mounts(home, &mounts).map_err(|error| MaaError::new(error.to_string()))
    })();
    if let Err(error) = result {
        if let Some(snapshot) = runtime_snapshot.take() {
            let _ = restore_runtime_snapshot(&preview.affected_target_path, snapshot);
        }
        let _ = fs::write(mount_registry_path(home), original_mounts);
        return Err(error);
    }
    if let Some(snapshot) = runtime_snapshot.take() {
        discard_runtime_snapshot(snapshot)?;
    }

    Ok(MountApplyResult {
        preview_id: preview.preview_id,
        asset_id: request.request.asset_id.clone(),
        target_id: request.request.target_id.clone(),
        mounted: true,
        backup_id,
        affected_paths: vec![preview.affected_target_path, mount_registry_path(home)],
        warnings,
    })
}

pub fn preview_unmount(home: &Path, request: &UnmountPreviewRequest) -> Result<UnmountPreview> {
    preview_unmount_at(home, request, epoch_seconds())
}

fn preview_unmount_at(
    home: &Path,
    request: &UnmountPreviewRequest,
    generated_at_epoch_seconds: u64,
) -> Result<UnmountPreview> {
    let (kind, name) =
        parse_asset_id(&request.asset_id).map_err(|error| MaaError::new(error.to_string()))?;
    let canonical = canonical_path(home, kind, &name);
    let targets = load_targets(home)?;
    let target = targets.resolve(&request.target_id)?;
    let affected_target_path = target_asset_path(target, kind, &name);
    guard_target_path(target, &affected_target_path)?;
    let mounts = load_mounts(home).map_err(|error| MaaError::new(error.to_string()))?;
    let binding_exists = mounts
        .for_asset(&request.asset_id)
        .iter()
        .any(|binding| binding.target_id == request.target_id);
    let mut warnings = Vec::new();
    let mut can_apply = binding_exists;
    if !binding_exists {
        warnings.push("no matching local mount binding exists".into());
    }
    if matches!(
        target.adapter,
        MountAdapter::SymlinkDirectory
            | MountAdapter::WindowsDirectoryJunction
            | MountAdapter::SymlinkFile
    ) && !link_points_to(&affected_target_path, &canonical)?
    {
        can_apply = false;
        warnings.push(
            "runtime target is not a link to this canonical asset; refusing to delete it".into(),
        );
    }
    if matches!(
        target.adapter,
        MountAdapter::JsonMcpPatch | MountAdapter::TomlMcpPatch
    ) {
        preview_mcp_removal(&affected_target_path, target, &name)?;
    }
    let preview_id = unmount_fingerprint(
        home,
        request,
        &canonical,
        &affected_target_path,
        generated_at_epoch_seconds,
    )?;
    Ok(UnmountPreview {
        preview_id,
        asset_id: request.asset_id.clone(),
        target_id: request.target_id.clone(),
        affected_target_path: affected_target_path.clone(),
        planned_effects: vec![if kind == crate::targets::AssetKind::Mcp {
            format!(
                "remove only MCP server '{name}' from {}",
                affected_target_path.display()
            )
        } else {
            format!(
                "remove runtime link {} without deleting canonical content",
                affected_target_path.display()
            )
        }],
        warnings,
        backup_required: affected_target_path.exists(),
        can_apply,
        generated_at_epoch_seconds,
        expires_at_epoch_seconds: generated_at_epoch_seconds.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

pub fn apply_unmount(home: &Path, request: &UnmountApplyRequest) -> Result<UnmountApplyResult> {
    let _operation_lock = OperationLock::acquire(home)?;
    if epoch_seconds()
        > request
            .preview_generated_at_epoch_seconds
            .saturating_add(PREVIEW_TTL_SECONDS)
    {
        return Err(MaaError::new(
            "unmount preview expired; generate a new preview before applying",
        ));
    }
    let preview = preview_unmount_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    if preview.preview_id != request.preview_id {
        return Err(MaaError::new(
            "unmount preview is stale; generate a new preview before applying",
        ));
    }
    if !preview.can_apply {
        return Err(MaaError::new(
            preview
                .warnings
                .first()
                .cloned()
                .unwrap_or_else(|| "unmount is blocked".into()),
        ));
    }

    let (kind, name) = parse_asset_id(&request.request.asset_id)
        .map_err(|error| MaaError::new(error.to_string()))?;
    let targets = load_targets(home)?;
    let target = targets.resolve(&request.request.target_id)?;
    let mut mounts = load_mounts(home).map_err(|error| MaaError::new(error.to_string()))?;
    let original_mounts = fs::read(mount_registry_path(home))?;
    let operation_id = operation_id();
    let mut journal = OperationJournal::start_recoverable(
        home,
        &operation_id,
        "unmount",
        vec![
            RecoveryTarget::asset_center(mount_registry_path(home)),
            RecoveryTarget::registered_target(
                request.request.target_id.clone(),
                preview.affected_target_path.clone(),
            ),
        ],
    )?;
    let mut snapshot = Some(snapshot_runtime_path(&preview.affected_target_path)?);
    let backup_id = if preview.backup_required {
        Some(create_local_backup(
            home,
            &preview.affected_target_path,
            &original_mounts,
        )?)
    } else {
        None
    };

    if let Err(error) = remove_runtime_mount(&preview.affected_target_path, target, kind, &name) {
        if let Some(snapshot) = snapshot.take() {
            let _ = restore_runtime_snapshot(&preview.affected_target_path, snapshot);
        }
        return rollback_after_error(home, &mut journal, error, "unmount");
    }
    mounts.remove(&request.request.asset_id, &request.request.target_id);
    if let Err(error) = save_mounts(home, &mounts).map_err(|error| MaaError::new(error.to_string()))
    {
        if let Some(snapshot) = snapshot.take() {
            let _ = restore_runtime_snapshot(&preview.affected_target_path, snapshot);
        }
        let _ = fs::write(mount_registry_path(home), original_mounts);
        return rollback_after_error(home, &mut journal, error, "unmount");
    }
    if let Some(snapshot) = snapshot.take() {
        discard_runtime_snapshot(snapshot)?;
    }
    journal.record_step("unmount_applied")?;
    journal.complete()?;
    Ok(UnmountApplyResult {
        preview_id: preview.preview_id,
        asset_id: request.request.asset_id.clone(),
        target_id: request.request.target_id.clone(),
        unmounted: true,
        backup_id,
        affected_paths: vec![preview.affected_target_path, mount_registry_path(home)],
    })
}

fn rollback_after_error<T>(
    home: &Path,
    journal: &mut OperationJournal,
    error: MaaError,
    operation: &str,
) -> Result<T> {
    let original = error.to_string();
    journal.rollback_now(home).map_err(|rollback| {
        MaaError::new(format!(
            "{original}; persistent {operation} rollback failed: {rollback}"
        ))
    })?;
    Err(error)
}

fn preview_disposition(
    canonical: &Path,
    target_path: &Path,
    target: &MountTarget,
    warnings: &mut Vec<String>,
) -> Result<(MountDisposition, bool, bool)> {
    match target.adapter {
        MountAdapter::SymlinkDirectory
        | MountAdapter::WindowsDirectoryJunction
        | MountAdapter::SymlinkFile => {
            if link_points_to(target_path, canonical)? {
                Ok((MountDisposition::AlreadyMounted, false, true))
            } else if target_path.exists() || fs::symlink_metadata(target_path).is_ok() {
                Ok((MountDisposition::ReplaceRuntimePath, true, true))
            } else {
                Ok((MountDisposition::CreateLink, false, true))
            }
        }
        MountAdapter::JsonMcpPatch | MountAdapter::TomlMcpPatch => {
            let canonical = load_canonical_mcp(canonical)?;
            match render_mcp_preview(target_path, target, &canonical) {
                Ok(renderer_warnings) => {
                    warnings.extend(renderer_warnings);
                    Ok((MountDisposition::CompileMcp, target_path.exists(), true))
                }
                Err(error) => {
                    warnings.push(error.to_string());
                    Ok((MountDisposition::Blocked, false, false))
                }
            }
        }
    }
}

fn apply_runtime(
    canonical_path: &Path,
    target_path: &Path,
    target: &MountTarget,
    kind: crate::targets::AssetKind,
    name: &str,
) -> Result<Vec<String>> {
    match target.adapter {
        MountAdapter::SymlinkDirectory | MountAdapter::WindowsDirectoryJunction => {
            replace_with_directory_link(canonical_path, target_path, target.adapter)?;
            Ok(Vec::new())
        }
        MountAdapter::SymlinkFile => {
            replace_with_file_link(canonical_path, target_path)?;
            Ok(Vec::new())
        }
        MountAdapter::JsonMcpPatch | MountAdapter::TomlMcpPatch => {
            if kind != crate::targets::AssetKind::Mcp {
                return Err(MaaError::new("MCP renderer requires an MCP asset"));
            }
            let canonical = load_canonical_mcp(canonical_path)?;
            if canonical.name != name {
                return Err(MaaError::new(
                    "canonical MCP name does not match the asset ID",
                ));
            }
            let rendered = render_mcp_preview(target_path, target, &canonical)?;
            let content = render_mcp_content(target_path, target, &canonical)?;
            atomic_runtime_write(target_path, content.as_bytes())?;
            Ok(rendered)
        }
    }
}

fn render_mcp_preview(
    path: &Path,
    target: &MountTarget,
    canonical: &CanonicalMcp,
) -> Result<Vec<String>> {
    match target.adapter {
        MountAdapter::JsonMcpPatch => {
            let existing = read_json_or_empty(path)?;
            let scope = claude_scope(target)?;
            Ok(patch_claude_json(existing, canonical, scope, cfg!(windows))
                .map_err(|error| MaaError::new(error.to_string()))?
                .warnings)
        }
        MountAdapter::TomlMcpPatch => {
            let existing = read_text_or_empty(path)?;
            Ok(
                patch_codex_toml(&existing, canonical, codex_scope(target), cfg!(windows))
                    .map_err(|error| MaaError::new(error.to_string()))?
                    .warnings,
            )
        }
        _ => Err(MaaError::new("target does not use an MCP renderer")),
    }
}

pub(crate) fn validate_mcp_target_preview(
    target: &MountTarget,
    canonical: &CanonicalMcp,
) -> Result<Vec<String>> {
    let compatibility = target.compatibility(crate::targets::AssetKind::Mcp);
    if !compatibility.compatible {
        return Err(MaaError::new(compatibility.reason.unwrap_or_else(|| {
            "target is incompatible with MCP assets".into()
        })));
    }
    render_mcp_preview(&target.path, target, canonical)
}

fn render_mcp_content(
    path: &Path,
    target: &MountTarget,
    canonical: &CanonicalMcp,
) -> Result<String> {
    match target.adapter {
        MountAdapter::JsonMcpPatch => {
            let existing = read_json_or_empty(path)?;
            let rendered =
                patch_claude_json(existing, canonical, claude_scope(target)?, cfg!(windows))
                    .map_err(|error| MaaError::new(error.to_string()))?;
            serde_json::to_string_pretty(&rendered.content)
                .map_err(|error| MaaError::new(error.to_string()))
        }
        MountAdapter::TomlMcpPatch => {
            let existing = read_text_or_empty(path)?;
            Ok(
                patch_codex_toml(&existing, canonical, codex_scope(target), cfg!(windows))
                    .map_err(|error| MaaError::new(error.to_string()))?
                    .content,
            )
        }
        _ => Err(MaaError::new("target does not use an MCP renderer")),
    }
}

fn preview_mcp_removal(path: &Path, target: &MountTarget, name: &str) -> Result<()> {
    match target.adapter {
        MountAdapter::JsonMcpPatch => {
            remove_from_claude_json(read_json_or_empty(path)?, name, claude_scope(target)?)
                .map_err(|error| MaaError::new(error.to_string()))?;
        }
        MountAdapter::TomlMcpPatch => {
            remove_from_codex_toml(&read_text_or_empty(path)?, name, codex_scope(target))
                .map_err(|error| MaaError::new(error.to_string()))?;
        }
        _ => return Err(MaaError::new("target does not use an MCP renderer")),
    }
    Ok(())
}

pub(crate) fn remove_runtime_mount(
    path: &Path,
    target: &MountTarget,
    kind: crate::targets::AssetKind,
    name: &str,
) -> Result<()> {
    match target.adapter {
        MountAdapter::SymlinkDirectory
        | MountAdapter::WindowsDirectoryJunction
        | MountAdapter::SymlinkFile => remove_path_if_present(path),
        MountAdapter::JsonMcpPatch => {
            if kind != crate::targets::AssetKind::Mcp {
                return Err(MaaError::new("JSON MCP removal requires an MCP asset"));
            }
            let rendered =
                remove_from_claude_json(read_json_or_empty(path)?, name, claude_scope(target)?)
                    .map_err(|error| MaaError::new(error.to_string()))?;
            let content = serde_json::to_vec_pretty(&rendered.content)
                .map_err(|error| MaaError::new(error.to_string()))?;
            atomic_runtime_write(path, &content)
        }
        MountAdapter::TomlMcpPatch => {
            if kind != crate::targets::AssetKind::Mcp {
                return Err(MaaError::new("TOML MCP removal requires an MCP asset"));
            }
            let rendered =
                remove_from_codex_toml(&read_text_or_empty(path)?, name, codex_scope(target))
                    .map_err(|error| MaaError::new(error.to_string()))?;
            atomic_runtime_write(path, rendered.content.as_bytes())
        }
    }
}

fn claude_scope(target: &MountTarget) -> Result<ClaudeScope<'_>> {
    match target.kind {
        MountTargetKind::ClaudeUserMcpJson | MountTargetKind::CustomClaudeMcpJson => {
            Ok(ClaudeScope::User)
        }
        MountTargetKind::ClaudeLocalMcpJson => Ok(ClaudeScope::Local {
            project_path: target
                .project_path
                .as_deref()
                .ok_or_else(|| MaaError::new("local MCP target requires projectPath"))?
                .to_str()
                .ok_or_else(|| MaaError::new("projectPath is not valid UTF-8"))?,
        }),
        MountTargetKind::ClaudeProjectMcpJson => Ok(ClaudeScope::Project),
        _ => Err(MaaError::new("target is not a Claude MCP target")),
    }
}

fn codex_scope(target: &MountTarget) -> CodexScope {
    if matches!(target.kind, MountTargetKind::CodexProjectMcpToml) {
        CodexScope::Project
    } else {
        CodexScope::User
    }
}

pub(crate) fn target_asset_path(
    target: &MountTarget,
    kind: crate::targets::AssetKind,
    name: &str,
) -> PathBuf {
    match kind {
        crate::targets::AssetKind::Skill => target.path.join(name),
        crate::targets::AssetKind::Command => target.path.join(format!("{name}.md")),
        crate::targets::AssetKind::Mcp => target.path.clone(),
    }
}

pub(crate) fn guard_target_path(target: &MountTarget, affected_path: &Path) -> Result<()> {
    let authorized_root = match target.adapter {
        MountAdapter::SymlinkDirectory
        | MountAdapter::WindowsDirectoryJunction
        | MountAdapter::SymlinkFile => target.path.as_path(),
        MountAdapter::JsonMcpPatch | MountAdapter::TomlMcpPatch => target
            .path
            .parent()
            .ok_or_else(|| MaaError::new("MCP target path has no parent"))?,
    };
    guard_write_path(authorized_root, affected_path)?;
    Ok(())
}

fn load_canonical_mcp(path: &Path) -> Result<CanonicalMcp> {
    let text = fs::read_to_string(path)?;
    let canonical: CanonicalMcp = serde_json::from_str(&text)
        .map_err(|error| MaaError::new(format!("invalid canonical MCP: {error}")))?;
    canonical
        .validate()
        .map_err(|error| MaaError::new(error.to_string()))?;
    Ok(canonical)
}

fn read_json_or_empty(path: &Path) -> Result<JsonValue> {
    if !path.exists() {
        return Ok(JsonValue::Object(Default::default()));
    }
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text)
        .map_err(|error| MaaError::new(format!("invalid target JSON {}: {error}", path.display())))
}

fn read_text_or_empty(path: &Path) -> Result<String> {
    if path.exists() {
        Ok(fs::read_to_string(path)?)
    } else {
        Ok(String::new())
    }
}

fn replace_with_directory_link(
    canonical: &Path,
    target: &Path,
    adapter: MountAdapter,
) -> Result<()> {
    remove_path_if_present(target)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    create_directory_link(canonical, target, adapter)
}

fn replace_with_file_link(canonical: &Path, target: &Path) -> Result<()> {
    remove_path_if_present(target)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    create_file_link(canonical, target)
}

#[cfg(unix)]
fn create_directory_link(canonical: &Path, target: &Path, _: MountAdapter) -> Result<()> {
    std::os::unix::fs::symlink(canonical, target)?;
    Ok(())
}

#[cfg(windows)]
fn create_directory_link(canonical: &Path, target: &Path, adapter: MountAdapter) -> Result<()> {
    match adapter {
        MountAdapter::WindowsDirectoryJunction => {
            // Keep the command text constant. Runtime paths are positional
            // PowerShell arguments, so cmd.exe metacharacters in an approved
            // Windows path cannot be interpreted as another command.
            let status = std::process::Command::new("powershell.exe")
                .args([
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "New-Item -ItemType Junction -LiteralPath $args[0] -Target $args[1] -ErrorAction Stop | Out-Null",
                ])
                .arg(target)
                .arg(canonical)
                .status()?;
            if !status.success() {
                return Err(MaaError::new("failed to create Windows directory junction"));
            }
            Ok(())
        }
        _ => {
            std::os::windows::fs::symlink_dir(canonical, target)?;
            Ok(())
        }
    }
}

#[cfg(unix)]
fn create_file_link(canonical: &Path, target: &Path) -> Result<()> {
    std::os::unix::fs::symlink(canonical, target)?;
    Ok(())
}

#[cfg(windows)]
fn create_file_link(canonical: &Path, target: &Path) -> Result<()> {
    std::os::windows::fs::symlink_file(canonical, target)?;
    Ok(())
}

fn link_points_to(path: &Path, canonical: &Path) -> Result<bool> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if !is_link_or_junction(&metadata) {
        return Ok(false);
    }
    let link = fs::read_link(path)?;
    let link = if link.is_absolute() {
        link
    } else {
        path.parent().unwrap_or_else(|| Path::new(".")).join(link)
    };
    Ok(link == canonical)
}

#[derive(Debug)]
pub(crate) enum RuntimeSnapshot {
    Missing,
    File(Vec<u8>),
    Directory(PathBuf),
    Symlink(PathBuf),
}

pub(crate) fn snapshot_runtime_path(path: &Path) -> Result<RuntimeSnapshot> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(RuntimeSnapshot::Missing)
        }
        Err(error) => return Err(error.into()),
    };
    if is_link_or_junction(&metadata) {
        Ok(RuntimeSnapshot::Symlink(fs::read_link(path)?))
    } else if metadata.is_file() {
        Ok(RuntimeSnapshot::File(fs::read(path)?))
    } else {
        let temporary = path.with_extension(format!("maa-rollback-{}", operation_id()));
        copy_directory(path, &temporary)?;
        Ok(RuntimeSnapshot::Directory(temporary))
    }
}

pub(crate) fn restore_runtime_snapshot(path: &Path, snapshot: RuntimeSnapshot) -> Result<()> {
    remove_path_if_present(path)?;
    match snapshot {
        RuntimeSnapshot::Missing => Ok(()),
        RuntimeSnapshot::File(content) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, content)?;
            Ok(())
        }
        RuntimeSnapshot::Directory(temporary) => {
            fs::rename(temporary, path)?;
            Ok(())
        }
        RuntimeSnapshot::Symlink(target) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            if target.is_dir() {
                create_directory_link(&target, path, MountAdapter::SymlinkDirectory)
            } else {
                create_file_link(&target, path)
            }
        }
    }
}

pub(crate) fn discard_runtime_snapshot(snapshot: RuntimeSnapshot) -> Result<()> {
    if let RuntimeSnapshot::Directory(temporary) = snapshot {
        remove_path_if_present(&temporary)?;
    }
    Ok(())
}

fn create_local_backup(home: &Path, target: &Path, mounts: &[u8]) -> Result<String> {
    let root = home.join(".my-agent-assets");
    let backup_id = format!("mount-{}", operation_id());
    let backup = guard_write_path(&root, &root.join("backups/local").join(&backup_id))?;
    fs::create_dir_all(&backup)?;
    copy_any(target, &backup.join("content"))?;
    fs::write(backup.join("mounts.yaml"), mounts)?;
    fs::write(
        backup.join("manifest.yaml"),
        format!(
            "schemaVersion: 1\noperation: mount\ntarget: {}\n",
            target.display()
        ),
    )?;
    Ok(backup_id)
}

fn atomic_runtime_write(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| MaaError::new("runtime config path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".maa-runtime-{}.tmp", operation_id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(content)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        sync_directory(parent)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
}

fn mount_fingerprint(
    home: &Path,
    request: &MountPreviewRequest,
    canonical: &Path,
    target: &Path,
    generated_at: u64,
) -> Result<String> {
    let mut fingerprint = PreviewFingerprint::new("mount");
    fingerprint.add_bytes(
        "request",
        &serde_json::to_vec(request).map_err(|error| MaaError::new(error.to_string()))?,
    );
    fingerprint.add_u64("generated-at", generated_at);
    fingerprint.add_path("canonical", canonical)?;
    fingerprint.add_path_if_present("target", target)?;
    for path in [
        asset_registry_path(home),
        crate::targets::registry_path(home),
        mount_registry_path(home),
    ] {
        fingerprint.add_path("registry", &path)?;
    }
    Ok(fingerprint.finish("mount"))
}

fn unmount_fingerprint(
    home: &Path,
    request: &UnmountPreviewRequest,
    canonical: &Path,
    target: &Path,
    generated_at: u64,
) -> Result<String> {
    let mut fingerprint = PreviewFingerprint::new("unmount");
    fingerprint.add_bytes(
        "request",
        &serde_json::to_vec(request).map_err(|error| MaaError::new(error.to_string()))?,
    );
    fingerprint.add_u64("generated-at", generated_at);
    fingerprint.add_path_if_present("canonical", canonical)?;
    fingerprint.add_path_if_present("target", target)?;
    for path in [
        crate::targets::registry_path(home),
        mount_registry_path(home),
    ] {
        fingerprint.add_path("registry", &path)?;
    }
    Ok(fingerprint.finish("unmount"))
}

pub(crate) fn copy_any(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if is_link_or_junction(&metadata) {
        let link = fs::read_link(source)?;
        if link.is_dir() {
            create_directory_link(&link, destination, MountAdapter::SymlinkDirectory)
        } else {
            create_file_link(&link, destination)
        }
    } else if metadata.is_dir() {
        copy_directory(source, destination)
    } else {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, destination)?;
        Ok(())
    }
}

fn copy_directory(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        copy_any(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

pub(crate) fn remove_path_if_present(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if is_link_or_junction(&metadata) {
        remove_link_or_junction(path, &metadata)?;
    } else if metadata.is_file() {
        fs::remove_file(path)?;
    } else {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn remove_link_or_junction(path: &Path, metadata: &fs::Metadata) -> Result<()> {
    #[cfg(windows)]
    {
        if metadata.is_dir() {
            fs::remove_dir(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        fs::remove_file(path)?;
    }
    Ok(())
}

fn operation_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = OPERATION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos}-{}-{counter}", std::process::id())
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_registry::{save as save_assets, AssetRecord, AssetRegistry};
    use crate::mount_registry::MountRegistry;
    use crate::operation::{crash_test, recover_incomplete};
    use crate::targets::{
        save as save_targets, AssetKind, MountAdapter, ProviderState, TargetRegistry,
    };
    use serde_json::json;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[cfg(unix)]
    #[test]
    fn skill_mount_uses_target_id_and_creates_binding() {
        let home = initialized_home("skill");
        register_asset(&home, AssetKind::Skill, "review", "# Review");
        let request = MountPreviewRequest {
            asset_id: "skill:review".into(),
            target_id: "claude-user-skills".into(),
        };
        let preview = preview_mount(&home, &request).unwrap();
        assert!(preview.can_apply);
        assert_eq!(
            preview.affected_target_path,
            home.join(".claude/skills/review")
        );
        apply_preview(&home, request, preview).unwrap();
        assert!(fs::symlink_metadata(home.join(".claude/skills/review"))
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            load_mounts(&home).unwrap().for_asset("skill:review")[0].target_id,
            "claude-user-skills"
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn claude_and_codex_mcp_mounts_preserve_unrelated_configuration() {
        let home = initialized_home("mcp");
        register_mcp(&home, "remote");
        fs::write(home.join(".claude.json"), r#"{"theme":"dark"}"#).unwrap();
        fs::create_dir_all(home.join(".codex")).unwrap();
        fs::write(
            home.join(".codex/config.toml"),
            "# keep\nmodel = \"gpt-5\"\n",
        )
        .unwrap();

        for target_id in ["claude-user-mcp", "codex-user-mcp"] {
            let request = MountPreviewRequest {
                asset_id: "mcp:remote".into(),
                target_id: target_id.into(),
            };
            let preview = preview_mount(&home, &request).unwrap();
            apply_preview(&home, request, preview).unwrap();
        }
        let claude: JsonValue =
            serde_json::from_str(&fs::read_to_string(home.join(".claude.json")).unwrap()).unwrap();
        assert_eq!(claude["theme"], "dark");
        assert_eq!(
            claude["mcpServers"]["remote"]["url"],
            "https://example.test/mcp"
        );
        let codex = fs::read_to_string(home.join(".codex/config.toml")).unwrap();
        assert!(codex.contains("# keep"));
        assert!(codex.contains("model = \"gpt-5\""));
        assert!(codex.contains("[mcp_servers.remote]"));
        assert!(codex.contains("http_headers"));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn stale_preview_is_rejected_before_runtime_write() {
        let home = initialized_home("stale");
        register_mcp(&home, "remote");
        fs::write(home.join(".claude.json"), "{}").unwrap();
        let request = MountPreviewRequest {
            asset_id: "mcp:remote".into(),
            target_id: "claude-user-mcp".into(),
        };
        let preview = preview_mount(&home, &request).unwrap();
        fs::write(home.join(".claude.json"), r#"{"changed":true}"#).unwrap();
        let error = apply_preview(&home, request, preview).unwrap_err();
        assert!(error.to_string().contains("stale"));
        let content = fs::read_to_string(home.join(".claude.json")).unwrap();
        assert_eq!(content, r#"{"changed":true}"#);
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(unix)]
    #[test]
    fn unmount_removes_only_the_managed_link_and_blocks_user_replacement() {
        let home = initialized_home("unmount-skill");
        register_asset(&home, AssetKind::Skill, "review", "# Review");
        let mount_request = MountPreviewRequest {
            asset_id: "skill:review".into(),
            target_id: "claude-user-skills".into(),
        };
        let mount_preview = preview_mount(&home, &mount_request).unwrap();
        apply_preview(&home, mount_request, mount_preview).unwrap();

        let unmount_request = UnmountPreviewRequest {
            asset_id: "skill:review".into(),
            target_id: "claude-user-skills".into(),
        };
        let unmount_preview = preview_unmount(&home, &unmount_request).unwrap();
        apply_unmount_preview(&home, unmount_request.clone(), unmount_preview).unwrap();
        assert!(!home.join(".claude/skills/review").exists());
        assert!(canonical_path(&home, AssetKind::Skill, "review").exists());

        let mut mounts = load_mounts(&home).unwrap();
        mounts
            .upsert(
                MountBinding::new("skill:review", "claude-user-skills", BindingStatus::Mounted)
                    .unwrap(),
            )
            .unwrap();
        save_mounts(&home, &mounts).unwrap();
        fs::create_dir_all(home.join(".claude/skills/review")).unwrap();
        fs::write(home.join(".claude/skills/review/SKILL.md"), "# User").unwrap();
        let blocked = preview_unmount(&home, &unmount_request).unwrap();
        assert!(!blocked.can_apply);
        assert!(blocked.warnings[0].contains("refusing to delete"));
        assert!(home.join(".claude/skills/review/SKILL.md").is_file());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn mcp_unmount_removes_only_selected_server() {
        let home = initialized_home("unmount-mcp");
        register_mcp(&home, "remote");
        fs::write(
            home.join(".claude.json"),
            r#"{"other":"keep","mcpServers":{"keep":{"command":"keep"}}}"#,
        )
        .unwrap();
        let mount_request = MountPreviewRequest {
            asset_id: "mcp:remote".into(),
            target_id: "claude-user-mcp".into(),
        };
        let mount_preview = preview_mount(&home, &mount_request).unwrap();
        apply_preview(&home, mount_request, mount_preview).unwrap();
        let request = UnmountPreviewRequest {
            asset_id: "mcp:remote".into(),
            target_id: "claude-user-mcp".into(),
        };
        let preview = preview_unmount(&home, &request).unwrap();
        apply_unmount_preview(&home, request, preview).unwrap();
        let config: JsonValue =
            serde_json::from_str(&fs::read_to_string(home.join(".claude.json")).unwrap()).unwrap();
        assert_eq!(config["other"], "keep");
        assert_eq!(config["mcpServers"]["keep"]["command"], "keep");
        assert!(config["mcpServers"].get("remote").is_none());
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(unix)]
    #[test]
    fn skill_mount_recovers_when_process_crashes_after_persisted_step() {
        let home = initialized_home("crash-mount");
        register_asset(&home, AssetKind::Skill, "review", "# Review");
        let request = MountPreviewRequest {
            asset_id: "skill:review".into(),
            target_id: "claude-user-skills".into(),
        };
        let preview = preview_mount(&home, &request).unwrap();
        let runtime_path = home.join(".claude/skills/review");
        let crash = crash_test::crash_after_step("mount", "mount_applied");

        let result = catch_unwind(AssertUnwindSafe(|| {
            apply_mount(
                &home,
                &MountApplyRequest {
                    preview_id: preview.preview_id,
                    preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                    request,
                },
            )
        }));
        assert!(result.is_err());
        drop(crash);

        assert!(fs::symlink_metadata(&runtime_path)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            load_mounts(&home).unwrap().for_asset("skill:review")[0].target_id,
            "claude-user-skills"
        );

        let report = recover_incomplete(&home).unwrap();
        assert!(report.attempted);
        assert!(!report.writes_blocked);
        assert!(report.attempts[0].recovered);
        assert!(!runtime_path.exists());
        assert!(load_mounts(&home)
            .unwrap()
            .for_asset("skill:review")
            .is_empty());
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(unix)]
    #[test]
    fn skill_unmount_recovers_when_process_crashes_after_persisted_step() {
        let home = initialized_home("crash-unmount");
        register_asset(&home, AssetKind::Skill, "review", "# Review");
        let mount_request = MountPreviewRequest {
            asset_id: "skill:review".into(),
            target_id: "claude-user-skills".into(),
        };
        let mount_preview = preview_mount(&home, &mount_request).unwrap();
        apply_preview(&home, mount_request, mount_preview).unwrap();

        let request = UnmountPreviewRequest {
            asset_id: "skill:review".into(),
            target_id: "claude-user-skills".into(),
        };
        let preview = preview_unmount(&home, &request).unwrap();
        let runtime_path = home.join(".claude/skills/review");
        let crash = crash_test::crash_after_step("unmount", "unmount_applied");

        let result = catch_unwind(AssertUnwindSafe(|| {
            apply_unmount(
                &home,
                &UnmountApplyRequest {
                    preview_id: preview.preview_id,
                    preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                    request,
                },
            )
        }));
        assert!(result.is_err());
        drop(crash);

        assert!(!runtime_path.exists());
        assert!(load_mounts(&home)
            .unwrap()
            .for_asset("skill:review")
            .is_empty());
        let report = recover_incomplete(&home).unwrap();
        assert!(report.attempted);
        assert!(!report.writes_blocked);
        assert!(report.attempts[0].recovered);
        assert!(fs::symlink_metadata(&runtime_path)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            load_mounts(&home).unwrap().for_asset("skill:review")[0].target_id,
            "claude-user-skills"
        );
        let _ = fs::remove_dir_all(home);
    }

    fn apply_preview(
        home: &Path,
        request: MountPreviewRequest,
        preview: MountPreview,
    ) -> Result<MountApplyResult> {
        crate::fingerprint::assert_sha256_preview_id(&preview.preview_id, "mount-");
        apply_mount(
            home,
            &MountApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
    }

    fn apply_unmount_preview(
        home: &Path,
        request: UnmountPreviewRequest,
        preview: UnmountPreview,
    ) -> Result<UnmountApplyResult> {
        crate::fingerprint::assert_sha256_preview_id(&preview.preview_id, "unmount-");
        apply_unmount(
            home,
            &UnmountApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
    }

    fn initialized_home(name: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!(
            "maa-mount-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = home.join(".my-agent-assets");
        for path in [
            root.join("assets/skills"),
            root.join("assets/commands"),
            root.join("assets/mcps"),
            root.join("backups/local"),
        ] {
            fs::create_dir_all(path).unwrap();
        }
        save_assets(&home, &AssetRegistry::default()).unwrap();
        save_mounts(&home, &MountRegistry::default()).unwrap();
        let targets = TargetRegistry::standard_user_targets(
            &home,
            ProviderState::Initialized,
            ProviderState::Initialized,
            MountAdapter::SymlinkDirectory,
        )
        .unwrap();
        save_targets(&home, &targets).unwrap();
        home
    }

    fn register_asset(home: &Path, kind: AssetKind, name: &str, content: &str) {
        let mut registry = load_assets(home).unwrap();
        registry
            .upsert(AssetRecord::new(kind, name).unwrap())
            .unwrap();
        save_assets(home, &registry).unwrap();
        let path = canonical_path(home, kind, name);
        match kind {
            AssetKind::Skill => {
                fs::create_dir_all(&path).unwrap();
                fs::write(path.join("SKILL.md"), content).unwrap();
            }
            AssetKind::Command | AssetKind::Mcp => fs::write(path, content).unwrap(),
        }
    }

    fn register_mcp(home: &Path, name: &str) {
        let canonical = json!({
            "schemaVersion": 1,
            "name": name,
            "spec": {
                "type": "http",
                "url": "https://example.test/mcp",
                "headers": {"X-Test": "value"}
            },
            "providerExtensions": {}
        });
        register_asset(
            home,
            AssetKind::Mcp,
            name,
            &serde_json::to_string_pretty(&canonical).unwrap(),
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_directory_junction_is_verified_and_removed_without_following_it() {
        let root = std::env::temp_dir().join(format!(
            "maa-windows-junction-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let canonical = root.join("canonical & %literal%");
        let junction = root.join("runtime link & %literal%");
        fs::create_dir_all(&canonical).unwrap();
        fs::write(canonical.join("SKILL.md"), "# Review").unwrap();

        create_directory_link(
            &canonical,
            &junction,
            MountAdapter::WindowsDirectoryJunction,
        )
        .unwrap();
        assert!(link_points_to(&junction, &canonical).unwrap());

        remove_path_if_present(&junction).unwrap();
        assert!(fs::symlink_metadata(&junction).is_err());
        assert!(canonical.join("SKILL.md").exists());
        let _ = fs::remove_dir_all(root);
    }
}
