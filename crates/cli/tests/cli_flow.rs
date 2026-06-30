use my_agent_assets_core::operation::{OperationJournal, RecoveryTarget};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TestHome {
    path: PathBuf,
}

impl TestHome {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "maa-cli-flow-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.path.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn maa(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_maa"))
        .arg("--home")
        .arg(home)
        .args(args)
        .env_remove("MY_AGENT_ASSETS_HOME")
        .output()
        .unwrap()
}

fn success(home: &Path, args: &[&str]) -> String {
    let output = maa(home, args);
    assert!(
        output.status.success(),
        "maa {} failed:\nstdout={}\nstderr={}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn json_output(home: &Path, args: &[&str]) -> Value {
    let output = success(home, args);
    let json_end = output
        .rfind('}')
        .expect("command output should contain a JSON object");
    serde_json::from_str(&output[..=json_end]).unwrap()
}

fn source_id(scan: &Value, provider: &str, kind: &str, name: &str) -> String {
    scan["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| {
            source["provider"] == provider
                && source["assetKind"] == kind
                && source["assetName"] == name
        })
        .and_then(|source| source["sourceId"].as_str())
        .unwrap()
        .to_string()
}

#[test]
fn shared_core_cli_flow_uses_source_and_target_ids() {
    let home = TestHome::new();
    home.write(".claude/skills/review/SKILL.md", "# Review\n");
    home.write(".agents/skills/codex-only/SKILL.md", "# Codex only\n");
    home.write(".claude/commands/deploy.md", "# Deploy\n");
    home.write(".claude.json", "{\"mcpServers\":{}}\n");
    home.write(".codex/config.toml", "# codex\n");
    let project = home.path.join("workspace/project-a");
    fs::create_dir_all(&project).unwrap();

    success(&home.path, &["init", "--apply"]);
    let scan = json_output(&home.path, &["scan", "--scope", "user"]);
    assert_eq!(scan["sources"].as_array().unwrap().len(), 3);
    assert!(
        fs::read_dir(home.path.join(".my-agent-assets/assets/skills"))
            .unwrap()
            .next()
            .is_none()
    );

    let review = source_id(&scan, "claude_code", "skill", "review");
    let codex_only = source_id(&scan, "codex", "skill", "codex-only");
    let deploy = source_id(&scan, "claude_code", "command", "deploy");

    json_output(
        &home.path,
        &["import", &review, "--scope", "user", "--apply"],
    );
    json_output(
        &home.path,
        &["import", &codex_only, "--scope", "user", "--apply"],
    );
    json_output(
        &home.path,
        &["import", &deploy, "--scope", "user", "--apply"],
    );
    assert!(home
        .path
        .join(".my-agent-assets/assets/skills/review/SKILL.md")
        .is_file());
    assert!(home
        .path
        .join(".my-agent-assets/assets/commands/deploy.md")
        .is_file());

    let project_text = project.to_string_lossy().to_string();
    json_output(
        &home.path,
        &[
            "target",
            "add",
            "claude-project-skills",
            "project-a-claude-skills",
            "--project",
            &project_text,
            "--apply",
        ],
    );
    json_output(
        &home.path,
        &[
            "target",
            "add",
            "codex-project-skills",
            "project-a-codex-skills",
            "--project",
            &project_text,
            "--apply",
        ],
    );

    json_output(
        &home.path,
        &[
            "mount",
            "skill:review",
            "--target",
            "project-a-claude-skills",
            "--apply",
        ],
    );
    json_output(
        &home.path,
        &[
            "mount",
            "skill:review",
            "--target",
            "project-a-codex-skills",
            "--apply",
        ],
    );
    assert!(project.join(".claude/skills/review").exists());
    assert!(project.join(".agents/skills/review").exists());

    let command_to_codex = maa(
        &home.path,
        &[
            "mount",
            "command:deploy",
            "--target",
            "project-a-codex-skills",
            "--apply",
        ],
    );
    assert!(!command_to_codex.status.success());
    assert!(String::from_utf8_lossy(&command_to_codex.stderr).contains("Codex"));

    let blocked_remove = maa(&home.path, &["remove", "skill:review", "--apply"]);
    assert!(!blocked_remove.status.success());
    assert!(String::from_utf8_lossy(&blocked_remove.stderr).contains("binding"));

    json_output(
        &home.path,
        &["remove", "skill:review", "--unmount-all", "--apply"],
    );
    assert!(!home
        .path
        .join(".my-agent-assets/assets/skills/review")
        .exists());
    assert!(!project.join(".claude/skills/review").exists());
    assert!(!project.join(".agents/skills/review").exists());

    let status = json_output(&home.path, &["status"]);
    assert_eq!(status["assetCount"], 2);
    assert_eq!(status["bindingCount"], 0);
    assert_eq!(status["targetCount"], 7);
}

#[test]
fn scan_is_read_only_sync_is_previewed_and_restore_is_disabled() {
    let home = TestHome::new();
    home.write(".claude.json", "{}\n");
    success(&home.path, &["init", "--apply"]);

    let scan_apply = maa(&home.path, &["scan", "--apply"]);
    assert!(!scan_apply.status.success());
    assert!(String::from_utf8_lossy(&scan_apply.stderr).contains("scan is read-only"));

    let sync = json_output(&home.path, &["sync", "push"]);
    assert_eq!(sync["direction"], "push");
    assert_eq!(sync["canApply"], false);
    assert!(sync["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning.as_str().unwrap_or_default().contains("remote")));

    let restore = maa(&home.path, &["restore", "backup-1", "--apply"]);
    assert!(!restore.status.success());
    assert!(String::from_utf8_lossy(&restore.stderr).contains("not supported"));
}

#[test]
fn cli_startup_recovers_an_interrupted_transaction_before_reading_status() {
    let home = TestHome::new();
    success(&home.path, &["init", "--apply"]);
    let registry = home.path.join(".my-agent-assets/assets.yaml");
    let original = fs::read(&registry).unwrap();

    let journal = OperationJournal::start_recoverable(
        &home.path,
        "cli-crash-recovery",
        "test_crash",
        vec![RecoveryTarget::asset_center(registry.clone())],
    )
    .unwrap();
    drop(journal);
    fs::write(&registry, "schemaVersion: 1\nassets: broken\n").unwrap();

    let output = maa(&home.path, &["status"]);
    assert!(
        output.status.success(),
        "status failed after recovery: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(&registry).unwrap(), original);
    assert!(String::from_utf8_lossy(&output.stderr).contains("成功回滚 1 个"));
    let journal_text = fs::read_to_string(
        home.path
            .join(".my-agent-assets/operations/cli-crash-recovery.yaml"),
    )
    .unwrap();
    assert!(journal_text.contains("status: recovered"));
}
