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
