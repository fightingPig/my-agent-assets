use crate::path_safety::guard_write_path;
use crate::{MaaError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const TARGET_REGISTRY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AssetKind {
    #[serde(rename = "skill")]
    Skill,
    #[serde(rename = "command")]
    Command,
    #[serde(rename = "mcp")]
    Mcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeProvider {
    #[serde(rename = "claude_code")]
    ClaudeCode,
    #[serde(rename = "codex")]
    Codex,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MountTargetKind {
    #[serde(rename = "claude_user_skills")]
    ClaudeUserSkills,
    #[serde(rename = "claude_project_skills")]
    ClaudeProjectSkills,
    #[serde(rename = "codex_user_skills")]
    CodexUserSkills,
    #[serde(rename = "codex_project_skills")]
    CodexProjectSkills,
    #[serde(rename = "custom_skill_directory")]
    CustomSkillDirectory,
    #[serde(rename = "claude_user_commands")]
    ClaudeUserCommands,
    #[serde(rename = "claude_project_commands")]
    ClaudeProjectCommands,
    #[serde(rename = "custom_command_directory")]
    CustomCommandDirectory,
    #[serde(rename = "claude_user_mcp_json")]
    ClaudeUserMcpJson,
    #[serde(rename = "claude_local_mcp_json")]
    ClaudeLocalMcpJson,
    #[serde(rename = "claude_project_mcp_json")]
    ClaudeProjectMcpJson,
    #[serde(rename = "codex_user_mcp_toml")]
    CodexUserMcpToml,
    #[serde(rename = "codex_project_mcp_toml")]
    CodexProjectMcpToml,
    #[serde(rename = "custom_claude_mcp_json")]
    CustomClaudeMcpJson,
    #[serde(rename = "custom_codex_mcp_toml")]
    CustomCodexMcpToml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MountAdapter {
    #[serde(rename = "symlink_directory")]
    SymlinkDirectory,
    #[serde(rename = "symlink_file")]
    SymlinkFile,
    #[serde(rename = "windows_directory_junction")]
    WindowsDirectoryJunction,
    #[serde(rename = "json_mcp_patch")]
    JsonMcpPatch,
    #[serde(rename = "toml_mcp_patch")]
    TomlMcpPatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetScope {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "project")]
    Project,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderState {
    #[serde(rename = "not_installed")]
    NotInstalled,
    #[serde(rename = "installed_not_initialized")]
    InstalledNotInitialized,
    #[serde(rename = "initialized")]
    Initialized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetStatus {
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "blocked")]
    Blocked,
    #[serde(rename = "invalid")]
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountTarget {
    pub id: String,
    pub kind: MountTargetKind,
    pub provider: RuntimeProvider,
    pub accepts: Vec<AssetKind>,
    pub adapter: MountAdapter,
    pub scope: TargetScope,
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<PathBuf>,
    pub provider_state: ProviderState,
    pub status: TargetStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetRegistry {
    pub schema_version: u32,
    #[serde(default)]
    pub targets: Vec<MountTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetCompatibility {
    pub compatible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl TargetCompatibility {
    fn compatible() -> Self {
        Self {
            compatible: true,
            reason: None,
        }
    }

    fn blocked(reason: impl Into<String>) -> Self {
        Self {
            compatible: false,
            reason: Some(reason.into()),
        }
    }
}

impl MountTarget {
    pub fn project(
        id: impl Into<String>,
        kind: MountTargetKind,
        project_path: PathBuf,
    ) -> Result<Self> {
        validate_authorized_path(&project_path, "project path")?;
        let (provider, asset_kind, adapter, relative_path) = match kind {
            MountTargetKind::ClaudeProjectSkills => (
                RuntimeProvider::ClaudeCode,
                AssetKind::Skill,
                directory_mount_adapter(),
                Path::new(".claude/skills"),
            ),
            MountTargetKind::CodexProjectSkills => (
                RuntimeProvider::Codex,
                AssetKind::Skill,
                directory_mount_adapter(),
                Path::new(".agents/skills"),
            ),
            MountTargetKind::ClaudeProjectCommands => (
                RuntimeProvider::ClaudeCode,
                AssetKind::Command,
                MountAdapter::SymlinkFile,
                Path::new(".claude/commands"),
            ),
            MountTargetKind::ClaudeProjectMcpJson => (
                RuntimeProvider::ClaudeCode,
                AssetKind::Mcp,
                MountAdapter::JsonMcpPatch,
                Path::new(".mcp.json"),
            ),
            MountTargetKind::CodexProjectMcpToml => (
                RuntimeProvider::Codex,
                AssetKind::Mcp,
                MountAdapter::TomlMcpPatch,
                Path::new(".codex/config.toml"),
            ),
            _ => {
                return Err(MaaError::new(format!(
                    "target kind {:?} is not a project target",
                    kind
                )))
            }
        };
        let target = Self {
            id: id.into(),
            kind,
            provider,
            accepts: vec![asset_kind],
            adapter,
            scope: TargetScope::Project,
            path: project_path.join(relative_path),
            project_path: Some(project_path),
            provider_state: ProviderState::Initialized,
            status: TargetStatus::Ready,
        };
        target.validate()?;
        Ok(target)
    }

    pub fn custom(id: impl Into<String>, kind: MountTargetKind, path: PathBuf) -> Result<Self> {
        validate_authorized_path(&path, "target path")?;
        let (asset_kind, adapter) = match kind {
            MountTargetKind::CustomSkillDirectory => (AssetKind::Skill, directory_mount_adapter()),
            MountTargetKind::CustomCommandDirectory => {
                (AssetKind::Command, MountAdapter::SymlinkFile)
            }
            MountTargetKind::CustomClaudeMcpJson => (AssetKind::Mcp, MountAdapter::JsonMcpPatch),
            MountTargetKind::CustomCodexMcpToml => (AssetKind::Mcp, MountAdapter::TomlMcpPatch),
            _ => {
                return Err(MaaError::new(format!(
                    "target kind {:?} is not a custom target",
                    kind
                )))
            }
        };
        let target = Self {
            id: id.into(),
            kind,
            provider: RuntimeProvider::Custom,
            accepts: vec![asset_kind],
            adapter,
            scope: TargetScope::Custom,
            path,
            project_path: None,
            provider_state: ProviderState::Initialized,
            status: TargetStatus::Ready,
        };
        target.validate()?;
        Ok(target)
    }

    pub fn compatibility(&self, asset_kind: AssetKind) -> TargetCompatibility {
        if self.status != TargetStatus::Ready {
            return TargetCompatibility::blocked(format!(
                "target '{}' is not ready ({:?})",
                self.id, self.status
            ));
        }

        if asset_kind == AssetKind::Command && self.provider == RuntimeProvider::Codex {
            return TargetCompatibility::blocked("Codex does not support Command targets");
        }

        if !self.accepts.contains(&asset_kind) {
            return TargetCompatibility::blocked(format!(
                "target '{}' does not accept {:?} assets",
                self.id, asset_kind
            ));
        }

        let adapter_matches = match asset_kind {
            AssetKind::Skill => matches!(
                self.adapter,
                MountAdapter::SymlinkDirectory | MountAdapter::WindowsDirectoryJunction
            ),
            AssetKind::Command => self.adapter == MountAdapter::SymlinkFile,
            AssetKind::Mcp => matches!(
                self.adapter,
                MountAdapter::JsonMcpPatch | MountAdapter::TomlMcpPatch
            ),
        };

        if !adapter_matches {
            return TargetCompatibility::blocked(format!(
                "adapter {:?} is incompatible with {:?} assets",
                self.adapter, asset_kind
            ));
        }

        TargetCompatibility::compatible()
    }

    pub fn ensure_compatible(&self, asset_kind: AssetKind) -> Result<()> {
        let compatibility = self.compatibility(asset_kind);
        if compatibility.compatible {
            Ok(())
        } else {
            Err(MaaError::new(
                compatibility
                    .reason
                    .unwrap_or_else(|| "target is incompatible".to_string()),
            ))
        }
    }

    pub fn validate(&self) -> Result<()> {
        validate_target_id(&self.id)?;
        validate_authorized_path(&self.path, "target path")?;

        if self.accepts.is_empty() {
            return Err(MaaError::new(format!(
                "target '{}' must accept exactly one asset kind",
                self.id
            )));
        }

        let unique_accepts: BTreeSet<_> = self.accepts.iter().copied().collect();
        if unique_accepts.len() != self.accepts.len() {
            return Err(MaaError::new(format!(
                "target '{}' contains duplicate accepted asset kinds",
                self.id
            )));
        }

        let expected = expected_shape(self.kind);
        if self.provider != expected.provider
            || self.scope != expected.scope
            || !expected.adapters.contains(&self.adapter)
            || self.accepts.as_slice() != [expected.asset_kind]
        {
            return Err(MaaError::new(format!(
                "target '{}' fields do not match kind {:?}",
                self.id, self.kind
            )));
        }

        match expected.project_path {
            ProjectPathRequirement::Required => {
                let project_path = self.project_path.as_ref().ok_or_else(|| {
                    MaaError::new(format!("target '{}' requires projectPath", self.id))
                })?;
                validate_authorized_path(project_path, "project path")?;
                if !path_is_within(&self.path, project_path) {
                    return Err(MaaError::new(format!(
                        "target '{}' path must be inside projectPath",
                        self.id
                    )));
                }
            }
            ProjectPathRequirement::Forbidden if self.project_path.is_some() => {
                return Err(MaaError::new(format!(
                    "target '{}' must not define projectPath",
                    self.id
                )));
            }
            ProjectPathRequirement::Forbidden => {}
        }

        if self.scope == TargetScope::User
            && self.provider_state != ProviderState::Initialized
            && self.status != TargetStatus::Blocked
        {
            return Err(MaaError::new(format!(
                "uninitialized user target '{}' must be blocked",
                self.id
            )));
        }

        Ok(())
    }
}

pub(crate) fn directory_mount_adapter() -> MountAdapter {
    if cfg!(windows) {
        MountAdapter::WindowsDirectoryJunction
    } else {
        MountAdapter::SymlinkDirectory
    }
}

impl TargetRegistry {
    pub fn new(targets: Vec<MountTarget>) -> Result<Self> {
        let registry = Self {
            schema_version: TARGET_REGISTRY_SCHEMA_VERSION,
            targets,
        };
        registry.validate()?;
        Ok(registry)
    }

    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let registry: Self = serde_yaml::from_str(yaml)
            .map_err(|error| MaaError::new(format!("invalid target registry YAML: {error}")))?;
        registry.validate()?;
        Ok(registry)
    }

    pub fn to_yaml(&self) -> Result<String> {
        self.validate()?;
        serde_yaml::to_string(self)
            .map_err(|error| MaaError::new(format!("cannot serialize target registry: {error}")))
    }

    pub fn validate(&self) -> Result<()> {
        if self.schema_version != TARGET_REGISTRY_SCHEMA_VERSION {
            return Err(MaaError::new(format!(
                "unsupported target registry schemaVersion: {}",
                self.schema_version
            )));
        }

        let mut ids = BTreeSet::new();
        let mut paths = BTreeMap::<String, &str>::new();
        for target in &self.targets {
            target.validate()?;

            if !ids.insert(target.id.as_str()) {
                return Err(MaaError::new(format!("duplicate target id: {}", target.id)));
            }

            let path_key = target_locator_key(target)?;
            if let Some(existing_id) = paths.insert(path_key, &target.id) {
                return Err(MaaError::new(format!(
                    "targets '{}' and '{}' use the same path",
                    existing_id, target.id
                )));
            }
        }
        Ok(())
    }

    /// Resolve an already-authorized target. Apply APIs should accept this ID,
    /// never an arbitrary path supplied by a frontend or CLI caller.
    pub fn resolve(&self, target_id: &str) -> Result<&MountTarget> {
        validate_target_id(target_id)?;
        self.targets
            .iter()
            .find(|target| target.id == target_id)
            .ok_or_else(|| MaaError::new(format!("unknown target id: {target_id}")))
    }

    pub fn compatibility(
        &self,
        target_id: &str,
        asset_kind: AssetKind,
    ) -> Result<TargetCompatibility> {
        Ok(self.resolve(target_id)?.compatibility(asset_kind))
    }

    /// The only registry entry point intended for an Apply operation. Callers
    /// provide an authorized ID and an asset kind; the registry owns the path.
    pub fn resolve_for_apply(
        &self,
        target_id: &str,
        asset_kind: AssetKind,
    ) -> Result<&MountTarget> {
        let target = self.resolve(target_id)?;
        target.ensure_compatible(asset_kind)?;
        Ok(target)
    }

    pub fn standard_user_targets(
        home: &Path,
        claude_state: ProviderState,
        codex_state: ProviderState,
        directory_adapter: MountAdapter,
    ) -> Result<Self> {
        validate_authorized_path(home, "home")?;
        if !matches!(
            directory_adapter,
            MountAdapter::SymlinkDirectory | MountAdapter::WindowsDirectoryJunction
        ) {
            return Err(MaaError::new(
                "standard user skill targets require a directory link adapter",
            ));
        }

        let claude_status = status_for_provider_state(claude_state);
        let codex_status = status_for_provider_state(codex_state);
        Self::new(vec![
            MountTarget {
                id: "claude-user-skills".to_string(),
                kind: MountTargetKind::ClaudeUserSkills,
                provider: RuntimeProvider::ClaudeCode,
                accepts: vec![AssetKind::Skill],
                adapter: directory_adapter,
                scope: TargetScope::User,
                path: home.join(".claude/skills"),
                project_path: None,
                provider_state: claude_state,
                status: claude_status,
            },
            MountTarget {
                id: "claude-user-commands".to_string(),
                kind: MountTargetKind::ClaudeUserCommands,
                provider: RuntimeProvider::ClaudeCode,
                accepts: vec![AssetKind::Command],
                adapter: MountAdapter::SymlinkFile,
                scope: TargetScope::User,
                path: home.join(".claude/commands"),
                project_path: None,
                provider_state: claude_state,
                status: claude_status,
            },
            MountTarget {
                id: "claude-user-mcp".to_string(),
                kind: MountTargetKind::ClaudeUserMcpJson,
                provider: RuntimeProvider::ClaudeCode,
                accepts: vec![AssetKind::Mcp],
                adapter: MountAdapter::JsonMcpPatch,
                scope: TargetScope::User,
                path: home.join(".claude.json"),
                project_path: None,
                provider_state: claude_state,
                status: claude_status,
            },
            MountTarget {
                id: "codex-user-skills".to_string(),
                kind: MountTargetKind::CodexUserSkills,
                provider: RuntimeProvider::Codex,
                accepts: vec![AssetKind::Skill],
                adapter: directory_adapter,
                scope: TargetScope::User,
                path: home.join(".agents/skills"),
                project_path: None,
                provider_state: codex_state,
                status: codex_status,
            },
            MountTarget {
                id: "codex-user-mcp".to_string(),
                kind: MountTargetKind::CodexUserMcpToml,
                provider: RuntimeProvider::Codex,
                accepts: vec![AssetKind::Mcp],
                adapter: MountAdapter::TomlMcpPatch,
                scope: TargetScope::User,
                path: home.join(".codex/config.toml"),
                project_path: None,
                provider_state: codex_state,
                status: codex_status,
            },
        ])
    }
}

pub fn registry_path(home: &Path) -> PathBuf {
    home.join(".my-agent-assets/targets.yaml")
}

pub fn load(home: &Path) -> Result<TargetRegistry> {
    let path = registry_path(home);
    let yaml = fs::read_to_string(&path).map_err(|error| {
        MaaError::new(format!(
            "failed to read target registry {}: {error}",
            path.display()
        ))
    })?;
    TargetRegistry::from_yaml(&yaml)
}

pub fn save(home: &Path, registry: &TargetRegistry) -> Result<()> {
    let root = home.join(".my-agent-assets");
    let path = guard_write_path(&root, &registry_path(home))?;
    let yaml = registry.to_yaml()?;
    let parent = path
        .parent()
        .ok_or_else(|| MaaError::new("target registry path has no parent"))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(".targets.yaml.tmp-{}-{nonce}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(yaml.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temporary, &path)?;
        OpenOptions::new().read(true).open(parent)?.sync_all()
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(MaaError::new(format!(
            "failed to save target registry {}: {error}",
            path.display()
        )));
    }
    Ok(())
}

pub fn validate_target_id(id: &str) -> Result<()> {
    let valid = !id.is_empty()
        && id.len() <= 128
        && id != "."
        && id != ".."
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if valid {
        Ok(())
    } else {
        Err(MaaError::new(format!("unsafe target id: {id:?}")))
    }
}

fn validate_authorized_path(path: &Path, label: &str) -> Result<()> {
    if !is_absolute_cross_platform(path) {
        return Err(MaaError::new(format!(
            "{label} must be an absolute normalized path: {}",
            path.display()
        )));
    }

    let slash_path = path.to_string_lossy().replace('\\', "/");
    let has_relative_component = path
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
        || slash_path
            .split('/')
            .any(|component| matches!(component, "." | ".."));
    if has_relative_component {
        return Err(MaaError::new(format!(
            "{label} must not contain relative components: {}",
            path.display()
        )));
    }

    if is_filesystem_root(path) {
        return Err(MaaError::new(format!(
            "{label} must not be a filesystem root: {}",
            path.display()
        )));
    }
    Ok(())
}

fn is_absolute_cross_platform(path: &Path) -> bool {
    if path.is_absolute() {
        return true;
    }
    let value = path.to_string_lossy().replace('\\', "/");
    let bytes = value.as_bytes();
    (bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/')
        || value.starts_with("//")
}

fn is_filesystem_root(path: &Path) -> bool {
    let value = path.to_string_lossy().replace('\\', "/");
    value == "/"
        || value == "//"
        || (value.len() == 3
            && value.as_bytes()[0].is_ascii_alphabetic()
            && &value.as_bytes()[1..] == b":/")
}

fn normalized_path_key(path: &Path) -> Result<String> {
    validate_authorized_path(path, "target path")?;
    let value = path.to_string_lossy().replace('\\', "/");
    Ok(value.trim_end_matches('/').to_string())
}

fn target_locator_key(target: &MountTarget) -> Result<String> {
    let path = normalized_path_key(&target.path)?;
    let project_path = target
        .project_path
        .as_deref()
        .map(normalized_path_key)
        .transpose()?
        .unwrap_or_default();
    Ok(format!("{:?}\0{path}\0{project_path}", target.kind))
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    match (normalized_path_key(path), normalized_path_key(root)) {
        (Ok(path), Ok(root)) => path == root || path.starts_with(&(root + "/")),
        _ => false,
    }
}

fn status_for_provider_state(state: ProviderState) -> TargetStatus {
    match state {
        ProviderState::Initialized => TargetStatus::Ready,
        ProviderState::NotInstalled | ProviderState::InstalledNotInitialized => {
            TargetStatus::Blocked
        }
    }
}

#[derive(Clone, Copy)]
enum ProjectPathRequirement {
    Required,
    Forbidden,
}

struct ExpectedShape {
    provider: RuntimeProvider,
    asset_kind: AssetKind,
    adapters: &'static [MountAdapter],
    scope: TargetScope,
    project_path: ProjectPathRequirement,
}

fn expected_shape(kind: MountTargetKind) -> ExpectedShape {
    use AssetKind::{Command, Mcp, Skill};
    use MountAdapter::{
        JsonMcpPatch, SymlinkDirectory, SymlinkFile, TomlMcpPatch, WindowsDirectoryJunction,
    };
    use MountTargetKind::*;
    use RuntimeProvider::{ClaudeCode, Codex, Custom};
    use TargetScope::{Custom as CustomScope, Local, Project, User};

    const DIRECTORY_ADAPTERS: &[MountAdapter] = &[SymlinkDirectory, WindowsDirectoryJunction];
    const FILE_ADAPTER: &[MountAdapter] = &[SymlinkFile];
    const JSON_ADAPTER: &[MountAdapter] = &[JsonMcpPatch];
    const TOML_ADAPTER: &[MountAdapter] = &[TomlMcpPatch];

    match kind {
        ClaudeUserSkills => shape(ClaudeCode, Skill, DIRECTORY_ADAPTERS, User, false),
        ClaudeProjectSkills => shape(ClaudeCode, Skill, DIRECTORY_ADAPTERS, Project, true),
        CodexUserSkills => shape(Codex, Skill, DIRECTORY_ADAPTERS, User, false),
        CodexProjectSkills => shape(Codex, Skill, DIRECTORY_ADAPTERS, Project, true),
        CustomSkillDirectory => shape(Custom, Skill, DIRECTORY_ADAPTERS, CustomScope, false),
        ClaudeUserCommands => shape(ClaudeCode, Command, FILE_ADAPTER, User, false),
        ClaudeProjectCommands => shape(ClaudeCode, Command, FILE_ADAPTER, Project, true),
        CustomCommandDirectory => shape(Custom, Command, FILE_ADAPTER, CustomScope, false),
        ClaudeUserMcpJson => shape(ClaudeCode, Mcp, JSON_ADAPTER, User, false),
        ClaudeLocalMcpJson => shape(ClaudeCode, Mcp, JSON_ADAPTER, Local, true),
        ClaudeProjectMcpJson => shape(ClaudeCode, Mcp, JSON_ADAPTER, Project, true),
        CodexUserMcpToml => shape(Codex, Mcp, TOML_ADAPTER, User, false),
        CodexProjectMcpToml => shape(Codex, Mcp, TOML_ADAPTER, Project, true),
        CustomClaudeMcpJson => shape(Custom, Mcp, JSON_ADAPTER, CustomScope, false),
        CustomCodexMcpToml => shape(Custom, Mcp, TOML_ADAPTER, CustomScope, false),
    }
}

fn shape(
    provider: RuntimeProvider,
    asset_kind: AssetKind,
    adapters: &'static [MountAdapter],
    scope: TargetScope,
    project_path: bool,
) -> ExpectedShape {
    ExpectedShape {
        provider,
        asset_kind,
        adapters,
        scope,
        project_path: if project_path {
            ProjectPathRequirement::Required
        } else {
            ProjectPathRequirement::Forbidden
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn custom_json_target() -> MountTarget {
        MountTarget {
            id: "custom-json".to_string(),
            kind: MountTargetKind::CustomClaudeMcpJson,
            provider: RuntimeProvider::Custom,
            accepts: vec![AssetKind::Mcp],
            adapter: MountAdapter::JsonMcpPatch,
            scope: TargetScope::Custom,
            path: PathBuf::from("/tmp/custom-mcp.json"),
            project_path: None,
            provider_state: ProviderState::Initialized,
            status: TargetStatus::Ready,
        }
    }

    #[test]
    fn enum_wire_values_are_explicit_and_stable() {
        assert_wire_values(&[
            (AssetKind::Skill, "skill"),
            (AssetKind::Command, "command"),
            (AssetKind::Mcp, "mcp"),
        ]);
        assert_wire_values(&[
            (RuntimeProvider::ClaudeCode, "claude_code"),
            (RuntimeProvider::Codex, "codex"),
            (RuntimeProvider::Custom, "custom"),
        ]);
        assert_wire_values(&[
            (MountTargetKind::ClaudeUserSkills, "claude_user_skills"),
            (
                MountTargetKind::ClaudeProjectSkills,
                "claude_project_skills",
            ),
            (MountTargetKind::CodexUserSkills, "codex_user_skills"),
            (MountTargetKind::CodexProjectSkills, "codex_project_skills"),
            (
                MountTargetKind::CustomSkillDirectory,
                "custom_skill_directory",
            ),
            (MountTargetKind::ClaudeUserCommands, "claude_user_commands"),
            (
                MountTargetKind::ClaudeProjectCommands,
                "claude_project_commands",
            ),
            (
                MountTargetKind::CustomCommandDirectory,
                "custom_command_directory",
            ),
            (MountTargetKind::ClaudeUserMcpJson, "claude_user_mcp_json"),
            (MountTargetKind::ClaudeLocalMcpJson, "claude_local_mcp_json"),
            (
                MountTargetKind::ClaudeProjectMcpJson,
                "claude_project_mcp_json",
            ),
            (MountTargetKind::CodexUserMcpToml, "codex_user_mcp_toml"),
            (
                MountTargetKind::CodexProjectMcpToml,
                "codex_project_mcp_toml",
            ),
            (
                MountTargetKind::CustomClaudeMcpJson,
                "custom_claude_mcp_json",
            ),
            (MountTargetKind::CustomCodexMcpToml, "custom_codex_mcp_toml"),
        ]);
        assert_wire_values(&[
            (MountAdapter::SymlinkDirectory, "symlink_directory"),
            (MountAdapter::SymlinkFile, "symlink_file"),
            (
                MountAdapter::WindowsDirectoryJunction,
                "windows_directory_junction",
            ),
            (MountAdapter::JsonMcpPatch, "json_mcp_patch"),
            (MountAdapter::TomlMcpPatch, "toml_mcp_patch"),
        ]);
        assert_wire_values(&[
            (TargetScope::User, "user"),
            (TargetScope::Local, "local"),
            (TargetScope::Project, "project"),
            (TargetScope::Custom, "custom"),
        ]);
        assert_wire_values(&[
            (ProviderState::NotInstalled, "not_installed"),
            (
                ProviderState::InstalledNotInitialized,
                "installed_not_initialized",
            ),
            (ProviderState::Initialized, "initialized"),
        ]);
        assert_wire_values(&[
            (TargetStatus::Ready, "ready"),
            (TargetStatus::Blocked, "blocked"),
            (TargetStatus::Invalid, "invalid"),
        ]);
    }

    fn assert_wire_values<T>(cases: &[(T, &str)])
    where
        T: Serialize + Copy,
    {
        for (value, wire) in cases {
            assert_eq!(serde_json::to_value(value).unwrap(), json!(wire));
        }
    }

    #[test]
    fn command_is_rejected_for_codex_even_if_target_is_tampered() {
        let mut target = custom_json_target();
        target.provider = RuntimeProvider::Codex;
        target.accepts = vec![AssetKind::Command];
        target.adapter = MountAdapter::SymlinkFile;

        let result = target.compatibility(AssetKind::Command);
        assert!(!result.compatible);
        assert!(result.reason.unwrap().contains("Codex"));
    }

    #[test]
    fn custom_mcp_targets_require_the_declared_adapter() {
        let target = custom_json_target();
        target.validate().unwrap();
        assert!(target.compatibility(AssetKind::Mcp).compatible);

        let mut invalid = target;
        invalid.adapter = MountAdapter::TomlMcpPatch;
        assert!(invalid.validate().is_err());

        let custom_toml = MountTarget {
            id: "custom-toml".to_string(),
            kind: MountTargetKind::CustomCodexMcpToml,
            provider: RuntimeProvider::Custom,
            accepts: vec![AssetKind::Mcp],
            adapter: MountAdapter::TomlMcpPatch,
            scope: TargetScope::Custom,
            path: PathBuf::from("/tmp/custom-mcp.toml"),
            project_path: None,
            provider_state: ProviderState::Initialized,
            status: TargetStatus::Ready,
        };
        custom_toml.validate().unwrap();
        assert!(custom_toml.compatibility(AssetKind::Mcp).compatible);
    }

    #[test]
    fn registry_rejects_duplicate_ids_and_paths() {
        let first = custom_json_target();
        let mut duplicate_id = first.clone();
        duplicate_id.path = PathBuf::from("/tmp/other-mcp.json");
        assert!(TargetRegistry::new(vec![first.clone(), duplicate_id]).is_err());

        let mut duplicate_path = first.clone();
        duplicate_path.id = "other-target".to_string();
        assert!(TargetRegistry::new(vec![first, duplicate_path]).is_err());
    }

    #[test]
    fn unsafe_ids_are_rejected() {
        for id in ["", ".", "..", "../escape", "with/slash", "with space"] {
            assert!(validate_target_id(id).is_err(), "{id:?} must be rejected");
        }
        validate_target_id("project-a.codex_mcp").unwrap();
    }

    #[test]
    fn yaml_round_trip_and_id_only_resolution() {
        let registry = TargetRegistry::new(vec![custom_json_target()]).unwrap();
        let yaml = registry.to_yaml().unwrap();
        assert!(yaml.contains("schemaVersion: 1"));
        assert!(yaml.contains("custom_claude_mcp_json"));

        let restored = TargetRegistry::from_yaml(&yaml).unwrap();
        assert_eq!(restored, registry);
        assert_eq!(
            restored.resolve("custom-json").unwrap().path,
            PathBuf::from("/tmp/custom-mcp.json")
        );
        assert!(restored.resolve("/tmp/custom-mcp.json").is_err());
        assert!(restored
            .resolve_for_apply("custom-json", AssetKind::Mcp)
            .is_ok());
        assert!(restored
            .resolve_for_apply("custom-json", AssetKind::Skill)
            .is_err());
    }

    #[test]
    fn standard_user_targets_block_uninitialized_providers() {
        let registry = TargetRegistry::standard_user_targets(
            Path::new("/tmp/fake-home"),
            ProviderState::InstalledNotInitialized,
            ProviderState::Initialized,
            MountAdapter::SymlinkDirectory,
        )
        .unwrap();

        assert_eq!(
            registry.resolve("claude-user-skills").unwrap().status,
            TargetStatus::Blocked
        );
        assert_eq!(
            registry.resolve("codex-user-skills").unwrap().status,
            TargetStatus::Ready
        );
        assert!(
            !registry
                .compatibility("claude-user-skills", AssetKind::Skill)
                .unwrap()
                .compatible
        );
    }

    #[test]
    fn project_target_must_stay_inside_registered_project() {
        let target = MountTarget {
            id: "project-skills".to_string(),
            kind: MountTargetKind::ClaudeProjectSkills,
            provider: RuntimeProvider::ClaudeCode,
            accepts: vec![AssetKind::Skill],
            adapter: MountAdapter::SymlinkDirectory,
            scope: TargetScope::Project,
            path: PathBuf::from("/tmp/other/.claude/skills"),
            project_path: Some(PathBuf::from("/tmp/project")),
            provider_state: ProviderState::Initialized,
            status: TargetStatus::Ready,
        };

        assert!(target.validate().is_err());
    }

    #[test]
    fn registry_file_round_trips() {
        let home = std::env::temp_dir().join(format!(
            "maa-target-registry-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(home.join(".my-agent-assets")).unwrap();
        let registry = TargetRegistry::standard_user_targets(
            &home,
            ProviderState::Initialized,
            ProviderState::Initialized,
            MountAdapter::SymlinkDirectory,
        )
        .unwrap();
        save(&home, &registry).unwrap();
        assert_eq!(load(&home).unwrap(), registry);
        let _ = fs::remove_dir_all(home);
    }
}
