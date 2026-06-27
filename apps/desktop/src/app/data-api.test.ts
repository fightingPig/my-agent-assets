import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DesktopSettings, GitStatus } from "./contracts";

const { invoke, isTauriRuntime } = vi.hoisted(() => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(),
}));

const savedSettings: DesktopSettings = {
  assetCenterPath: "~/.my-agent-assets",
  scanRoots: ["~/workspace"],
  maxDepth: 4,
  backupBeforeApply: true,
  planOnlyByDefault: true,
  gitDefaultBranch: "main",
  gitRemote: "origin",
  appearanceTheme: "system",
  density: "compact",
  logLevel: "info",
  logRetentionDays: 14,
  cliPath: "maa",
};

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

    invoke.mockResolvedValueOnce([]);
    await api.listBackups();
    expect(invoke).toHaveBeenLastCalledWith("list_backups");

    invoke.mockResolvedValueOnce({} satisfies Partial<GitStatus>);
    await api.gitStatus();
    expect(invoke).toHaveBeenLastCalledWith("git_status");

    invoke.mockResolvedValueOnce({ assetCenterPath: "~/.my-agent-assets" });
    await api.settingsLoad();
    expect(invoke).toHaveBeenLastCalledWith("settings_load");

    invoke.mockResolvedValueOnce(savedSettings);
    await api.settingsSave({ settings: savedSettings });
    expect(invoke).toHaveBeenLastCalledWith("settings_save", {
      input: { settings: savedSettings },
    });

    invoke.mockResolvedValueOnce({ assets: [] });
    await api.scanAssets({ scope: { kind: "custom", path: "~/workspace/project-a" } });
    expect(invoke).toHaveBeenLastCalledWith("scan_assets", {
      input: { scope: { kind: "custom", path: "~/workspace/project-a" } },
    });
  });

  it("calls preview-only command names with the expected input envelope", async () => {
    const api = await import("./data-api");

    invoke.mockResolvedValueOnce({ assets: [], conflicts: [], steps: [] });
    await api.previewImport({
      scope: { kind: "user" },
      assetIds: ["skill:review"],
      conflictResolutions: [],
    });
    expect(invoke).toHaveBeenLastCalledWith("preview_import", {
      input: { scope: { kind: "user" }, assetIds: ["skill:review"], conflictResolutions: [] },
    });

    invoke.mockResolvedValueOnce({
      asset: {},
      target: {},
      steps: [],
      warnings: [],
      backupRequired: true,
      canApply: true,
    });
    await api.previewMount({
      assetId: "skill:review",
      target: { scope: "project", runtimePath: "~/workspace/project-a/.claude/skills/review", projectPath: "~/workspace/project-a" },
    });
    expect(invoke).toHaveBeenLastCalledWith("preview_mount", {
      input: {
        assetId: "skill:review",
        target: { scope: "project", runtimePath: "~/workspace/project-a/.claude/skills/review", projectPath: "~/workspace/project-a" },
      },
    });

    invoke.mockResolvedValueOnce([]);
    await api.previewConflicts({ scope: { kind: "user" }, assetIds: ["mcp:PostgreSQL"] });
    expect(invoke).toHaveBeenLastCalledWith("preview_conflicts", {
      input: { scope: { kind: "user" }, assetIds: ["mcp:PostgreSQL"] },
    });

    invoke.mockResolvedValueOnce({ backup: {}, affectedPaths: [], steps: [] });
    await api.previewRestore({ backupId: "backup-20260621-1842" });
    expect(invoke).toHaveBeenLastCalledWith("preview_restore", {
      input: { backupId: "backup-20260621-1842" },
    });

    invoke.mockResolvedValueOnce({ direction: "pull", steps: [], warnings: [] });
    await api.previewSync({ direction: "pull" });
    expect(invoke).toHaveBeenLastCalledWith("preview_sync", {
      input: { direction: "pull" },
    });

    invoke.mockResolvedValueOnce({
      mode: "apply",
      ok: true,
      previewId: "preview-sync-1",
      backup: null,
      steps: [],
      warnings: [],
      errors: [],
    });
    await api.syncApply({
      previewId: "preview-sync-1",
      mode: "apply",
      direction: "push",
    });
    expect(invoke).toHaveBeenLastCalledWith("sync_apply", {
      input: { previewId: "preview-sync-1", mode: "apply", direction: "push" },
    });
  });

  it("calls apply command names with the expected input envelope", async () => {
    const api = await import("./data-api");

    invoke.mockResolvedValueOnce({
      mode: "planOnly",
      ok: true,
      previewId: "preview-import-1",
      backup: null,
      steps: [],
      warnings: [],
      errors: [],
    });
    await api.importApply({
      previewId: "preview-import-1",
      mode: "planOnly",
      scope: { kind: "user" },
      assetIds: ["skill:review"],
      conflictResolutions: [],
      backupBeforeApply: true,
    });
    expect(invoke).toHaveBeenLastCalledWith("import_apply", {
      input: {
        previewId: "preview-import-1",
        mode: "planOnly",
        scope: { kind: "user" },
        assetIds: ["skill:review"],
        conflictResolutions: [],
        backupBeforeApply: true,
      },
    });

    invoke.mockResolvedValueOnce({
      mode: "planOnly",
      ok: true,
      previewId: "preview-conflict-1",
      backup: null,
      steps: [],
      warnings: [],
      errors: [],
    });
    await api.conflictApply({
      previewId: "preview-conflict-1",
      mode: "planOnly",
      scope: { kind: "user" },
      assetIds: ["skill:review"],
      conflictResolutions: [{
        conflictId: "conflict:skill:review",
        resolution: "overwrite",
        renameTo: null,
      }],
      backupBeforeApply: true,
    });
    expect(invoke).toHaveBeenLastCalledWith("conflict_apply", {
      input: {
        previewId: "preview-conflict-1",
        mode: "planOnly",
        scope: { kind: "user" },
        assetIds: ["skill:review"],
        conflictResolutions: [{
          conflictId: "conflict:skill:review",
          resolution: "overwrite",
          renameTo: null,
        }],
        backupBeforeApply: true,
      },
    });

    invoke.mockResolvedValueOnce({
      mode: "planOnly",
      ok: true,
      previewId: "preview-mount-1",
      backup: null,
      steps: [],
      warnings: [],
      errors: [],
    });
    await api.mountApply({
      previewId: "preview-mount-1",
      mode: "planOnly",
      assetId: "skill:review",
      target: {
        scope: "project",
        runtimePath: "~/workspace/project-a/.claude/skills/review",
        projectPath: "~/workspace/project-a",
      },
      backupBeforeApply: true,
    });
    expect(invoke).toHaveBeenLastCalledWith("mount_apply", {
      input: {
        previewId: "preview-mount-1",
        mode: "planOnly",
        assetId: "skill:review",
        target: {
          scope: "project",
          runtimePath: "~/workspace/project-a/.claude/skills/review",
          projectPath: "~/workspace/project-a",
        },
        backupBeforeApply: true,
      },
    });

    invoke.mockResolvedValueOnce({
      mode: "planOnly",
      ok: true,
      previewId: "preview-restore-1",
      backup: null,
      steps: [],
      warnings: [],
      errors: [],
    });
    await api.restoreApply({
      previewId: "preview-restore-1",
      mode: "planOnly",
      backupId: "backup-1",
      backupBeforeRestore: true,
    });
    expect(invoke).toHaveBeenLastCalledWith("restore_apply", {
      input: {
        previewId: "preview-restore-1",
        mode: "planOnly",
        backupId: "backup-1",
        backupBeforeRestore: true,
      },
    });
  });

  it("returns safe fallbacks outside Tauri", async () => {
    const api = await import("./data-api");
    isTauriRuntime.mockReturnValue(false);

    await expect(api.listAssets()).resolves.toEqual([]);
    await expect(api.listProjects()).resolves.toEqual([]);
    await expect(api.listBackups()).resolves.toEqual([]);
    await expect(api.settingsLoad()).resolves.toMatchObject({
      assetCenterPath: "~/.my-agent-assets",
      scanRoots: ["~/.claude", "~/workspace", "~/code"],
    });
    await expect(api.settingsSave({ settings: savedSettings })).resolves.toEqual(savedSettings);
    await expect(api.gitStatus()).resolves.toMatchObject({
      isRepository: false,
      statusMessage: "Tauri runtime is unavailable.",
    });
    await expect(api.scanAssets({ scope: { kind: "user" } })).resolves.toMatchObject({
      counts: { total: 0, skills: 0, commands: 0, mcps: 0 },
      warnings: ["Tauri runtime is unavailable; scan skipped."],
    });
    await expect(api.previewConflicts({ scope: { kind: "user" }, assetIds: [] })).resolves.toEqual([]);
    await expect(api.previewMount({
      assetId: "skill:review",
      target: { scope: "user", runtimePath: "~/.claude/skills/review", projectPath: null },
    })).resolves.toBeNull();
    await expect(api.previewImport({ scope: { kind: "user" }, assetIds: [], conflictResolutions: [] })).resolves.toMatchObject({
      canApply: false,
      warnings: ["Tauri runtime is unavailable; import preview skipped."],
    });
    await expect(api.previewRestore({ backupId: "backup-1" })).resolves.toMatchObject({
      canApply: false,
      warnings: ["Tauri runtime is unavailable; restore preview skipped."],
    });
    await expect(api.previewSync({ direction: "push" })).resolves.toMatchObject({
      direction: "push",
      canApply: false,
      warnings: ["Tauri runtime is unavailable; sync preview skipped."],
    });
    await expect(api.syncApply({
      previewId: "preview-sync-1",
      mode: "apply",
      direction: "push",
    })).resolves.toMatchObject({
      ok: false,
      previewId: "preview-sync-1",
      warnings: ["Tauri runtime is unavailable; sync apply skipped."],
      errors: ["sync_apply could not run outside the Tauri runtime."],
    });
    await expect(api.importApply({
      previewId: "preview-import-1",
      mode: "apply",
      scope: { kind: "user" },
      assetIds: ["skill:review"],
      conflictResolutions: [],
      backupBeforeApply: true,
    })).resolves.toMatchObject({
      ok: false,
      previewId: "preview-import-1",
      warnings: ["Tauri runtime is unavailable; import apply skipped."],
      errors: ["import_apply could not run outside the Tauri runtime."],
    });
    await expect(api.conflictApply({
      previewId: "preview-conflict-1",
      mode: "apply",
      scope: { kind: "user" },
      assetIds: ["skill:review"],
      conflictResolutions: [{
        conflictId: "conflict:skill:review",
        resolution: "overwrite",
        renameTo: null,
      }],
      backupBeforeApply: true,
    })).resolves.toMatchObject({
      ok: false,
      previewId: "preview-conflict-1",
      errors: ["conflict_apply could not run outside the Tauri runtime."],
    });
    await expect(api.mountApply({
      previewId: "preview-mount-1",
      mode: "apply",
      assetId: "skill:review",
      target: {
        scope: "project",
        runtimePath: "~/workspace/project-a/.claude/skills/review",
        projectPath: "~/workspace/project-a",
      },
      backupBeforeApply: true,
    })).resolves.toMatchObject({
      ok: false,
      previewId: "preview-mount-1",
      warnings: ["Tauri runtime is unavailable; mount apply skipped."],
      errors: ["mount_apply could not run outside the Tauri runtime."],
    });
    await expect(api.restoreApply({
      previewId: "preview-restore-1",
      mode: "apply",
      backupId: "backup-1",
      backupBeforeRestore: true,
    })).resolves.toMatchObject({
      ok: false,
      previewId: "preview-restore-1",
      warnings: ["Tauri runtime is unavailable; restore apply skipped."],
      errors: ["restore_apply could not run outside the Tauri runtime."],
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
