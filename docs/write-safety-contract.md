# Write Safety Contract

This document defines the safety boundary for future write/apply commands. It is a contract milestone only: no apply command is registered or implemented in this step.

## Scope

Future write commands may cover:

- Import apply
- Mount apply
- Restore apply
- Settings save

They must not be implemented until this contract is satisfied by code, tests, and fake HOME end-to-end verification.

## Required Apply Contract

Every apply command must receive a single `input` object containing:

- `previewId`
- `mode`
- command-specific identifiers
- explicit backup preference

Apply commands must not accept arbitrary frontend paths as sufficient authority to write. The backend must rebuild or validate the plan from trusted state and compare it with the preview identity before writing.

`mode` has two wire values:

- `planOnly`
- `apply`

`planOnly` must produce an `ApplyResult` without writing files.

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

No Tauri command is registered for these DTOs yet.

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

## Forbidden In This Milestone

This milestone does not add:

- `import_apply`
- `mount_apply`
- `restore_apply`
- `settings_save`
- Git pull
- Git push
- file writes
- symlink creation
- MCP JSON writes
- backup creation
- restore execution

## Next Implementation Gate

Before registering any apply command, add tests proving:

- fake HOME isolation
- no writes in `planOnly`
- backup-before-write
- interrupted write reporting
- restore from backup manifest
- no direct real HOME access
