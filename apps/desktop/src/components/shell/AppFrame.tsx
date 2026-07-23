import type { ReactNode } from "react";
import type { PageId } from "../../app/pages";
import type { DesktopPlatform } from "../../lib/platform";
import { MacOverlayDragArea } from "./MacOverlayDragArea";
import { Sidebar } from "./Sidebar";

type AppFrameProps = {
  platform: DesktopPlatform;
  activePage: PageId;
  onPageChange: (page: PageId) => void;
  children: ReactNode;
};

export function AppFrame({
  platform,
  activePage,
  onPageChange,
  children,
}: AppFrameProps) {
  return (
    <div className={`app-frame platform-${platform}`}>
      {platform === "macos" && <MacOverlayDragArea />}
      <div className="app-body">
        <Sidebar
          activePage={activePage}
          onPageChange={onPageChange}
        />
        <main className="app-main">{children}</main>
      </div>
    </div>
  );
}
