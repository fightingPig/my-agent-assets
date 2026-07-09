# My Agent Assets V1 Execution Plan

## Phases

1. Create Rust workspace and CLI skeleton.
2. Implement asset center initialization and config persistence.
3. Implement discovery for fake HOME and configured scan roots.
4. Implement scan planning and scan apply.
5. Implement Skill and Command backup/adopt/symlink behavior.
6. Implement MCP extract and compile for user, local, and project scope.
7. Implement list, status, doctor, mount, unmount, remove, Backup History, and sync.
8. Add tests and an isolated fake-runtime e2e script.
9. Optionally create a private GitHub test repository and use it as the fake
   asset center remote for sync validation.

## Validation Data

All e2e validation uses `/tmp/my-agent-assets-e2e-*`:

```text
fake-home/
  .claude/
    skills/review/SKILL.md
    commands/commit.md
  .claude.json

fake-workspace/project-a/
  .claude/
    skills/db-review/SKILL.md
    commands/deploy.md
  .mcp.json
```

## Commands

```bash
cargo test
cargo run -p my-agent-assets-cli --bin maa -- --help
./scripts/e2e_fake_runtime.sh
```

The e2e script asserts that `maa init --apply` creates `.git` inside the fake
asset center:

```text
fake-home/.my-agent-assets/.git
```

## Safety Checks

- Tests pass explicit fake HOME paths.
- The e2e script refuses to run if the fake root is empty or equals the real
  home directory.
- No test invokes `claude mcp list` or `claude mcp get`.
- No test writes to real `~/.claude`, `~/.claude.json`, or
  `~/.my-agent-assets`.
- MCP conflict tests must verify that scan displays both JSON bodies, default
  apply fails without a decision, and rename imports without rewriting the
  original runtime JSON source.
- Security tests must cover Backup History manifest path validation, malformed
  backup manifests, git output sanitization, YAML config parsing, and registry
  round-tripping.
