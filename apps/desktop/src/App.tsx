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
import type { AssetProvider } from "./app/provider";
import { AppFrame } from "./components/shell/AppFrame";
import { PageHeader } from "./components/shell/PageHeader";
import {
  getDesktopPlatform,
  isTauriRuntime,
} from "./lib/platform";

const fallbackInfo: AppInfo = {
  name: "My Agent Assets",
  version: "0.1.0",
  platform: "macOS",
  arch: "arm64",
  backendReady: false,
};

type AppProps = {
  demoMode?: boolean;
};

function App({ demoMode = false }: AppProps = {}) {
  const [appInfo, setAppInfo] = useState<AppInfo>(fallbackInfo);
  const [activePage, setActivePage] = useState<PageId>("dashboard");
  const [provider, setProvider] = useState<AssetProvider>("claude");
  const [assetDetail, setAssetDetail] = useState<AssetDetailContext | null>(null);
  const [projectDetail, setProjectDetail] = useState<ProjectDetailContext | null>(null);
  const [conflictContext, setConflictContext] = useState<ConflictResolverContext | null>(null);
  const platform = getDesktopPlatform();
  const currentPage = getPageById(activePage);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    invoke<AppInfo>("app_info").then(setAppInfo).catch(() => setAppInfo(fallbackInfo));
  }, []);

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

  const changeProvider = (nextProvider: AssetProvider) => {
    setProvider(nextProvider);
  };

  return (
    <AppFrame
      activePage={activePage}
      onPageChange={setActivePage}
      onProviderChange={changeProvider}
      platform={platform}
      provider={provider}
    >
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
        provider={provider}
        projectDetail={projectDetail}
        demoMode={demoMode}
      />
    </AppFrame>
  );
}

export default App;
