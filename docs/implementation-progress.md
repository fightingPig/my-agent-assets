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
- Rust workspace: core 64 tests and desktop 89 tests passed
- frontend: 81 tests passed
- shared discovery: 7 fake HOME tests passed
- canonical MCP import/rendering: 10 tests passed

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
- implemented strict local `mounts.yaml` bindings with mounted/outOfSync/orphaned states
- implemented targetId-only Mount preview/apply for Skill links, Command links, Claude JSON MCP, and Codex TOML MCP
- implemented targetId-only Unmount preview/apply with precise MCP entry removal and protection against deleting user-replaced runtime content
- added Tauri and TypeScript adapters for shared discovery, canonical Import, target listing, Mount, and Unmount
- retained disabled/failing browser fallbacks for apply commands; browser mode cannot report a write as successful
- initialization now creates portable/local backup roots, schema-versioned state files, standard targets, local-state Git exclusions, and a `main` Git branch
- initialization idempotency and fake-HOME isolation are covered by regression tests

Not implemented:
- shared Git service
- Desktop/CLI adapters for the new unified source-discovery DTO
- migrate production pages from legacy Scan/Import/Mount commands to the shared adapters
- delete-asset multi-target transaction
- Import and Adopt composition
- operation journal and stale fingerprint cache

Next:
- add project/custom target authorization workflows
- implement delete-asset and Import-and-Adopt transactions
- migrate Desktop pages and CLI commands to shared-core APIs, then remove legacy runtimePath commands

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
