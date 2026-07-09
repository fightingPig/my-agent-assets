# Preview-only Workflows v1

> **Superseded transport:** The Desktop-only `preview_import`,
> `preview_mount`, and `preview_conflicts` commands below have been removed.
> Production uses shared-core `canonical_import_preview`,
> `canonical_batch_import_preview`, `canonical_mount_preview`,
> `preview_adopt`, and `preview_sync`.

This milestone adds registered Tauri commands for safe workflow previews:

- `preview_import`
- `preview_mount`
- `preview_conflicts`

These commands are deterministic and input-driven. They return plan, warning, conflict, target, and impact DTOs only.

Import and mount preview DTOs include a bound `previewId`. The related apply
commands reject mismatched IDs before writing.

## Safety Boundary

Preview commands must not:

- Write files
- Create directories
- Create or remove symlinks
- Modify `.claude`, `.claude.json`, `.mcp.json`, or asset-center files
- Create backups
- Restore backups
- Run Git commands
- Perform import, mount, conflict resolution, restore, Pull, or Push apply behavior

## Frontend Integration

The static workflow pages now call preview wrappers from `apps/desktop/src/app/data-api.ts`:

- `ScanImportPage` calls `previewImport` only after `scanAssets` returns discovered assets, and can call `importApply` in `planOnly` mode with the preview's `previewId` to generate an import plan without writing files.
- `MountManagerPage` calls `previewMount` when selected asset or target changes, and can call `mountApply` in `planOnly` mode with the preview's `previewId` to generate a mount plan without writing files.
- `ConflictResolverPage` calls `previewConflicts` for a static preview scope and keeps `skip` / `rename` / `overwrite` as local resolution preview state.
- `BackupRestorePage` is now Backup History only. It lists manifests and manual
  restore guidance without a preview/apply Restore command.
- `SyncPage` calls `previewSync` for Pull/Push plan generation without running `git fetch`, `git pull`, or `git push`, then can call `syncApply` after preview-bound ordinary confirmation. Typed `APPLY` is not required.

Current write pages use preview plus ordinary button confirmation. Typed
`APPLY` is not required.

## Remaining UI Non-goals

- No conflict apply

Later milestones replaced these page flows with shared-core canonical Import,
Mount, Target Registry, Backup History, and Git Sync contracts. Automatic
historical Restore is not supported.
