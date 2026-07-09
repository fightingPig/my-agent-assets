# Final Beta Readiness

Date: 2026-07-09

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

## Current Package Evidence

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
