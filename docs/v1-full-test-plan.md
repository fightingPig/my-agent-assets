# V1 Full Test Plan

Date: 2026-06-27

This plan covers the current My Agent Assets V1 desktop and CLI implementation. All automated write tests must use a disposable fake HOME. Real `~/.claude`, `~/.claude.json`, and `~/.my-agent-assets` are excluded from automated testing.

## Status Legend

- `PASS`: executed successfully in the current environment
- `MANUAL`: requires human observation, OS permission, another machine, or Windows
- `BLOCKED`: attempted but prevented by an environment limitation
- `NOT RUN`: not yet executed

## Test Environments

| Environment | Purpose |
| --- | --- |
| Temporary Rust test directories | Backend read, preview, apply, path guard, backup-history, operation recovery, and Git tests |
| `/tmp/my-agent-assets-e2e-*` fake HOME | CLI lifecycle tests |
| `/tmp/my-agent-assets-v1-*` fake HOME | Tauri dev and packaged app smoke |
| Headless Chrome | 13-page Visual QA at 1440×900 and 1180×760 |
| Current Apple Silicon Mac | arm64 build, ad-hoc signing, DMG, native process launch |
| Another Apple Silicon Mac | Gatekeeper and clean-machine installation; manual |
| Windows 10/11 | native titlebar, DPI, path, symlink, MSI/EXE; manual |

## Gate A: Source And Contract Validation

| ID | Test | Procedure | Expected | Status |
| --- | --- | --- | --- | --- |
| A-01 | TypeScript contracts | Run `npm run typecheck` in `apps/desktop` | No type errors | PASS |
| A-02 | Frontend tests | Run `npm test` in `apps/desktop` | Full Vitest suite passes | PASS |
| A-03 | Renderer production build | Run `npm run build:renderer` | Vite production build succeeds | PASS |
| A-04 | Rust formatting | Run `cargo fmt --all -- --check` | No formatting diff | PASS |
| A-05 | Rust tests | Run `cargo test -p my-agent-assets-desktop` | Full desktop Rust suite passes | PASS |
| A-06 | Wire values | Run contract tests | Explicit enum values and camelCase DTOs remain stable | PASS |
| A-07 | Tauri command envelopes | Run data API tests | Exact command names and `{ input }` envelopes are used | PASS |
| A-08 | No-account UI | Run rendered-page tests and source scan | No login/account/OAuth/cloud/team/billing/subscription UI | PASS |

## Gate B: Fake HOME Isolation And Safety

| ID | Test | Procedure | Expected | Status |
| --- | --- | --- | --- | --- |
| B-01 | Non-target HOME isolation | Run fake HOME write E2E test with sentinel HOME | Sentinel tree remains byte-for-byte unchanged | PASS |
| B-02 | Import planOnly | Snapshot fake HOME before/after import plan | Entire tree unchanged; no asset center or backup created | PASS |
| B-03 | Mount planOnly | Snapshot fake HOME before/after Skill mount plan | Entire tree unchanged; no symlink created | PASS |
| B-04 | MCP mount planOnly | Snapshot fake HOME before/after MCP mount plan | Runtime JSON unchanged and no file created | PASS |
| B-05 | Backup History read-only | Snapshot fake HOME before/after backup listing and manifest reveal preview | Entire tree unchanged; no restore or current-state backup created | PASS |
| B-06 | Sync planOnly | Compare local and remote refs before/after | No Git mutation; ahead count unchanged | PASS |
| B-07 | Unsafe asset IDs | Test `/`, `\\`, `:`, control chars, `.`, `..`, traversal | Rejected before path construction or write | PASS |
| B-08 | Symlink parent escape | Point guarded parent outside fake HOME | Operation rejected; outside tree unchanged | PASS |
| B-09 | Nested Skill symlink | Put symlink inside imported Skill directory | Import rejected; no asset-center write | PASS |
| B-10 | Preview ID tampering | Change previewId for each apply command | Rejected before write/Git execution | PASS |
| B-11 | Backup manifest reveal tampering | Alter backup ID, manifest path, symlink target, or escaping entry | Reveal/listing rejects unsafe entries before opening any path | PASS |
| B-12 | Rename validation | Use traversal name or existing rename target | Rejected; existing assets unchanged | PASS |

## Gate C: Read-only Discovery

| ID | Test | Procedure | Expected | Status |
| --- | --- | --- | --- | --- |
| C-01 | Missing asset center | Call `list_assets` with empty fake HOME | Empty list, no directory creation | PASS |
| C-02 | Skill discovery | Add directory Skill and Markdown Skill | Both returned with correct IDs and paths | PASS |
| C-03 | Command discovery | Add Markdown Command | Command returned with metadata | PASS |
| C-04 | MCP discovery | Add valid and invalid MCP JSON | Valid is ready; invalid is marked invalid | PASS |
| C-05 | Mount derivation | Add runtime symlink and MCP config reference | Asset mountTargets and mounted status are derived | PASS |
| C-06 | User scan | Add user Skills, Commands, `.claude.json` | All assets found; top-level `mcpServers` parsed | PASS |
| C-07 | Project/custom scan | Add project runtime and `.mcp.json` | Scope-specific assets found without writes | PASS |
| C-08 | Invalid MCP config | Use malformed runtime JSON | Warning returned; command does not fail fatally | PASS |
| C-09 | Project discovery depth | Add first-level and nested projects | First-level markers found; nested child excluded | PASS |
| C-10 | Project counts | Add project Skills, Commands, MCP | Counts and mounted names returned | PASS |
| C-11 | Git status safety | Test missing dir, non-repo, repo without upstream | Safe status and explanatory message returned | PASS |
| C-12 | Backup listing | Add valid/invalid manifests | Valid summaries returned; invalid entries skipped | PASS |

## Gate D: Import Workflow

| ID | Test | Procedure | Expected | Status |
| --- | --- | --- | --- | --- |
| D-01 | Skill file import | Apply user Skill Markdown import | Verified copy appears in asset center | PASS |
| D-02 | Skill directory import | Apply directory Skill import | Full directory copied and verified | PASS |
| D-03 | Project Command import | Apply project-scope Command import | Command copied to asset center | PASS |
| D-04 | MCP extraction | Import selected MCP server | Only selected JSON object is stored; source config unchanged | PASS |
| D-05 | Replacement backup | Import over existing destination | Old destination stored in manifest backup | PASS |
| D-06 | Missing source | Import nonexistent asset | Failure reported; destination not created | PASS |
| D-07 | UI confirmation | Preview import, review impact summary, click explicit confirm | Apply remains disabled until backend preview is valid; no typed token required | PASS |
| D-08 | UI refresh | Complete import | Scan and preview data reload; result remains visible | PASS |

## Gate E: Mount Workflow

| ID | Test | Procedure | Expected | Status |
| --- | --- | --- | --- | --- |
| E-01 | Skill mount | Mount Skill to project runtime | Symlink points to asset-center source | PASS |
| E-02 | Command replacement mount | Mount over existing Command | Existing target backed up; symlink created | PASS |
| E-03 | MCP new compile | Mount MCP into missing `.mcp.json` | Valid top-level `mcpServers` file created | PASS |
| E-04 | MCP merge | Mount MCP into existing JSON | Unrelated keys and other servers preserved; backup created | PASS |
| E-05 | Invalid existing MCP JSON | Mount into malformed JSON | Failure reported; original file unchanged | PASS |
| E-06 | Outside-HOME target | Provide external or traversing target | Backend rejects target | PASS |
| E-07 | UI confirmation | Preview mount, review target/backup impact, click explicit confirm | Same preview-bound ordinary confirmation as import | PASS |
| E-08 | Detail-page mount | Open selected Asset/Project detail and mount | Selected real ID/path used; detail data refreshes | PASS |

## Gate F: Conflict Workflow

| ID | Test | Procedure | Expected | Status |
| --- | --- | --- | --- | --- |
| F-01 | Exact MCP diff | Compare asset JSON and runtime `mcpServers.<name>` | Both pretty JSON originals displayed | PASS |
| F-02 | Identical content | Compare semantically equal JSON | No conflict returned | PASS |
| F-03 | Skip | Apply skip decision | No files written | PASS |
| F-04 | Rename | Apply safe new name | Existing asset preserved; incoming asset stored under new name | PASS |
| F-05 | Overwrite | Apply overwrite | Existing asset backed up, then replaced | PASS |
| F-06 | MCP rename extraction | Rename incoming MCP | Original runtime server is extracted under new asset name | PASS |
| F-07 | One decision per asset | Omit or duplicate a decision | Apply rejected | PASS |
| F-08 | UI decision binding | Change decision and regenerate preview | previewId is bound to scope, IDs, decisions, renameTo | PASS |
| F-09 | UI confirmation | Generate conflict plan and click explicit confirm for selected decisions | Conflict apply remains disabled until backend preview is valid; no typed token required | PASS |

## Gate G: Backup History And Manual Restore Guidance

| ID | Test | Procedure | Expected | Status |
| --- | --- | --- | --- | --- |
| G-01 | Portable backup listing | Add portable backup manifests in fake asset center | Backup History lists manifest path, operation, size, count, and affected paths | PASS |
| G-02 | Local backup listing | Add local runtime backup manifests | Backup History lists local backups without Git-syncing them | PASS |
| G-03 | Unsafe backup entries | Add symlinked or escaping backup/manifest entries | Unsafe entries are skipped or rejected; outside tree is untouched | PASS |
| G-04 | Manifest reveal | Select a listed backup and reveal its manifest | Backend accepts only a listed entry ID and opens the manifest location without shell command strings | PASS |
| G-05 | Manual restore guide | Open Backup History page | Page shows read-only affected paths and a manual restore procedure; no automatic Restore button or command exists | PASS |
| G-06 | CLI restore boundary | Run `maa restore` | CLI rejects automatic historical Restore and points to Backup History/manual guide | PASS |
| G-07 | Backup deletion preview/apply | Preview deletion of one fake-HOME backup in GUI or `maa backup delete`, then confirm | Selected backup is removed only after preview; stale preview and journal references are blocked | PASS |
| G-08 | Backup capacity reminder | Load a configurable fake-HOME threshold and list oversized history | UI shows count, total size, oldest backup, and a manual-cleanup reminder; it never auto-deletes | PASS |

## Gate G1: Diagnostics And Local Audit Logs

| ID | Test | Procedure | Expected | Status |
| --- | --- | --- | --- | --- |
| G1-01 | Redacted operation log | Complete a fake-HOME transaction and read its audit entry | Entry contains only operation type, outcome, and timestamp; no paths, MCP values, credentials, errors, or user content | PASS |
| G1-02 | Log retention | Add an expired regular log file and append a new entry | Expired log is removed according to the configured retention window; unrelated files and symlinks are not followed | PASS |
| G1-03 | Diagnostic export Preview | Run `maa doctor export` or Dashboard Preview | No package is written; logical source categories are shown before confirmation | PASS |
| G1-04 | Diagnostic export apply | Confirm export in fake HOME | Package contains only allowlisted metadata, redacted entries, and path-free status; runtime/canonical/backup/settings content is excluded | PASS |
| G1-05 | Export stale binding | Change logs or registry after Preview then apply | Apply is rejected and writes no package | PASS |

## Gate H: Settings And Git Sync

| ID | Test | Procedure | Expected | Status |
| --- | --- | --- | --- | --- |
| H-01 | Default settings | Load settings in empty fake HOME | Defaults returned; no file created | PASS |
| H-02 | Save/load settings | Save modified values then reload | Normalized values persist under fake asset center | PASS |
| H-03 | Settings symlink escape | Symlink asset center outside fake HOME | Save rejected; outside file unchanged | PASS |
| H-04 | UI settings refresh | Save from Settings page | Backend value is reloaded and success/failure guidance shown | PASS |
| H-05 | Sync preview | Preview Pull/Push | Direction/status-bound preview, no Git mutation | PASS |
| H-06 | Push apply | Use disposable local bare remote | Ahead commit pushed; ahead becomes zero | PASS |
| H-07 | Pull apply | Use disposable local remote update | Fast-forward pull succeeds | PASS |
| H-08 | Dirty/conflict/no-upstream | Attempt sync in unsafe states | Apply rejected | PASS |
| H-09 | Git argument safety | Inspect implementation/tests | `std::process::Command` argument arrays, no shell string | PASS |
| H-10 | UI sync refresh | Complete sync | Git status reloads and operation result remains visible | PASS |

## Gate I: CLI End-to-end

| ID | Test | Procedure | Expected | Status |
| --- | --- | --- | --- | --- |
| I-01 | Fake runtime lifecycle | Run `./scripts/e2e_fake_runtime.sh` | init/scan/import/adopt/MCP conflict/mount/unmount/remove/list/status/doctor pass; `maa restore` remains disabled | PASS |
| I-02 | Scan defaults to plan | Inspect fixture before/after scan without `--apply` | Runtime and asset center unchanged | PASS |
| I-03 | MCP source preservation | Complete scan/import | `.claude.json` and `.mcp.json` retain source entries and unrelated keys | PASS |
| I-04 | Fake Git remote lifecycle | Run Git E2E against disposable private/local remote | Commit, Push and Pull succeed without real Claude data | PASS |

## Gate J: Static GUI And Responsive Layout

| ID | Test | Procedure | Expected | Status |
| --- | --- | --- | --- | --- |
| J-01 | Full page manifest | Run `npm run qa:visual` | 13 registered pages rendered | PASS |
| J-02 | Default viewport | Capture every page at 1440×900 | No severe overflow/collapse/clipping | PASS |
| J-03 | Minimum viewport | Capture every page at 1180×760 | Local scrolling works; no severe overflow/collapse/clipping | PASS |
| J-04 | Screenshot integrity | Inspect generated PNGs | No black/unpainted tiles; toolbar and panels paint fully | MANUAL |
| J-05 | Navigation | Run App tests | All visible pages switch and PageHeader updates | PASS |
| J-06 | Detail navigation | Open detail from list inspector | Hidden detail pages open without sidebar routes | PASS |
| J-07 | Search/filter/selection | Run page/component tests | Local interactions update lists and inspectors | PASS |
| J-08 | No custom controls | Inspect DOM tests | No React traffic lights or Windows controls | PASS |
| J-09 | Drag/no-drag contract | Run App/platform tests | Overlay drags; interactive controls do not | PASS |

## Gate K: macOS Packaging And Native Smoke

| ID | Test | Procedure | Expected | Status |
| --- | --- | --- | --- | --- |
| K-01 | Tauri dev compile | Start `npm run dev` with fake HOME | Vite, Rust build, and app process start | PASS |
| K-02 | Release bundle | Run `npm run build` with fake HOME | arm64 `.app` and `.dmg` generated | PASS |
| K-03 | Architecture | Run `file` on release binary | Mach-O 64-bit arm64 | PASS |
| K-04 | Signature | Run `codesign --verify --deep --strict` | Ad-hoc signature valid | PASS |
| K-05 | DMG integrity | Run `hdiutil verify` | Checksum valid | PASS |
| K-06 | Packaged process | Launch bundled executable with fake HOME | Process remains running until intentionally terminated | PASS |
| K-07 | Native traffic lights | Observe app window | Native red/yellow/green controls appear | PASS |
| K-08 | Continuous drag | Drag top 28px repeatedly | Every drag moves window without refocus workaround | MANUAL |
| K-09 | Window lifecycle | Minimize, zoom, resize, close, relaunch | Native behavior remains stable | MANUAL |
| K-10 | Installed app | Mount DMG, copy to Applications, launch | App installs and launches from installation path | MANUAL |
| K-11 | Gatekeeper clean-machine | Open on another Mac | Expected warning/launch behavior documented | MANUAL |

## Gate L: Windows Manual Qualification

| ID | Test | Procedure | Expected | Status |
| --- | --- | --- | --- | --- |
| L-01 | Windows build | Build MSI/EXE on Windows | Installer artifacts generated | MANUAL |
| L-02 | Native titlebar | Launch installed app | Native Windows titlebar; no macOS overlay or 28px gap | MANUAL |
| L-03 | Minimum size and DPI | Test 100%, 125%, 150%, 200% scaling | No clipping or incoherent overlap | MANUAL |
| L-04 | Path behavior | Test drive letters, spaces, Unicode, long paths | Discovery and guards behave correctly | MANUAL |
| L-05 | Symlink permissions | Test standard and elevated configurations | Clear success or actionable permission failure | MANUAL |
| L-06 | Installer lifecycle | Install, upgrade, uninstall | App data policy and shortcuts behave as documented | MANUAL |

## Current Execution Record

This section must be updated with actual command output and evidence after each run.

| Area | Result | Evidence |
| --- | --- | --- |
| Automated frontend | PASS | TypeScript passed; Vitest suite passed; renderer production build passed |
| Automated Rust | PASS | Full workspace passed, including shared-core operation recovery and desktop adapter tests |
| Windows core compile | PASS | `cargo check -p my-agent-assets-core --target x86_64-pc-windows-msvc` |
| CLI fake runtime | PASS | `./scripts/e2e_fake_runtime.sh`; disposable fake HOME only |
| CLI fake Git | PASS | Disposable local bare remote: `/tmp/my-agent-assets-local-remote-8Ydafn/remote.git` |
| Visual QA | PASS | 13 pages, 26 screenshots, 0 severe, 0 warnings; `apps/desktop/artifacts/visual-qa/summary.json` |
| Tauri dev | PASS | Started with `MY_AGENT_ASSETS_HOME` pointing to `/tmp` |
| Release build/signature/DMG | PASS | arm64 app, valid ad-hoc signature, valid DMG checksum |
| Native window interaction | PARTIAL | Current candidate Accessibility inspection exposes native close/minimize/zoom controls, navigation, and readable empty states; continuous drag, resize, full-screen, and relaunch require current-package manual acceptance |
| Installed application | PARTIAL | Current candidate `.app`/`.dmg` is ad-hoc signed and DMG-verified; exact-package install and fake-HOME launch remain in the manual checklist |
| Cross-machine macOS | MANUAL | Requires another Apple Silicon Mac |
| Windows | MANUAL | Requires Windows 10/11 environment |

### Native UI Evidence

- Historical native interaction records for commit `b7208e9` remain useful for
  regression context, but are not release evidence for the current candidate.
- The current candidate package evidence, exact source commit, signature result,
  DMG checksum, and Accessibility inspection scope are maintained in
  `docs/final-beta-readiness.md`.
- Computer Use inspection confirms an installed ad-hoc candidate's native
  close/minimize/zoom controls, sidebar navigation, and readable empty states.
  The bridge cannot retain `MY_AGENT_ASSETS_HOME` when it relaunches the app,
  so it cannot replace the fake-HOME installed-app workflow checklist.

### Beta Regression

- Commit `b7208e9` adds user/project/custom discovery for directory Skills at `<name>/SKILL.md` while retaining direct Markdown Skills.
- Differing same-ID assets now increment `conflictCount`, render as conflicts, and block direct Scan Import apply.
- Backend import apply independently rejects unresolved content conflicts.
- Preview asset IDs use strict type and safe-component validation.
- Settings save failures reject the Tauri invocation; `assetCenterPath` is read-only and fixed in V1.
- Apply confirmation uses ordinary preview-bound buttons; typed `APPLY` prompts are intentionally absent.
- Backup History is read-only and manual-restore-only; historical `preview_restore`, `restore_apply`, and `maa restore` are intentionally absent or rejected.
- The regression suite covers directory/direct Skills, conflict detection and blocking, explicit overwrite/skip/rename, invalid preview IDs, settings write failures, and fixed asset-center behavior.
- The latest ad-hoc-signed installed build passed direct macOS AX API validation. `System Events` did not enumerate its window, but `AXUIElement` exposed one window and all native controls without requiring a permission change.
- Current-package AX validation covered enabled close/minimize/zoom controls, two consecutive real pointer drags, minimize/restore, full-screen enter/exit, 1180×760 resize, close/exit, and relaunch to one `1440×901` window.

## Human Handoff Rule

Only cases left as `MANUAL` or `BLOCKED` after the automated run should be handed to the user. The handoff must include exact artifact paths, prerequisites, numbered actions, and expected results.

Use `docs/manual-acceptance-checklist.md` as the authoritative handoff checklist for macOS Beta and Windows Stable manual qualification.

## Remaining Manual Run

1. Review all 26 PNG files in `apps/desktop/artifacts/visual-qa/`, especially code, diff, inspector, and settings panels at 1180×760. The automated report found no overflow, collapse, or clipping, but semantic visual quality still needs human judgment.
2. Open Scan Import with `/tmp/my-agent-assets-beta-regression` and visually confirm `dir-skill`, `direct-skill`, and the conflict warning are visible.
3. Run K-11 on another Apple Silicon Mac to record Gatekeeper behavior for the ad-hoc signed, non-notarized build.
4. Run L-01 through L-06 on Windows 10/11, including 100%, 125%, 150%, and 200% DPI.
