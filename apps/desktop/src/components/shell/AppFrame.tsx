import type { ReactNode } from "react";
import type { DesktopPlatform } from "../../lib/platform";
import { MacOverlayDragArea } from "./MacOverlayDragArea";
import { Sidebar } from "./Sidebar";

type AppFrameProps = {
  platform: DesktopPlatform;
  children: ReactNode;
};

export function AppFrame({ platform, children }: AppFrameProps) {
  return (
    <div className={`app-frame platform-${platform}`}>
      {platform === "macos" && <MacOverlayDragArea />}
      <div className="app-body">
        <Sidebar />
        <main className="app-main">{children}</main>
      </div>
    </div>
  );
}
