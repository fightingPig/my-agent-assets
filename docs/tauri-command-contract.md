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

### `initialization_preview` / `initialization_apply`

- **Purpose:** Explicitly create the fixed `~/.my-agent-assets` asset center and
  initialize its local Git repository on branch `main`.
- **Preview input:** None.
- **Apply input:** `{ previewId, previewGeneratedAtEpochSeconds }`.
- **Output:** Shared-core `InitializationPreview` /
  `InitializationApplyResult`.
- **Side effect:** Read-only preview / explicit write.
- **Consumer:** Dashboard empty-environment initialization panel and `maa init`.
- **Status:** Implemented in shared core and registered.

Preview never creates the asset center. Apply builds the complete structure in
a sibling staging directory, flushes it, initializes Git, and publishes it
with one same-directory rename. An already valid asset center is an idempotent
no-op. An existing partial, symlinked, malformed, or schema-incompatible asset
center is blocked and preserved for diagnosis.

### `discover_runtime_sources`

- **Purpose:** Discover Claude Code, Codex, or approved custom runtime sources without importing them.
- **Input:** Shared-core `DiscoveryScope`.
- **Output:** Shared-core `DiscoveryResult { sources, warnings }`.
- **Side effect:** Read-only.
- **Consumers:** Scan Import and provider-filtered Asset Center views.
- **Status:** Implemented in shared core and registered.

The shared discovery service scans canonical source formats for both Claude Code
and Codex. MCP discovery parses Claude JSON `mcpServers` or Codex TOML
`mcp_servers`; provider-specific parsing is not implemented in React or the
Tauri transport.

### `canonical_import_preview` / `canonical_import_apply`

- **Purpose:** Preview and apply one source-ID-bound canonical import.
- **Preview input:** `{ scope, sourceId, resolution }`.
- **Apply input:** `{ previewId, previewGeneratedAtEpochSeconds, request }`.
- **Output:** Shared-core `ImportPreview` / `ImportApplyResult`.
- **Side effect:** Preview-only / explicit write.
- **Consumers:** Shared adapters and single-item workflows.
- **Status:** Implemented in shared core and registered.

### `canonical_batch_import_preview` / `canonical_batch_import_apply`

- **Purpose:** Preview and atomically apply multiple imports and explicit
  skip/rename/overwrite conflict resolutions.
- **Input:** A scope plus source-ID-bound selections.
- **Output:** Shared-core `BatchImportPreview` / `BatchImportApplyResult`.
- **Side effect:** Preview-only / explicit transactional write.
- **Consumers:** Scan Import and Conflict Resolver.
- **Status:** Implemented in shared core and registered.

### `list_assets`

- **Purpose:** List asset-center summaries, optionally filtered by type.
- **Input:** `ListAssetsInput { assetType: AssetType | null }`.
- **Output:** `AssetSummary[]`.
- **Side effect:** Read-only.
- **Future consumer:** Skills, Commands, MCP Servers, Asset Detail, and Mount Manager.
- **Status:** Implemented in shared core and registered as read-only. Tauri and
  CLI use the same query service.

### `list_projects`

- **Purpose:** Discover local projects and mounted-asset counts from
  `config.yaml.scan_roots`.
- **Input:** None.
- **Output:** `ProjectSummary[]`.
- **Side effect:** Read-only.
- **Future consumer:** Projects and Project Detail.
- **Status:** Implemented in shared core and registered as read-only.

Project discovery uses the configured `max_depth` (default `5`), supports
nested/monorepo projects, follows the shared fixed skip list, and never follows
directory symlinks. Missing scan roots and an uninitialized asset center return
safe read-only results without creating files.

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

#### `canonical_mcp_get`

- **Purpose:** Read one canonical MCP definition plus its machine-local target binding states.
- **Input:** `{ assetId }`.
- **Output:** `McpAssetDefinition`.
- **Side effect:** Read-only.
- **Consumer:** MCP Servers structured editor.
- **Status:** Implemented in shared core and registered.

#### `canonical_mcp_save_preview` / `canonical_mcp_save_apply`

- **Purpose:** Create or edit a canonical MCP definition without writing any Claude Code or Codex live config.
- **Preview input:** `{ assetId?, canonical, title?, description? }`; `assetId` is required for edit and the existing name is immutable.
- **Apply input:** `{ previewId, previewGeneratedAtEpochSeconds, request }`.
- **Output:** `McpSavePreview` / `McpSaveApplyResult`.
- **Side effect:** Preview-only / explicit canonical write.
- **Consumer:** MCP Servers structured editor.
- **Status:** Implemented in shared core and registered.

Preview validates the canonical schema and every existing target renderer.
Apply writes only `assets/mcps/<name>.json`, `assets.yaml`, and `mounts.yaml`.
Existing bindings become `out_of_sync`; live configs are updated only by a
separate explicit `canonical_mount_preview` / `canonical_mount_apply` Sync.

### `list_backups`

- **Purpose:** List local backup manifests without reading backup contents into the UI.
- **Input:** None.
- **Output:** `BackupSummary[]`.
- **Side effect:** Read-only.
- **Consumer:** Backup History.
- **Status:** Implemented and registered as read-only.

### `canonical_asset_content`

- **Purpose:** Read the real canonical preview content for one registered Skill, Command, or MCP asset.
- **Input:** `{ assetId }`.
- **Output:** `CanonicalAssetContent { assetId, assetType, canonicalPath, contentPath, content, truncated }`.
- **Side effect:** Read-only.
- **Consumers:** Skills, Commands, MCP Servers, and Asset Detail.
- **Status:** Implemented in shared core and registered.

The shared core resolves the canonical path from `assets.yaml`, rejects
unregistered IDs and symlink traversal, and caps UI preview content at 256 KiB.
React never supplies a filesystem path.

### `canonical_asset_open`

- **Purpose:** Reveal a Skill `SKILL.md` in the file manager or open a Command Markdown file with the system application.
- **Input:** `{ assetId, action }`, where action is `reveal | open_external`.
- **Output:** `{ assetId, path }`.
- **Side effect:** Local system UI action; no file write.
- **Consumer:** Asset Detail.
- **Status:** Implemented in shared core and registered.

Actions are kind-restricted: Skill supports only `reveal`, Command supports
only `open_external`, and MCP is rejected because MCP uses the structured
editor. The platform process is invoked with an argument array and never a
shell command string.

### `reveal_backup_manifest`

- **Purpose:** Reveal one listed backup manifest in the native file manager.
- **Input:** `BackupRevealInput { entryId }`.
- **Output:** `BackupRevealResult { manifestPath }`.
- **Side effect:** Local UI action only; no file write or Restore.
- **Consumer:** Backup History.
- **Status:** Implemented and registered.

The command does not accept a frontend path. Shared core resolves `entryId`
against the current backup history, rejects symlinked manifests, canonicalizes
the result beneath `~/.my-agent-assets/backups`, and only then asks Finder,
Explorer, or the Linux file manager to reveal it. Platform commands are invoked
with argument arrays and never through a shell.

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

- Settings are stored as YAML at `~/.my-agent-assets/config.yaml`.
- Missing config files are not created by `settings_load`.
- `assetCenterPath` is normalized to the fixed `~/.my-agent-assets` V1 location and is read-only in the GUI.
- Save failures reject the Tauri invocation instead of returning successful-looking defaults.
- Other empty path fields fall back to safe defaults.
- Numeric settings are clamped to supported ranges.
- The GUI Settings page can call `settings_save`; this writes only local desktop configuration and does not touch Claude runtime files.

The former Desktop-only `scan_assets`, `preview_import`, `preview_mount`,
`preview_conflicts`, `import_apply`, `mount_apply`, and `conflict_apply`
commands have been removed. They accepted frontend-built paths or duplicated
shared-core behavior and are not compatibility aliases.

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
