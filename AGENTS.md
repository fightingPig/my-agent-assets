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
- Backup
- Sync
- Settings

Out of scope for V1:

- Codex assets
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

The desktop app should eventually call the Rust core directly through Tauri commands.

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

- Search
- Preview Data
- Quick Action
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
11. 备份恢复
12. 同步
13. 设置

For this phase:

- Use mock data only.
- Do not access real Claude data.
- Do not add new Tauri commands.
- Do not change Rust business logic.
- Do not modify window config.
- Do not modify AppShell window strategy.

## Current Frontend Structure

The desktop frontend currently uses this structure:

```txt
src/
├── App.tsx
├── app/
│   ├── CurrentPage.tsx
│   └── pages.ts
├── components/
│   ├── assets/
│   │   └── AssetCenterLayout.tsx
│   ├── shell/
│   │   ├── AppFrame.tsx
│   │   ├── MacOverlayDragArea.tsx
│   │   ├── Sidebar.tsx
│   │   └── PageHeader.tsx
│   └── ui/
│       └── StaticActionButton.tsx
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
└── styles.css
```

`App.tsx` orchestrates platform state, page selection, `app_info`, and page composition.

`app/pages.ts` owns page metadata and primary navigation visibility.

`app/CurrentPage.tsx` maps page IDs to page components.

Shell components own the frozen window layout and navigation frame.

Page components may use local static data during the current frontend-only phase.

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

## Codex Task: Create This File And Freeze Foundation

Use this task prompt when asking Codex to create or update this file:

```text
Create or update AGENTS.md at the repository root using the provided content.

After AGENTS.md is created, treat AppShell, platform window behavior, macOS overlay, Windows native titlebar behavior, Sidebar dimensions, PageHeader placement, and drag-region rules as frozen.

Do not modify window configuration, MacOverlayDragArea, Sidebar base layout, PageHeader base layout, app-main padding, or platform-specific titlebar strategy unless explicitly instructed.
```

## Codex Task: Next Step After AGENTS.md

Use this task prompt after AGENTS.md is committed:

```text
Read AGENTS.md first.

Then perform a behavior-preserving frontend structure refactor.

Goal:
Split the current large App.tsx into shell components, UI components, pages, and mock data modules.

Allowed changes:
- Move existing AppShell code into components/shell.
- Move current Dashboard content into pages/DashboardPage.tsx.
- Move reusable visual pieces into components/ui.
- Keep current visual behavior unchanged.
- Keep macOS drag behavior unchanged.
- Keep Windows native titlebar behavior unchanged.
- Keep current mock data behavior unchanged.

Not allowed:
- Do not modify tauri.conf.json.
- Do not modify tauri.macos.conf.json.
- Do not modify capabilities/default.json unless tests prove it is broken.
- Do not change MacOverlayDragArea behavior.
- Do not redesign the app.
- Do not add new pages yet.
- Do not add real Claude data access.
- Do not add new Tauri commands.
```
