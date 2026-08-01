# AGENTS.md

## Project

My Agent Assets is a local-first desktop GUI for managing Claude agent assets.

Current V1 scope:

- Skills
- Commands
- MCP Servers
- Projects
- Scan
- Mount
- Conflict
- Backup History
- Sync
- Settings

Current final product provider scope:

- Claude Code and Codex runtime source discovery
- Canonical Skill import from Claude Code, Codex, and approved custom sources
- Canonical Command import from Claude-compatible sources
- Canonical MCP import from Claude JSON and Codex TOML
- Compatible target mount through Claude/Codex/custom adapters

Out of scope:

- Codex AGENTS.md assets
- Codex custom commands
- Codex Command assets or Command targets
- Codex OAuth token management
- Cursor rules
- Hooks bundle
- Prompt marketplace
- Team collaboration
- Cloud account

## No Login / No Account

My Agent Assets V1 does not require login.

Do not implement:

- Login page
- Signup page
- Account center
- User avatar menu
- OAuth flow
- GitHub login
- Cloud sync login
- Team workspace
- Subscription or billing UI

The app is local-first.

All core features must work without an account.

Git sync, if shown in the UI, means repository-based sync using the user’s local Git configuration.

Do not design Git sync as account login, cloud account binding, or GitHub OAuth login.

Settings must not include account login, cloud account, billing, team, or subscription sections.

Do not add authentication dependencies or authentication-related Tauri commands.

Connection status in the UI should mean local environment status, local Git repository status, or preview/mock status. It must not mean user account status.

## Tech Stack

Use a single codebase:

- Tauri 2
- React
- TypeScript
- Vite
- Rust backend

The desktop app and CLI must call the same Rust core directly. Tauri commands
and CLI handlers are transport adapters, not separate business implementations.

Do not implement business logic in React.

Do not use React to directly manipulate the filesystem.

## Foundation Freeze

The following foundation is frozen.

Do not change it unless explicitly instructed.

### Window Strategy

#### macOS

- Use native macOS window controls.
- Do not render React traffic lights.
- Use `tauri.macos.conf.json` as the macOS-only overlay config source.
- Keep:
  - `title: ""`
  - `titleBarStyle: "Overlay"`
  - `decorations: true`
- Render one 28px `MacOverlayDragArea`.
- The drag area must be only for dragging, not business UI.

#### Windows

- Use the native Windows title bar.
- Do not render macOS overlay.
- Do not render custom minimize / maximize / close buttons.
- Do not leave a 28px top offset.
- Windows content height must be `100vh`.

### AppShell Structure

Keep this structure:

```tsx
<div className="app-frame">
  {platform === "macos" && <MacOverlayDragArea />}
  <div className="app-body">
    <Sidebar />
    <main className="app-main">
      <PageHeader />
      <PageContent />
    </main>
  </div>
</div>
```

Do not restore:

- `.desktop-bg`
- fake inner window
- fake titlebar
- standalone `.app-header`
- centered product name in titlebar
- React-rendered traffic lights
- React-rendered Windows controls

### Layout Tokens

Keep these values:

- Sidebar width: `250px`
- macOS overlay height: `28px`
- app-main padding: `34px 36px 36px`
- macOS overlay grid: `250px 1fr`
- Sidebar background: `#F6F7FA`
- Main background: `#FCFCFE`
- Accent color: `#6253E8`

### Drag Region Rules

`MacOverlayDragArea` must:

- Use `data-tauri-drag-region`
- Use `WebkitAppRegion: "drag"`
- Call `getCurrentWindow().startDragging()` on valid pointer down
- Ignore:
  - `button`
  - `input`
  - `textarea`
  - `select`
  - `a`
  - `[data-no-drag="true"]`

Required permission:

- `core:window:allow-start-dragging`

Do not use:

- global `.app-frame * { -webkit-app-region: no-drag; }`
- runtime `setDecorations`
- custom React window buttons

Interactive controls must be no-drag:

- Provider switch
- Sidebar nav item
- Dropdown menu
- Buttons
- Inputs

## Page Development Rules

After foundation freeze, implement pages without changing AppShell or platform window behavior.

Pages to implement:

1. Dashboard / 首页
2. Skills 列表
3. Commands 列表
4. MCP Servers 列表
5. Asset 详情页
6. 项目列表
7. 项目详情
8. 扫描导入
9. 挂载管理
10. 冲突处理
11. 备份历史
12. 同步
13. 设置

Production pages must use Tauri data or explicit empty/error states.

Mock data is allowed only in tests, Visual QA, or an explicitly enabled demo mode. It must never be the default production fallback.

Provider-specific discovery and renderer logic belongs in the shared Rust core.
Provider is a runtime source or mount-target adapter, not a separate asset
center. Claude Code and Codex assets import into one canonical asset center.

Codex supports compatible Skill and MCP import/mount workflows. Do not
implement Codex Commands, Codex AGENTS.md assets, or Codex OAuth token
management.

All writes require a preview and explicit confirmation. High-risk operations
must show highlighted impact information, but must not require typed `APPLY`.

The application does not provide automatic historical Restore. It provides
portable/local backup history, file reveal, and a manual restore guide.
Internal operation-journal rollback is allowed only for recovering an
interrupted application transaction.

Do not modify window config or AppShell window strategy while adding provider support.

## V1 Beta Product Rules

### Managed Projects

- The Projects page manages an explicit local project registry. Do not populate
  it by listing arbitrary directories from `~/workspace`, `~/code`, Git
  repositories, `package.json`, or `Cargo.toml`.
- Add projects through a native directory picker. Store a stable project ID,
  display name, normalized path, and the latest asset-health inspection
  summary.
- Reject duplicate paths and parent/child path overlaps.
- Block project path edits and project removal while mounts still reference the
  project. The user must remove those mounts first.
- Removing a managed project removes only the registry entry. It must never
  delete the project directory or canonical assets.
- Project refresh discovers Claude Code and Codex runtime asset markers only.
  The project root is depth `0`; the default maximum depth is `5` and remains
  configurable.
- Project health describes asset-maintenance state, not source-repository Git
  cleanliness.
- Local project paths, mount associations, inspection caches, and advanced
  custom targets are machine-local state and must not be committed to the asset
  Git repository.

### Mount Targets

- Standard user and project targets are derived by the Rust core from asset
  type, location type, provider, project, and MCP scope. React and CLI callers
  must not construct target paths.
- Apply APIs accept an authorized target ID, never an arbitrary frontend path.
- Project-level targets are not manually registered. Manual target
  registration is reserved for advanced non-standard directories or MCP config
  files.
- Commands support Claude Code targets only.
- Skills are canonical directory assets and mount through the platform adapter.
- MCP targets are compiled by patching the target JSON or TOML section. Never
  symlink an entire Claude or Codex live config file.

### Write And Conflict Safety

- Every persistent mutation uses preview plus explicit confirmation. Apply must
  validate the preview ID, freshness, request fingerprint, and current state.
- Scan/import conflicts are resolved only through explicit skip, manual rename,
  or overwrite. Never silently overwrite or automatically rename.
- Production error messages must be safe and redacted. Detailed diagnostics
  belong in the local diagnostic export, not in user-facing errors.
- Interrupted transactions may use the operation journal for internal rollback.
  Historical backups remain history and manual recovery material, not an
  in-app Restore workflow.

### Asset Git Sync

- The asset repository is the Git repository inside the user's asset center.
  It is separate from this application's source-code repository.
- The default asset center is `~/.my-agent-assets-data`. The legacy
  `~/.my-agent-assets` directory is not read or migrated automatically.
- The Git remote alias defaults to `origin`. The recommended remote repository
  slug is `my-agent-assets-data`; the owner and SSH/HTTPS URL must be supplied
  and confirmed by the user.
- Push defaults to verified GitHub private repositories.
- Public remote Push is allowed only after the user explicitly enables the
  setting through preview and confirmation. Public or unknown visibility must
  be highlighted before execution.
- Pull and Push must use preview/apply, detect stale remote state, and stop on
  conflicts. Do not fetch, merge, or resolve conflicts implicitly.
- Push stages only the canonical allowlist. Machine-local project, target, and
  mount state must remain excluded.

## Cross-Platform File Rules

- All filesystem writes and mount operations belong in the shared Rust core and
  must be tested with isolated fake homes.
- Do not assume Unix symlink behavior on Windows.
- Directory mounts on Windows use the platform junction adapter. Creation,
  verification, replacement, and removal must all recognize junctions and must
  not follow or recursively delete the junction target.
- Use native filesystem APIs or argument-array process calls. Never construct
  shell command strings from paths.
- Atomic write and sync behavior must account for Windows file-sharing and
  read-handle restrictions.

## Readability Baseline

- V1 pages use a readability-first desktop hierarchy: page titles around
  `32px`, section titles around `18px`, body text `15-16px`, and secondary,
  table, and code text no smaller than `12-13px`.
- Do not reduce type below this baseline to fit more content. Prefer removing
  duplicated presentation, improving grouping, or adding local scrolling.
- Layout changes must be checked at `1440x900` and `1180x760` for both macOS and
  Windows through Visual QA.

## Static GUI Freeze

The V1 static GUI pages are implemented, and their current page layouts are frozen.

Do not redesign static pages unless explicitly requested.

Visual QA tooling is available and must be run before and after any future layout change.

## Current Frontend Structure

The desktop frontend currently uses this structure:

```txt
src/
├── App.tsx
├── app/
│   ├── CurrentPage.tsx
│   ├── contracts.ts
│   ├── defaults.ts
│   ├── data-api.ts
│   ├── detail-context.ts
│   ├── provider.ts
│   └── pages.ts
├── components/
│   ├── assets/
│   │   └── AssetCenterLayout.tsx
│   ├── shell/
│   │   ├── AppFrame.tsx
│   │   ├── MacOverlayDragArea.tsx
│   │   ├── Sidebar.tsx
│   │   └── PageHeader.tsx
│   ├── targets/
│   │   └── TargetRegistryPanel.tsx
│   └── ui/
│       └── ApplyConfirmationPanel.tsx
├── lib/
│   └── platform.ts
├── pages/
│   ├── DashboardPage.tsx
│   ├── SkillsListPage.tsx
│   ├── CommandsListPage.tsx
│   ├── McpServersListPage.tsx
│   ├── AssetDetailPage.tsx
│   ├── ProjectsListPage.tsx
│   ├── ProjectDetailPage.tsx
│   ├── project-data.ts
│   ├── ScanImportPage.tsx
│   ├── MountManagerPage.tsx
│   ├── ConflictResolverPage.tsx
│   ├── BackupRestorePage.tsx
│   ├── SyncPage.tsx
│   └── SettingsPage.tsx
├── mock-data.ts
├── styles.css
├── visual-qa.tsx
└── visual-qa/
    ├── config.ts
    ├── diagnostics.ts
    └── visual-qa.test.tsx
```

`App.tsx` orchestrates platform state, page selection, `app_info`, and page composition.

`app/pages.ts` owns page metadata and primary navigation visibility.

`app/CurrentPage.tsx` maps page IDs to page components.

`app/contracts.ts` defines the frontend TypeScript DTO boundary for future Tauri/Rust integration.

`app/data-api.ts` wraps Tauri command calls and safe browser fallbacks.

`app/detail-context.ts` defines the local frontend context passed from list inspectors into hidden detail pages.

Shell components own the frozen window layout and navigation frame.

`components/targets/TargetRegistryPanel.tsx` owns project/custom target
registration and removal through shared-core preview/apply commands.

Page components use real Tauri data in production and may use local static fixtures only in tests, Visual QA, or explicit demo mode.

`ApplyConfirmationPanel.tsx` owns ordinary preview-bound button confirmation.
It must not require typed confirmation and must not expose historical Restore
actions.

`visual-qa/` contains reusable static GUI screenshot and layout diagnostics tooling.

## Version Iteration Rules

### Version Branches

- `main` is the latest integrated and accepted baseline. Do not start a new
  version from an older release branch or an unmerged feature branch.
- Create one release branch for each planned product version before version
  work begins.
- Codex-owned release branches use:

  ```text
  codex/release/v<major>.<minor>.<patch>
  ```

  Example:

  ```text
  codex/release/v0.1.2
  ```

- All Beta and RC iterations for the same product version stay on that one
  release branch. Do not create a branch for every prerelease number.
- Urgent fixes based on an already released version use:

  ```text
  codex/hotfix/v<major>.<minor>.<patch>
  ```

- Do not continue the next product version on the previous version branch.
- Do not force-push or rewrite a version branch after any public tag has been
  created from it.

### Semantic Versions And Tags

- Use semantic versioning:
  - patch: compatible fixes only;
  - minor: backward-compatible product capability;
  - major: incompatible persisted-data, contract, or workflow change.
- Prerelease tags use:

  ```text
  v<major>.<minor>.<patch>-beta.<number>
  v<major>.<minor>.<patch>-rc.<number>
  ```

- Stable tags use:

  ```text
  v<major>.<minor>.<patch>
  ```

- Prerelease numbers increase monotonically on the same version branch.
- Never move, overwrite, or reuse a tag that has been pushed publicly. A fix
  after a failed or withdrawn prerelease receives the next prerelease number.
- The tag must point to the exact commit used to build every attached
  installer.

### Version Synchronization

- Before tagging, synchronize the product version in:
  - `apps/desktop/package.json`;
  - `apps/desktop/src-tauri/Cargo.toml`;
  - `apps/desktop/src-tauri/tauri.conf.json`;
  - `crates/core/Cargo.toml`;
  - `crates/cli/Cargo.toml`;
  - generated lock files affected by those changes.
- `tauri.windows.conf.json` may use a Windows-compatible bundle version when a
  prerelease SemVer string is not accepted by MSI or NSIS tooling. The mapping
  must preserve the product version and prerelease sequence, and the
  user-facing installer filename must still use the Git tag.
- The application version returned by `app_info`, the CLI version, Release
  title, tag, and installer names must identify the same product version.
- Version synchronization is a release gate. Do not tag when any version source
  disagrees.

### Iteration Sequence

For each version:

1. Update local `main` and verify it matches `origin/main`.
2. Create the version branch from that exact baseline.
3. Record the version scope and explicit non-goals.
4. Implement changes without mixing unrelated future-version work.
5. Run the required frontend, Rust, Visual QA, and platform validation.
6. Update version fields and release notes.
7. Commit and push the version branch.
8. Create and push the next immutable Beta, RC, or stable tag.
9. Wait for both platform builds and the GitHub Release publish job.
10. Verify installer names, checksums, download URLs, and tag commit.
11. Merge the accepted version branch back into `main`.
12. Start later version work from the updated `main`.

### Release Acceptance

- A Beta is testable but may be unsigned or unnotarized when that limitation is
  clearly stated.
- An RC must have no known release-blocking functional, data-safety, or
  cross-platform packaging defect.
- A stable release requires the complete automated suite plus the documented
  native macOS and Windows acceptance checks.
- Failed required validation stops tagging and publishing.
- A GitHub Actions success without a GitHub Release and downloadable installers
  is not a completed release.

## Desktop Release Rules

- The application source repository may remain public. Its visibility must
  never be used as the asset-repository privacy policy.
- Beta tags matching `v*-beta.*` trigger the cross-platform desktop release
  workflow.
- A downloadable release is complete only after both macOS and Windows builds
  succeed and the installers are attached to a GitHub Prerelease. Actions
  artifacts alone are temporary CI evidence, not a user-facing release.
- Publish an Apple Silicon macOS DMG, a Windows x64 MSI, a Windows x64 NSIS
  setup executable, and `SHA256SUMS.txt`.
- Use stable public asset names containing product, version, platform, and
  architecture.
- Installer names use these exact patterns:

  ```text
  My-Agent-Assets-v<version>-macOS-arm64.dmg
  My-Agent-Assets-v<version>-Windows-x64.msi
  My-Agent-Assets-v<version>-Windows-x64-Setup.exe
  SHA256SUMS.txt
  ```

- `<version>` includes the prerelease suffix when applicable, for example
  `0.1.2-beta.1`.
- Release notes must state signing and notarization status. Do not present
  ad-hoc-signed, unnotarized, or unsigned installers as production-signed.
- The release tag, installers, checksum file, and GitHub Release page must be
  verified before announcing availability.

## Validation Before Full Page Work

Before implementing all pages, verify:

- macOS top 28px drag area can continuously drag the window.
- Windows has no overlay and no 28px top blank space.
- React DOM contains no `.traffic-lights`.
- React DOM contains no `.windows-controls`.
- `tauri dev` works.
- `tauri build` works.
- TypeScript passes.
- Rust tests pass.
