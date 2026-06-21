import { afterEach, describe, expect, it, vi } from "vitest";

function setNavigatorPlatform(userAgent: string, platform: string) {
  Object.defineProperty(navigator, "userAgent", { configurable: true, value: userAgent });
  Object.defineProperty(navigator, "platform", { configurable: true, value: platform });
}

afterEach(() => vi.resetModules());

describe("platform drag regions", () => {
  it("enables drag regions on macOS without false-valued attributes", async () => {
    setNavigatorPlatform("Mozilla/5.0 (Macintosh; Intel Mac OS X)", "MacIntel");
    const platform = await import("./platform");
    expect(platform.isMac()).toBe(true);
    expect(platform.DRAG_REGION_ATTR).toEqual({ "data-tauri-drag-region": true });
    expect(platform.getPlatformShortcuts()).toEqual({ globalSearch: "⌘K", pageSearch: "⌘F" });
  });

  it("omits the drag attribute completely on Linux", async () => {
    setNavigatorPlatform("Mozilla/5.0 (X11; Linux x86_64)", "Linux x86_64");
    const platform = await import("./platform");
    expect(platform.isLinux()).toBe(true);
    expect(platform.DRAG_REGION_ATTR).toEqual({});
    expect("data-tauri-drag-region" in platform.DRAG_REGION_ATTR).toBe(false);
  });
});
