# Read-only Real Data Integration v1

This milestone connects the frozen desktop GUI contracts to safe read-only Tauri commands. It does not switch any page from static data to real data yet.

## Implemented Commands

- `settings_load`
- `git_status`
- `list_assets`
- `list_projects`
- `scan_assets`

The following commands remain contract-only for a later preview/write phase:

- `preview_import`
- `preview_mount`
- `preview_conflicts`
- `preview_restore`
- `settings_save`

## HOME Resolution

Rust command wrappers resolve HOME in this order:

1. `MY_AGENT_ASSETS_HOME`
2. `HOME`
3. `USERPROFILE`

Internal read functions accept an explicit `Path`, so tests can use temporary fake HOME directories without touching real Claude or asset-center data.

## Data Sources

`settings_load` returns default settings only. It does not read, create, or save a config file.

`list_assets` reads:

- `~/.my-agent-assets/assets/skills/`
- `~/.my-agent-assets/assets/commands/`
- `~/.my-agent-assets/assets/mcps/`

Skills support both `<name>/` directories and root `.md` files. Commands read `.md` files. MCP assets read `.json` files, with invalid JSON marked as `invalid`.

`list_projects` scans only one level under:

- `~/workspace`
- `~/code`

A directory is treated as a project when it contains `package.json`, `Cargo.toml`, `.git/`, or `.claude/`.

`scan_assets` is read-only and imports nothing. It scans:

- User scope: `~/.claude/skills/*.md`, `~/.claude/commands/*.md`, and `~/.claude.json`
- Project/custom scope: `<runtime-root>/.claude/skills/*.md`, `<runtime-root>/.claude/commands/*.md`, and `<runtime-root>/.mcp.json`

MCP discovery reads the JSON config file and parses the top-level `mcpServers` object. `.mcpServers` is not treated as a path.

## Git Safety

`git_status` only reads `~/.my-agent-assets` repository state. It may run these Git commands using `std::process::Command` argument arrays:

- `git rev-parse --is-inside-work-tree`
- `git branch --show-current`
- `git status --porcelain`
- `git rev-parse --abbrev-ref --symbolic-full-name @{upstream}`
- `git rev-list --left-right --count HEAD...@{upstream}`

It never runs:

- `git fetch`
- `git pull`
- `git push`
- `git init`
- `git add`
- `git commit`

If the asset center directory is missing, is not a Git repository, has no upstream, or Git is unavailable, the command returns a safe `GitStatus` with `isRepository` and `statusMessage` explaining the state.

## Frontend Boundary

`apps/desktop/src/app/data-api.ts` provides typed wrappers for the five read-only commands. In a non-Tauri runtime, or when an invoke call fails, wrappers return safe fallback data:

- Empty lists for assets and projects
- Default settings
- Safe non-repository Git status
- Empty scan result with a warning

No page currently consumes these wrappers. The V1 static GUI remains static/mock until a future explicit UI integration phase.

## Non-goals

This milestone does not:

- Write files
- Create `~/.my-agent-assets`
- Save settings
- Import assets
- Mount or unmount assets
- Restore backups
- Run Git pull, push, fetch, init, add, or commit
- Change page layouts
- Enable visual-only action buttons
