# Write Safety Contract

This document defines the safety boundary for write/apply commands. Canonical
Import, Adopt, Mount, Unmount, Delete, Target Registry, Git Sync, and local
settings writes are implemented. Historical automatic Restore commands have
been removed.

## Scope

Implemented write commands cover:

- Import apply
- Conflict apply
- Mount apply
- Sync apply
- Settings save

They remain supported only while this contract is satisfied by code, tests, and fake HOME end-to-end verification.

## Required Apply Contract

Every apply command must receive a single `input` object containing:

- `previewId`
- `mode`
- command-specific identifiers
- explicit backup preference

Apply commands must not accept arbitrary frontend paths as sufficient authority to write. The backend must rebuild or validate the plan from trusted state and compare it with the preview identity before writing.

Preview commands return bound `previewId` values for supported write workflows.
Apply commands recompute the expected identity from trusted state and fail
before any write when the supplied ID does not match.

`mode` has two wire values:

- `planOnly`
- `apply`

`planOnly` must produce an `ApplyResult` without writing files.

All path-bearing inputs are untrusted. Before path construction or filesystem mutation, the backend:

- validates asset IDs and target IDs as safe path components
- rejects `/`, `\`, `:`, control characters, leading/trailing whitespace, `.` and `..`
- rejects `ParentDir` components instead of normalizing them away
- requires every write target to remain below the resolved fake or real HOME
- rejects symlinked parent components that could redirect a write outside its allowed root

These checks are backend invariants and do not rely on frontend validation.

The desktop UI must first produce a successful preview and then require an
ordinary explicit button confirmation. Typed `APPLY` is not required. The
backend still owns the final safety checks through `previewId`, expiry,
stale-state validation, locks, backups, journals, and rollback.

## Required DTOs

The frontend and Rust contract layers define:

- `ApplyMode`
- `ApplyStepStatus`
- `ApplyStepResult`
- `BackupManifestSummary`
- `ApplyResult`
- `ImportApplyInput`
- `ConflictApplyInput`
- `MountApplyInput`
- `SyncApplyInput`

Production pages use shared-core canonical contracts. Legacy Desktop
`import_apply`, `conflict_apply`, and `mount_apply` transports remain only
during migration and must not receive new consumers.

## Backup Rule

All destructive or replacing writes must create a backup first unless a future command is explicitly proven non-destructive.

Backup manifests must record:

- backup id
- manifest path
- runtime root
- affected paths
- created time
- size and entry count

Backup history must expose enough information for the documented manual restore
guide. The application does not expose an arbitrary historical Restore command.

## Write Algorithm

Apply implementations must use:

```text
copy -> verify -> replace -> verify
```

For symlink operations:

1. inspect target
2. backup existing target
3. create replacement in a temporary path when possible
4. atomically replace or carefully swap
5. verify final symlink target

For MCP JSON compilation:

1. parse existing JSON
2. backup original file
3. produce normalized JSON in memory
4. write temporary file
5. verify parse result
6. replace original
7. verify final JSON

For restore:

1. validate backup manifest
2. backup current state
3. restore each path
4. verify restored paths
5. stop on first unrecoverable failure and report partial state

Backup manifests are treated as untrusted input. Restore verifies the requested backup ID, manifest
ID, `runtimeRoot`, every `originalPath`, every `backupPath`, entry kind, and stored symlink target
before plan-only reporting or apply. Backup content paths must be regular descendants of the
selected backup directory's fixed `files/` subtree and may not traverse symlinks. Restore targets
cannot overwrite the selected backup directory. Restored symlinks must resolve below HOME.

## Fake HOME Requirement

All apply tests must run under fake HOME or explicit temporary runtime roots.

Tests must not read or write:

- real `~/.claude`
- real `~/.claude.json`
- real `~/.my-agent-assets`
- real project `.claude`
- real project `.mcp.json`

## Failure Behavior

`ApplyResult` must report:

- `ok`
- `mode`
- `previewId`
- optional backup summary
- step results
- warnings
- errors

Each step result must include:

- `stepId`
- `kind`
- `label`
- `status`
- `message`
- `affectedPaths`

If any step fails, later write steps must not continue unless explicitly marked as safe cleanup.

## Current Implementation

`import_apply` currently supports fake-HOME-tested imports into the asset center:

- Skills from runtime `.claude/skills/<name>/` directories or `.claude/skills/<name>.md`
- Commands from runtime `.claude/commands/<name>.md`
- MCP servers by reading the top-level `mcpServers.<name>` object from `.claude.json` or `.mcp.json`
- Destination replacement with backup when `backupBeforeApply` is true
- `planOnly` mode with no filesystem writes
- Unsafe asset IDs, scope traversal, symlinked runtime roots, and symlinks nested inside imported
  directories are rejected before asset-center or backup creation

`import_apply` does not delete or modify runtime Claude files. MCP import extracts a server JSON object into the asset center and leaves the source config unchanged.

`mount_apply` currently supports fake-HOME-tested Skill and Command symlink mounts plus MCP runtime config compilation:

- Source assets are resolved from `~/.my-agent-assets/assets/skills` or `~/.my-agent-assets/assets/commands`
- MCP source assets are resolved from `~/.my-agent-assets/assets/mcps/<name>.json`
- Mount or compile targets must resolve under the backend's HOME
- Mount sources must resolve inside the asset center without symlink traversal
- ParentDir targets and symlinked target parents are rejected before creating directories or backups
- Existing mount targets are backed up before replacement when `backupBeforeApply` is true
- MCP compile merges into the target JSON file's top-level `mcpServers.<name>` while preserving other top-level fields and other MCP servers
- `planOnly` mode creates no symlink, writes no JSON, and creates no backup

`settings_save` currently supports fake-HOME-tested settings persistence:

- Settings are written to `~/.my-agent-assets/config.json`
- `settings_load` returns defaults when no config exists and reads the saved config when present
- Write failures are returned through Tauri as command errors; the frontend must not display success unless save and reload both complete
- `assetCenterPath` is fixed to `~/.my-agent-assets` in V1 and is not an editable relocation setting
- Settings writes do not touch Claude runtime files
- The fixed settings destination is guarded against symlinked parent directories
- Empty path fields are normalized to defaults
- Numeric settings are clamped to supported ranges

`sync_apply` currently supports fake-HOME-tested Git sync execution:

- Targets only `~/.my-agent-assets`
- Rejects a symlinked asset-center repository before running Git
- Recomputes `previewId` from current Git status before running a command
- Rejects dirty worktrees, conflicts, missing upstreams, and non-repositories
- `planOnly` mode runs no Git commands
- Pull executes `git pull --ff-only`
- Push executes `git push`
- Git commands are executed through `std::process::Command` argument arrays, not shell strings

`conflict_apply` currently supports fake-HOME-tested per-asset conflict decisions:

- Uses the deterministic import preview ID for the same scope, asset IDs, and decisions
- Direct `import_apply` rejects differing existing content unless an explicit conflict resolution is supplied
- Requires exactly one unambiguous `skip`, `rename`, or `overwrite` decision per asset
- `skip` writes nothing
- `rename` validates one safe new name and rejects an already-existing target
- `overwrite` uses the existing import backup-before-replacement path
- MCP preview shows the exact existing asset JSON and incoming top-level `mcpServers.<name>` JSON
- `planOnly` validates paths and decisions without writing

## Next Implementation Gate

Before registering additional apply commands, add tests proving:

- fake HOME isolation
- no writes in `planOnly`
- backup-before-write
- interrupted write reporting
- restore from backup manifest
- no direct real HOME access

## Safety Hardening Verification

The Rust suite includes:

- strict unsafe asset ID and backup ID rejection tests
- ParentDir and outside-root path tests
- import, mount, settings, sync, manifest, backup-content, and restored-symlink escape tests
- deterministic preview ID mismatch tests that assert no write occurs
- full-tree snapshots proving import, mount, MCP compile, and restore `planOnly` modes create or
  change no files, directories, symlinks, or backups
- a single fake-HOME end-to-end workflow covering import, mount, restore, settings save/load, and
  sync `planOnly`, with a local bare remote proving no Git mutation

All test roots are created under the operating system temporary directory and passed explicitly to
backend functions. Tests do not resolve or touch the developer's real HOME.
