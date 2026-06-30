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

- **Purpose:** Legacy mount preview contract retained temporarily for old transport tests.
- **Input:** Legacy `PreviewMountInput`.
- **Output:** Legacy `MountPreview`.
- **Side effect:** Preview-only.
- **Consumer:** None in the production GUI.
- **Status:** Superseded by `canonical_mount_preview`.

Production pages must not provide a runtime path to this contract.

### Target Registry commands

#### `list_mount_targets`

- **Purpose:** Enumerate machine-local, already-authorized mount targets.
- **Input:** None.
- **Output:** `RegisteredMountTarget[]`.
- **Side effect:** Read-only.
- **Consumers:** Mount Manager, Asset Detail, Project Detail, and Settings.
- **Status:** Implemented in shared core and registered.

#### `target_registration_preview` / `target_registration_apply`

- **Purpose:** Register a project or custom target after an explicit preview.
- **Preview input:** `{ id, kind, location }`.
- **Apply input:** `{ previewId, previewGeneratedAtEpochSeconds, request }`.
- **Output:** `TargetChangePreview` / `TargetChangeResult`.
- **Side effect:** Preview-only / explicit write.
- **Consumer:** Settings path section.
- **Status:** Implemented in shared core and registered.

`location` is a project root for project target kinds and the authorized live
path for custom target kinds. Rust expands `~`, requires project roots to
exist, and derives provider, adapter, accepted asset kind, and final runtime
path. React does not construct a runtime path.

#### `target_removal_preview` / `target_removal_apply`

- **Purpose:** Remove an authorized non-user target after verifying that no mount bindings remain.
- **Preview input:** `{ targetId }`.
- **Apply input:** `{ previewId, previewGeneratedAtEpochSeconds, request }`.
- **Output:** `TargetChangePreview` / `TargetChangeResult`.
- **Side effect:** Preview-only / explicit write.
- **Consumer:** Settings path section.
- **Status:** Implemented in shared core and registered.

#### `canonical_mount_preview` / `canonical_mount_apply`

- **Purpose:** Preview and apply Skill/Command links or MCP compilation using an authorized target.
- **Preview input:** `{ assetId, targetId }`.
- **Apply input:** `{ previewId, previewGeneratedAtEpochSeconds, request }`.
- **Output:** `CanonicalMountPreview` / `CanonicalMountApplyResult`.
- **Side effect:** Preview-only / explicit write.
- **Consumers:** Mount Manager, Asset Detail, and Project Detail.
- **Status:** Implemented in shared core and registered.

Apply resolves the path and adapter from `targets.yaml`; it never accepts
`runtimePath` from the frontend.

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

- **Status:** Historical contract only; not registered.
- **Current product boundary:** Backup History is read-only and exposes
  portable/local manifests, affected paths, file reveal, and manual restore
  guidance. The application does not provide automatic historical Restore.

### `preview_sync`

- **Purpose:** Build a local Git Pull or Push plan from current repository status.
- **Input:** `PreviewSyncInput { direction }`, where `direction` is `pull | push`.
- **Output:** `SyncPreview { previewId, direction, status, repositoryVisibility, plannedEffects, warnings, backupRequired, canApply, generatedAtEpochSeconds, expiresAtEpochSeconds }`.
- **Side effect:** Preview-only. It may run read-only `git` and `gh api`
  commands, including `git ls-remote`, but does not modify the repository.
- **Consumer:** Sync.
- **Status:** Implemented in shared core and registered.

`previewId` is a SHA-256 digest bound to direction, current Git state,
canonical sync paths, remote identity, visibility result, and generation time.
`sync_apply` re-runs visibility verification and rejects stale previews.

### `sync_apply`

- **Purpose:** Execute a previously previewed local Git Pull or Push for the asset center repository.
- **Input:** `SyncApplyInput { previewId, previewGeneratedAtEpochSeconds, request: { direction } }`.
- **Output:** `SyncApplyResult { previewId, direction, affectedPaths, backupId, committed, pushed, pulled, warnings, journalPath }`.
- **Side effect:** Explicit write after preview and button confirmation.
- **Consumer:** Sync.
- **Status:** Implemented in shared core and registered.

Current behavior:

- The target repository is `~/.my-agent-assets`.
- The backend locks, revalidates `previewId`, and verifies remote identity again.
- Pull requires a clean worktree, creates a local canonical backup, and uses
  `git pull --ff-only`.
- Push stages only `.gitignore`, `assets/`, `assets.yaml`, and
  `backups/portable/` through a temporary index.
- Push never stages machine-local target or mount state.
- Push is blocked unless `gh api` reports GitHub visibility `PRIVATE`.
- Public, internal, unknown, unverifiable, changed, ahead-remote, and diverged
  remotes are blocked.
- Push does not use force, stash, merge, rebase, or reset.
- Git commands are invoked with `std::process::Command` argument arrays, not shell strings.

### `git_status`

- **Purpose:** Read the asset center's local Git state without pull, push, fetch, or credential interaction.
- **Input:** None.
- **Output:** `GitStatus { repositoryPath, isRepository, statusMessage, branch, remoteName, remoteIdentity, upstream, clean, ahead, behind, changedFiles, conflicts, syncableChanges, blockedChanges }`.
- **Side effect:** Read-only.
- **Future consumer:** Sync and Dashboard.
- **Status:** Implemented and registered as read-only.

### `recovery_status`

- **Purpose:** Report incomplete operation journals and whether new writes are blocked.
- **Input:** None.
- **Output:** `RecoveryStatus { writesBlocked, journals, recentRecoveries, message }`.
- **Side effect:** Read-only.
- **Consumer:** Dashboard system status.
- **Status:** Implemented in shared core and registered.

Every shared-core write acquires the global operation lock. After acquiring the
lock, it rejects the write when any journal is still `started` or
`rollback_required`. Read-only commands remain available. Tauri and CLI startup
invoke the shared-core recovery routine before normal work. Recoverable schema
v2 journals retain pre-operation path snapshots and, for Git sync, a guarded
branch-ref recovery record. Successful rollback is retained with status
`recovered`; failed or externally changed recovery state remains blocked for
manual diagnosis.

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
- `assetCenterPath` is normalized to the fixed `~/.my-agent-assets` V1 location and is read-only in the GUI.
- Save failures reject the Tauri invocation instead of returning successful-looking defaults.
- Other empty path fields fall back to safe defaults.
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

- **Status:** Legacy registered contract, no longer consumed by production GUI.
- **Replacement:** `canonical_mount_preview` / `canonical_mount_apply`.
- **Migration boundary:** Remove after the remaining legacy transport tests and
  duplicate Desktop implementation are deleted.

### `conflict_apply`

- **Purpose:** Apply previewed per-asset conflict decisions.
- **Input:** `ConflictApplyInput { previewId, mode, scope, assetIds, conflictResolutions, backupBeforeApply }`.
- **Output:** `ApplyResult { mode, ok, previewId, backup, steps, warnings, errors }`.
- **Side effect:** Write when `mode` is `apply`; no writes when `mode` is `planOnly`.
- **Consumer:** Conflict Resolver.
- **Status:** Implemented and registered.

Current behavior:

- Uses the `preview_import` identity generated from the same scope, selected assets, and conflict decisions.
- Requires exactly one `skip`, `rename`, or `overwrite` decision for every selected asset.
- Rename targets are validated as a single safe path component and must not already exist.
- Overwrite uses import backup-before-replacement behavior.
- MCP conflict previews read and display the exact existing asset JSON and incoming top-level `mcpServers.<name>` object.

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

The production desktop now delegates canonical discovery, Import, Mount,
Unmount, Delete, Target Registry, Backup History, and Git Sync behavior to
`my-agent-assets-core`. Historical automatic Restore is not registered or
exposed. Legacy transport DTOs remain only while their old test-only
implementations are removed.
