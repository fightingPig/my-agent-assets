# My Agent Assets UI System

Version: `1.0.0`

This is the reusable UI asset behind the desktop edition. It is designed for
local-first management apps: dense enough for operational work, calm enough to
scan for long periods, and explicit about every persistent write.

## Canonical visual references

- `reference-dashboard.png` — shell, navigation, overview density, semantic color balance.
- `reference-skill-mount-matrix.png` — asset master-detail layout and direct provider toggles.
- `reference-mcp-mount-matrix.png` — configuration asset variant and preview-first MCP copy.

The coded system is the source of truth for exact tokens and responsive
behavior. The PNGs preserve the intended visual direction.

## Reusable entry points

| Asset | Purpose | Portability |
| --- | --- | --- |
| `src/ui-assets/tokens.css` | Color, typography, spacing, shape, elevation, layout, focus and motion tokens | Copy directly |
| `src/ui-assets/contracts.ts` | Small structural DTOs for asset/provider/mount UI | Copy directly |
| `src/ui-assets/components/ProviderMark.tsx` | Claude Code and Codex brand marks | Copy with the icon package |
| `src/ui-assets/mounts/model.ts` | Pure location grouping and mount-state derivation | Copy directly |
| `src/ui-assets/mounts/MountDraftContext.tsx` | Preview-first draft queue | Copy directly |
| `src/ui-assets/mounts/MountMatrix.tsx` | This app's Tauri adapter for the reusable pattern | Adapt the data-loading imports |
| `src/ui-assets/edition.css` | Complete page-pattern skin for this application | Copy selectively or as a baseline |

Brand assets come from `@lobehub/icons-static-svg`; it ships static SVG files
without a runtime UI framework. The product mark remains app-owned.

## Design contract

### Foundations

- Canvas `#FCFCFE`, sidebar `#F6F7FA`, surface `#FFFFFF`.
- Accent `#6253E8`; semantic colors are reserved for state, never decoration.
- Sidebar width `250px`; page padding `34px 36px 36px`.
- Page title `32px`, section title `18px`, primary body `15px`, supporting UI `13px`, minimum `12px` only for compact metadata.
- Surface radius `8–10px`; shadows are exceptional. Prefer borders and spacing.
- Every keyboard focus state uses the shared violet focus ring.

### Page patterns

1. **Overview** — four compact metrics, divider-led activity and project lists,
   then a full-width system-status section.
2. **Asset center** — searchable master list on the left; selected asset,
   location matrix, metadata and source preview on the right.
3. **Direct mount matrix** — one user row plus the managed project list. Provider
   icons are the switches. Commands expose Claude Code only.
4. **Mount preview** — a global read-first queue plus current mount overview.
   The page never asks the user to reselect asset, location or provider.
5. **Operational workflow** — stepper, preview table, impact warning and explicit
   confirmation. Persistent writes never happen from a decorative toggle.
6. **Master-detail operations** — conflicts and backup history keep list context
   visible while inspecting detail.
7. **Settings** — two-column sections at desktop widths, local scrolling at the
   compact desktop breakpoint.

### Mount interaction contract

```text
Select asset → inspect all user/project locations → toggle provider icon
→ stage draft → backend preview → explicit confirmation → apply
```

- A provider toggle changes only frontend draft state.
- Preview resolves authorized target IDs and shows exact planned effects.
- Apply must submit the preview ID, generation time and original fingerprint.
- MCP changes patch only the selected JSON/TOML section.
- Unmount preserves the canonical asset.
- Commands never expose a Codex target.

## Lowest-cost reuse

For another React app:

1. Copy `tokens.css`, `contracts.ts`, `ProviderMark.tsx`, and `mounts/model.ts`.
2. Install `@lobehub/icons-static-svg` at the version recorded in the desktop
   `package.json`.
3. Import `tokens.css` before the app stylesheet.
4. Map the new app's backend DTOs to `UiMountTarget` and `UiMountBinding`.
5. Copy only the page-pattern sections needed from `edition.css`.
6. Keep app-specific data loading in an adapter; do not put filesystem or
   business logic in the reusable UI layer.

For a non-React app, reuse `tokens.css` and the PNG references, then reproduce
the seven page patterns while preserving the mount interaction contract.

## Responsive and accessibility baseline

- Required desktop checks: `1440×900` and `1180×760`, macOS and Windows.
- At compact width the list gives space to the inspector; both provider columns
  remain visible without shrinking text below the readability baseline.
- Inputs and actions are at least `40px` high; provider icon switches are
  `42×42px`.
- State is conveyed by label and icon in addition to color.
- Interactive controls are keyboard reachable and show a visible focus ring.
- Reduced-motion preference collapses transition duration to zero.

## Non-negotiables

- No login, account, billing, team or cloud-workspace UI.
- No mock fallback in normal production runtime.
- No React filesystem writes and no frontend-composed target paths.
- No automatic historical restore action.
- No custom window controls or changes to the frozen AppShell strategy.
- No silent overwrite, automatic rename, or apply without a fresh preview.
