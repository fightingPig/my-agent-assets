#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_ROOT="$(mktemp -d /tmp/my-agent-assets-e2e-XXXXXX)"
FAKE_HOME="$TMP_ROOT/fake-home"
FAKE_WORKSPACE="$TMP_ROOT/fake-workspace"
PROJECT_A="$FAKE_WORKSPACE/project-a"

cleanup() {
  rm -rf "$TMP_ROOT"
}
trap cleanup EXIT

assert_file() {
  test -f "$1" || {
    echo "expected file: $1" >&2
    exit 1
  }
}

assert_dir() {
  test -d "$1" || {
    echo "expected dir: $1" >&2
    exit 1
  }
}

assert_symlink() {
  test -L "$1" || {
    echo "expected symlink: $1" >&2
    exit 1
  }
}

assert_not_symlink() {
  if test -L "$1"; then
    echo "expected non-symlink: $1" >&2
    exit 1
  fi
}

assert_json_key() {
  jq -e "$2" "$1" >/dev/null || {
    echo "expected jq key $2 in $1" >&2
    exit 1
  }
}

if [[ -z "$TMP_ROOT" || "$TMP_ROOT" == "$HOME" || "$TMP_ROOT" == "/" ]]; then
  echo "unsafe temp root: $TMP_ROOT" >&2
  exit 1
fi

mkdir -p "$FAKE_HOME/.claude/skills/review" "$FAKE_HOME/.claude/commands"
mkdir -p "$PROJECT_A/.claude/skills/db-review" "$PROJECT_A/.claude/commands"

cat >"$FAKE_HOME/.claude/skills/review/SKILL.md" <<'DATA'
# Review Skill
DATA

cat >"$FAKE_HOME/.claude/commands/commit.md" <<'DATA'
# Commit Command
DATA

cat >"$PROJECT_A/.claude/skills/db-review/SKILL.md" <<'DATA'
# DB Review Skill
DATA

cat >"$PROJECT_A/.claude/commands/deploy.md" <<'DATA'
# Deploy Command
DATA

cat >"$FAKE_HOME/.claude.json" <<DATA
{
  "theme": "dark",
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["github-mcp"]
    }
  },
  "projects": {
    "$PROJECT_A": {
      "mcpServers": {
        "local-tool": {
          "command": "node",
          "args": ["local.js"]
        }
      }
    }
  }
}
DATA

cat >"$PROJECT_A/.mcp.json" <<'DATA'
{
  "projectOnly": true,
  "mcpServers": {
    "project-tool": {
      "command": "node",
      "args": ["project.js"]
    }
  }
}
DATA

cd "$ROOT_DIR"
cargo build -p my-agent-assets-cli --bin maa >/dev/null
BIN="$ROOT_DIR/target/debug/maa"

"$BIN" --home "$FAKE_HOME" init --apply >/tmp/maa-init.out
assert_dir "$FAKE_HOME/.my-agent-assets/.git"
cat >"$FAKE_HOME/.my-agent-assets/config.yaml" <<DATA
asset_center: $FAKE_HOME/.my-agent-assets
git_repo:
scan_roots:
  - $FAKE_WORKSPACE
max_depth: 5
runtime:
  provider: claude
DATA

"$BIN" --home "$FAKE_HOME" scan >/tmp/maa-scan-plan.out
assert_not_symlink "$FAKE_HOME/.claude/skills/review"
assert_file "$FAKE_HOME/.claude/commands/commit.md"
if find "$FAKE_HOME/.my-agent-assets/assets" -type f | grep -q .; then
  echo "scan without --apply unexpectedly wrote assets" >&2
  exit 1
fi

"$BIN" --home "$FAKE_HOME" scan --apply >/tmp/maa-scan-apply.out

assert_dir "$FAKE_HOME/.my-agent-assets/assets/skills/review"
assert_file "$FAKE_HOME/.my-agent-assets/assets/commands/commit.md"
assert_file "$FAKE_HOME/.my-agent-assets/assets/mcps/github.json"
assert_file "$FAKE_HOME/.my-agent-assets/assets/mcps/local-tool.json"
assert_file "$FAKE_HOME/.my-agent-assets/assets/mcps/project-tool.json"
assert_symlink "$FAKE_HOME/.claude/skills/review"
assert_symlink "$FAKE_HOME/.claude/commands/commit.md"
assert_symlink "$PROJECT_A/.claude/skills/db-review"
assert_symlink "$PROJECT_A/.claude/commands/deploy.md"
assert_json_key "$FAKE_HOME/.claude.json" '.mcpServers.github'
assert_json_key "$FAKE_HOME/.claude.json" '.projects["'"$PROJECT_A"'"].mcpServers["local-tool"]'
assert_json_key "$PROJECT_A/.mcp.json" '."projectOnly"'
assert_json_key "$PROJECT_A/.mcp.json" '.mcpServers["project-tool"]'

"$BIN" --home "$FAKE_HOME" mount review --type skill --project "$PROJECT_A" --apply >/tmp/maa-mount.out
assert_symlink "$PROJECT_A/.claude/skills/review"

"$BIN" --home "$FAKE_HOME" unmount review --type skill --apply >/tmp/maa-unmount.out
if test -e "$PROJECT_A/.claude/skills/review"; then
  echo "expected project review mount to be removed" >&2
  exit 1
fi

"$BIN" --home "$FAKE_HOME" remove deploy --type command >/tmp/maa-remove-plan.out
assert_file "$FAKE_HOME/.my-agent-assets/assets/commands/deploy.md"
"$BIN" --home "$FAKE_HOME" remove deploy --type command --apply >/tmp/maa-remove-apply.out
if test -e "$FAKE_HOME/.my-agent-assets/assets/commands/deploy.md"; then
  echo "expected deploy asset to be removed" >&2
  exit 1
fi

BACKUP_ID="$(find "$FAKE_HOME/.my-agent-assets/backups" -mindepth 1 -maxdepth 1 -type d -exec basename {} \; | sort | tail -n 1)"
test -n "$BACKUP_ID"
"$BIN" --home "$FAKE_HOME" restore "$BACKUP_ID" --apply >/tmp/maa-restore.out
assert_not_symlink "$FAKE_HOME/.claude/skills/review"
assert_file "$FAKE_HOME/.claude/skills/review/SKILL.md"
assert_not_symlink "$FAKE_HOME/.claude/commands/commit.md"
assert_file "$FAKE_HOME/.claude/commands/commit.md"
assert_file "$PROJECT_A/.claude/commands/deploy.md"

"$BIN" --home "$FAKE_HOME" list >/tmp/maa-list.out
"$BIN" --home "$FAKE_HOME" status >/tmp/maa-status.out
"$BIN" --home "$FAKE_HOME" doctor >/tmp/maa-doctor.out

echo "E2E fake runtime passed: $TMP_ROOT"
