# My Agent Assets v0.1.1-beta.3

## Beta fixes

- Windows Git and GitHub CLI child processes now run without opening a console
  window, with non-interactive input and bounded execution time.
- Settings save and diagnostic export now require a complete asset center before
  preview or apply. Failed calls cannot leave a partial `.my-agent-assets`
  directory or operation lock behind.
- Dashboard recovery-status read failures are fail-safe and report that writes
  are blocked instead of showing a healthy state.
- Frontend command errors use the redacted structured desktop error contract
  across asset, mount, backup, sync, settings, and target workflows.
- Settings that are not behaviorally implemented are read-only and labeled as
  future capabilities. The displayed version now comes from `app_info`.
- Release CI validates tag and version consistency before building installers.
- Visual QA covers the 13-page baseline plus project editor, uninitialized scan,
  uninitialized settings, and recovery-read failure states on macOS and Windows.

## Packaging

- macOS: Apple Silicon DMG, ad-hoc signed and not notarized.
- Windows: unsigned x64 MSI and NSIS setup executable.

## Manual Windows acceptance

1. Launching the installed application must not open a command window.
2. Git status, project refresh, initialization, and sync preview must not flash a
   child console window.
3. The project editor must remain readable at the minimum supported window size.
4. Before initialization, settings and scan discovery remain readable while all
   write previews are disabled.
