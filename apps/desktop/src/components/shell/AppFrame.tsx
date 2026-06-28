import type { ReactNode } from "react";
import type { PageId } from "../../app/pages";
import type { AssetProvider } from "../../app/provider";
import type { DesktopPlatform } from "../../lib/platform";
import { MacOverlayDragArea } from "./MacOverlayDragArea";
import { Sidebar } from "./Sidebar";

type AppFrameProps = {
  platform: DesktopPlatform;
  activePage: PageId;
  onPageChange: (page: PageId) => void;
  provider: AssetProvider;
  onProviderChange: (provider: AssetProvider) => void;
  children: ReactNode;
};

export function AppFrame({
  platform,
  activePage,
  onPageChange,
  provider,
  onProviderChange,
  children,
}: AppFrameProps) {
  return (
    <div className={`app-frame platform-${platform}`}>
      {platform === "macos" && <MacOverlayDragArea />}
      <div className="app-body">
        <Sidebar
          activePage={activePage}
          onPageChange={onPageChange}
          onProviderChange={onProviderChange}
          provider={provider}
        />
        <main className="app-main">{children}</main>
      </div>
    </div>
  );
}
