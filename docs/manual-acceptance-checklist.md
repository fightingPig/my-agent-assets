# Manual Acceptance Checklist

Date: 2026-07-09

This checklist is the human-run companion to `docs/v1-full-test-plan.md`.
Automated tests must continue to use disposable fake HOME directories. Do not
run destructive apply flows against real `~/.claude`, `~/.codex`, or
`~/.my-agent-assets` unless a tester intentionally opts into a real-data trial.

## Scope

Use this checklist before calling a build:

- macOS Beta ready for controlled local testing.
- Cross-platform V1 Stable ready after Windows qualification is also complete.

Windows incomplete status blocks V1 Stable, but does not block a clearly labeled
macOS Beta.

## Required Evidence Before Manual QA

Record the latest command output or CI link before starting manual checks.

| Area | Command | Expected |
| --- | --- | --- |
| TypeScript | `cd apps/desktop && npm run typecheck` | Pass |
| Frontend tests | `cd apps/desktop && npm test` | Pass |
| Renderer build | `cd apps/desktop && npm run build:renderer` | Pass |
| Visual QA | `cd apps/desktop && npm run qa:visual` | 13 pages, 26 screenshots, 0 severe issues |
| Rust format | `cargo fmt --all -- --check` | Pass |
| Desktop Rust tests | `cargo test -p my-agent-assets-desktop` | Pass |
| CLI Rust tests | `cargo test -p my-agent-assets-cli` | Pass if package is available |
| CLI fake runtime | `./scripts/e2e_fake_runtime.sh` | Pass |
| Tauri production build | `cd apps/desktop && npm run build` | `.app` and `.dmg` on macOS |

## macOS Beta Checklist

Use an Apple Silicon Mac and a disposable runtime root such as:

```bash
export MY_AGENT_ASSETS_HOME=/tmp/my-agent-assets-manual-acceptance
```

| ID | Check | Action | Expected |
| --- | --- | --- | --- |
| M-01 | Install App | Open the generated DMG and copy `My Agent Assets.app` to `~/Applications` or `/Applications` | App is installed from the packaged artifact, not run from the build folder |
| M-02 | Empty HOME startup | Launch with an empty disposable `MY_AGENT_ASSETS_HOME` | App opens without fake production data or fatal error |
| M-03 | Native window controls | Inspect and use close, minimize, zoom/full-screen | Native macOS traffic lights are present; no React traffic lights |
| M-04 | Continuous drag | Drag the top 28px area repeatedly | Every drag moves the window without needing a refocus workaround |
| M-05 | Window lifecycle | Resize to minimum, close, relaunch | Window remains usable and relaunches cleanly |
| M-06 | Visual QA review | Open all screenshots in `apps/desktop/artifacts/visual-qa/` | No obvious semantic overlap, clipped controls, black tiles, or unreadable panels |
| M-07 | Logo | Inspect Dock, app switcher, window, DMG, and package icon | Logo is visible and not replaced by a default placeholder |

## Functional Manual Checklist

Run these against disposable Claude/Codex fixtures. Keep screenshots, fixture
paths, and resulting backup manifest paths with the acceptance record.

| ID | Check | Action | Expected |
| --- | --- | --- | --- |
| F-01 | Scan Claude assets | Create fake Claude Skills, Commands, and `~/.claude.json` top-level `mcpServers`; run Scan | Claude sources appear with provider/source information |
| F-02 | Scan Codex assets | Create fake Codex Skills and `~/.codex/config.toml` `[mcp_servers]`; run Scan | Codex-compatible Skill and MCP sources appear |
| F-03 | Import Skill | Preview and apply a Skill import | Canonical Skill appears in the shared asset center |
| F-04 | Import and adopt Skill | Preview and apply adopt for a runtime Skill | Runtime path is replaced by the managed binding after backup |
| F-05 | Mount Skill to Claude user target | Mount the same Skill to the Claude user target | Runtime link points to the canonical asset |
| F-06 | Mount Skill to Codex user target | Mount the same Skill to the Codex user target | Codex-compatible target receives the Skill binding |
| F-07 | Mount Skill to project target | Register/select a project target and mount the same Skill | Project runtime receives the Skill binding |
| F-08 | Import Command | Import a Claude-compatible Command | Canonical Command appears in the asset center |
| F-09 | Mount Command to Claude commands | Mount the Command to a Claude commands target | Runtime command link points to the canonical asset |
| F-10 | Command to Codex rejected | Attempt to mount the Command to a Codex target | UI/backend rejects it as incompatible |
| F-11 | Import Claude MCP | Import an MCP server from Claude JSON | Canonical MCP JSON is stored; source Claude JSON is not reverse-written |
| F-12 | Import Codex MCP | Import an MCP server from Codex TOML | Canonical MCP JSON is stored; source Codex TOML is not reverse-written |
| F-13 | Mount MCP to Claude JSON | Mount the same MCP to Claude | Only top-level `mcpServers.<name>` is patched; unrelated keys are preserved |
| F-14 | Mount MCP to Codex TOML | Mount the same MCP to Codex | Only `[mcp_servers.<name>]` is patched; unrelated TOML and comments are preserved where supported |
| F-15 | Conflict handling | Create an incoming asset with the same ID and different content | Conflict Resolver shows both sides and requires skip, rename, or overwrite |
| F-16 | Skip conflict | Apply skip | Existing canonical asset remains unchanged |
| F-17 | Rename conflict | Apply rename with a safe new name | Incoming asset is stored under the new canonical ID |
| F-18 | Overwrite conflict | Apply overwrite | Existing canonical asset is backed up, then replaced |
| F-19 | Backup History | Open Backup History after writes | Portable/local backups are listed with affected paths and manifest locations |
| F-20 | Manual restore guide | Inspect Backup History guidance | No automatic Restore button exists; manual restore instructions are visible |
| F-20a | Backup deletion | Preview deletion of one disposable backup, review warning, then confirm | Only the selected backup is removed; stale/journal-referenced backups remain blocked; no runtime file is restored or modified |
| F-20b | Backup capacity reminder | Set a low local capacity reminder threshold and open Backup History | Count, total size, oldest backup, and manual-cleanup warning are visible; nothing is auto-deleted |
| F-21 | Git sync | Use a disposable private/local remote and run preview/apply | Only sync whitelist paths are staged; Pull/Push obey safety checks |
| F-22 | Restart persistence | Quit and relaunch the app | Assets, targets, mounts, settings, and backup history remain correct |
| F-23 | No fake data | Inspect all production pages | Empty/error states or real data are shown; static demo fixtures are not default production data |
| F-24 | No invalid buttons | Inspect all write-capable pages | No clickable no-op business buttons are shown; writes use preview plus ordinary confirmation |

## Windows Stable Checklist

Run on Windows 10 or Windows 11 before claiming cross-platform V1 Stable.

| ID | Check | Action | Expected |
| --- | --- | --- | --- |
| W-01 | Installer artifact | Build MSI or NSIS EXE on Windows CI or a Windows build host | Installer artifact is generated |
| W-02 | Signature status | Inspect the installer signature | Production Stable uses code signing; unsigned builds are labeled test-only |
| W-03 | Install/start/upgrade/uninstall | Install, launch, upgrade, relaunch, uninstall | App lifecycle works and uninstall does not delete the asset center |
| W-04 | Native titlebar | Launch installed app | Native Windows titlebar appears; no macOS overlay and no 28px top gap |
| W-05 | DPI and fonts | Test 100%, 125%, 150%, and 200% scaling | No incoherent overlap or clipped primary controls |
| W-06 | Path handling | Test drive letters, spaces, Unicode, and long paths | Guards and discovery work with Windows paths |
| W-07 | Cross-volume behavior | Try mount targets on another drive | Unsafe/incompatible mount plans are rejected with actionable guidance |
| W-08 | Symlink permission | Test with and without Developer Mode/elevation | Success or a clear permission failure; no copy/hardlink fallback |
| W-09 | Windows Claude JSON patch | Mount MCP to Claude JSON on Windows | JSON patch preserves unrelated fields and target path semantics |
| W-10 | Windows Codex TOML patch | Mount MCP to Codex TOML on Windows | TOML patch preserves unrelated config and uses Codex-compatible keys |
| W-11 | Basic accessibility | Keyboard focus, readable names, non-color-only status, and scaling | Basic controls remain operable and understandable |

## Release Decision

Use these labels:

- `macOS Beta`: macOS packaging and manual macOS checklist pass, Windows remains
  explicitly incomplete.
- `V1 Stable`: macOS Beta passes, Developer ID notarization is complete, and the
  Windows Stable checklist passes with a signed installer.

Do not claim V1 Stable while Windows packaging, signing, or manual Windows
qualification is incomplete.
