import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { CurrentPage } from "../app/CurrentPage";
import { PAGE_REGISTRY } from "../app/pages";
import entrySource from "../visual-qa.tsx?raw";
import { isExpectedVisualQaReport } from "../../scripts/visual-qa-readiness.mjs";
import { parseVisualQaQuery, VISUAL_QA_PAGES } from "./config";
import { collectVisualQaReport, createVisualQaSummary, OVERFLOW_TOLERANCE } from "./diagnostics";

afterEach(() => cleanup());

const appInfo = {
  name: "My Agent Assets",
  version: "0.1.0-visual-qa",
  platform: "macos",
  arch: "arm64",
  backendReady: true,
};

function setSize(element: Element, sizes: Partial<Record<"clientWidth" | "clientHeight" | "scrollWidth" | "scrollHeight", number>>) {
  for (const [property, value] of Object.entries(sizes)) {
    Object.defineProperty(element, property, { configurable: true, value });
  }
}

describe("Visual QA harness", () => {
  it("accepts only reports matching the expected page, platform, and viewport", () => {
    const expected = { pageId: "skills", platform: "macos", width: 1440, height: 900 };
    const report = {
      pageId: "skills",
      platform: "macos",
      viewport: { width: 1440, height: 900 },
    };

    expect(isExpectedVisualQaReport(report, expected)).toBe(true);
    expect(isExpectedVisualQaReport(null, expected)).toBe(false);
    expect(isExpectedVisualQaReport({}, expected)).toBe(false);
    expect(isExpectedVisualQaReport({ ...report, pageId: "commands" }, expected)).toBe(false);
    expect(isExpectedVisualQaReport({ ...report, platform: "windows" }, expected)).toBe(false);
    expect(isExpectedVisualQaReport({ ...report, viewport: { width: 1180, height: 900 } }, expected)).toBe(false);
    expect(isExpectedVisualQaReport({ ...report, viewport: { width: 1440, height: 760 } }, expected)).toBe(false);
  });

  it("covers every registered V1 page in registry order", () => {
    expect(VISUAL_QA_PAGES).toEqual(PAGE_REGISTRY.map(({ id, title }) => ({ id, title })));
    expect(VISUAL_QA_PAGES).toHaveLength(13);
  });

  it("parses pages and platforms with macOS defaults and warnings", () => {
    expect(parseVisualQaQuery("?page=asset-detail&platform=windows")).toEqual({
      pageId: "asset-detail",
      platform: "windows",
      warnings: [],
    });
    expect(parseVisualQaQuery("?page=missing&platform=linux")).toEqual({
      pageId: "dashboard",
      platform: "macos",
      warnings: [
        "Unknown page 'missing', using dashboard.",
        "Unknown platform 'linux', using macos.",
      ],
    });
    expect(parseVisualQaQuery("")).toEqual({ pageId: "dashboard", platform: "macos", warnings: [] });
  });

  it("renders hidden detail pages directly with mock application information", () => {
    const { rerender } = render(<CurrentPage activePage="asset-detail" appInfo={appInfo} demoMode />);
    expect(screen.getByRole("heading", { name: "SKILL.md 内容预览" })).toBeInTheDocument();
    rerender(<CurrentPage activePage="project-detail" appInfo={appInfo} demoMode />);
    expect(screen.getByRole("heading", { name: "已挂载资产" })).toBeInTheDocument();
  });

  it("uses a one-pixel overflow tolerance and emits the report schema", () => {
    document.body.innerHTML = '<div id="root"><main class="app-main"><header></header><section class="qa-page"></section></main></div>';
    const root = document.getElementById("root")!;
    const main = document.querySelector(".app-main")!;
    const page = document.querySelector(".qa-page")!;
    setSize(document.documentElement, { clientWidth: 1000, scrollWidth: 1001 });
    setSize(root, { clientWidth: 900, scrollWidth: 900 });
    setSize(main, { clientWidth: 800, scrollWidth: 800 });
    setSize(page, { clientWidth: 700, scrollWidth: 702 });

    const report = collectVisualQaReport({ pageId: "dashboard", pageTitle: "首页", platform: "macos" });

    expect(OVERFLOW_TOLERANCE).toBe(1);
    expect(report).toMatchObject({
      pageId: "dashboard",
      pageTitle: "首页",
      platform: "macos",
      overflow: { document: false, root: false, appMain: false, page: true },
      screenshotPath: "",
    });
    expect(report.severeIssues).toContain("page has horizontal overflow greater than 1px.");
    expect(report.warningIssues).toEqual([]);
  });

  it("builds run-level summary metadata and issue totals", () => {
    const report = {
      pageId: "dashboard" as const,
      pageTitle: "首页",
      viewport: { width: 1440, height: 900 },
      platform: "macos" as const,
      overflow: { document: false, root: false, appMain: false, page: false },
      severeIssues: ["severe"],
      warningIssues: ["warning"],
      screenshotPath: "/tmp/dashboard.png",
    };
    expect(createVisualQaSummary({
      generatedAt: "2026-06-23T00:00:00.000Z",
      chromePath: "/Applications/Google Chrome",
      viteUrl: "http://127.0.0.1:54321",
      totalPages: 13,
    }, [report])).toEqual({
      generatedAt: "2026-06-23T00:00:00.000Z",
      chromePath: "/Applications/Google Chrome",
      viteUrl: "http://127.0.0.1:54321",
      totalPages: 13,
      totalScreenshots: 1,
      severeCount: 1,
      warningCount: 1,
      results: [report],
    });
  });

  it("does not reuse App.tsx or invoke a Tauri command", () => {
    expect(entrySource).not.toMatch(/from ["']\.\/App["']/);
    expect(entrySource).not.toContain("invoke(");
    expect(entrySource).not.toContain("app_info");
    expect(entrySource).toContain("<AppFrame");
    expect(entrySource).toContain("<CurrentPage");
  });
});
