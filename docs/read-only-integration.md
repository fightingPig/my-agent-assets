# Read-only And Preview-only Integration

This milestone connects the frozen desktop GUI contracts to safe read-only and preview-only Tauri commands, then wires selected static pages to those commands.

## Implemented Commands

- `settings_load`
- `settings_save`
- `git_status`
- `list_assets`
- `list_projects`
- `list_backups`
- `scan_assets`

`settings_save` was implemented after the original read-only milestone. The Settings page now exposes the first controlled write UI action, limited to local settings persistence.

## Implemented Preview-only Commands

- `preview_import`
- `preview_mount`
- `preview_conflicts`
- `preview_restore`

Preview-only commands synthesize deterministic DTOs from their input or read existing manifests for preview. They do not write files, create directories, create symlinks, modify MCP JSON, restore backups, or perform apply operations.

## HOME Resolution

Rust command wrappers resolve HOME in this order:

1. `MY_AGENT_ASSETS_HOME`
2. `HOME`
3. `USERPROFILE`

Internal read functions accept an explicit `Path`, so tests can use temporary fake HOME directories without touching real Claude or asset-center data.

## Data Sources

`settings_load` returns default settings when no config exists. After `settings_save`, it reads `~/.my-agent-assets/config.json`.

`list_assets` reads:

- `~/.my-agent-assets/assets/skills/`
- `~/.my-agent-assets/assets/commands/`
- `~/.my-agent-assets/assets/mcps/`

Skills support both `<name>/` directories and root `.md` files. Commands read `.md` files. MCP assets read `.json` files, with invalid JSON marked as `invalid`.

`list_projects` scans only one level under:

- `~/workspace`
- `~/code`

A directory is treated as a project when it contains `package.json`, `Cargo.toml`, `.git/`, or `.claude/`.

`list_backups` reads `~/.my-agent-assets/backups/*/manifest.json` and returns manifest summaries only. It does not read backed-up file contents, create backup directories, or restore files. Missing or invalid manifests are skipped.

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

`apps/desktop/src/app/data-api.ts` provides typed wrappers for the read-only, preview, and controlled write commands. In a non-Tauri runtime, or when an invoke call fails, wrappers return safe fallback data:

- Empty lists for assets and projects
- Default settings
- Safe non-repository Git status
- Empty scan result with a warning

These pages now consume read-only data through the wrapper layer:

- Skills list: `list_assets` with `assetType: "skill"`
- Commands list: `list_assets` with `assetType: "command"`
- MCP Servers list: `list_assets` with `assetType: "mcp"`
- Projects list: `list_projects`
- Backup Restore: `list_backups`
- Sync: `git_status`
- Settings: `settings_load`
- Scan Import: `scan_assets`

Each page keeps its previous static data as an initial placeholder or fallback. If a command returns an empty result, rejects, or runs outside Tauri, the UI stays usable and clearly labels the view as static preview or fallback data.

Conflict apply remains disabled. `StaticActionButton` is still used for visual-only business actions that do not have a safe apply workflow. Settings can call `settings_save` to persist local desktop configuration only. Scan Import can call `import_apply` in `planOnly` mode to generate an import plan, Mount Manager can call `mount_apply` in `planOnly` mode to generate a mount plan, Sync can call `preview_sync` to generate Pull/Push plans, and Backup Restore can call `restore_apply` in `planOnly` mode to generate a restore plan; all plan actions avoid file writes.

Import, mount, and restore previews return deterministic `previewId` values. Their plan-only apply calls pass the preview's ID, and the backend rejects mismatched IDs before reading or writing runtime data.

The Scan Import, Mount Manager, Backup Restore, and Sync pages can execute real `apply` mode only after:

1. A preview exists.
2. A plan-only apply succeeds.
3. The user types `APPLY` in the local confirmation field.
4. The backend validates the deterministic `previewId`.

The preview workflow pages now consume preview-only data through the wrapper layer:

- Scan Import: `preview_import` after a non-empty `scan_assets` result, plus `import_apply` with `mode: "planOnly"` when generating an import plan
- Mount Manager: `preview_mount` for the selected asset and target, plus `mount_apply` with `mode: "planOnly"` when generating a mount plan
- Conflict Resolver: `preview_conflicts` for static preview asset IDs, plus local `skip` / `rename` / `overwrite` resolution preview state
- Backup Restore: `preview_restore` for the selected backup ID, plus `restore_apply` with `mode: "planOnly"` when generating a restore plan
- Sync: `preview_sync` for local Pull/Push plan generation without running Git sync commands

The UI continues to keep conflict-resolution apply buttons disabled. Preview and plan-only data affects plan text, warnings, affected paths, conflicts, and summaries; import, mount, restore, and Git sync can proceed to real apply only through the typed confirmation gate.

`preview_restore` now prefers `~/.my-agent-assets/backups/<backupId>/manifest.json` when present, returning the manifest's affected paths and backup summary. Missing or invalid manifests safely fall back to synthetic preview data with a warning.

## Non-goals

The read-only UI milestone still does not:

- Unmount assets
- Apply conflict resolutions
- Run Git fetch, init, add, or commit
- Change page layouts
- Enable conflict apply-style action buttons
- Call conflict write commands from enabled UI actions
