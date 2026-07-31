# Design QA — UI Asset Edition

Date: 2026-08-01  
Branch: `codex/ui-asset-edition`

## Source of truth

- Dashboard: `apps/desktop/docs/ui-assets/reference-dashboard.png`
- Skill mount matrix: `apps/desktop/docs/ui-assets/reference-skill-mount-matrix.png`
- MCP mount matrix: `apps/desktop/docs/ui-assets/reference-mcp-mount-matrix.png`
- User-provided CC-Switch reference: `/var/folders/7q/8xcgl2tn2xd42tq6k8pz_pg80000gn/T/codex-clipboard-4881167d-8d2e-4a98-990f-957521b04369.png`

The three canonical source images are all `1586×992` pixels. The implementation
was rendered in the in-app browser at a `1586×992` CSS-pixel viewport. The
browser reported device pixel ratio `2`; its screenshot API produced
`1586×992` images, so no resizing or crop normalization was applied to the
full-view comparisons.

State used for comparison: macOS shell, explicit Visual QA demo mode, default
selected asset, one staged mount draft. The implementation intentionally omits
image-generated traffic lights because the frozen production shell requires
native macOS controls and forbids React-rendered window controls.

## Comparison evidence

Full view, source on the left and implementation on the right:

- `apps/desktop/artifacts/design-qa/comparison-dashboard.png`
- `apps/desktop/artifacts/design-qa/comparison-skills.png`
- `apps/desktop/artifacts/design-qa/comparison-mcp.png`

Focused mount-region comparisons:

- `apps/desktop/artifacts/design-qa/comparison-skills-mount-region.png`
- `apps/desktop/artifacts/design-qa/comparison-mcp-mount-region.png`

Implementation captures:

- `apps/desktop/artifacts/design-qa/implementation-dashboard-1586x992.png`
- `apps/desktop/artifacts/design-qa/implementation-skills-1586x992.png`
- `apps/desktop/artifacts/design-qa/implementation-mcp-1586x992.png`
- `apps/desktop/artifacts/design-qa/implementation-mounts-1586x992.png`

## Visible comparison assessment

- Typography: the implementation preserves the reference hierarchy with a
  `32px` page title, `18px` section title, `15px` primary body text and
  `12–13px` compact metadata. Chinese and Latin text use the system-native
  stack defined in `tokens.css`; no text was reduced below the project baseline.
- Layout: sidebar, page gutters, overview metrics, master-detail proportions,
  dividers and inspector hierarchy align with the references. The direct mount
  matrix keeps user level first, followed by the managed-project registry.
- Color and surfaces: canvas, sidebar, white surfaces, violet selection, green
  success, amber pending and restrained borders match the selected light,
  readability-first direction.
- Provider imagery: Claude Code and Codex use packaged static brand marks. The
  marks are real assets, not CSS drawings or text substitutes, and their icon
  containers carry mounted, unmounted and pending states.
- Copy: production-safe wording remains authoritative where mock copy differed.
  The standalone workflow is renamed `挂载预览`; MCP copy explicitly describes
  precise JSON/TOML patch behavior and preview-before-apply safety.
- Responsiveness: the provider columns remain visible at `1180×760`; pages use
  local scrolling instead of shrinking typography. Both macOS and Windows shell
  variants were captured at `1440×900` and `1180×760`.

## Interaction and runtime verification

Verified in the in-app browser:

1. Skills search filtered the four-item list to the React result and restored
   the full list when cleared.
2. A `my-app · Codex` provider toggle changed staged drafts from one to two; it
   did not write immediately.
3. `挂载预览` generated two executable previews. Explicit confirmation cleared
   the queue and reported `执行完成：成功 2 项，跳过 0 项。` in demo mode.
4. `新增 MCP` opened the canonical MCP editor and exposed a labeled close
   control.
5. Browser console check returned zero warnings and zero errors.

Automated evidence:

- TypeScript: passed.
- Frontend tests: `12` files, `106` tests passed.
- Visual QA: `68` macOS/Windows screenshots; severe `0`, warnings `0`.
- Screenshot matrix: dashboard plus all production pages and explicit
  uninitialized/error variants at `1440×900` and `1180×760`.

## Comparison history

The initial post-implementation review found five P2 visual issues: default
button borders on dashboard project rows, a clipped Codex column at the compact
desktop breakpoint, a mismatched Claude mark, project ordering that did not
keep registry order, and an MCP information banner that shifted the core layout
away from the reference. All five were corrected before the final comparison.

No open P0, P1 or P2 visual mismatch remains. Small differences in fixture
counts and status text are intentional state differences, not styling defects.
The native-window-control deviation is required by the frozen shell contract.

final result: passed
