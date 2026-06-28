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
- `list_codex_skills`
- `list_codex_mcp_servers`

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

`settings_load` returns default settings when no config exists. After `settings_save`, it reads `~/.my-agent-assets/config.json`. In V1, `assetCenterPath` is informational and normalized to the fixed `~/.my-agent-assets` location; the Settings UI exposes it as read-only until relocation is implemented consistently across all commands.

`list_assets` reads:

- `~/.my-agent-assets/assets/skills/`
- `~/.my-agent-assets/assets/commands/`
- `~/.my-agent-assets/assets/mcps/`

Skills support both `<name>/` directories and root `.md` files. Commands read `.md` files. MCP assets read `.json` files, with invalid JSON marked as `invalid`.

Runtime scans use the same Skill forms: `.claude/skills/<name>/SKILL.md` and `.claude/skills/<name>.md`. Scan results compare discovered assets with asset-center content, mark differing same-ID assets as `conflict`, and report `conflictCount`.

Asset summaries also derive current local usage:

- Skill and Command mounts are recognized when user/project runtime symlinks resolve to the asset-center source.
- MCP usage is recognized when user `.claude.json` or project `.mcp.json` contains the server name in top-level `mcpServers`.

`list_projects` scans only one level under:

- `~/workspace`
- `~/code`

A directory is treated as a project when it contains `package.json`, `Cargo.toml`, `.git/`, or `.claude/`.
Project summaries count project runtime Skills, Commands, and MCP servers and return their names as current mounts.

`list_backups` reads `~/.my-agent-assets/backups/*/manifest.json` and returns manifest summaries only. It does not read backed-up file contents, create backup directories, or restore files. Missing or invalid manifests are skipped.

`scan_assets` is read-only and imports nothing. It scans:

- User scope: `~/.claude/skills/*.md`, `~/.claude/commands/*.md`, and `~/.claude.json`
- Project/custom scope: `<runtime-root>/.claude/skills/*.md`, `<runtime-root>/.claude/commands/*.md`, and `<runtime-root>/.mcp.json`

MCP discovery reads the JSON config file and parses the top-level `mcpServers` object. `.mcpServers` is not treated as a path.

Codex discovery is intentionally separate from the Claude asset-center path:

- `list_codex_skills` reads valid `SKILL.md` directories under `~/.agents/skills`, project/repository ancestor `.agents/skills` roots, and `/etc/codex/skills` when readable.
- `list_codex_mcp_servers` reads `[mcp_servers.<name>]` tables from `~/.codex/config.toml` and the selected/current project `.codex/config.toml`.

Both commands are read-only. They do not create directories, write TOML, import assets, mount assets, or manage authentication tokens.

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

`apps/desktop/src/app/data-api.ts` provides typed wrappers for the read-only, preview, and controlled write commands. Outside Tauri, read wrappers return safe empty/default data for browser previews. Inside Tauri, read-command failures are surfaced to the page so the UI can show a non-blocking error instead of masking the failure with sample data:

- Empty lists for assets and projects
- Default settings
- Safe non-repository Git status
- Empty scan result with a warning
- Empty Codex Skill/MCP results

These pages now consume read-only data through the wrapper layer:

- Skills list: `list_assets` with `assetType: "skill"`
- Commands list: `list_assets` with `assetType: "command"`
- MCP Servers list: `list_assets` with `assetType: "mcp"`
- Projects list: `list_projects`
- Backup Restore: `list_backups`
- Sync: `git_status`
- Settings: `settings_load`
- Scan Import: `scan_assets`
- Codex Skills: `list_codex_skills`
- Codex MCP Servers: `list_codex_mcp_servers`

Production pages do not fall back to sample rows. Empty commands produce explicit empty states, and rejected Tauri reads produce explicit error states. Static fixtures remain available only through explicit `demoMode` for tests and Visual QA.

The Provider switch selects `Claude Code` or `Codex` within the existing Asset Center. Codex exposes only Skills and MCP Servers; Commands are hidden. Codex data is never passed into Claude import, mount, conflict apply, or adoption workflows.

`StaticActionButton` is still used for visual-only business actions that do not have a safe apply workflow. Settings can call `settings_save` to persist local desktop configuration only. Scan Import can call `import_apply` in `planOnly` mode to generate an import plan, Mount Manager can call `mount_apply` in `planOnly` mode to generate a mount plan, Sync can call `preview_sync` to generate Pull/Push plans, Backup Restore can call `restore_apply` in `planOnly` mode to generate a restore plan, and Conflict Resolver can call `conflict_apply` in `planOnly` mode; all plan actions avoid file writes.

Import, mount, and restore previews return deterministic `previewId` values. Their plan-only apply calls pass the preview's ID, and the backend rejects mismatched IDs before reading or writing runtime data.

The Scan Import, Mount Manager, Backup Restore, and Sync pages can execute real `apply` mode only after:

1. A preview exists.
2. A plan-only apply succeeds.
3. The user types `APPLY` in the local confirmation field.
4. The backend validates the deterministic `previewId`.

The preview workflow pages now consume preview-only data through the wrapper layer:

- Scan Import: `preview_import` after a non-empty `scan_assets` result, plus `import_apply` with `mode: "planOnly"` when generating an import plan
- Mount Manager: `preview_mount` for the selected asset and target, plus `mount_apply` with `mode: "planOnly"` when generating a mount plan
- Conflict Resolver: `preview_conflicts` for exact existing/incoming content, `preview_import` for a decision-bound preview ID, and `conflict_apply` behind plan-only plus typed confirmation
- Backup Restore: `preview_restore` for the selected backup ID, plus `restore_apply` with `mode: "planOnly"` when generating a restore plan
- Sync: `preview_sync` for local Pull/Push plan generation without running Git sync commands

Preview and plan-only data affects plan text, warnings, affected paths, conflicts, and summaries; import, conflict resolution, mount, restore, and Git sync can proceed to real apply only through the typed confirmation gate.

Asset Detail and Project Detail consume the selected list context rather than replacing it with a fixed entity. They use `preview_mount` and `mount_apply` for their existing mount workflow. After a successful apply, Asset Detail reloads `list_assets` and Project Detail reloads `list_projects`, so derived mount targets and project counts reflect the backend state.

`preview_restore` now prefers `~/.my-agent-assets/backups/<backupId>/manifest.json` when present, returning the manifest's affected paths and backup summary. Missing or invalid manifests safely fall back to synthetic preview data with a warning.

## Non-goals

The read-only UI milestone still does not:

- Unmount assets
- Run Git fetch, init, add, or commit
- Change page layouts
