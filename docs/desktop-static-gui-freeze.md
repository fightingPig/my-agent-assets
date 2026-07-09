> **Historical milestone — superseded where scope has evolved**
>
> This document records the static GUI freeze baseline. It does not define the final provider or workflow scope.
>
> The final model uses **one canonical asset center, multiple runtime sources, and multiple compatible mount targets**. Codex-compatible Skills and MCP servers will support discovery, import into the shared canonical asset center, and compatible mounting. Codex Commands and Codex OAuth token management remain prohibited. Final workflow decisions in `docs/final-product-model.md` and `my_agent_assets_final_goal.md` take precedence over historical references to read-only Codex support, typed confirmation, or Restore behavior.

# Desktop Static GUI Freeze

The My Agent Assets V1 desktop static GUI milestone is complete. This document records the frozen frontend baseline before real Tauri and Rust business integration begins.

## Implemented V1 Pages

The page registry contains 13 implemented static pages:

1. Dashboard / 首页
2. Skills
3. Commands
4. MCP Servers
5. Asset Detail / 资产详情
6. Projects / 项目列表
7. Project Detail / 项目详情
8. Scan Import / 扫描导入
9. Mount Manager / 挂载管理
10. Conflict Resolver / 冲突处理
11. Backup History / 备份历史
12. Sync / 同步
13. Settings / 设置

Eleven primary pages are reachable from the Sidebar. Asset Detail and Project Detail are implemented, registered, and reachable from their list-page inspector actions with the currently selected asset or project context, but intentionally have no primary navigation entry.

## Current Interaction Model

Navigation uses local React state rather than React Router. The static GUI permits only presentation-level interactions:

- Sidebar page selection.
- Asset and project search, filtering, and row selection.
- Scan scope selection.
- Mount asset and target selection.
- Conflict and backup-history master-detail selection.
- Conflict resolution preview selection.
- Settings controls were static at freeze time; a later controlled-write milestone enabled local settings save without changing the page layout.

These interactions update only local component state. They do not read or write the filesystem, execute Git operations, scan Claude data, mount assets, compile MCP configuration, create backups, restore data, or synchronize repositories.

## StaticActionButton Rule

At the static-GUI freeze milestone, business-action placeholders used
`StaticActionButton`.

The component exposes visual props only and does not accept event-handler props. It always renders:

- `type="button"`
- `disabled`
- `aria-disabled="true"`
- `data-no-drag="true"`
- the final `NO_DRAG_REGION_STYLE`

Static action buttons must not trigger toast messages, dialogs, Tauri commands, filesystem access, or business operations.

## Visual QA

Run Visual QA from `apps/desktop`:

```bash
npm run qa:visual
```

The runner uses `CHROME_BIN` when provided, otherwise it checks the default macOS Google Chrome path. It starts an isolated Vite server on an available port and generates 13 pages at `1440x900` and `1180x760` for a total of 26 screenshots.

Artifacts are generated under:

```text
apps/desktop/artifacts/visual-qa/
```

The structured report is:

```text
apps/desktop/artifacts/visual-qa/summary.json
```

Generated artifacts are intentionally ignored by Git. Visual QA must be run before and after any explicitly requested static layout change.

### Freeze Validation Baseline

Validated on 2026-06-23:

- TypeScript typecheck passed.
- The full Vitest suite passed at the time of freeze.
- Renderer production build passed.
- Visual QA generated 13 pages and 26 screenshots.
- Visual QA reported 0 severe issues and 0 warnings.

## What Remains Static Or Mocked

Production pages now use Tauri data, empty states, or error states. Static fixtures remain only in tests, Visual QA, and explicit demo mode.

Recent activity has no backend event source yet and therefore displays an empty state in production. Visual QA continues to use deterministic fixtures so frozen layouts remain screenshot-testable. React still does not directly manipulate local files.

## Ready For Tauri And Rust Integration

- `app/pages.ts` provides stable page identity and metadata boundaries.
- `app/CurrentPage.tsx` provides the page composition boundary for future data and command wiring.
- List, inspector, detail, plan, warning, diff, and status surfaces define the static presentation targets for structured DTOs.
- The historical placeholder component was removed during final product
  integration. Production pages omit unimplemented optional actions and use
  explicit Preview/Apply controls for supported writes.
- Typed Tauri wrappers now provide Claude read/apply flows and read-only Codex Skills/MCP discovery. Page components receive structured DTOs and keep filesystem/config parsing in Rust.

Future integration should place filesystem, Git, scan, mount, MCP compile, backup-history, operation recovery, and sync logic in Rust. React should receive structured data and invoke explicit Tauri commands rather than implementing those operations itself.

## Known Limitations

- Asset Detail and Project Detail are reached from list inspector actions rather than primary sidebar navigation.
- Codex support has since expanded to compatible Skill/MCP discovery, import, and mount targets; Codex Commands, AGENTS.md assets, and OAuth token management remain prohibited.
- Destructive apply-style business actions were intentionally disabled at static freeze time. Later controlled-write milestones added Settings save, Scan Import, Mount Manager, Sync, Target Registry, Backup History reveal, and other preview/apply flows without changing the frozen page layout.
- Visual QA currently batch-generates macOS-layout screenshots only.
- Headless Chrome does not validate native Tauri window chrome, macOS traffic lights, or Windows native titlebar behavior.
- Visual QA detects structural overflow and clipping risks, but final product review still requires human inspection on installed macOS and Windows builds.
