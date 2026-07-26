# Final Beta Readiness

Date: 2026-07-26

This document summarizes the current release boundary for the final V1 goal.
It does not replace `docs/v1-full-test-plan.md`; it points to the evidence and
remaining manual work required before release labels are used.

## Current Status

The implementation is moving toward a controlled macOS Beta first, followed by
a cross-platform V1 Stable after Windows qualification.

Current automated evidence is tracked in:

- `docs/v1-full-test-plan.md`
- `docs/v1-beta-readiness.md`
- `docs/implementation-progress.md`

The required human checklist is:

- `docs/manual-acceptance-checklist.md`

## macOS Beta Readiness

macOS Beta can be considered only when all of the following are true:

- TypeScript, frontend tests, renderer build, Visual QA, Rust format, desktop
  Rust tests, CLI tests when available, fake HOME E2E, Tauri dev smoke, and
  Tauri production build pass.
- The build produces an installable `.app` and `.dmg`.
- The app is launched from an installed location, not only from the build
  directory.
- The macOS manual checklist in `docs/manual-acceptance-checklist.md` passes.
- The build is clearly labeled `macOS Beta`.
- The release notes state that the current build is ad-hoc signed and not
  notarized unless a Developer ID notarized build has actually been produced.
- The release notes state that first launch may require manual user approval
  when the build is not notarized.
- The release does not claim Windows readiness or cross-platform V1 Stable.

Known macOS Beta limitation:

- Without Apple Developer ID notarization, Gatekeeper behavior on another Mac
  must be documented as part of manual acceptance.

## Cross-platform V1 Stable Readiness

V1 Stable requires macOS Beta readiness plus Windows qualification.

Do not call the product V1 Stable until all of the following are true:

- macOS production distribution uses Developer ID signing and Apple
  notarization.
- Windows CI or a Windows build host produces an MSI or NSIS EXE.
- Windows production distribution uses code signing. Unsigned Windows artifacts
  are test packages only.
- Windows 10/11 manual installation, upgrade, startup, core workflows, and
  uninstall behavior pass.
- Windows native titlebar, no macOS overlay, no 28px top blank space, DPI
  scaling, drive-letter paths, path separators, cross-volume behavior, symlink
  permissions, Claude JSON patching, and Codex TOML patching pass.
- Installing, upgrading, and uninstalling the app do not delete the asset
  center.

The repository includes a Windows GitHub Actions workflow that produces unsigned
MSI and NSIS **test packages**. Its artifacts are not code-signed and must not
be labeled Stable; they exist to make the required Windows installation and
runtime validation reproducible on a Windows runner.

The tag-driven `Desktop Beta Release` workflow builds the Apple Silicon DMG and
unsigned Windows MSI/NSIS installers from the same `v*-beta.*` source tag, then
publishes them together as a GitHub prerelease. Because this repository is
private, the release is private to authorized repository users. The workflow
labels the artifacts as controlled Beta/test packages and never claims Stable
signing or notarization.

## Current Package Evidence

Latest automated macOS package verification:

- candidate branch: `codex/final-product-v1-next`
- candidate version: `0.1.1-beta.1`
- build command:
  `cd apps/desktop && npm run tauri -- build --target aarch64-apple-darwin`
- app signature: `codesign --verify --deep --strict` passed
- DMG integrity: `hdiutil verify` passed
- DMG SHA-256:
  `48a4388a18f06b0270f34aa174c0f6beff1df2ad32c7f644c250620a7174a949`
- Tauri dev smoke: passed; the packaged executable also launched with
  `MY_AGENT_ASSETS_HOME` set to an empty disposable path and did not create or
  write any entry in that path
- all 52 macOS/Windows Visual QA screenshots were generated at `1440x900` and
  `1180x760`, and representative minimum-size pages were reviewed;
  no overlap, black tiles, horizontal overflow, or unreadable primary controls
  were found, long pages retained their expected local scrolling, and the
  automated structural report returned zero severe issues and zero warnings
- verification date: 2026-07-26

Installed-app native evidence on 2026-07-26:

- the exact candidate app was installed from the generated bundle into
  `~/Applications/My Agent Assets.app` and passed `codesign --verify --deep --strict`
  at that installed location
- the latest installed candidate was launched with the isolated fake HOME
  `/tmp/my-agent-assets-native-qa-L5WHnt`; Computer Use discovered five user
  sources, deselected the Command to leave four selected items, and generated
  an import preview with exactly two content conflicts
- Conflict Resolver displayed `0 / 2 已决策` for `claude-review` and
  `codex-review`, showed localized conflict reasons, excluded the two
  structurally unchanged MCP entries, and kept import/conflict apply disabled
  while decisions were unresolved
- the installed app previewed and applied isolated Skill mounts to Claude Code,
  Codex, and a registered project target; it also applied a Claude Command
  mount and rejected Command-to-Codex as incompatible
- isolated Claude JSON and Codex TOML MCP mounts preserved unrelated
  `theme`, `model`, and `history` configuration while updating only the
  selected server entry
- Backup History displayed 15 real portable/local records, affected paths,
  manifest locations, file reveal, and the five-step manual restore guide
- a disposable local Git remote completed preview-bound Push; the worktree
  became clean and the persisted `git-sync` audit entry remained visible after
  app restart
- native QA exposed and fixed two status defects before this package was built:
  a successful warning-free mount preview no longer claims that no preview
  exists, and successful Git sync now refreshes persisted sync history
- the remaining native window evidence in this section was collected from the
  immediately preceding readability candidate, whose frozen shell is unchanged
- the macOS Accessibility tree exposed the native close, minimize, and zoom
  controls, the sidebar navigation, and readable empty-state content; it did
  not expose React-rendered traffic-light controls
- Computer Use navigated the installed app through Projects, Scan, Mount,
  Sync, and Settings, switched between Claude Code and Codex, and verified
  keyboard focus on the provider control
- the 28px overlay drag area accepted two consecutive drag gestures without
  requiring an application switch
- the window respected the configured `1180x760` minimum, remained readable,
  and passed native minimize, zoom, close, and relaunch checks
- the rebuilt readability candidate was installed from source commit
  `57181b243545fd15354d3410f49b20226e82cb10`; Computer Use rechecked Dashboard,
  Scan Import, Settings, and two consecutive overlay drags in the installed app
- a native read-only command failure was shown as a partial-read warning while
  successful repository, runtime, backup, and project summaries remained
  visible
- the installed app and read-only mounted DMG both referenced a valid
  `icon.icns`; Finder icon view visually showed the intended product icon for
  `~/Applications/My Agent Assets.app` rather than a default placeholder
- the Dock accessibility target timed out, so Dock and app-switcher icon
  appearance still require visual human confirmation
- Computer Use bridge-initiated relaunches do not reliably retain
  `MY_AGENT_ASSETS_HOME`; the successful isolated workflow therefore launched
  the installed app explicitly after setting the variable with `launchctl`,
  then stopped the app and removed the variable after validation
- an earlier isolated flow persisted a Settings `maxDepth` change only after
  Preview/Apply confirmation and exposed the Conflict Resolver filtering
  defect; the latest installed-candidate recheck above confirms the fix

Windows automated package evidence on 2026-07-26:

- `cargo check -p my-agent-assets-core --tests --target x86_64-pc-windows-msvc`
  passed, including compilation of the Windows-only junction regression test.
- GitHub Actions workflow run `30201644197` for commit `2e219a0` passed frontend contract validation
  and all 22 Desktop shared-core adapter tests on a native Windows runner.
- The same run produced the unsigned test installers
  `My Agent Assets_0.1.0_x64_en-US.msi` and
  `My Agent Assets_0.1.0_x64-setup.exe`.
- The artifact archive SHA-256 is
  `b9fd385c8a84058ac582b33b4c79f21c3f1a50b969c56b1d44d27c7d1490f188`;
  the MSI SHA-256 is
  `2b552d434a6c1d0c6de277c95b153903eb07027d68bf7663822adc1eb96b528b`;
  the NSIS SHA-256 is
  `d8511c2ee02ff15770582a226c8d5aa03334c78db521017d60ca07d4483961a9`.

This is automated package evidence only. It does not replace installation,
upgrade, launch, workflow, or accessibility manual acceptance on the exact
candidate build.

The current macOS artifact locations used by the existing test plan are:

- `target/aarch64-apple-darwin/release/bundle/macos/My Agent Assets.app`
- `target/aarch64-apple-darwin/release/bundle/dmg/My Agent Assets_0.1.1-beta.1_aarch64.dmg`

Before publishing a Beta, regenerate these artifacts from the exact release
commit and record:

- commit hash
- build command
- app path
- DMG path
- `codesign --verify --deep --strict` result
- `hdiutil verify` result
- installed-app launch result with a disposable `MY_AGENT_ASSETS_HOME`

## No Updater In V1

V1 does not implement automatic updates:

- no updater endpoint
- no background update network request
- no automatic install or rollback path

Users install updates manually. Any automatic updater requires a separate
design for signing, release source, integrity checks, and rollback.

## Remaining Manual Work

As of this readiness note, the remaining work before any final release decision
is:

1. Run the remaining macOS functional checklist from
   `docs/manual-acceptance-checklist.md` against
   the exact package intended for Beta.
2. Recheck the latest installed candidate's Conflict Resolver against the
   isolated fixture and confirm that only the two actual conflicts require
   decisions.
3. Visually confirm the Dock and app-switcher icon on the installed candidate.
4. Run Gatekeeper validation on another Apple Silicon Mac for ad-hoc,
   non-notarized builds.
5. Complete the Windows Stable checklist before claiming cross-platform V1
   Stable.

## Release Report Template

Use this template when a Beta or Stable candidate is ready:

```text
Release label:
Latest commit:
Build command:
App artifact:
Installer artifact:
Validation commands:
Visual QA summary:
Manual checklist result:
Known limitations:
Unsupported environment notes:
```
