import { getCurrentWindow } from "@tauri-apps/api/window";
import type { PointerEvent as ReactPointerEvent } from "react";
import { DRAG_REGION_ATTR, DRAG_REGION_STYLE } from "../../lib/platform";

export function MacOverlayDragArea() {
  const handlePointerDown = async (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;

    const target = event.target as HTMLElement;
    if (target.closest("button,input,textarea,select,a,[data-no-drag='true']")) return;

    try {
      await getCurrentWindow().startDragging();
    } catch (error) {
      console.error("[MacOverlayDragArea] startDragging failed", error);
    }
  };

  return (
    <div
      className="mac-overlay-drag-area"
      {...DRAG_REGION_ATTR}
      style={DRAG_REGION_STYLE}
      onPointerDown={handlePointerDown}
    >
      <div className="mac-overlay-sidebar" {...DRAG_REGION_ATTR} style={DRAG_REGION_STYLE} />
      <div className="mac-overlay-main" {...DRAG_REGION_ATTR} style={DRAG_REGION_STYLE} />
    </div>
  );
}
