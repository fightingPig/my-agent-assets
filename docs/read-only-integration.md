> **Historical milestone — superseded for final workflow scope**
>
> This document records the read-only and preview-only integration milestone. It is not the final product contract.
>
> The final model uses **one canonical asset center, multiple runtime sources, and multiple compatible mount targets**. Codex-compatible Skills and MCP servers will support discovery, import into the shared canonical asset center, and compatible user/project mounting. Codex Commands, Command targets, and Codex OAuth token management remain prohibited. Final confirmation, backup, Restore, Git, and apply semantics are defined by `docs/final-product-model.md` and `my_agent_assets_final_goal.md`.
>
> The prototype `scan_assets`, `list_codex_skills`, and
> `list_codex_mcp_servers` commands have since been removed. Production source
> discovery uses shared-core `discover_runtime_sources`.
>
> The implementation details below are retained as historical evidence and may describe behavior that later work must replace.

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

Historical Restore preview is not registered. Backup History exposes manifests,
affected paths, and manual restore guidance only.

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

The historical `StaticActionButton` placeholder has been removed. Production
pages now expose only real read/preview/apply controls; unimplemented optional
actions are omitted instead of rendered as permanently disabled buttons.
Typed `APPLY` is not required.

The preview workflow pages now consume preview-only data through the wrapper layer:

- Scan Import: shared runtime discovery, atomic Batch Import preview/apply, and
  backend-composed Adopt.
- Mount Manager: Target Registry enumeration and targetId-only Mount
  preview/apply.
- Settings: Target Registry registration/removal preview and apply. Rust derives
  the final runtime path and adapter from `{ id, kind, location }`.
- Conflict Resolver: canonical Batch Import preview/apply with exact
  existing/incoming content and explicit skip/rename/overwrite.
- Backup History: read-only portable/local manifests and manual restore guide;
  no historical Restore command.
- Sync: shared Git preview/apply with whitelist staging, fast-forward Pull, and
  live GitHub Private verification before every Push.

Preview data controls plan text, warnings, affected paths, conflicts, and
whether the confirmation button is enabled. Apply revalidates the preview in
the Rust backend.

Asset Detail and Project Detail consume the selected list context rather than
replacing it with a fixed entity. They enumerate authorized targets and use
`canonical_mount_preview` / `canonical_mount_apply` with `{ assetId, targetId }`.
React does not derive runtime paths. After a successful apply, Asset Detail
reloads `list_assets` and Project Detail reloads `list_projects`, so derived
mount targets and project counts reflect the backend state.

## Non-goals

The read-only UI milestone still does not:

- Unmount assets
- Automatic historical Restore
- Change page layouts
