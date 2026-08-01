# My Agent Assets v0.1.1-beta.4

Cross-platform beta candidate for the renamed asset center.

- Default asset center: `~/.my-agent-assets-data`.
- The application does not automatically read or migrate the legacy
  `~/.my-agent-assets` directory. Existing legacy data and runtime mounts are
  left untouched; copy assets and recreate mounts manually before removing the
  legacy directory.
- The recommended remote repository slug is `my-agent-assets-data`.
  Configure the actual GitHub owner and SSH or HTTPS URL in Settings, then use
  the existing preview and confirmation flow. The Git remote alias remains
  `origin`.
- Push remains restricted to verified GitHub Private repositories unless the
  explicit public-remote setting is enabled.
- macOS: Apple Silicon DMG, ad-hoc signed and not notarized.
- Windows: x64 MSI and NSIS installers, unsigned for beta validation.

This is a prerelease for controlled validation and is not V1 Stable.
