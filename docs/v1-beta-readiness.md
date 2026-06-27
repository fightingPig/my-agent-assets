# V1 Beta Readiness

Date: 2026-06-27

My Agent Assets V1 is ready for controlled local beta testing. It remains a local-first desktop application with no login, account, cloud workspace, billing, or authentication dependency.

## Implemented Features

- Read-only discovery of asset-center Skills, Commands, and MCP servers
- One-level project discovery under `~/workspace` and `~/code`
- User, project, and custom runtime asset scans
- Exact MCP conflict preview from top-level `mcpServers.<name>` JSON
- Import preview, plan-only validation, typed confirmation, apply, and backup
- Skill/Command symlink mount and MCP runtime config compilation
- Conflict decisions: skip, rename, and overwrite
- Backup manifest listing, restore preview, typed confirmation, and restore
- Local settings load/save
- Local Git status, Pull preview/apply, and Push preview/apply
- Real selected asset/project details with mount preview/apply
- Success/failure feedback, backup summaries, failure guidance, and post-operation refresh

## Safety Guarantees

- Automated write tests use explicit temporary fake HOME directories.
- Production apply commands resolve HOME through the centralized path layer.
- `planOnly` performs no filesystem or Git mutations.
- Every apply command validates its deterministic `previewId`.
- UI apply requires a successful preview, successful plan-only result, and typed `APPLY`.
- Asset names, backup IDs, rename targets, runtime paths, and manifest paths are validated before writes.
- Writes must stay below the resolved HOME; symlinked parent traversal is rejected.
- Existing destructive targets are backed up before replacement when backup is enabled.
- Restore validates manifest identity, runtime root, backup subtree, entry kind, and symlink target.
- MCP import leaves source Claude JSON unchanged; MCP mount merges a top-level `mcpServers` entry.
- Git commands use `std::process::Command` argument arrays, never shell-concatenated command strings.
- Sync rejects dirty worktrees, unresolved conflicts, missing upstreams, and non-repositories.

## Validation Results

The following validation passed on Apple Silicon macOS:

| Check | Result |
| --- | --- |
| `npm run typecheck` | Passed |
| `npm test` | 10 files, 73 tests passed |
| `npm run build:renderer` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo test -p my-agent-assets-desktop` | 80 tests passed |
| `npm run qa:visual` | 13 pages, 26 screenshots, 0 severe issues, 0 warnings |
| Tauri dev smoke | Started successfully with `MY_AGENT_ASSETS_HOME` set to an empty `/tmp` directory |
| Tauri release build | `.app` and arm64 `.dmg` generated |
| Packaged app smoke | Built `.app` executable launched with an empty fake HOME |
| Code signature verification | Ad-hoc signature valid |
| DMG verification | Checksum valid |

Visual QA artifacts:

- Screenshots: `apps/desktop/artifacts/visual-qa/`
- Summary: `apps/desktop/artifacts/visual-qa/summary.json`

Build artifacts:

- `target/release/bundle/macos/My Agent Assets.app`
- `target/release/bundle/dmg/My Agent Assets_0.1.0_aarch64.dmg`

## Known Limitations

- The macOS build is ad-hoc signed and not notarized. Gatekeeper behavior on another Mac still requires manual validation.
- Windows packaging, native titlebar behavior, DPI scaling, path handling, and symlink permissions were not validated in this macOS run.
- Automated Visual QA runs in headless Chrome. It does not validate native traffic lights, overlay dragging, Dock behavior, or OS window shadows.
- Project discovery is limited to one directory level under `~/workspace` and `~/code`.
- Conflict Resolver currently requests its configured preview selection; automatic aggregation of every conflict from every scan root is not yet implemented.
- Asset content shown in details is summary-derived; the frontend does not directly read source files.
- Unmount, asset removal, repository initialization, fetch, add, and commit are not exposed as desktop apply workflows.
- Git Pull is fast-forward only. Sync requires an existing clean repository and configured upstream.
- There is no updater, notarized distribution channel, telemetry, account service, or cloud service.

## Manual Beta Checklist

1. Install the DMG on a non-development Apple Silicon Mac.
2. Confirm Gatekeeper behavior and document the steps required for the ad-hoc build.
3. Verify native macOS traffic lights, continuous top drag behavior, minimize, close, resize, and relaunch.
4. Check all 13 pages at the default and minimum window sizes.
5. Start with a disposable HOME or disposable Claude fixtures before testing apply operations.
6. Verify import creates the expected asset-center file and backup without modifying source runtime files.
7. Verify Skill and Command mounts resolve to the asset-center source.
8. Verify MCP mount preserves unrelated JSON keys and other MCP servers.
9. Verify skip, rename, and overwrite conflict decisions, including the displayed JSON content.
10. Verify restore recreates files/directories/symlinks from the selected manifest.
11. Verify settings survive app restart.
12. Verify Git Pull/Push only against a disposable local test remote.
13. Repeat packaging and native-window checks on Windows before claiming Windows beta readiness.

## Beta Decision

The codebase satisfies the automated macOS V1 beta gate. Distribution beyond controlled local testing remains blocked on notarization and Windows-specific packaging/manual QA.
