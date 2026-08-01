# My Agent Assets v0.1.1-beta.2

## Beta scope

- Windows release builds use the GUI subsystem, so starting the installed app does not open a separate console window.
- Project management keeps the project list and inspector readable while the add/edit/remove management panel is open.
- Tauri command failures are surfaced as safe, structured Chinese messages instead of being reported as an unavailable runtime.
- Project management and import write previews are blocked until the asset center is initialized. Read-only discovery remains available.
- An asset-center initialization check runs before project and batch-import write-side operations, before any operation lock or registry state can be created.

## Known beta.1 cleanup note

Some beta.1 failure paths may have left an empty `~/.my-agent-assets/locks` directory behind. Beta.2 does not remove user directories automatically. After confirming that the directory is empty and no My Agent Assets operation is running, it may be removed manually.

## Packaging

- macOS: Apple Silicon DMG, ad-hoc signed and not notarized.
- Windows: unsigned x64 MSI and NSIS setup executable.
- This is a public prerelease for controlled validation and is not V1 Stable.
