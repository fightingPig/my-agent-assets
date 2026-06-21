# My Agent Assets

`my-agent-assets` is a local-first Claude asset manager. The V1 command line
tool is `maa`.

V1 manages Claude assets from fake or explicit runtime roots during tests:

- Skills from `.claude/skills/<name>/`
- Commands from `.claude/commands/<name>.md`
- MCP servers from Claude MCP JSON configuration sources

The default asset center is `~/.my-agent-assets`, but tests and examples should
use `--home <fake-home>` or `MY_AGENT_ASSETS_HOME` to avoid touching real data.

## Quick Commands

```bash
cargo test
cargo run -p my-agent-assets-cli --bin maa -- --help
./scripts/e2e_fake_runtime.sh
```

## macOS Desktop Preview

The first GUI milestone is an installable, home-page-only Tauri preview. It
uses mock content and does not read Claude runtime or asset-center data.

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
