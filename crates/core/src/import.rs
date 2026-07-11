use crate::asset_registry::{
    canonical_path, load as load_registry, registry_path, save as save_registry, AssetRecord,
    AssetRegistry,
};
use crate::discovery::{
    discover, load_mcp_source, AssetKind, DiscoveredSource, DiscoveryScope, SourceFormat,
};
use crate::fingerprint::PreviewFingerprint;
use crate::mount_registry::{
    load as load_mounts, registry_path as mount_registry_path, save as save_mounts,
};
use crate::operation::{OperationJournal, OperationLock, RecoveryTarget};
use crate::path_safety::{guard_write_path, is_link_or_junction, validate_single_path_component};
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PREVIEW_TTL_SECONDS: u64 = 300;
static OPERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImportResolution {
    Unresolved,
    Skip,
    Overwrite,
    Rename {
        #[serde(rename = "newName")]
        new_name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportDisposition {
    #[serde(rename = "create")]
    Create,
    #[serde(rename = "conflict")]
    Conflict,
    #[serde(rename = "skip")]
    Skip,
    #[serde(rename = "overwrite")]
    Overwrite,
    #[serde(rename = "rename")]
    Rename,
    #[serde(rename = "unchanged")]
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreviewRequest {
    pub scope: DiscoveryScope,
    pub source_id: String,
    pub resolution: ImportResolution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportConflict {
    pub asset_id: String,
    pub reason: String,
    pub existing_content: String,
    pub incoming_content: String,
    pub raw_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub preview_id: String,
    pub source_id: String,
    pub asset_id: String,
    pub asset_type: AssetKind,
    pub source_name: String,
    pub destination_name: String,
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub disposition: ImportDisposition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict: Option<ImportConflict>,
    #[serde(default)]
    pub warnings: Vec<String>,
    pub can_apply: bool,
    pub generated_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportApplyRequest {
    pub preview_id: String,
    pub preview_generated_at_epoch_seconds: u64,
    pub request: ImportPreviewRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportApplyStatus {
    #[serde(rename = "imported")]
    Imported,
    #[serde(rename = "skipped")]
    Skipped,
    #[serde(rename = "unchanged")]
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportApplyResult {
    pub preview_id: String,
    pub asset_id: String,
    pub status: ImportApplyStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,
    pub affected_paths: Vec<PathBuf>,
}

pub fn preview_import(home: &Path, request: &ImportPreviewRequest) -> Result<ImportPreview> {
    preview_import_at(home, request, epoch_seconds())
}

pub(crate) fn preview_import_at(
    home: &Path,
    request: &ImportPreviewRequest,
    generated_at_epoch_seconds: u64,
) -> Result<ImportPreview> {
    ensure_initialized(home)?;
    let source = find_source(home, &request.scope, &request.source_id)?;
    if source.is_managed {
        return Err(MaaError::new(
            "managed runtime sources are already backed by the asset center",
        ));
    }
    if !source.eligible_import {
        return Err(MaaError::new(format!(
            "source '{}' is not eligible for import",
            source.source_id
        )));
    }

    let destination_name = match &request.resolution {
        ImportResolution::Rename { new_name } => {
            validate_single_path_component(new_name, "renamed asset").map_err(MaaError::new)?;
            new_name.clone()
        }
        _ => source.asset_name.clone(),
    };
    let destination_path = canonical_path(home, source.asset_kind, &destination_name);
    let registry = load_registry(home).map_err(|error| MaaError::new(error.to_string()))?;
    let conflict = detect_conflict(home, &registry, &source, &destination_name)?;
    let (disposition, can_apply) = match (&conflict, &request.resolution) {
        (None, ImportResolution::Unresolved) => (ImportDisposition::Create, true),
        (None, ImportResolution::Skip) => (ImportDisposition::Skip, true),
        (None, ImportResolution::Overwrite) => (ImportDisposition::Create, true),
        (None, ImportResolution::Rename { .. }) => (ImportDisposition::Rename, true),
        (Some(conflict), _) if conflict.reason == "canonical MCP is structurally identical" => {
            (ImportDisposition::Unchanged, false)
        }
        (Some(_), ImportResolution::Unresolved) => (ImportDisposition::Conflict, false),
        (Some(_), ImportResolution::Skip) => (ImportDisposition::Skip, true),
        (Some(_), ImportResolution::Overwrite) => (ImportDisposition::Overwrite, true),
        (Some(_), ImportResolution::Rename { .. }) => {
            // A rename may itself collide, in which case it remains unresolved.
            (ImportDisposition::Conflict, false)
        }
    };

    let preview_id = preview_fingerprint(
        home,
        request,
        &source,
        &destination_name,
        &registry,
        generated_at_epoch_seconds,
    )?;
    Ok(ImportPreview {
        preview_id,
        source_id: source.source_id,
        asset_id: crate::asset_registry::asset_id(source.asset_kind, &destination_name),
        asset_type: source.asset_kind,
        source_name: source.asset_name,
        destination_name,
        source_path: source.source_path,
        destination_path,
        disposition,
        conflict,
        warnings: source.warnings,
        can_apply,
        generated_at_epoch_seconds,
        expires_at_epoch_seconds: generated_at_epoch_seconds.saturating_add(PREVIEW_TTL_SECONDS),
    })
}

pub fn apply_import(home: &Path, request: &ImportApplyRequest) -> Result<ImportApplyResult> {
    let _operation_lock = OperationLock::acquire(home)?;
    let preview = preview_import_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    if matches!(
        preview.disposition,
        ImportDisposition::Skip | ImportDisposition::Unchanged
    ) || !preview.can_apply
    {
        return apply_import_locked(home, request, None);
    }
    let operation_id = operation_id();
    let staging = home.join(".my-agent-assets/operations").join(&operation_id);
    let mut recovery_targets = vec![
        RecoveryTarget::asset_center(registry_path(home)),
        RecoveryTarget::asset_center(preview.destination_path.clone()),
        RecoveryTarget::asset_center(staging),
    ];
    if preview.asset_type == AssetKind::Mcp && preview.disposition == ImportDisposition::Overwrite {
        recovery_targets.push(RecoveryTarget::asset_center(mount_registry_path(home)));
    }
    let mut journal =
        OperationJournal::start_recoverable(home, &operation_id, "import", recovery_targets)?;
    match apply_import_locked(home, request, Some(&operation_id)) {
        Ok(result) => {
            journal.record_step("import_applied")?;
            journal.complete()?;
            Ok(result)
        }
        Err(error) => {
            let original = error.to_string();
            journal.rollback_now(home).map_err(|rollback| {
                MaaError::new(format!(
                    "{original}; persistent import rollback failed: {rollback}"
                ))
            })?;
            Err(error)
        }
    }
}

pub(crate) fn apply_import_locked(
    home: &Path,
    request: &ImportApplyRequest,
    transaction_operation_id: Option<&str>,
) -> Result<ImportApplyResult> {
    if epoch_seconds()
        > request
            .preview_generated_at_epoch_seconds
            .saturating_add(PREVIEW_TTL_SECONDS)
    {
        return Err(MaaError::new(
            "import preview expired; generate a new preview before applying",
        ));
    }
    let preview = preview_import_at(
        home,
        &request.request,
        request.preview_generated_at_epoch_seconds,
    )?;
    if preview.preview_id != request.preview_id {
        return Err(MaaError::new(
            "import preview is stale; generate a new preview before applying",
        ));
    }
    if preview.disposition == ImportDisposition::Unchanged {
        return Ok(ImportApplyResult {
            preview_id: preview.preview_id,
            asset_id: preview.asset_id,
            status: ImportApplyStatus::Unchanged,
            backup_id: None,
            affected_paths: Vec::new(),
        });
    }
    if !preview.can_apply {
        return Err(MaaError::new(
            "import cannot be applied until every conflict has an explicit resolution",
        ));
    }
    if preview.disposition == ImportDisposition::Skip {
        return Ok(ImportApplyResult {
            preview_id: preview.preview_id,
            asset_id: preview.asset_id,
            status: ImportApplyStatus::Skipped,
            backup_id: None,
            affected_paths: Vec::new(),
        });
    }

    let source = find_source(home, &request.request.scope, &request.request.source_id)?;
    let root = home.join(".my-agent-assets");
    let destination = guard_write_path(&root, &preview.destination_path)?;
    let mut registry = load_registry(home).map_err(|error| MaaError::new(error.to_string()))?;
    let original_registry = fs::read(registry_path(home))?;
    let mut mount_registry = if source.asset_kind == AssetKind::Mcp
        && preview.disposition == ImportDisposition::Overwrite
    {
        Some(load_mounts(home).map_err(|error| MaaError::new(error.to_string()))?)
    } else {
        None
    };
    let original_mount_registry = if mount_registry.is_some() {
        Some(fs::read(mount_registry_path(home))?)
    } else {
        None
    };
    let generated_operation_id;
    let operation_id = if let Some(operation_id) = transaction_operation_id {
        operation_id
    } else {
        generated_operation_id = operation_id();
        &generated_operation_id
    };
    let staging = root.join("operations").join(operation_id);
    let staged_content = staging.join("content");
    let guarded_staging = guard_write_path(&root, &staging)?;
    fs::create_dir_all(&guarded_staging)?;
    stage_source(&source, &staged_content)?;

    let had_destination = destination.exists();
    let backup_id = if had_destination {
        Some(create_portable_backup(
            home,
            operation_id,
            &destination,
            &original_registry,
        )?)
    } else {
        None
    };
    let rollback_content = staging.join("rollback-content");
    if had_destination {
        fs::rename(&destination, &rollback_content)?;
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let result = (|| -> Result<()> {
        fs::rename(&staged_content, &destination)?;
        registry
            .upsert(
                AssetRecord::new(source.asset_kind, preview.destination_name.clone())
                    .map_err(|error| MaaError::new(error.to_string()))?,
            )
            .map_err(|error| MaaError::new(error.to_string()))?;
        save_registry(home, &registry).map_err(|error| MaaError::new(error.to_string()))?;
        if let Some(mounts) = mount_registry.as_mut() {
            mounts.mark_asset_out_of_sync(&preview.asset_id);
            save_mounts(home, mounts).map_err(|error| MaaError::new(error.to_string()))?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        let _ = remove_any(&destination);
        if had_destination {
            let _ = fs::rename(&rollback_content, &destination);
        }
        let _ = fs::write(registry_path(home), original_registry);
        if let Some(original_mount_registry) = original_mount_registry {
            let _ = fs::write(mount_registry_path(home), original_mount_registry);
        }
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    let _ = fs::remove_dir_all(&staging);

    let mut affected_paths = vec![destination, registry_path(home)];
    if mount_registry.is_some() {
        affected_paths.push(mount_registry_path(home));
    }
    Ok(ImportApplyResult {
        preview_id: preview.preview_id,
        asset_id: preview.asset_id,
        status: ImportApplyStatus::Imported,
        backup_id,
        affected_paths,
    })
}

pub(crate) fn find_source(
    home: &Path,
    scope: &DiscoveryScope,
    source_id: &str,
) -> Result<DiscoveredSource> {
    discover(home, scope.clone())
        .sources
        .into_iter()
        .find(|source| source.source_id == source_id)
        .ok_or_else(|| {
            MaaError::new(format!(
                "runtime source '{source_id}' no longer exists in the selected scope"
            ))
        })
}

fn detect_conflict(
    home: &Path,
    registry: &AssetRegistry,
    source: &DiscoveredSource,
    destination_name: &str,
) -> Result<Option<ImportConflict>> {
    let destination = canonical_path(home, source.asset_kind, destination_name);
    let registered = registry.get(source.asset_kind, destination_name).is_some();
    if !registered && !destination.exists() {
        return Ok(None);
    }

    if source.asset_kind == AssetKind::Mcp && destination.is_file() {
        let incoming = load_mcp_source(source)?;
        let existing_text = fs::read_to_string(&destination)?;
        let existing = serde_json::from_str::<crate::mcp::CanonicalMcp>(&existing_text)
            .map_err(|error| MaaError::new(format!("invalid existing canonical MCP: {error}")))?;
        if existing == incoming.canonical {
            return Ok(Some(ImportConflict {
                asset_id: crate::asset_registry::asset_id(source.asset_kind, destination_name),
                reason: "canonical MCP is structurally identical".into(),
                existing_content: serde_json::to_string_pretty(&existing)
                    .map_err(|error| MaaError::new(error.to_string()))?,
                incoming_content: serde_json::to_string_pretty(&incoming.canonical)
                    .map_err(|error| MaaError::new(error.to_string()))?,
                raw_source: incoming.raw_source,
            }));
        }
        return Ok(Some(ImportConflict {
            asset_id: crate::asset_registry::asset_id(source.asset_kind, destination_name),
            reason: "canonical MCP differs from the asset center".into(),
            existing_content: existing_text,
            incoming_content: serde_json::to_string_pretty(&incoming.canonical)
                .map_err(|error| MaaError::new(error.to_string()))?,
            raw_source: incoming.raw_source,
        }));
    }

    Ok(Some(ImportConflict {
        asset_id: crate::asset_registry::asset_id(source.asset_kind, destination_name),
        reason: "asset kind and name already exist in the asset center".into(),
        existing_content: preview_existing(&destination, source.asset_kind)?,
        incoming_content: preview_incoming(source)?,
        raw_source: preview_incoming(source)?,
    }))
}

fn stage_source(source: &DiscoveredSource, destination: &Path) -> Result<()> {
    match source.asset_kind {
        AssetKind::Skill => {
            fs::create_dir_all(destination)?;
            match source.source_format {
                SourceFormat::SkillDirectory => copy_directory(&source.source_path, destination),
                SourceFormat::Markdown => {
                    fs::copy(&source.source_path, destination.join("SKILL.md"))?;
                    Ok(())
                }
                _ => Err(MaaError::new("unsupported Skill source format")),
            }
        }
        AssetKind::Command => {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source.source_path, destination)?;
            Ok(())
        }
        AssetKind::Mcp => {
            let loaded = load_mcp_source(source)?;
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = serde_json::to_vec_pretty(&loaded.canonical)
                .map_err(|error| MaaError::new(error.to_string()))?;
            fs::write(destination, content)?;
            Ok(())
        }
    }
}

fn copy_directory(source: &Path, destination: &Path) -> Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let metadata = fs::symlink_metadata(&source_path)?;
        if is_link_or_junction(&metadata) {
            return Err(MaaError::new(format!(
                "nested symlink is not allowed in imported Skill: {}",
                source_path.display()
            )));
        }
        let destination_path = destination.join(entry.file_name());
        if metadata.is_dir() {
            fs::create_dir_all(&destination_path)?;
            copy_directory(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn create_portable_backup(
    home: &Path,
    operation_id: &str,
    destination: &Path,
    registry: &[u8],
) -> Result<String> {
    let root = home.join(".my-agent-assets");
    let backup_id = format!("import-{operation_id}");
    let backup = guard_write_path(&root, &root.join("backups/portable").join(&backup_id))?;
    fs::create_dir_all(&backup)?;
    copy_any(destination, &backup.join("content"))?;
    fs::write(backup.join("assets.yaml"), registry)?;
    fs::write(
        backup.join("manifest.yaml"),
        format!(
            "schemaVersion: 1\noperation: import-overwrite\ndestination: {}\n",
            destination
                .strip_prefix(&root)
                .map_err(|_| MaaError::new("backup destination escaped the asset center"))?
                .display()
        ),
    )?;
    Ok(backup_id)
}

fn preview_existing(path: &Path, kind: AssetKind) -> Result<String> {
    match kind {
        AssetKind::Skill => read_preview(&path.join("SKILL.md")),
        AssetKind::Command | AssetKind::Mcp => read_preview(path),
    }
}

fn preview_incoming(source: &DiscoveredSource) -> Result<String> {
    match source.asset_kind {
        AssetKind::Skill if source.source_path.is_dir() => {
            read_preview(&source.source_path.join("SKILL.md"))
        }
        AssetKind::Mcp => Ok(load_mcp_source(source)?.raw_source),
        _ => read_preview(&source.source_path),
    }
}

fn read_preview(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut buffer = vec![0; 16 * 1024];
    let read = file.read(&mut buffer)?;
    buffer.truncate(read);
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

fn preview_fingerprint(
    home: &Path,
    request: &ImportPreviewRequest,
    source: &DiscoveredSource,
    destination_name: &str,
    registry: &AssetRegistry,
    generated_at_epoch_seconds: u64,
) -> Result<String> {
    let mut fingerprint = PreviewFingerprint::new("import");
    fingerprint.add_bytes(
        "request",
        &serde_json::to_vec(request).map_err(|error| MaaError::new(error.to_string()))?,
    );
    fingerprint.add_bytes("source-id", source.source_id.as_bytes());
    fingerprint.add_bytes("destination-name", destination_name.as_bytes());
    fingerprint.add_u64("generated-at", generated_at_epoch_seconds);
    fingerprint.add_path("source", &source.source_path)?;
    fingerprint.add_bytes(
        "asset-registry",
        &serde_yaml::to_string(registry)
            .map_err(|error| MaaError::new(error.to_string()))?
            .into_bytes(),
    );
    let destination = canonical_path(home, source.asset_kind, destination_name);
    fingerprint.add_path_if_present("destination", &destination)?;
    Ok(fingerprint.finish("import"))
}

fn copy_any(source: &Path, destination: &Path) -> Result<()> {
    if source.is_dir() {
        fs::create_dir_all(destination)?;
        copy_directory(source, destination)
    } else {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, destination)?;
        Ok(())
    }
}

fn remove_any(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn ensure_initialized(home: &Path) -> Result<()> {
    let root = home.join(".my-agent-assets");
    if !root.is_dir() || !registry_path(home).is_file() {
        return Err(MaaError::new(
            "asset center is not initialized; run initialization first",
        ));
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
    use crate::asset_registry::AssetRegistry;
    use crate::mount_registry::{BindingStatus, MountBinding, MountRegistry};
    use std::time::Duration;

    #[test]
    fn contract_wire_values_are_explicit_and_camel_case() {
        assert_eq!(
            serde_json::to_value(ImportResolution::Rename {
                new_name: "review-new".into()
            })
            .unwrap(),
            serde_json::json!({"kind": "rename", "newName": "review-new"})
        );
        assert_eq!(
            serde_json::to_value(DiscoveryScope::Project {
                project_path: PathBuf::from("/tmp/project")
            })
            .unwrap(),
            serde_json::json!({"kind": "project", "projectPath": "/tmp/project"})
        );
        assert_eq!(
            serde_json::to_value(DiscoveryScope::Custom {
                path: PathBuf::from("/tmp/skills"),
                asset_kind: AssetKind::Skill,
                source_format: SourceFormat::SkillDirectory
            })
            .unwrap(),
            serde_json::json!({
                "kind": "custom",
                "path": "/tmp/skills",
                "assetKind": "skill",
                "sourceFormat": "skill_directory"
            })
        );
    }

    #[test]
    fn imports_claude_and_codex_skills_without_modifying_sources() {
        let home = initialized_home("skills");
        let claude = home.join(".claude/skills/review");
        let codex = home.join(".agents/skills/api-design");
        for (path, content) in [(&claude, "# Review"), (&codex, "# API Design")] {
            fs::create_dir_all(path).unwrap();
            fs::write(path.join("SKILL.md"), content).unwrap();
        }
        let before_claude = fs::read(claude.join("SKILL.md")).unwrap();
        let before_codex = fs::read(codex.join("SKILL.md")).unwrap();

        for source in discover(&home, DiscoveryScope::User)
            .sources
            .into_iter()
            .filter(|source| source.asset_kind == AssetKind::Skill)
        {
            apply_source(&home, source.source_id, ImportResolution::Unresolved).unwrap();
        }

        assert_eq!(fs::read(claude.join("SKILL.md")).unwrap(), before_claude);
        assert_eq!(fs::read(codex.join("SKILL.md")).unwrap(), before_codex);
        assert!(canonical_path(&home, AssetKind::Skill, "review").is_dir());
        assert!(canonical_path(&home, AssetKind::Skill, "api-design").is_dir());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn imports_claude_and_codex_mcp_as_canonical_json_without_live_config_writes() {
        let home = initialized_home("mcp");
        let claude_config = home.join(".claude.json");
        let codex_config = home.join(".codex/config.toml");
        fs::write(
            &claude_config,
            r#"{"theme":"dark","mcpServers":{"postgres":{"command":"npx","args":["postgres"]}}}"#,
        )
        .unwrap();
        fs::create_dir_all(codex_config.parent().unwrap()).unwrap();
        fs::write(
            &codex_config,
            "model = \"gpt-5\"\n[mcp_servers.filesystem]\ncommand = \"npx\"\n",
        )
        .unwrap();
        let before_claude = fs::read(&claude_config).unwrap();
        let before_codex = fs::read(&codex_config).unwrap();
        for source in discover(&home, DiscoveryScope::User)
            .sources
            .into_iter()
            .filter(|source| source.asset_kind == AssetKind::Mcp)
        {
            apply_source(&home, source.source_id, ImportResolution::Unresolved).unwrap();
        }
        assert_eq!(fs::read(&claude_config).unwrap(), before_claude);
        assert_eq!(fs::read(&codex_config).unwrap(), before_codex);
        assert!(serde_json::from_slice::<crate::mcp::CanonicalMcp>(
            &fs::read(canonical_path(&home, AssetKind::Mcp, "postgres")).unwrap()
        )
        .is_ok());
        assert!(serde_json::from_slice::<crate::mcp::CanonicalMcp>(
            &fs::read(canonical_path(&home, AssetKind::Mcp, "filesystem")).unwrap()
        )
        .is_ok());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn same_name_skill_conflicts_even_when_content_is_identical() {
        let home = initialized_home("skill-conflict");
        let source = home.join(".claude/skills/review");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "# Same").unwrap();
        let first = discover(&home, DiscoveryScope::User).sources[0]
            .source_id
            .clone();
        apply_source(&home, first.clone(), ImportResolution::Unresolved).unwrap();
        let preview = preview_import(
            &home,
            &ImportPreviewRequest {
                scope: DiscoveryScope::User,
                source_id: first,
                resolution: ImportResolution::Unresolved,
            },
        )
        .unwrap();
        crate::fingerprint::assert_sha256_preview_id(&preview.preview_id, "import-");
        assert_eq!(preview.disposition, ImportDisposition::Conflict);
        assert!(!preview.can_apply);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn conflict_skip_overwrite_and_rename_are_explicit_and_safe() {
        let home = initialized_home("resolutions");
        let source = home.join(".claude/commands/deploy.md");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "old").unwrap();
        let source_id = discover(&home, DiscoveryScope::User).sources[0]
            .source_id
            .clone();
        apply_source(&home, source_id.clone(), ImportResolution::Unresolved).unwrap();
        fs::write(&source, "new").unwrap();

        let skipped = apply_source(&home, source_id.clone(), ImportResolution::Skip).unwrap();
        assert_eq!(skipped.status, ImportApplyStatus::Skipped);
        assert_eq!(
            fs::read_to_string(canonical_path(&home, AssetKind::Command, "deploy")).unwrap(),
            "old"
        );

        let renamed = apply_source(
            &home,
            source_id.clone(),
            ImportResolution::Rename {
                new_name: "deploy-new".into(),
            },
        )
        .unwrap();
        assert_eq!(renamed.status, ImportApplyStatus::Imported);
        assert_eq!(
            fs::read_to_string(canonical_path(&home, AssetKind::Command, "deploy-new")).unwrap(),
            "new"
        );

        let overwritten = apply_source(&home, source_id, ImportResolution::Overwrite).unwrap();
        let backup_id = overwritten.backup_id.unwrap();
        assert_eq!(
            fs::read_to_string(canonical_path(&home, AssetKind::Command, "deploy")).unwrap(),
            "new"
        );
        let manifest = fs::read_to_string(
            home.join(".my-agent-assets/backups/portable")
                .join(backup_id)
                .join("manifest.yaml"),
        )
        .unwrap();
        assert!(manifest.contains("destination: assets/commands/deploy.md"));
        assert!(!manifest.contains(home.to_string_lossy().as_ref()));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn structurally_identical_mcp_is_unchanged_despite_json_key_order() {
        let home = initialized_home("mcp-equal");
        let config = home.join(".claude.json");
        fs::write(
            &config,
            r#"{"mcpServers":{"postgres":{"command":"npx","args":["server"],"env":{"A":"1"}}}}"#,
        )
        .unwrap();
        let source_id = discover(&home, DiscoveryScope::User).sources[0]
            .source_id
            .clone();
        apply_source(&home, source_id.clone(), ImportResolution::Unresolved).unwrap();
        fs::write(
            &config,
            r#"{"mcpServers":{"postgres":{"env":{"A":"1"},"args":["server"],"command":"npx"}}}"#,
        )
        .unwrap();
        let preview = preview_import(
            &home,
            &ImportPreviewRequest {
                scope: DiscoveryScope::User,
                source_id,
                resolution: ImportResolution::Unresolved,
            },
        )
        .unwrap();
        assert_eq!(preview.disposition, ImportDisposition::Unchanged);
        assert!(!preview.can_apply);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn mcp_overwrite_marks_bindings_out_of_sync_without_writing_live_config() {
        let home = initialized_home("mcp-out-of-sync");
        let config = home.join(".claude.json");
        fs::write(
            &config,
            r#"{"other":"keep","mcpServers":{"postgres":{"command":"old"}}}"#,
        )
        .unwrap();
        let source_id = discover(&home, DiscoveryScope::User).sources[0]
            .source_id
            .clone();
        apply_source(&home, source_id.clone(), ImportResolution::Unresolved).unwrap();
        let mut mounts = MountRegistry::default();
        mounts
            .upsert(
                MountBinding::new("mcp:postgres", "claude-user-mcp", BindingStatus::Mounted)
                    .unwrap(),
            )
            .unwrap();
        save_mounts(&home, &mounts).unwrap();

        fs::write(
            &config,
            r#"{"other":"keep","mcpServers":{"postgres":{"command":"new"}}}"#,
        )
        .unwrap();
        let live_before = fs::read(&config).unwrap();
        apply_source(&home, source_id, ImportResolution::Overwrite).unwrap();

        assert_eq!(fs::read(&config).unwrap(), live_before);
        assert_eq!(
            load_mounts(&home).unwrap().for_asset("mcp:postgres")[0].status,
            BindingStatus::OutOfSync
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn expired_preview_is_rejected_without_writing() {
        let home = initialized_home("expired");
        let source = home.join(".claude/commands/test.md");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "test").unwrap();
        let source_id = discover(&home, DiscoveryScope::User).sources[0]
            .source_id
            .clone();
        let request = ImportPreviewRequest {
            scope: DiscoveryScope::User,
            source_id,
            resolution: ImportResolution::Unresolved,
        };
        let generated_at = epoch_seconds().saturating_sub(PREVIEW_TTL_SECONDS + 1);
        let preview = preview_import_at(&home, &request, generated_at).unwrap();
        let error = apply_import(
            &home,
            &ImportApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: generated_at,
                request,
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("expired"));
        assert!(!canonical_path(&home, AssetKind::Command, "test").exists());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn stale_preview_and_plan_only_preview_do_not_write() {
        let home = initialized_home("stale");
        let source = home.join(".claude/commands/test.md");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "one").unwrap();
        let source_id = discover(&home, DiscoveryScope::User).sources[0]
            .source_id
            .clone();
        let request = ImportPreviewRequest {
            scope: DiscoveryScope::User,
            source_id,
            resolution: ImportResolution::Unresolved,
        };
        let preview = preview_import(&home, &request).unwrap();
        assert!(!preview.destination_path.exists());
        fs::write(&source, "two").unwrap();
        let error = apply_import(
            &home,
            &ImportApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("stale"));
        assert!(!canonical_path(&home, AssetKind::Command, "test").exists());
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(unix)]
    #[test]
    fn nested_skill_symlink_is_rejected_without_registry_change() {
        use std::os::unix::fs::symlink;

        let home = initialized_home("nested-link");
        let source = home.join(".claude/skills/review");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "# Review").unwrap();
        symlink("/tmp", source.join("linked")).unwrap();
        let source_id = discover(&home, DiscoveryScope::User).sources[0]
            .source_id
            .clone();
        let before = fs::read(registry_path(&home)).unwrap();
        assert!(apply_source(&home, source_id, ImportResolution::Unresolved).is_err());
        assert_eq!(fs::read(registry_path(&home)).unwrap(), before);
        assert!(!canonical_path(&home, AssetKind::Skill, "review").exists());
        let _ = fs::remove_dir_all(home);
    }

    fn apply_source(
        home: &Path,
        source_id: String,
        resolution: ImportResolution,
    ) -> Result<ImportApplyResult> {
        let request = ImportPreviewRequest {
            scope: DiscoveryScope::User,
            source_id,
            resolution,
        };
        let preview = preview_import(home, &request)?;
        apply_import(
            home,
            &ImportApplyRequest {
                preview_id: preview.preview_id,
                preview_generated_at_epoch_seconds: preview.generated_at_epoch_seconds,
                request,
            },
        )
    }

    fn initialized_home(name: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!(
            "maa-import-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = home.join(".my-agent-assets");
        fs::create_dir_all(root.join("assets/skills")).unwrap();
        fs::create_dir_all(root.join("assets/commands")).unwrap();
        fs::create_dir_all(root.join("assets/mcps")).unwrap();
        fs::create_dir_all(root.join("backups/portable")).unwrap();
        fs::write(
            registry_path(&home),
            serde_yaml::to_string(&AssetRegistry::default()).unwrap(),
        )
        .unwrap();
        fs::write(
            mount_registry_path(&home),
            serde_yaml::to_string(&MountRegistry::default()).unwrap(),
        )
        .unwrap();
        // Ensure coarse filesystems cannot make stale tests depend on mtime.
        std::thread::sleep(Duration::from_millis(1));
        home
    }
}
