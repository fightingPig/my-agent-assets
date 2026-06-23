import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { CurrentPage } from "./app/CurrentPage";
import { getPageById } from "./app/pages";
import { AppFrame } from "./components/shell/AppFrame";
import { PageHeader } from "./components/shell/PageHeader";
import { getPlatformShortcuts } from "./lib/platform";
import type { AppInfo } from "./pages/DashboardPage";
import "./styles.css";
import { parseVisualQaQuery, VISUAL_QA_PAGES, type VisualQaPage } from "./visual-qa/config";
import { collectVisualQaReport, type VisualQaPageReport } from "./visual-qa/diagnostics";

declare global {
  interface Window {
    __VISUAL_QA_READY__?: boolean;
    __VISUAL_QA_MANIFEST__?: readonly VisualQaPage[];
    __VISUAL_QA_REPORT__?: VisualQaPageReport;
  }
}

const query = parseVisualQaQuery(window.location.search);
const page = getPageById(query.pageId);
window.__VISUAL_QA_MANIFEST__ = VISUAL_QA_PAGES;
const appInfo: AppInfo = {
  name: "My Agent Assets",
  version: "0.1.0-visual-qa",
  platform: query.platform,
  arch: query.platform === "macos" ? "arm64" : "x86_64",
  backendReady: true,
};

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AppFrame platform={query.platform} activePage={query.pageId} onPageChange={() => undefined}>
      <PageHeader page={page} shortcuts={getPlatformShortcuts(query.platform)} />
      <CurrentPage activePage={query.pageId} appInfo={appInfo} />
    </AppFrame>
  </StrictMode>,
);

async function finishVisualQa() {
  await document.fonts.ready;
  await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
  window.__VISUAL_QA_REPORT__ = collectVisualQaReport({
    pageId: query.pageId,
    pageTitle: page.title,
    platform: query.platform,
    initialWarnings: query.warnings,
  });
  window.__VISUAL_QA_READY__ = true;
}

void finishVisualQa();
