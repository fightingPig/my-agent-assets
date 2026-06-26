# Write Safety Contract

This document defines the safety boundary for write/apply commands. `import_apply`, `mount_apply`, `restore_apply`, `sync_apply`, and `settings_save` are implemented.

## Scope

Implemented write commands cover:

- Import apply
- Mount apply
- Restore apply
- Sync apply
- Settings save

They must not be implemented until this contract is satisfied by code, tests, and fake HOME end-to-end verification.

## Required Apply Contract

Every apply command must receive a single `input` object containing:

- `previewId`
- `mode`
- command-specific identifiers
- explicit backup preference

Apply commands must not accept arbitrary frontend paths as sufficient authority to write. The backend must rebuild or validate the plan from trusted state and compare it with the preview identity before writing.

Preview commands return deterministic `previewId` values for import, mount, restore, and sync previews. Apply commands recompute the expected `previewId` from their input and fail before any write when the supplied ID does not match.

`mode` has two wire values:

- `planOnly`
- `apply`

`planOnly` must produce an `ApplyResult` without writing files.

The desktop UI must not call `mode: "apply"` directly from a primary action. It must first produce a successful preview and plan-only apply result, then require a local typed confirmation of `APPLY`. The backend still owns the final safety check by validating `previewId`.

## Required DTOs

The frontend and Rust contract layers define:

- `ApplyMode`
- `ApplyStepStatus`
- `ApplyStepResult`
- `BackupManifestSummary`
- `ApplyResult`
- `ImportApplyInput`
- `MountApplyInput`
- `RestoreApplyInput`
- `SyncApplyInput`

`import_apply`, `mount_apply`, `restore_apply`, `sync_apply`, and `settings_save` are registered and use these DTOs.

## Backup Rule

All destructive or replacing writes must create a backup first unless a future command is explicitly proven non-destructive.

Backup manifests must record:

- backup id
- manifest path
- runtime root
- affected paths
- created time
- size and entry count

Restore must be possible from the manifest without consulting UI state.

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

`import_apply` does not delete or modify runtime Claude files. MCP import extracts a server JSON object into the asset center and leaves the source config unchanged.

`mount_apply` currently supports fake-HOME-tested Skill and Command symlink mounts plus MCP runtime config compilation:

- Source assets are resolved from `~/.my-agent-assets/assets/skills` or `~/.my-agent-assets/assets/commands`
- MCP source assets are resolved from `~/.my-agent-assets/assets/mcps/<name>.json`
- Mount or compile targets must resolve under the backend's HOME
- Existing mount targets are backed up before replacement when `backupBeforeApply` is true
- MCP compile merges into the target JSON file's top-level `mcpServers.<name>` while preserving other top-level fields and other MCP servers
- `planOnly` mode creates no symlink, writes no JSON, and creates no backup

`restore_apply` currently supports fake-HOME-tested restore from backup manifests:

- Manifests are loaded from `~/.my-agent-assets/backups/<backupId>/manifest.json`
- File, directory, and symlink backup entries can be restored
- Restore targets must stay under the backend's HOME
- Backup entry paths must stay under the selected backup directory
- Current runtime state is backed up before replacement when `backupBeforeRestore` is true
- `planOnly` mode reads the manifest but restores no files and creates no backup

`settings_save` currently supports fake-HOME-tested settings persistence:

- Settings are written to `~/.my-agent-assets/config.json`
- `settings_load` returns defaults when no config exists and reads the saved config when present
- Settings writes do not touch Claude runtime files
- Empty path fields are normalized to defaults
- Numeric settings are clamped to supported ranges

`sync_apply` currently supports fake-HOME-tested Git sync execution:

- Targets only `~/.my-agent-assets`
- Recomputes `previewId` from current Git status before running a command
- Rejects dirty worktrees, conflicts, missing upstreams, and non-repositories
- `planOnly` mode runs no Git commands
- Pull executes `git pull --ff-only`
- Push executes `git push`
- Git commands are executed through `std::process::Command` argument arrays, not shell strings

## Still Forbidden

These operations are still not implemented:

- Conflict apply

## Next Implementation Gate

Before registering additional apply commands, add tests proving:

- fake HOME isolation
- no writes in `planOnly`
- backup-before-write
- interrupted write reporting
- restore from backup manifest
- no direct real HOME access
