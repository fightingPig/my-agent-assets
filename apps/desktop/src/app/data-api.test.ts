import { beforeEach, describe, expect, it, vi } from "vitest";
import type { GitStatus } from "./contracts";

const { invoke, isTauriRuntime } = vi.hoisted(() => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../lib/platform", () => ({ isTauriRuntime }));

describe("read-only desktop data api", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    isTauriRuntime.mockReturnValue(true);
  });

  it("calls read-only command names with the expected input envelope", async () => {
    const api = await import("./data-api");
    invoke.mockResolvedValueOnce([]);
    await api.listAssets({ assetType: "skill" });
    expect(invoke).toHaveBeenLastCalledWith("list_assets", { input: { assetType: "skill" } });

    invoke.mockResolvedValueOnce([]);
    await api.listProjects();
    expect(invoke).toHaveBeenLastCalledWith("list_projects");

    invoke.mockResolvedValueOnce({} satisfies Partial<GitStatus>);
    await api.gitStatus();
    expect(invoke).toHaveBeenLastCalledWith("git_status");

    invoke.mockResolvedValueOnce({ assetCenterPath: "~/.my-agent-assets" });
    await api.settingsLoad();
    expect(invoke).toHaveBeenLastCalledWith("settings_load");

    invoke.mockResolvedValueOnce({ assets: [] });
    await api.scanAssets({ scope: { kind: "custom", path: "~/workspace/project-a" } });
    expect(invoke).toHaveBeenLastCalledWith("scan_assets", {
      input: { scope: { kind: "custom", path: "~/workspace/project-a" } },
    });
  });

  it("returns safe fallbacks outside Tauri", async () => {
    const api = await import("./data-api");
    isTauriRuntime.mockReturnValue(false);

    await expect(api.listAssets()).resolves.toEqual([]);
    await expect(api.listProjects()).resolves.toEqual([]);
    await expect(api.settingsLoad()).resolves.toMatchObject({
      assetCenterPath: "~/.my-agent-assets",
      scanRoots: ["~/.claude", "~/workspace", "~/code"],
    });
    await expect(api.gitStatus()).resolves.toMatchObject({
      isRepository: false,
      statusMessage: "Tauri runtime is unavailable.",
    });
    await expect(api.scanAssets({ scope: { kind: "user" } })).resolves.toMatchObject({
      counts: { total: 0, skills: 0, commands: 0, mcps: 0 },
      warnings: ["Tauri runtime is unavailable; scan skipped."],
    });
    expect(invoke).not.toHaveBeenCalled();
  });

  it("falls back when invoke rejects", async () => {
    const api = await import("./data-api");
    invoke.mockRejectedValue(new Error("command unavailable"));

    await expect(api.listAssets()).resolves.toEqual([]);
    await expect(api.gitStatus()).resolves.toMatchObject({
      isRepository: false,
      statusMessage: "Tauri runtime is unavailable.",
    });
  });
});
