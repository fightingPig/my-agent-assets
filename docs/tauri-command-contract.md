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
- **Status:** Contract only; not registered.

### `preview_import`

- **Purpose:** Build an import plan for explicitly selected assets and conflict decisions.
- **Input:** `PreviewImportInput { scope, assetIds, conflictResolutions }`.
- **Output:** `ImportPreview { scope, assets, conflicts, steps, warnings, canApply }`.
- **Side effect:** Preview-only.
- **Future consumer:** Scan Import and Conflict Resolver.
- **Status:** Contract only; not registered.

`PreviewImportInput` is self-contained. The backend will rebuild the preview from `scope`, `assetIds`, and `conflictResolutions`; it must not require a prior scan session.

### `list_assets`

- **Purpose:** List asset-center summaries, optionally filtered by type.
- **Input:** `ListAssetsInput { assetType: AssetType | null }`.
- **Output:** `AssetSummary[]`.
- **Side effect:** Read-only.
- **Future consumer:** Skills, Commands, MCP Servers, Asset Detail, and Mount Manager.
- **Status:** Contract only; not registered.

### `list_projects`

- **Purpose:** List configured local projects and mounted-asset counts.
- **Input:** None.
- **Output:** `ProjectSummary[]`.
- **Side effect:** Read-only.
- **Future consumer:** Projects and Project Detail.
- **Status:** Contract only; not registered.

### `preview_mount`

- **Purpose:** Build a mount or MCP compile plan for one asset and runtime target.
- **Input:** `PreviewMountInput { assetId, target }`, where `target` is `MountTarget { scope, runtimePath, projectPath }`.
- **Output:** `MountPreview { asset, target, steps, warnings, backupRequired, canApply }`.
- **Side effect:** Preview-only.
- **Future consumer:** Mount Manager, Asset Detail, and Project Detail.
- **Status:** Contract only; not registered.

### `preview_conflicts`

- **Purpose:** Recompute conflicts for selected assets in a scan scope.
- **Input:** `PreviewConflictsInput { scope, assetIds }`.
- **Output:** `ConflictPreview[]`.
- **Side effect:** Preview-only.
- **Future consumer:** Conflict Resolver and Scan Import.
- **Status:** Contract only; not registered.

### `list_backups`

- **Purpose:** List local backup manifests without reading backup contents into the UI.
- **Input:** None.
- **Output:** `BackupSummary[]`.
- **Side effect:** Read-only.
- **Future consumer:** Backup Restore.
- **Status:** Contract only; not registered.

### `preview_restore`

- **Purpose:** Validate a backup and show affected paths and restore steps.
- **Input:** `PreviewRestoreInput { backupId }`.
- **Output:** `RestorePreview { backup, affectedPaths, steps, warnings, backupBeforeRestore, canApply }`.
- **Side effect:** Preview-only.
- **Future consumer:** Backup Restore.
- **Status:** Contract only; not registered.

### `git_status`

- **Purpose:** Read the asset center's local Git state without pull, push, fetch, or credential interaction.
- **Input:** None.
- **Output:** `GitStatus { repositoryPath, branch, remote, clean, ahead, behind, changedFiles, conflicts, lastSyncedAt }`.
- **Side effect:** Read-only.
- **Future consumer:** Sync and Dashboard.
- **Status:** Contract only; not registered.

### `settings_load`

- **Purpose:** Load desktop configuration for paths, scanning, safety, local Git, appearance, logs, and CLI display.
- **Input:** None.
- **Output:** `DesktopSettings`.
- **Side effect:** Read-only.
- **Future consumer:** Settings.
- **Status:** Contract only; not registered.

### `settings_save`

- **Purpose:** Validate and persist the complete desktop settings object.
- **Input:** `SettingsSaveInput { settings: DesktopSettings }`.
- **Output:** The normalized and persisted `DesktopSettings`.
- **Side effect:** Write.
- **Future consumer:** Settings.
- **Status:** Contract only; not registered.

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

type ConflictPreview = {
  id; assetId; assetType; name; reason;
  existingContent; incomingContent; allowedResolutions;
};

type BackupSummary = { id; label; createdAt; sizeBytes; entryCount };

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

The DTO module contains no Tauri commands and no filesystem, Git, scan, mount, backup, restore, or settings implementation. Future command handlers should translate between these transport DTOs and `my-agent-assets-core` types. Apply/write commands beyond `settings_save` are intentionally not part of this contract milestone.
