# Preview-only Workflows v1

This milestone adds registered Tauri commands for safe workflow previews:

- `preview_import`
- `preview_mount`
- `preview_conflicts`
- `preview_restore`

These commands are deterministic and input-driven. They return plan, warning, conflict, target, and impact DTOs only.

Import, mount, and restore preview DTOs include a deterministic `previewId`. The related apply commands must use that ID and will reject mismatched IDs before writing or restoring anything.

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
- `BackupRestorePage` calls `previewRestore` when the selected backup changes, and can call `restoreApply` in `planOnly` mode with the preview's `previewId` for restore-plan generation.
- `SyncPage` calls `previewSync` for Pull/Push plan generation without running `git fetch`, `git pull`, or `git push`, then can call `syncApply` after typed confirmation.

Conflict apply buttons remain `StaticActionButton` and stay disabled. Import, mount, restore, and Sync pages use `ApplyConfirmationPanel` after a successful preview or plan-only result; the user must type `APPLY` before real apply mode can run.

## Remaining UI Non-goals

- No conflict apply

Backend `settings_save`, `import_apply`, `mount_apply`, `restore_apply`, `preview_sync`, and `sync_apply` were implemented in later safety milestones. The Settings page now has a controlled save action for local desktop configuration, Scan Import can generate a plan-only import result and then run confirmed import apply, Mount Manager can generate a plan-only mount result and then run confirmed mount apply, Sync can generate Pull/Push plans and then run confirmed Git sync apply, and Backup Restore can generate a plan-only restore result and then run confirmed restore apply.
