#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REMOTE_URL="${MAA_REMOTE_URL:-}"
TMP_ROOT="$(mktemp -d /tmp/my-agent-assets-git-e2e-XXXXXX)"
FAKE_HOME="$TMP_ROOT/fake-home"
BRANCH="maa-e2e-$(date +%s)-$$"
BIN="$ROOT_DIR/target/debug/maa"

cleanup() {
  if [[ -n "$REMOTE_URL" && -d "$FAKE_HOME/.my-agent-assets/.git" ]]; then
    git -C "$FAKE_HOME/.my-agent-assets" push origin --delete "$BRANCH" >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP_ROOT"
}
trap cleanup EXIT INT TERM

[[ -n "$REMOTE_URL" ]] || {
  echo "MAA_REMOTE_URL must point to a GitHub Private repository" >&2
  exit 1
}
[[ "$TMP_ROOT" == /tmp/my-agent-assets-git-e2e-* ]] || {
  echo "unsafe temp root: $TMP_ROOT" >&2
  exit 1
}
command -v gh >/dev/null || {
  echo "gh is required for live GitHub Private visibility verification" >&2
  exit 1
}

mkdir -p "$FAKE_HOME/.claude/skills/review"
printf '# Review Skill\n' >"$FAKE_HOME/.claude/skills/review/SKILL.md"
printf '{"mcpServers":{}}\n' >"$FAKE_HOME/.claude.json"

cd "$ROOT_DIR"
cargo build -p my-agent-assets-cli --bin maa >/dev/null
"$BIN" --home "$FAKE_HOME" init --apply >/dev/null

"$BIN" --home "$FAKE_HOME" scan --scope user >"$TMP_ROOT/scan.json"
SOURCE_ID="$(jq -r '.sources[] | select(.assetName == "review") | .sourceId' "$TMP_ROOT/scan.json")"
"$BIN" --home "$FAKE_HOME" import "$SOURCE_ID" --scope user --apply >/dev/null

REPOSITORY="$FAKE_HOME/.my-agent-assets"
git -C "$REPOSITORY" remote add origin "$REMOTE_URL"
git -C "$REPOSITORY" branch -M "$BRANCH"

"$BIN" --home "$FAKE_HOME" sync push >"$TMP_ROOT/push-preview.json"
sed -i.bak '/^Run the same command/,$d' "$TMP_ROOT/push-preview.json"
jq -e '.repositoryVisibility == "private" and .canApply == true' \
  "$TMP_ROOT/push-preview.json" >/dev/null
"$BIN" --home "$FAKE_HOME" sync push --apply >"$TMP_ROOT/push-result.json"
jq -e '.pushed == true and .committed == true' "$TMP_ROOT/push-result.json" >/dev/null

git clone --branch "$BRANCH" --single-branch "$REMOTE_URL" "$TMP_ROOT/device-b" >/dev/null
test -f "$TMP_ROOT/device-b/assets/skills/review/SKILL.md"
test -f "$TMP_ROOT/device-b/assets.yaml"
test ! -e "$TMP_ROOT/device-b/backups/local"
test ! -e "$TMP_ROOT/device-b/mounts.yaml"

echo "E2E private Git sync passed: $TMP_ROOT"
