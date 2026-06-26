# V1 Tauri Command Contract

This document defines the future JSON boundary between the desktop React frontend and the Tauri Rust backend. It does not imply that contract-only commands are registered or implemented.

## Transport Rules

- JSON object fields use `camelCase`.
- Asset IDs use `type:name`, for example `skill:review` or `mcp:PostgreSQL`.
- Paths are transported as platform-native display strings.
- Timestamp fields use ISO 8601 strings; unavailable timestamps are `null`.
- Commands with input receive one Tauri argument named `input`: `invoke(command, { input })`.
- Preview commands are stateless. They do not depend on a scan session, `scanId`, or backend session lookup.
- This milestone defines successful request and response shapes only. A shared error envelope will be designed when real commands are implemented.

## Enum Wire Values

These values are part of the public JSON contract and must not be inferred only from Rust variant names.

| Type | Exact wire values |
|---|---|
| `AssetType` | `skill`, `command`, `mcp` |
| `AssetStatus` | `ready`, `mounted`, `unmounted`, `conflict`, `invalid` |
| `ProjectStatus` | `ready`, `changed`, `needsSync`, `invalid` |
| `RuntimeScope` | `user`, `local`, `project` |
| `ConflictResolution` | `skip`, `rename`, `overwrite` |
| `PlanStepKind` | `check`, `import`, `mount`, `compileMcp`, `backup`, `restore`, `git`, `settings` |
| `RiskLevel` | `none`, `low`, `medium`, `high` |
| `AppearanceTheme` | `system`, `light`, `dark` |
| `DensityPreference` | `compact`, `comfortable` |
| `LogLevel` | `error`, `warn`, `info`, `debug` |
| `ApplyMode` | `planOnly`, `apply` |
| `ApplyStepStatus` | `pending`, `skipped`, `success`, `failed` |
| `SyncDirection` | `pull`, `push` |

`ScanScope` is a discriminated object with these exact shapes:

```ts
{ kind: "user" }
{ kind: "project", projectPath: string }
{ kind: "custom", path: string }
```

## Commands

### `app_info`

- **Purpose:** Return application and backend runtime metadata.
- **Input:** None.
- **Output:** `AppInfo { name, version, platform, arch, backendReady }`.
- **Side effect:** Read-only.
- **Future consumer:** Dashboard system status.
- **Status:** Implemented and registered. Its existing JSON shape is unchanged.

### `scan_assets`

- **Purpose:** Read the selected runtime scope and return discovered asset summaries without importing them.
- **Input:** `ScanAssetsInput { scope: ScanScope }`.
- **Output:** `ScanResult { scope, scannedAt, assets, counts, conflictCount, warnings }`.
- **Side effect:** Read-only.
- **Future consumer:** Scan Import.
- **Status:** Implemented and registered as read-only.

The read-only implementation scans Markdown Skills and Commands from the selected runtime root. MCP discovery reads the relevant JSON config file and parses its top-level `mcpServers` object; `.mcpServers` is not a path.

### `preview_import`

- **Purpose:** Build an import plan for explicitly selected assets and conflict decisions.
- **Input:** `PreviewImportInput { scope, assetIds, conflictResolutions }`.
- **Output:** `ImportPreview { previewId, scope, assets, conflicts, steps, warnings, canApply }`.
- **Side effect:** Preview-only.
- **Future consumer:** Scan Import and Conflict Resolver.
- **Status:** Implemented and registered as preview-only.

`PreviewImportInput` is self-contained. The backend will rebuild the preview from `scope`, `assetIds`, and `conflictResolutions`; it must not require a prior scan session. `previewId` is generated from those inputs and later validated by `import_apply`.

### `list_assets`

- **Purpose:** List asset-center summaries, optionally filtered by type.
- **Input:** `ListAssetsInput { assetType: AssetType | null }`.
- **Output:** `AssetSummary[]`.
- **Side effect:** Read-only.
- **Future consumer:** Skills, Commands, MCP Servers, Asset Detail, and Mount Manager.
- **Status:** Implemented and registered as read-only.

### `list_projects`

- **Purpose:** List configured local projects and mounted-asset counts.
- **Input:** None.
- **Output:** `ProjectSummary[]`.
- **Side effect:** Read-only.
- **Future consumer:** Projects and Project Detail.
- **Status:** Implemented and registered as read-only.

### `preview_mount`

- **Purpose:** Build a mount or MCP compile plan for one asset and runtime target.
- **Input:** `PreviewMountInput { assetId, target }`, where `target` is `MountTarget { scope, runtimePath, projectPath }`.
- **Output:** `MountPreview { previewId, asset, target, steps, warnings, backupRequired, canApply }`.
- **Side effect:** Preview-only.
- **Future consumer:** Mount Manager, Asset Detail, and Project Detail.
- **Status:** Implemented and registered as preview-only.

`previewId` is generated from `assetId` and `target` and later validated by `mount_apply`.

### `preview_conflicts`

- **Purpose:** Recompute conflicts for selected assets in a scan scope.
- **Input:** `PreviewConflictsInput { scope, assetIds }`.
- **Output:** `ConflictPreview[]`.
- **Side effect:** Preview-only.
- **Future consumer:** Conflict Resolver and Scan Import.
- **Status:** Implemented and registered as preview-only.

### `list_backups`

- **Purpose:** List local backup manifests without reading backup contents into the UI.
- **Input:** None.
- **Output:** `BackupSummary[]`.
- **Side effect:** Read-only.
- **Future consumer:** Backup Restore.
- **Status:** Implemented and registered as read-only.

### `preview_restore`

- **Purpose:** Validate a backup and show affected paths and restore steps.
- **Input:** `PreviewRestoreInput { backupId }`.
- **Output:** `RestorePreview { previewId, backup, affectedPaths, steps, warnings, backupBeforeRestore, canApply }`.
- **Side effect:** Preview-only. Reads an existing backup manifest when present, but does not restore files or create backups.
- **Future consumer:** Backup Restore.
- **Status:** Implemented and registered as preview-only.

`previewId` is generated from `backupId` and later validated by `restore_apply`.

### `preview_sync`

- **Purpose:** Build a local Git Pull or Push plan from current repository status.
- **Input:** `PreviewSyncInput { direction }`, where `direction` is `pull | push`.
- **Output:** `SyncPreview { direction, repositoryPath, branch, remote, steps, warnings, canApply }`.
- **Side effect:** Preview-only. It does not run `git fetch`, `git pull`, `git push`, or mutate the repository.
- **Future consumer:** Sync.
- **Status:** Implemented and registered as preview-only.

### `git_status`

- **Purpose:** Read the asset center's local Git state without pull, push, fetch, or credential interaction.
- **Input:** None.
- **Output:** `GitStatus { repositoryPath, isRepository, statusMessage, branch, remote, clean, ahead, behind, changedFiles, conflicts, lastSyncedAt }`.
- **Side effect:** Read-only.
- **Future consumer:** Sync and Dashboard.
- **Status:** Implemented and registered as read-only.

### `settings_load`

- **Purpose:** Load desktop configuration for paths, scanning, safety, local Git, appearance, logs, and CLI display.
- **Input:** None.
- **Output:** `DesktopSettings`.
- **Side effect:** Read-only.
- **Future consumer:** Settings.
- **Status:** Implemented and registered as read-only defaults.

### `settings_save`

- **Purpose:** Validate and persist the complete desktop settings object.
- **Input:** `SettingsSaveInput { settings: DesktopSettings }`.
- **Output:** The normalized and persisted `DesktopSettings`.
- **Side effect:** Write.
- **Future consumer:** Settings.
- **Status:** Implemented and registered.

Current behavior:

- Settings are stored as JSON at `~/.my-agent-assets/config.json`.
- Missing config files are not created by `settings_load`.
- Empty path fields fall back to safe defaults.
- Numeric settings are clamped to supported ranges.
- The GUI Settings page can call `settings_save`; this writes only local desktop configuration and does not touch Claude runtime files.

### `import_apply`

- **Purpose:** Apply a previously previewed import selection by copying selected runtime assets into the asset center.
- **Input:** `ImportApplyInput { previewId, mode, scope, assetIds, conflictResolutions, backupBeforeApply }`.
- **Output:** `ApplyResult { mode, ok, previewId, backup, steps, warnings, errors }`.
- **Side effect:** Write when `mode` is `apply`; no writes when `mode` is `planOnly`.
- **Future consumer:** Scan Import and Conflict Resolver.
- **Status:** Implemented and registered for Skill, Command, and MCP import.

Current behavior:

- Skill imports support `.claude/skills/<name>/` directories and `.claude/skills/<name>.md` files.
- Command imports support `.claude/commands/<name>.md` files.
- MCP imports read the selected top-level `mcpServers.<name>` JSON object from `.claude.json` or `.mcp.json`, then write it to `assets/mcps/<name>.json`.
- Runtime Claude files are not deleted or modified.
- Existing asset-center destinations are backed up before replacement when `backupBeforeApply` is true.

### `mount_apply`

- **Purpose:** Apply a previously previewed mount by creating a Skill/Command runtime symlink or compiling an MCP server into runtime JSON.
- **Input:** `MountApplyInput { previewId, mode, assetId, target, backupBeforeApply }`.
- **Output:** `ApplyResult { mode, ok, previewId, backup, steps, warnings, errors }`.
- **Side effect:** Write when `mode` is `apply`; no writes when `mode` is `planOnly`.
- **Future consumer:** Mount Manager, Asset Detail, and Project Detail.
- **Status:** Implemented and registered for Skill/Command symlink mounts and MCP compile.

Current behavior:

- Skill sources are resolved from `assets/skills/<name>/` or `assets/skills/<name>.md`.
- Command sources are resolved from `assets/commands/<name>.md`.
- MCP sources are resolved from `assets/mcps/<name>.json`.
- The runtime target path is expanded from `~` and must stay under the resolved HOME.
- Existing target paths are backed up before replacement when `backupBeforeApply` is true.
- MCP compile writes the selected server into the target file's top-level `mcpServers.<name>` field while preserving existing JSON object fields and other MCP servers.

### `restore_apply`

- **Purpose:** Restore paths from a backup manifest and optionally back up current state before replacement.
- **Input:** `RestoreApplyInput { previewId, mode, backupId, backupBeforeRestore }`.
- **Output:** `ApplyResult { mode, ok, previewId, backup, steps, warnings, errors }`.
- **Side effect:** Write when `mode` is `apply`; no writes when `mode` is `planOnly`.
- **Future consumer:** Backup Restore.
- **Status:** Implemented and registered for file, directory, and symlink backup entries.

Current behavior:

- The backend loads `~/.my-agent-assets/backups/<backupId>/manifest.json`.
- Restore targets must stay under the resolved HOME.
- Backup entry paths must stay under the selected backup directory.
- Current state is backed up before restore when `backupBeforeRestore` is true.
- `planOnly` reads the manifest and returns skipped restore steps without writing files.
- `previewId` must match the deterministic preview identity for the apply input; mismatches fail before any write.

### Future write commands

Future write commands must receive a single `input` object with explicit command-specific identifiers and safety preferences. Apply-style commands must include `previewId` and `mode`. See `docs/write-safety-contract.md`.

## DTO Shapes

```ts
type AssetCounts = { total; skills; commands; mcps };

type AssetSummary = {
  id; name; title; assetType; status; category; description;
  sourcePath; scope; updatedAt; mountTargets;
};

type ProjectSummary = {
  id; name; title; path; status; description; updatedAt; assetCounts; mounts;
};

type PlanStep = { id; kind; label; description; risk };

type ApplyStepResult = {
  stepId; kind; label; status; message; affectedPaths;
};

type ConflictPreview = {
  id; assetId; assetType; name; reason;
  existingContent; incomingContent; allowedResolutions;
};

type BackupSummary = { id; label; createdAt; sizeBytes; entryCount };

type BackupManifestSummary = {
  id; label; createdAt; sizeBytes; entryCount;
  manifestPath; runtimeRoot; affectedPaths;
};

type ApplyResult = {
  mode; ok; previewId; backup; steps; warnings; errors;
};

type GitStatus = {
  repositoryPath; isRepository; statusMessage; branch; remote;
  clean; ahead; behind; changedFiles; conflicts; lastSyncedAt;
};

type DesktopSettings = {
  assetCenterPath; scanRoots; maxDepth;
  backupBeforeApply; planOnlyByDefault;
  gitDefaultBranch; gitRemote;
  appearanceTheme; density;
  logLevel; logRetentionDays; cliPath;
};
```

The authoritative field types are defined in:

- `apps/desktop/src/app/contracts.ts`
- `apps/desktop/src-tauri/src/contracts.rs`

## Implementation Boundary

The DTO module contains no filesystem, Git, scan, mount, backup, restore, or settings implementation. The first read-only integration registers `scan_assets`, `list_assets`, `list_projects`, `git_status`, and `settings_load`; the preview-only workflow integration registers `preview_import`, `preview_mount`, `preview_conflicts`, and `preview_restore`; the write integration now registers `import_apply`, `mount_apply`, `restore_apply`, and `settings_save`. Future command handlers should translate between these transport DTOs and `my-agent-assets-core` types.
