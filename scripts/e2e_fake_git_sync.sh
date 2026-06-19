#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REMOTE_URL="${MAA_REMOTE_URL:-}"
TMP_ROOT="$(mktemp -d /tmp/my-agent-assets-git-e2e-XXXXXX)"
FAKE_HOME="$TMP_ROOT/fake-home"
FAKE_WORKSPACE="$TMP_ROOT/fake-workspace"
PROJECT_A="$FAKE_WORKSPACE/project-a"
BRANCH="e2e-$(date +%s)-$$"

cleanup() {
  rm -rf "$TMP_ROOT"
}
trap cleanup EXIT INT TERM

if [[ -z "$REMOTE_URL" ]]; then
  echo "MAA_REMOTE_URL is required" >&2
  exit 1
fi

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

cat >"$FAKE_HOME/.claude.json" <<'DATA'
{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["github-mcp"]
    }
  }
}
DATA

cat >"$PROJECT_A/.mcp.json" <<'DATA'
{
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

"$BIN" --home "$FAKE_HOME" init --apply >/tmp/maa-git-init.out
cat >"$FAKE_HOME/.my-agent-assets/config.yaml" <<DATA
asset_center: $FAKE_HOME/.my-agent-assets
git_repo: $REMOTE_URL
scan_roots:
  - $FAKE_WORKSPACE
max_depth: 5
runtime:
  provider: claude
DATA

"$BIN" --home "$FAKE_HOME" scan --apply >/tmp/maa-git-scan.out

cd "$FAKE_HOME/.my-agent-assets"
git config user.name "My Agent Assets E2E"
git config user.email "my-agent-assets-e2e@example.invalid"
git branch -M "$BRANCH"
git remote remove origin >/dev/null 2>&1 || true
git remote add origin "$REMOTE_URL"
git add .
git commit -m "test: sync fake asset center" >/tmp/maa-git-commit.out
git push -u origin "$BRANCH" >/tmp/maa-git-initial-push.out

printf 'remote sync check\n' >sync-check.txt
git add sync-check.txt
git commit -m "test: verify maa sync push" >/tmp/maa-git-commit-2.out
"$BIN" --home "$FAKE_HOME" sync push >/tmp/maa-git-sync-push.out
"$BIN" --home "$FAKE_HOME" sync pull >/tmp/maa-git-sync-pull.out

echo "E2E fake git sync passed: $TMP_ROOT"
