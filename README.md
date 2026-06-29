# My Agent Assets

`my-agent-assets` is a local-first Claude Code and Codex asset manager. The V1
command line tool is `maa`.

The canonical asset center stores one copy of each compatible asset:

- Skills imported from Claude Code, Codex, or approved custom directories
- Commands imported from Claude-compatible Markdown sources
- MCP servers imported from Claude JSON or Codex TOML

The default asset center is `~/.my-agent-assets`, but tests and examples should
use `--home <fake-home>` or `MY_AGENT_ASSETS_HOME` to avoid touching real data.

## Quick Commands

```bash
cargo test
cargo run -p my-agent-assets-cli --bin maa -- --help
./scripts/e2e_fake_runtime.sh
```

CLI semantics are explicit:

```text
scan    = discover runtime sources without writing
import  = copy a selected source into the canonical asset center
mount   = materialize a canonical asset at a registered target
adopt   = import and mount back to the selected source
```

Write commands print a preview by default and require `--apply`. Mount and
unmount accept registered target IDs, never arbitrary runtime paths. Automatic
historical Restore is intentionally not provided.

`maa sync push` stages only the canonical sync whitelist and is blocked unless
the local `gh` authentication reports the configured GitHub repository as
Private. `maa sync pull` requires a clean worktree and uses fast-forward only.

## macOS Desktop Preview

The desktop app uses typed Tauri commands backed by the same Rust core as the
CLI. Tests and Visual QA may enable explicit demo fixtures, but production
pages show only real data, empty states, or read errors.

```bash
cd apps/desktop
npm install
npm run typecheck
npm test
npm run build
```

Build artifacts are written to `target/release/bundle/`. The local preview can
be installed at `~/Applications/My Agent Assets.app`.

Layout reference screenshots are stored in `docs/screenshots/` for the default
and minimum supported window sizes.
