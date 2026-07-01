use crate::asset_registry::{canonical_path, load as load_assets, parse_asset_id};
use crate::path_safety::guard_existing_path;
use crate::targets::AssetKind;
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

const PREVIEW_LIMIT_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CanonicalAssetContent {
    pub asset_id: String,
    pub asset_type: AssetKind,
    pub canonical_path: PathBuf,
    pub content_path: PathBuf,
    pub content: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetOpenAction {
    #[serde(rename = "reveal")]
    Reveal,
    #[serde(rename = "open_external")]
    OpenExternal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetOpenRequest {
    pub asset_id: String,
    pub action: AssetOpenAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetOpenTarget {
    pub asset_id: String,
    pub action: AssetOpenAction,
    pub path: PathBuf,
}

pub fn load_canonical_asset_content(
    home: &Path,
    asset_id_value: &str,
) -> Result<CanonicalAssetContent> {
    let (kind, _, canonical) = resolve_registered_asset(home, asset_id_value)?;
    let content_path = match kind {
        AssetKind::Skill => canonical.join("SKILL.md"),
        AssetKind::Command | AssetKind::Mcp => canonical.clone(),
    };
    let content_path = guard_existing_path(&home.join(".my-agent-assets"), &content_path)?;
    if !content_path.is_file() {
        return Err(MaaError::new(format!(
            "canonical asset content is not a file: {}",
            content_path.display()
        )));
    }
    let (content, truncated) = read_utf8_preview(&content_path)?;
    Ok(CanonicalAssetContent {
        asset_id: asset_id_value.to_string(),
        asset_type: kind,
        canonical_path: canonical,
        content_path,
        content,
        truncated,
    })
}

pub fn resolve_asset_open_target(
    home: &Path,
    request: &AssetOpenRequest,
) -> Result<AssetOpenTarget> {
    let (kind, _, canonical) = resolve_registered_asset(home, &request.asset_id)?;
    let path = match (kind, request.action) {
        (AssetKind::Skill, AssetOpenAction::Reveal) => canonical.join("SKILL.md"),
        (AssetKind::Command, AssetOpenAction::OpenExternal) => canonical,
        (AssetKind::Skill, AssetOpenAction::OpenExternal) => {
            return Err(MaaError::new(
                "Skill assets can only be revealed in the file manager",
            ));
        }
        (AssetKind::Command, AssetOpenAction::Reveal) => {
            return Err(MaaError::new(
                "Command assets can only be opened with the system application",
            ));
        }
        (AssetKind::Mcp, _) => {
            return Err(MaaError::new(
                "MCP assets are edited through the structured MCP editor",
            ));
        }
    };
    let path = guard_existing_path(&home.join(".my-agent-assets"), &path)?;
    if !path.is_file() {
        return Err(MaaError::new("asset open target is not a file"));
    }
    Ok(AssetOpenTarget {
        asset_id: request.asset_id.clone(),
        action: request.action,
        path,
    })
}

fn resolve_registered_asset(
    home: &Path,
    asset_id_value: &str,
) -> Result<(AssetKind, String, PathBuf)> {
    let (kind, name) =
        parse_asset_id(asset_id_value).map_err(|error| MaaError::new(error.to_string()))?;
    let assets = load_assets(home).map_err(|error| MaaError::new(error.to_string()))?;
    if assets.get(kind, &name).is_none() {
        return Err(MaaError::new(format!(
            "asset '{asset_id_value}' is not registered"
        )));
    }
    let canonical = canonical_path(home, kind, &name);
    let canonical = guard_existing_path(&home.join(".my-agent-assets"), &canonical)?;
    Ok((kind, name, canonical))
}

fn read_utf8_preview(path: &Path) -> Result<(String, bool)> {
    let metadata = fs::metadata(path)?;
    let truncated = metadata.len() > PREVIEW_LIMIT_BYTES as u64;
    let mut bytes = Vec::with_capacity(
        usize::try_from(metadata.len())
            .unwrap_or(PREVIEW_LIMIT_BYTES)
            .min(PREVIEW_LIMIT_BYTES),
    );
    File::open(path)?
        .take((PREVIEW_LIMIT_BYTES + 4) as u64)
        .read_to_end(&mut bytes)?;
    if truncated && bytes.len() > PREVIEW_LIMIT_BYTES {
        bytes.truncate(PREVIEW_LIMIT_BYTES);
        if let Err(error) = std::str::from_utf8(&bytes) {
            if error.error_len().is_some() {
                return Err(MaaError::new(format!(
                    "canonical content is not UTF-8: {error}"
                )));
            }
            bytes.truncate(error.valid_up_to());
        }
    }
    let content = String::from_utf8(bytes)
        .map_err(|error| MaaError::new(format!("canonical content is not UTF-8: {error}")))?;
    Ok((content, truncated))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_registry::{save, AssetRecord, AssetRegistry};

    fn home(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "maa-asset-access-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join(".my-agent-assets/assets/skills/review")).unwrap();
        fs::create_dir_all(root.join(".my-agent-assets/assets/commands")).unwrap();
        fs::create_dir_all(root.join(".my-agent-assets/assets/mcps")).unwrap();
        let mut registry = AssetRegistry::default();
        for (kind, name) in [
            (AssetKind::Skill, "review"),
            (AssetKind::Command, "commit"),
            (AssetKind::Mcp, "filesystem"),
        ] {
            registry
                .upsert(AssetRecord::new(kind, name).unwrap())
                .unwrap();
        }
        save(&root, &registry).unwrap();
        fs::write(
            root.join(".my-agent-assets/assets/skills/review/SKILL.md"),
            "# Review\nreal skill content",
        )
        .unwrap();
        fs::write(
            root.join(".my-agent-assets/assets/commands/commit.md"),
            "# Commit\nreal command content",
        )
        .unwrap();
        fs::write(
            root.join(".my-agent-assets/assets/mcps/filesystem.json"),
            r#"{"schemaVersion":1,"name":"filesystem","spec":{"type":"stdio","command":"npx"},"providerExtensions":{}}"#,
        )
        .unwrap();
        root
    }

    #[test]
    fn reads_real_skill_command_and_mcp_content_by_registered_id() {
        let home = home("read");
        let skill = load_canonical_asset_content(&home, "skill:review").unwrap();
        assert!(skill.content.contains("real skill content"));
        assert!(skill.content_path.ends_with("review/SKILL.md"));
        let command = load_canonical_asset_content(&home, "command:commit").unwrap();
        assert!(command.content.contains("real command content"));
        let mcp = load_canonical_asset_content(&home, "mcp:filesystem").unwrap();
        assert!(mcp.content.contains("\"filesystem\""));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn only_allows_kind_specific_open_actions() {
        let home = home("open");
        let skill = resolve_asset_open_target(
            &home,
            &AssetOpenRequest {
                asset_id: "skill:review".into(),
                action: AssetOpenAction::Reveal,
            },
        )
        .unwrap();
        assert!(skill.path.ends_with("review/SKILL.md"));
        let command = resolve_asset_open_target(
            &home,
            &AssetOpenRequest {
                asset_id: "command:commit".into(),
                action: AssetOpenAction::OpenExternal,
            },
        )
        .unwrap();
        assert!(command.path.ends_with("commit.md"));
        assert!(resolve_asset_open_target(
            &home,
            &AssetOpenRequest {
                asset_id: "mcp:filesystem".into(),
                action: AssetOpenAction::OpenExternal,
            },
        )
        .is_err());
        let _ = fs::remove_dir_all(home);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_canonical_content() {
        let home = home("symlink");
        let outside = home.join("outside.md");
        fs::write(&outside, "outside").unwrap();
        let command = home.join(".my-agent-assets/assets/commands/commit.md");
        fs::remove_file(&command).unwrap();
        std::os::unix::fs::symlink(&outside, &command).unwrap();
        assert!(load_canonical_asset_content(&home, "command:commit").is_err());
        let _ = fs::remove_dir_all(home);
    }
}
