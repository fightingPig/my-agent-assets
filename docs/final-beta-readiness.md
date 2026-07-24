# Final Beta Readiness

Date: 2026-07-24

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

## Current Package Evidence

Latest automated macOS package verification:

- source commit: `9cd17bc` (`codex/final-product-v1-next`)
- build command: `cd apps/desktop && npm run build`
- app signature: `codesign --verify --deep --strict` passed
- DMG integrity: `hdiutil verify` passed
- DMG SHA-256: `13b639723530a7bf3b59d5d7536402ce0bbd123f06d61447224758f83bc116d4`
- Tauri dev smoke: passed with `MY_AGENT_ASSETS_HOME` set to an empty
  disposable path; startup did not create or write that path
- verification date: 2026-07-24

Historical installed-app native evidence on 2026-07-11:

- the candidate app was installed from the generated bundle into
  `~/Applications/My Agent Assets.app` and passed `codesign --verify --deep --strict`
  at that installed location
- the macOS Accessibility tree exposed the native close, minimize, and zoom
  controls, the sidebar navigation, and readable empty-state content; it did
  not expose React-rendered traffic-light controls
- Computer Use cannot retain `MY_AGENT_ASSETS_HOME` when its bridge relaunches
  the target application, so that bridge result is installation/window-shell
  evidence only. Fake-HOME workflow validation remains covered by the CLI/E2E
  suite and needs a human desktop session for final installed-app flows.

Windows preflight evidence on 2026-07-11:

- `cargo check -p my-agent-assets-core --tests --target x86_64-pc-windows-msvc`
  passed, including compilation of the Windows-only junction regression test.
- Desktop cross-check reached Tauri's Windows resource-build stage after the
  configuration feature allowlist was corrected. It cannot complete on this
  macOS host because `llvm-rc` is unavailable; the Windows GitHub Actions
  workflow is configured to perform the native core/desktop tests and
  installer build.

This is automated package evidence only. It does not replace installation,
upgrade, launch, workflow, or accessibility manual acceptance on the exact
candidate build.

The current macOS artifact locations used by the existing test plan are:

- `target/release/bundle/macos/My Agent Assets.app`
- `target/release/bundle/dmg/My Agent Assets_0.1.0_aarch64.dmg`

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

1. Review the latest 26 Visual QA screenshots manually.
2. Run the macOS checklist from `docs/manual-acceptance-checklist.md` against
   the exact package intended for Beta.
3. Run Gatekeeper validation on another Apple Silicon Mac for ad-hoc,
   non-notarized builds.
4. Complete the Windows Stable checklist before claiming cross-platform V1
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
