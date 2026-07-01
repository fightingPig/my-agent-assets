# Final Product Implementation Progress

This file tracks progress toward `my_agent_assets_final_goal.md`.

Gate labels are requirement ordering and verification checkpoints. Implementation proceeds continuously on `codex/final-product-v1`; a Gate does not require a pause, standalone commit, or push.

## Gate 0: Current-state audit and model correction

Status:
- completed

Commits:
- not committed by this workstream

Validation:
- reviewed root `AGENTS.md`
- reviewed `my_agent_assets_final_goal.md`
- reviewed historical Codex read-only, static GUI freeze, and read-only integration documents
- frontend baseline: 79 tests passed
- Rust workspace baseline: core 10 tests and desktop 88 tests passed
- renderer production build passed

Implemented:
- created `docs/final-product-model.md` as the canonical product-model summary
- established one canonical asset center with multiple runtime sources and compatible mount targets
- clarified that providers are adapters and do not own separate asset pools
- documented final Codex-compatible Skill and MCP discovery, import, and mount support
- documented that Codex Command assets, Command targets, `AGENTS.md` assets, and OAuth token management remain forbidden
- documented file-based MCP SSOT and explicitly excluded SQLite
- marked milestone-only documents as historical/superseded where they conflict with the final goal
- preserved historical milestone text beneath each banner
- updated root `AGENTS.md` so Codex-compatible Skill/MCP import and mount work is allowed
- preserved Foundation Freeze, No Login, native window, and AppShell constraints

Not implemented:
- Target Registry and targetId-only apply contract
- shared-core Scan/Import/Mount/Backup/Git implementation
- canonical MCP model and Claude/Codex renderers
- final GUI/CLI behavior and packaging

Risks:
- Desktop and CLI currently use separate business implementations
- current Desktop mount DTO accepts a frontend-provided runtime path
- current restore commands conflict with the final backup-history-only product boundary

Next:
- move shared filesystem safety into `crates/core`
- implement Target Registry, canonical MCP, and target adapters in shared core
- migrate Tauri and CLI to the shared request/result APIs

## Workstream: Shared Rust Core

Status:
- in progress

Validation:
- Rust workspace: CLI 7 tests, core 90 tests, and desktop 17 tests passed
- Rust workspace Clippy passed with warnings denied
- frontend: 80 tests passed
- frontend TypeScript and renderer production build passed
- shared discovery: 7 fake HOME tests passed
- canonical MCP import/rendering: 10 tests passed
- shared CLI: 3 unit tests and 2 fake HOME integration tests passed
- `scripts/e2e_fake_runtime.sh`: passed with Claude/Codex import and targetId mount flow
- Visual QA: 26 screenshots, 0 severe issues, 0 warnings
- real GitHub Private fake-device sync E2E: passed with temporary branch creation, device-B clone verification, and cleanup

Implemented:
- moved path component validation, tilde expansion, guarded write/existing paths, symlink traversal rejection, and root containment checks into `crates/core`
- retained the existing Desktop adapter entry points while delegating to core
- added schema-versioned shared settings with explicit malformed/new-version failures and atomic fake-HOME writes
- added Target Registry, target compatibility validation, and targetId-only resolution primitives
- added canonical MCP DTO plus pure Claude JSON and Codex TOML import/render/remove adapters
- added unified Claude Code, Codex, and declared Custom runtime-source discovery
- user discovery does not infer a project from the process current directory
- project discovery only scans an explicitly supplied project path
- discovery reports provider, source/config path, format, scope, managed/link state, warnings, and import/adopt eligibility
- discovery can safely reload the exact selected Claude JSON or Codex TOML MCP entry into the canonical model and detects entries removed after discovery
- added a strict, portable `assets.yaml` registry model keyed by canonical asset ID
- added non-mutating registry/content consistency diagnostics for missing, unregistered, and invalid canonical content
- implemented sourceId-based canonical Import preview/apply with five-minute expiry, source/registry/content fingerprints, explicit skip/overwrite/rename, rollback, and Git-portable backups
- canonical Import supports Claude/Codex Skills, Claude Commands, Claude JSON MCP, and Codex TOML MCP without modifying source live configs
- MCP overwrite marks existing local bindings `outOfSync` without reverse-synchronizing live configs
- implemented structured canonical MCP create/edit with SHA-256-bound
  Preview/Apply, immutable existing asset identity, schema and bound-target
  renderer validation, and operation-journal rollback
- canonical MCP save updates only the canonical JSON, `assets.yaml`, and
  `mounts.yaml`; existing bindings become `outOfSync` and require explicit
  target Sync through the Mount renderer
- MCP Servers production UI now provides stdio/http/sse structured fields,
  advanced JSON preview, ordinary save confirmation, and per-target explicit
  Sync confirmation
- implemented strict local `mounts.yaml` bindings with mounted/outOfSync/orphaned states
- implemented targetId-only Mount preview/apply for Skill links, Command links, Claude JSON MCP, and Codex TOML MCP
- implemented targetId-only Unmount preview/apply with precise MCP entry removal and protection against deleting user-replaced runtime content
- added Tauri and TypeScript adapters for shared discovery, canonical Import, target listing, Mount, and Unmount
- retained disabled/failing browser fallbacks for apply commands; browser mode cannot report a write as successful
- added an exclusive cross-command operation lock and structured local operation journals
- implemented Delete Asset preview/apply with direct-delete blocking, unmount-all impact enumeration, portable/local backups, and full multi-target rollback
- implemented backend-composed batch Import-and-Adopt for Claude/Codex Skills, Claude Commands, Claude MCP, and Codex MCP
- Adopt refreshes internal Import/Mount previews inside one lock while preserving one outer stale-preview contract
- injected mid-operation failures prove Delete and Adopt restore runtime sources, canonical content, `assets.yaml`, and `mounts.yaml`
- added Tauri and TypeScript contracts for Delete and explicit `preview_adopt` / `adopt_apply`
- added atomic Batch Import preview/apply so production UI never sequences single-asset writes in React
- migrated Scan/Import production UI to unified Claude/Codex discovery, atomic Batch Import, and backend-composed Adopt
- migrated Mount production UI to Target Registry listing and targetId-only preview/apply
- removed frontend runtime-path construction from the primary Mount workflow
- initialization now creates portable/local backup roots, schema-versioned state files, standard targets, local-state Git exclusions, and a `main` Git branch
- initialization idempotency and fake-HOME isolation are covered by regression tests
- replaced legacy direct initialization with shared-core
  `initialization_preview` / `initialization_apply`; CLI and Dashboard use the
  same ten-minute SHA-256-bound preview
- initialization is zero-write during preview and publishes a fully flushed
  sibling staging tree with one rename; existing valid centers are idempotent,
  while partial, symlinked, malformed, or incompatible centers are preserved
  and blocked
- fake-HOME regression coverage proves startup recovery checks plus
  initialization preview leave an empty first-run HOME untouched
- added preview/apply Target Registry add/remove operations with local registry backups and active-binding removal protection
- migrated the CLI to sourceId-based discovery/import/adopt and targetId-only mount/unmount/delete
- added CLI project/custom target registration with derived provider paths and explicit authorization
- disabled automatic historical Restore and replaced legacy unrestricted Git sync with the shared safe Git service
- updated the fake runtime E2E flow to prove Claude/Codex canonical import, dual-provider Skill mount, Command-to-Codex rejection, and unmount-all delete
- connected Scan conflicts to Conflict Resolver through an explicit in-memory context carrying the canonical batch preview
- migrated Conflict Resolver from legacy synthesized commands to atomic canonical Batch Import preview/apply with sourceId-bound skip/rename/overwrite decisions
- MCP conflicts show canonical existing/incoming JSON and expandable raw source content
- added shared portable/local/legacy Backup History enumeration with manifest paths, affected paths, sizes, operation types, symlink-safe traversal, and sensitive MCP risk flags
- migrated the Desktop Backup History command to shared core; no historical Restore command is exposed
- added a real Backup History “reveal manifest” action that accepts only a
  listed entry ID, rejects symlinked/escaping manifests in shared core, and
  invokes Finder/Explorer/file manager without a shell
- expanded Backup History into a five-step manual restore guide while keeping
  automatic historical Restore absent
- implemented shared Git status and SHA-256-bound Pull/Push preview/apply
- Pull requires a clean worktree, creates a local canonical backup, and uses fast-forward only
- Push uses a temporary Git index and stages only `.gitignore`, `assets/`, `assets.yaml`, and `backups/portable/`
- Push performs live `gh api` visibility verification before preview and again under lock before apply; only `PRIVATE` is accepted
- Push blocks public/internal/unknown/unverifiable remotes, non-whitelist changes, staged user changes, remote-ahead state, divergence, and changed remote identity
- Push never uses force, stash, merge, rebase, or reset; failed Push restores only the app-created branch ref
- migrated Tauri, CLI, and Sync UI to the shared Git service
- added shared target registration requests that expand `~`, canonicalize
  existing project roots, derive provider/adapter/runtime paths, and preserve
  preview/apply stale-state validation
- registered Target Registry add/remove commands in Tauri and added typed
  frontend wrappers
- added Settings target registration/removal UI with preview plus ordinary
  confirmation; user-level built-in targets remain non-removable in the GUI
- migrated Asset Detail and Project Detail from frontend-built `runtimePath`
  requests to authorized targetId-only canonical Mount
- removed historical Restore preview/apply DTOs, implementations, and tests from
  both the legacy Desktop transport and the old monolithic core; Backup History
  remains read-only with manual restore guidance
- added read-only recovery status plus global write blocking whenever an
  incomplete operation journal exists; Dashboard reports the blocked state
- upgraded operation journals to schema v2 with pre-operation snapshots,
  atomic step persistence, stale process-lock reclamation, and automatic
  startup rollback in both Tauri and CLI
- added recovery coverage for canonical import, batch import, adopt, mount,
  unmount, delete, target registry changes, settings save, and Git sync
- Git recovery restores only the asset-center branch ref and index through
  guarded `update-ref`/`read-tree`; unexpected external ref changes fail closed
- removed the Desktop-only apply/preview implementations and their legacy
  Tauri commands; production Scan, Conflict, and Mount workflows now call
  shared-core discovery, batch import, target registry, and mount services
- migrated Codex Asset Center provider views from a Desktop-only parser to the
  shared-core runtime discovery adapter
- moved `list_assets` and `list_projects` filesystem queries into shared core;
  Desktop is now a transport adapter and CLI `list` uses the same asset query
- unified project discovery on `config.yaml.scan_roots` and configurable
  `max_depth` (default 5), including nested projects, fixed skip rules, and no
  directory-symlink traversal
- migrated Import, Batch Import, Adopt, Mount, Unmount, Delete, and Target
  Registry preview fingerprints from FNV/DefaultHasher to one domain-separated
  SHA-256 implementation that includes normalized request data, timestamp, and
  present/missing state plus content for every affected path
- added regression assertions that preview IDs use their stable operation
  prefix followed by exactly 64 lowercase SHA-256 hexadecimal characters
- removed the remaining 1,600-line monolithic `lib.rs` Scan/Mount/Remove/Sync
  implementation and its legacy public types, so production and CLI callers
  can no longer bypass the canonical preview/apply services
- replaced the legacy one-line doctor output with a structured read-only
  shared-core report covering initialization validity, registries, canonical
  content consistency, incomplete operations, runtime presence, Git, and the
  platform mount mechanism
- CLI fake-HOME coverage proves `maa doctor` emits stable JSON and leaves an
  uninitialized HOME untouched
- removed every production `StaticActionButton` placeholder and the component
  itself; unsupported optional actions such as exporting plans or saving scan
  previews are omitted, while supported writes retain real Preview/Apply
  controls
- replaced synthesized production Skill/Command/MCP previews with real,
  symlink-safe canonical content reads capped at 256 KiB
- added assetId-only Skill file reveal and Command external-open actions;
  shared core resolves the canonical path and platform adapters use argument
  arrays without shell command strings
- removed the final typed-confirmation compatibility props and dead local
  state; all write panels now use preview readiness plus ordinary button
  confirmation
- removed Provider-based filtering from canonical Asset Center navigation;
  Commands remain visible under either provider selection while target
  compatibility continues to block Codex Command mounts in shared core
- renamed the user-facing backup page to “备份历史” so the UI no longer implies
  an automatic Restore capability
- verified initial unborn-branch Push, regular Push, rejected Push rollback, Pull backup, and cross-device clone semantics
- `scripts/e2e_fake_runtime.sh`: passed after the SHA-256 migration without
  touching the real HOME

Not implemented:
- exhaustive process-crash injection at every journal step

Next:
- add crash-point integration coverage to each remaining write workflow

## Progress Update Template

```text
## Workstream / Gate N: <name>

Status:
- completed / in progress / blocked

Commits:
- optional until final commit consolidation

Validation:
- command or evidence: result

Implemented:
- ...

Not implemented:
- ...

Risks:
- ...

Next:
- ...
```
