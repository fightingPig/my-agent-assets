import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { CurrentPage } from "./app/CurrentPage";
import type { AppInfo } from "./app/contracts";
import type {
  AssetDetailContext,
  ConflictResolverContext,
  ProjectDetailContext,
} from "./app/detail-context";
import { getPageById, type PageId } from "./app/pages";
import { AppFrame } from "./components/shell/AppFrame";
import { PageHeader } from "./components/shell/PageHeader";
import {
  getDesktopPlatform,
  isTauriRuntime,
  type DesktopPlatform,
} from "./lib/platform";
import { MountDraftProvider } from "./ui-assets";

function fallbackInfo(platform: DesktopPlatform): AppInfo {
  return {
    name: "My Agent Assets",
    version: "preview",
    platform: platform === "macos" ? "macOS" : platform === "windows" ? "Windows" : "Unknown",
    arch: "unknown",
    backendReady: false,
  };
}

type AppProps = {
  demoMode?: boolean;
};

function App({ demoMode = false }: AppProps = {}) {
  const platform = getDesktopPlatform();
  const [appInfo, setAppInfo] = useState<AppInfo>(() => fallbackInfo(platform));
  const [activePage, setActivePage] = useState<PageId>("dashboard");
  const [assetDetail, setAssetDetail] = useState<AssetDetailContext | null>(null);
  const [projectDetail, setProjectDetail] = useState<ProjectDetailContext | null>(null);
  const [conflictContext, setConflictContext] = useState<ConflictResolverContext | null>(null);
  const currentPage = getPageById(activePage);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    invoke<AppInfo>("app_info").then(setAppInfo).catch(() => setAppInfo(fallbackInfo(platform)));
  }, [platform]);

  const openAssetDetail = (detail: AssetDetailContext) => {
    setAssetDetail(detail);
    setActivePage("asset-detail");
  };

  const openProjectDetail = (detail: ProjectDetailContext) => {
    setProjectDetail(detail);
    setActivePage("project-detail");
  };

  const openConflicts = (context: ConflictResolverContext) => {
    setConflictContext(context);
    setActivePage("conflicts");
  };

  return (
    <AppFrame
      activePage={activePage}
      onPageChange={setActivePage}
      platform={platform}
    >
      <MountDraftProvider>
        <PageHeader page={currentPage} />
        <CurrentPage
          activePage={activePage}
          appInfo={appInfo}
          assetDetail={assetDetail}
          conflictContext={conflictContext}
          onOpenAssetDetail={openAssetDetail}
          onOpenConflicts={openConflicts}
          onOpenProjectDetail={openProjectDetail}
          onPageChange={setActivePage}
          projectDetail={projectDetail}
          demoMode={demoMode}
        />
      </MountDraftProvider>
    </AppFrame>
  );
}

export default App;
