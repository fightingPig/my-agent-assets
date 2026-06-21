import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { AppFrame } from "./components/shell/AppFrame";
import { PageHeader } from "./components/shell/PageHeader";
import {
  getDesktopPlatform,
  getPlatformShortcuts,
  isTauriRuntime,
} from "./lib/platform";
import { DashboardPage, type AppInfo } from "./pages/DashboardPage";

const fallbackInfo: AppInfo = {
  name: "My Agent Assets",
  version: "0.1.0",
  platform: "macOS",
  arch: "arm64",
  backendReady: false,
};

function App() {
  const [appInfo, setAppInfo] = useState<AppInfo>(fallbackInfo);
  const platform = getDesktopPlatform();
  const shortcuts = getPlatformShortcuts(platform);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    invoke<AppInfo>("app_info").then(setAppInfo).catch(() => setAppInfo(fallbackInfo));
  }, []);

  return (
    <AppFrame platform={platform}>
      <PageHeader shortcuts={shortcuts} />
      <DashboardPage appInfo={appInfo} />
    </AppFrame>
  );
}

export default App;
