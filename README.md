# My Agent Assets

`my-agent-assets` is a local-first Claude asset manager. The V1 command line tool is `maa`.

![Latest Release](https://img.shields.io/github/v/release/fightingPig/my-agent-assets)

## Download Desktop App

Latest desktop builds are published through GitHub Releases.

- macOS: https://github.com/fightingPig/my-agent-assets/releases/latest
- Windows: https://github.com/fightingPig/my-agent-assets/releases/latest
- Linux: https://github.com/fightingPig/my-agent-assets/releases/latest

## Quick Commands

```bash
cargo test
cargo run -p my-agent-assets-cli --bin maa -- --help
./scripts/e2e_fake_runtime.sh
```

## macOS Desktop Preview

The first GUI milestone is an installable, home-page-only Tauri preview. It uses typed Tauri commands for local Claude data and read-only Codex Skill/MCP discovery. Tests and Visual QA may enable explicit demo fixtures, but production pages show only real data, empty states, or read errors.

```bash
cd apps/desktop
npm install
npm run typecheck
npm test
npm run build
```

Build artifacts are written to `target/release/bundle/`.

Layout reference screenshots are stored in `docs/screenshots/` for the default and minimum supported window sizes.
