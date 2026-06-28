use super::codex::{list_codex_mcp_servers_for_context, list_codex_skills_for_context};
use super::contracts::{AssetStatus, CodexMcpTransport, CodexScope};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempTree {
    path: PathBuf,
}

impl TempTree {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "my-agent-assets-codex-{}-{}-{}",
            name,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&path).expect("temp root should be created");
        Self { path }
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.path.join(relative);
        fs::create_dir_all(path.parent().expect("file should have parent"))
            .expect("parent should be created");
        fs::write(path, content).expect("fixture should be written");
    }

    fn mkdir(&self, relative: &str) {
        fs::create_dir_all(self.path.join(relative)).expect("fixture directory should be created");
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn codex_skills_discovers_global_project_ancestor_system_and_metadata() {
    let home = TempTree::new("skills-home");
    let system = TempTree::new("skills-system");
    home.write(
        ".agents/skills/global-review/SKILL.md",
        "---\nname: global-review\ndescription: Global review workflow\n---\n",
    );
    home.mkdir(".agents/skills/global-review/scripts");
    home.mkdir(".agents/skills/global-review/references");
    home.mkdir(".agents/skills/global-review/assets");
    home.write(
        ".agents/skills/global-review/agents/openai.yaml",
        "interface:\n  display_name: Review\n",
    );
    home.write("workspace/repo/package.json", "{}");
    home.write(
        "workspace/repo/.agents/skills/repo-skill/SKILL.md",
        "# Repo Skill\n\nProject-local workflow.",
    );
    home.write(
        "workspace/repo/packages/app/.agents/skills/nested-skill/SKILL.md",
        "---\ndescription: Nested workflow\n---\n",
    );
    system.write(
        "system-skill/SKILL.md",
        "---\ndescription: System workflow\n---\n",
    );

    let result = list_codex_skills_for_context(
        &home.path,
        Some(&home.path.join("workspace/repo/packages/app")),
        Some(&system.path),
    );

    assert!(result.warnings.is_empty());
    assert_eq!(result.skills.len(), 4);
    let global = result
        .skills
        .iter()
        .find(|skill| skill.name == "global-review")
        .expect("global skill");
    assert_eq!(global.scope, CodexScope::Global);
    assert_eq!(global.status, AssetStatus::Ready);
    assert!(global.has_scripts);
    assert!(global.has_references);
    assert!(global.has_assets);
    assert!(global.has_openai_metadata);
    assert!(result
        .skills
        .iter()
        .any(|skill| skill.name == "repo-skill" && skill.scope == CodexScope::Project));
    assert!(result
        .skills
        .iter()
        .any(|skill| skill.name == "nested-skill" && skill.scope == CodexScope::Project));
    assert!(result
        .skills
        .iter()
        .any(|skill| skill.name == "system-skill" && skill.scope == CodexScope::System));
}

#[cfg(unix)]
#[test]
fn codex_skills_identifies_symlinked_skill_folders() {
    let home = TempTree::new("symlink-home");
    let target = TempTree::new("symlink-target");
    target.write(
        "shared/SKILL.md",
        "---\ndescription: Shared workflow\n---\n",
    );
    let root = home.path.join(".agents/skills");
    fs::create_dir_all(&root).expect("skill root");
    std::os::unix::fs::symlink(target.path.join("shared"), root.join("shared"))
        .expect("skill symlink");

    let result = list_codex_skills_for_context(&home.path, None, None);

    assert_eq!(result.skills.len(), 1);
    assert!(result.skills[0].symlink_target.is_some());
}

#[test]
fn codex_mcp_discovers_global_and_project_transport_and_tool_fields() {
    let home = TempTree::new("mcp");
    home.write(
        ".codex/config.toml",
        r#"
[mcp_servers.filesystem]
command = "npx"
args = ["-y", "server-filesystem"]
enabled = true
enabled_tools = ["read_file"]
disabled_tools = ["write_file"]
approval_mode = "manual"
"#,
    );
    home.write(
        "workspace/repo/Cargo.toml",
        "[package]\nname='repo'\nversion='0.1.0'\n",
    );
    home.write(
        "workspace/repo/.codex/config.toml",
        r#"
[mcp_servers.remote]
url = "https://example.test/mcp"
enabled = false
bearer_token_env_var = "MCP_TOKEN"
"#,
    );

    let result =
        list_codex_mcp_servers_for_context(&home.path, Some(&home.path.join("workspace/repo/src")));

    assert!(result.warnings.is_empty());
    assert_eq!(result.servers.len(), 2);
    let filesystem = result
        .servers
        .iter()
        .find(|server| server.name == "filesystem")
        .expect("global server");
    assert_eq!(filesystem.scope, CodexScope::Global);
    assert_eq!(filesystem.transport, CodexMcpTransport::Stdio);
    assert_eq!(filesystem.command.as_deref(), Some("npx"));
    assert_eq!(filesystem.enabled_tools, vec!["read_file"]);
    assert_eq!(filesystem.disabled_tools, vec!["write_file"]);
    assert_eq!(filesystem.approval_mode.as_deref(), Some("manual"));

    let remote = result
        .servers
        .iter()
        .find(|server| server.name == "remote")
        .expect("project server");
    assert_eq!(remote.scope, CodexScope::Project);
    assert_eq!(remote.transport, CodexMcpTransport::StreamableHttp);
    assert!(!remote.enabled);
    assert!(remote
        .warnings
        .iter()
        .any(|warning| warning.contains("OAuth")));
}

#[test]
fn codex_mcp_reports_invalid_toml_without_failing_discovery() {
    let home = TempTree::new("invalid-mcp");
    home.write(".codex/config.toml", "[mcp_servers.");

    let result = list_codex_mcp_servers_for_context(&home.path, None);

    assert!(result.servers.is_empty());
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].contains("Could not parse"));
}
