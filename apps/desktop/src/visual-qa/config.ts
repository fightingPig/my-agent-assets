import { PAGE_REGISTRY, type PageId } from "../app/pages";

export type VisualQaPlatform = "macos" | "windows" | "unknown";

export type VisualQaPage = {
  id: PageId;
  title: string;
};

export type VisualQaState =
  | "default"
  | "project-editor"
  | "uninitialized"
  | "recovery-error";

export type VisualQaCase = {
  id: string;
  pageId: PageId;
  title: string;
  state: VisualQaState;
};

export const VISUAL_QA_PAGES: readonly VisualQaPage[] = PAGE_REGISTRY.map(({ id, title }) => ({
  id,
  title,
}));

export const VISUAL_QA_CASES: readonly VisualQaCase[] = [
  ...VISUAL_QA_PAGES.map((page) => ({
    id: page.id,
    pageId: page.id,
    title: page.title,
    state: "default" as const,
  })),
  { id: "projects-project-editor", pageId: "projects", title: "项目列表 · 编辑面板", state: "project-editor" },
  { id: "scan-uninitialized", pageId: "scan", title: "扫描导入 · 未初始化", state: "uninitialized" },
  { id: "settings-uninitialized", pageId: "settings", title: "设置 · 未初始化", state: "uninitialized" },
  { id: "dashboard-recovery-error", pageId: "dashboard", title: "首页 · 恢复状态异常", state: "recovery-error" },
];

const PAGE_IDS = new Set<PageId>(VISUAL_QA_PAGES.map(({ id }) => id));
const PLATFORMS = new Set<VisualQaPlatform>(["macos", "windows", "unknown"]);
const STATES = new Set<VisualQaState>(["default", "project-editor", "uninitialized", "recovery-error"]);
const PAGE_STATES = new Set(VISUAL_QA_CASES.map((entry) => `${entry.pageId}:${entry.state}`));

export type VisualQaQuery = {
  pageId: PageId;
  platform: VisualQaPlatform;
  state: VisualQaState;
  warnings: string[];
};

export function parseVisualQaQuery(search: string): VisualQaQuery {
  const params = new URLSearchParams(search);
  const requestedPage = params.get("page");
  const requestedPlatform = params.get("platform");
  const requestedState = params.get("state");
  const warnings: string[] = [];

  const pageId = requestedPage && PAGE_IDS.has(requestedPage as PageId)
    ? requestedPage as PageId
    : "dashboard";
  if (requestedPage && pageId === "dashboard" && requestedPage !== "dashboard") {
    warnings.push(`Unknown page '${requestedPage}', using dashboard.`);
  }

  const platform = requestedPlatform && PLATFORMS.has(requestedPlatform as VisualQaPlatform)
    ? requestedPlatform as VisualQaPlatform
    : "macos";
  if (requestedPlatform && platform === "macos" && requestedPlatform !== "macos") {
    warnings.push(`Unknown platform '${requestedPlatform}', using macos.`);
  }

  const knownState = requestedState && STATES.has(requestedState as VisualQaState)
    ? requestedState as VisualQaState
    : "default";
  const state = PAGE_STATES.has(`${pageId}:${knownState}`) ? knownState : "default";
  if (requestedState && state !== requestedState) {
    warnings.push(`State '${requestedState}' is not available for page '${pageId}', using default.`);
  }

  return { pageId, platform, state, warnings };
}
