import type { CSSProperties } from "react";

export type DesktopPlatform = "macos" | "windows" | "linux" | "unknown";

export const isMac = (): boolean => {
  try {
    const userAgent = navigator.userAgent || "";
    const platform = (navigator.platform || "").toLowerCase();
    return /mac/i.test(userAgent) || platform.includes("mac");
  } catch {
    return false;
  }
};

export const isWindows = (): boolean => {
  try {
    const userAgent = navigator.userAgent || "";
    const platform = (navigator.platform || "").toLowerCase();
    return /windows|win32|win64/i.test(userAgent) || platform.includes("win");
  } catch {
    return false;
  }
};

export const isLinux = (): boolean => {
  try {
    const userAgent = navigator.userAgent || "";
    return (
      /linux|x11/i.test(userAgent) &&
      !/android/i.test(userAgent) &&
      !isMac() &&
      !isWindows()
    );
  } catch {
    return false;
  }
};

export const getDesktopPlatform = (): DesktopPlatform => {
  if (!isTauriRuntime() && typeof window !== "undefined") {
    const previewPlatform = new URLSearchParams(window.location.search).get("platform");
    if (previewPlatform === "macos" || previewPlatform === "windows" || previewPlatform === "linux") {
      return previewPlatform;
    }
  }
  if (isMac()) return "macos";
  if (isWindows()) return "windows";
  if (isLinux()) return "linux";
  return "unknown";
};

export const isTauriRuntime = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// Tauri checks attribute presence. Never render false or an empty string.
export const DRAG_REGION_ENABLED = !isLinux();
export const DRAG_REGION_ATTR: Record<string, true> = DRAG_REGION_ENABLED
  ? { "data-tauri-drag-region": true }
  : {};
export const DRAG_REGION_STYLE = (DRAG_REGION_ENABLED
  ? { WebkitAppRegion: "drag" }
  : {}) as CSSProperties;
export const NO_DRAG_REGION_STYLE = {
  WebkitAppRegion: "no-drag",
} as CSSProperties;

export const getPlatformShortcuts = (platform = getDesktopPlatform()) =>
  platform === "macos"
    ? { globalSearch: "⌘K", pageSearch: "⌘F" }
    : { globalSearch: "Ctrl+K", pageSearch: "Ctrl+F" };
