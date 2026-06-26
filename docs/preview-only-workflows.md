# Preview-only Workflows v1

This milestone adds registered Tauri commands for safe workflow previews:

- `preview_import`
- `preview_mount`
- `preview_conflicts`
- `preview_restore`

These commands are deterministic and input-driven. They return plan, warning, conflict, target, and impact DTOs only.

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

- `ScanImportPage` calls `previewImport` only after `scanAssets` returns discovered assets, and can call `importApply` in `planOnly` mode to generate an import plan without writing files.
- `MountManagerPage` calls `previewMount` when selected asset or target changes, and can call `mountApply` in `planOnly` mode to generate a mount plan without writing files.
- `ConflictResolverPage` calls `previewConflicts` for a static preview scope.
- `BackupRestorePage` calls `previewRestore` when the selected backup changes, and later milestones allow `restoreApply` in `planOnly` mode for restore-plan generation.

Destructive apply buttons remain `StaticActionButton` and stay disabled.

## Remaining UI Non-goals

- No conflict apply
- No Git pull or push

Backend `settings_save`, `import_apply`, `mount_apply`, and `restore_apply` were implemented in later safety milestones. The Settings page now has a controlled save action for local desktop configuration, Scan Import can generate a plan-only import result, Mount Manager can generate a plan-only mount result, and Backup Restore can generate a plan-only restore result. Destructive asset operations remain disabled until a dedicated UI wiring milestone.
