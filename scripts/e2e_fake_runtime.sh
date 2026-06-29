#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_ROOT="$(mktemp -d /tmp/my-agent-assets-e2e-XXXXXX)"
FAKE_HOME="$TMP_ROOT/fake-home"
PROJECT_A="$TMP_ROOT/workspace/project-a"

cleanup() {
  rm -rf "$TMP_ROOT"
}
trap cleanup EXIT INT TERM

fail() {
  echo "$*" >&2
  exit 1
}

[[ "$TMP_ROOT" == /tmp/my-agent-assets-e2e-* ]] || fail "unsafe temp root: $TMP_ROOT"
[[ "$TMP_ROOT" != "$HOME" && "$TMP_ROOT" != "/" ]] || fail "refusing to use real HOME"

mkdir -p \
  "$FAKE_HOME/.claude/skills/review" \
  "$FAKE_HOME/.claude/commands" \
  "$FAKE_HOME/.agents/skills/codex-review" \
  "$FAKE_HOME/.codex" \
  "$PROJECT_A"

printf '# Review Skill\n' >"$FAKE_HOME/.claude/skills/review/SKILL.md"
printf '# Commit Command\n' >"$FAKE_HOME/.claude/commands/commit.md"
printf '# Codex Review Skill\n' >"$FAKE_HOME/.agents/skills/codex-review/SKILL.md"
printf '{"mcpServers":{"postgres":{"command":"postgres-mcp","args":["--read-only"]}}}\n' \
  >"$FAKE_HOME/.claude.json"
printf '[mcp_servers.filesystem]\ncommand = "npx"\nargs = ["-y", "filesystem-mcp"]\n' \
  >"$FAKE_HOME/.codex/config.toml"

cd "$ROOT_DIR"
cargo build -p my-agent-assets-cli --bin maa >/dev/null
BIN="$ROOT_DIR/target/debug/maa"

"$BIN" --home "$FAKE_HOME" init --apply >/tmp/maa-init.out
test -d "$FAKE_HOME/.my-agent-assets/.git"

"$BIN" --home "$FAKE_HOME" scan --scope user >"$TMP_ROOT/scan.json"
test -z "$(find "$FAKE_HOME/.my-agent-assets/assets" -type f -print -quit)"

source_id() {
  jq -r --arg provider "$1" --arg kind "$2" --arg name "$3" \
    '.sources[] | select(.provider == $provider and .assetKind == $kind and .assetName == $name) | .sourceId' \
    "$TMP_ROOT/scan.json"
}

REVIEW_SOURCE="$(source_id claude_code skill review)"
COMMAND_SOURCE="$(source_id claude_code command commit)"
CODEX_SOURCE="$(source_id codex skill codex-review)"
CLAUDE_MCP_SOURCE="$(source_id claude_code mcp postgres)"
CODEX_MCP_SOURCE="$(source_id codex mcp filesystem)"

for source in \
  "$REVIEW_SOURCE" \
  "$COMMAND_SOURCE" \
  "$CODEX_SOURCE" \
  "$CLAUDE_MCP_SOURCE" \
  "$CODEX_MCP_SOURCE"
do
  test -n "$source"
  "$BIN" --home "$FAKE_HOME" import "$source" --scope user --apply >/dev/null
done

test -f "$FAKE_HOME/.my-agent-assets/assets/skills/review/SKILL.md"
test -f "$FAKE_HOME/.my-agent-assets/assets/skills/codex-review/SKILL.md"
test -f "$FAKE_HOME/.my-agent-assets/assets/commands/commit.md"
test -f "$FAKE_HOME/.my-agent-assets/assets/mcps/postgres.json"
test -f "$FAKE_HOME/.my-agent-assets/assets/mcps/filesystem.json"

"$BIN" --home "$FAKE_HOME" target add claude-project-skills project-a-claude-skills \
  --project "$PROJECT_A" --apply >/dev/null
"$BIN" --home "$FAKE_HOME" target add codex-project-skills project-a-codex-skills \
  --project "$PROJECT_A" --apply >/dev/null

"$BIN" --home "$FAKE_HOME" mount skill:review \
  --target project-a-claude-skills --apply >/dev/null
"$BIN" --home "$FAKE_HOME" mount skill:review \
  --target project-a-codex-skills --apply >/dev/null

test -L "$PROJECT_A/.claude/skills/review"
test -L "$PROJECT_A/.agents/skills/review"

if "$BIN" --home "$FAKE_HOME" mount command:commit \
  --target project-a-codex-skills --apply >/tmp/maa-invalid-command.out 2>&1
then
  fail "Command to Codex unexpectedly succeeded"
fi
grep -q Codex /tmp/maa-invalid-command.out

if "$BIN" --home "$FAKE_HOME" remove skill:review --apply >/tmp/maa-bound-delete.out 2>&1
then
  fail "bound asset deletion unexpectedly succeeded"
fi
grep -q binding /tmp/maa-bound-delete.out

"$BIN" --home "$FAKE_HOME" remove skill:review --unmount-all --apply >/dev/null
test ! -e "$FAKE_HOME/.my-agent-assets/assets/skills/review"
test ! -e "$PROJECT_A/.claude/skills/review"
test ! -e "$PROJECT_A/.agents/skills/review"

"$BIN" --home "$FAKE_HOME" list >"$TMP_ROOT/list.json"
"$BIN" --home "$FAKE_HOME" status >"$TMP_ROOT/status.json"
"$BIN" --home "$FAKE_HOME" doctor >"$TMP_ROOT/doctor.txt"

"$BIN" --home "$FAKE_HOME" sync push >"$TMP_ROOT/sync-preview.json"
sed -i.bak '/^Run the same command/,$d' "$TMP_ROOT/sync-preview.json"
jq -e '.direction == "push" and .canApply == false' "$TMP_ROOT/sync-preview.json" >/dev/null
if "$BIN" --home "$FAKE_HOME" restore backup-1 --apply >/tmp/maa-restore.out 2>&1; then
  fail "automatic historical Restore unexpectedly succeeded"
fi

echo "E2E fake runtime passed: $TMP_ROOT"
