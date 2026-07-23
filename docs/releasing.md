# Release Process

## Create a release

1. Update all application versions:

- `apps/desktop/package.json`
- `apps/desktop/src-tauri/tauri.conf.json`

2. Commit changes.

3. Create and push a tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

4. GitHub Actions builds:

- Desktop application
  - macOS DMG
  - Windows installer
  - Linux AppImage/DEB
- CLI binary
  - macOS arm64
  - macOS x64
  - Windows x64
  - Linux x64

5. Review the generated draft Release and publish it.

## Automatic updater

The project reserves Tauri updater integration for a later phase.

Before enabling it:

- configure updater signing keys
- configure public update endpoint
- ensure clients can access releases

Private GitHub repositories should use a dedicated update service or authenticated update flow.
