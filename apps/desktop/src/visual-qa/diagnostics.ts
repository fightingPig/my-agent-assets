import type { PageId } from "../app/pages";
import type { VisualQaPlatform } from "./config";

export const OVERFLOW_TOLERANCE = 1;

export type OverflowResults = {
  document: boolean;
  root: boolean;
  appMain: boolean;
  page: boolean;
};

export type VisualQaPageReport = {
  pageId: PageId;
  pageTitle: string;
  viewport: { width: number; height: number };
  platform: VisualQaPlatform;
  overflow: OverflowResults;
  severeIssues: string[];
  warningIssues: string[];
  screenshotPath: string;
};

export type VisualQaSummary = {
  generatedAt: string;
  chromePath: string;
  viteUrl: string;
  totalPages: number;
  totalScreenshots: number;
  severeCount: number;
  warningCount: number;
  results: VisualQaPageReport[];
};

export function createVisualQaSummary(
  metadata: Pick<VisualQaSummary, "generatedAt" | "chromePath" | "viteUrl" | "totalPages">,
  results: VisualQaPageReport[],
): VisualQaSummary {
  return {
    ...metadata,
    totalScreenshots: results.length,
    severeCount: results.reduce((sum, result) => sum + result.severeIssues.length, 0),
    warningCount: results.reduce((sum, result) => sum + result.warningIssues.length, 0),
    results,
  };
}

function hasHorizontalOverflow(element: Element | null) {
  return element instanceof HTMLElement
    && element.scrollWidth > element.clientWidth + OVERFLOW_TOLERANCE;
}

function isVisible(element: HTMLElement) {
  const style = getComputedStyle(element);
  const rect = element.getBoundingClientRect();
  return style.display !== "none" && style.visibility !== "hidden" && rect.width > 0 && rect.height > 0;
}

function hasScrollContainer(element: HTMLElement, axis: "x" | "y") {
  let candidate: HTMLElement | null = element;
  while (candidate) {
    const style = getComputedStyle(candidate);
    const overflow = axis === "x" ? style.overflowX : style.overflowY;
    const scrollSize = axis === "x" ? candidate.scrollWidth : candidate.scrollHeight;
    const clientSize = axis === "x" ? candidate.clientWidth : candidate.clientHeight;
    if (/auto|scroll/.test(overflow) && scrollSize > clientSize + OVERFLOW_TOLERANCE) return true;
    candidate = candidate.parentElement;
  }
  return false;
}

function labelFor(element: HTMLElement) {
  if (element.getAttribute("aria-label")) return element.getAttribute("aria-label")!;
  if (element.className && typeof element.className === "string") {
    return `.${element.className.trim().split(/\s+/).join(".")}`;
  }
  return element.tagName.toLocaleLowerCase();
}

function inspectPanels(appMain: HTMLElement, severeIssues: string[]) {
  const panels = document.querySelectorAll<HTMLElement>(
    ".asset-inspector, .project-inspector, .master-inspector-panel, .content-preview-panel",
  );
  for (const panel of panels) {
    if (!isVisible(panel)) continue;
    const rect = panel.getBoundingClientRect();
    if (rect.width < 180 || rect.height < 120) {
      severeIssues.push(`${labelFor(panel)} collapsed to ${Math.round(rect.width)}x${Math.round(rect.height)}.`);
    }
    if (rect.right > appMain.getBoundingClientRect().right + OVERFLOW_TOLERANCE && !hasScrollContainer(panel, "x")) {
      severeIssues.push(`${labelFor(panel)} extends beyond app-main without horizontal scrolling.`);
    }
  }
}

function inspectClipping(appMain: HTMLElement, severeIssues: string[], warningIssues: string[]) {
  const targets = document.querySelectorAll<HTMLElement>(
    "table, pre, .preview-table, .side-by-side-diff, .settings-controls",
  );
  const mainRect = appMain.getBoundingClientRect();

  for (const target of targets) {
    if (!isVisible(target)) continue;
    const rect = target.getBoundingClientRect();
    const label = labelFor(target);
    const ownHorizontalOverflow = target.scrollWidth > target.clientWidth + OVERFLOW_TOLERANCE;
    const ownVerticalOverflow = target.scrollHeight > target.clientHeight + OVERFLOW_TOLERANCE;

    if ((rect.left < mainRect.left - OVERFLOW_TOLERANCE || rect.right > mainRect.right + OVERFLOW_TOLERANCE)
      && !hasScrollContainer(target, "x")) {
      severeIssues.push(`${label} is clipped horizontally without a scroll container.`);
    } else if (ownHorizontalOverflow && !hasScrollContainer(target, "x")) {
      severeIssues.push(`${label} has unhandled horizontal overflow.`);
    }

    const overflowY = getComputedStyle(target).overflowY;
    if (ownVerticalOverflow && overflowY === "hidden" && !hasScrollContainer(target.parentElement ?? target, "y")) {
      severeIssues.push(`${label} clips vertical content without a scroll container.`);
    } else if (rect.bottom > window.innerHeight + OVERFLOW_TOLERANCE && !hasScrollContainer(target, "y")) {
      warningIssues.push(`${label} extends below the viewport and relies on page scrolling.`);
    }
  }
}

function inspectLocalScrolling(severeIssues: string[]) {
  const localScrollRegions = document.querySelectorAll<HTMLElement>(
    ".asset-list, .asset-inspector-content, .project-list-dense, .project-inspector-content, .master-select-list, .master-inspector-panel",
  );
  for (const region of localScrollRegions) {
    if (!isVisible(region) || region.scrollHeight <= region.clientHeight + OVERFLOW_TOLERANCE) continue;
    if (!/auto|scroll/.test(getComputedStyle(region).overflowY)) {
      severeIssues.push(`${labelFor(region)} needs vertical scrolling but is not scrollable.`);
    }
  }
}

export function collectVisualQaReport(input: {
  pageId: PageId;
  pageTitle: string;
  platform: VisualQaPlatform;
  initialWarnings?: readonly string[];
}): VisualQaPageReport {
  const root = document.getElementById("root");
  const appMain = document.querySelector<HTMLElement>(".app-main");
  const page = appMain?.lastElementChild;
  if (!root || !appMain || !page) throw new Error("Visual QA shell did not render completely.");

  const overflow = {
    document: hasHorizontalOverflow(document.documentElement),
    root: hasHorizontalOverflow(root),
    appMain: hasHorizontalOverflow(appMain),
    page: hasHorizontalOverflow(page),
  };
  const severeIssues: string[] = [];
  const warningIssues = [...(input.initialWarnings ?? [])];

  for (const [scope, overflowing] of Object.entries(overflow)) {
    if (overflowing) severeIssues.push(`${scope} has horizontal overflow greater than ${OVERFLOW_TOLERANCE}px.`);
  }

  inspectPanels(appMain, severeIssues);
  inspectClipping(appMain, severeIssues, warningIssues);
  inspectLocalScrolling(severeIssues);

  return {
    pageId: input.pageId,
    pageTitle: input.pageTitle,
    viewport: { width: window.innerWidth, height: window.innerHeight },
    platform: input.platform,
    overflow,
    severeIssues: [...new Set(severeIssues)],
    warningIssues: [...new Set(warningIssues)],
    screenshotPath: "",
  };
}
