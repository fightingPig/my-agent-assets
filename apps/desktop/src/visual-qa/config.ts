import { PAGE_REGISTRY, type PageId } from "../app/pages";

export type VisualQaPlatform = "macos" | "windows" | "unknown";

export type VisualQaPage = {
  id: PageId;
  title: string;
};

export const VISUAL_QA_PAGES: readonly VisualQaPage[] = PAGE_REGISTRY.map(({ id, title }) => ({
  id,
  title,
}));

const PAGE_IDS = new Set<PageId>(VISUAL_QA_PAGES.map(({ id }) => id));
const PLATFORMS = new Set<VisualQaPlatform>(["macos", "windows", "unknown"]);

export type VisualQaQuery = {
  pageId: PageId;
  platform: VisualQaPlatform;
  warnings: string[];
};

export function parseVisualQaQuery(search: string): VisualQaQuery {
  const params = new URLSearchParams(search);
  const requestedPage = params.get("page");
  const requestedPlatform = params.get("platform");
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

  return { pageId, platform, warnings };
}
