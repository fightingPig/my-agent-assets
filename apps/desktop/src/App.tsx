import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { CurrentPage } from "./app/CurrentPage";
import type { AppInfo } from "./app/contracts";
import { getPageById, type PageId } from "./app/pages";
import { AppFrame } from "./components/shell/AppFrame";
import { PageHeader } from "./components/shell/PageHeader";
import {
  getDesktopPlatform,
  getPlatformShortcuts,
  isTauriRuntime,
} from "./lib/platform";

const fallbackInfo: AppInfo = {
  name: "My Agent Assets",
  version: "0.1.0",
  platform: "macOS",
  arch: "arm64",
  backendReady: false,
};

function App() {
  const [appInfo, setAppInfo] = useState<AppInfo>(fallbackInfo);
  const [activePage, setActivePage] = useState<PageId>("dashboard");
  const platform = getDesktopPlatform();
  const shortcuts = getPlatformShortcuts(platform);
  const currentPage = getPageById(activePage);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    invoke<AppInfo>("app_info").then(setAppInfo).catch(() => setAppInfo(fallbackInfo));
  }, []);

  return (
    <AppFrame platform={platform} activePage={activePage} onPageChange={setActivePage}>
      <PageHeader page={currentPage} shortcuts={shortcuts} />
      <CurrentPage activePage={activePage} appInfo={appInfo} onPageChange={setActivePage} />
    </AppFrame>
  );
}

export default App;
