import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  AdoptPreviewRequest,
  DesktopSettings,
  GitStatus,
} from "./contracts";

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

    invoke.mockResolvedValueOnce({ skills: [], warnings: [] });
    await api.listCodexSkills({ projectPath: "/tmp/project" });
    expect(invoke).toHaveBeenLastCalledWith("list_codex_skills", {
      input: { projectPath: "/tmp/project" },
    });

    invoke.mockResolvedValueOnce({ servers: [], warnings: [] });
    await api.listCodexMcpServers({ projectPath: null });
    expect(invoke).toHaveBeenLastCalledWith("list_codex_mcp_servers", {
      input: { projectPath: null },
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

    invoke.mockResolvedValueOnce({
      previewId: "sync-1",
      direction: "pull",
      status: {},
      repositoryVisibility: "unknown",
      plannedEffects: [],
      warnings: [],
      backupRequired: true,
      canApply: false,
      generatedAtEpochSeconds: 100,
      expiresAtEpochSeconds: 700,
    });
    await api.previewSync({ direction: "pull" });
    expect(invoke).toHaveBeenLastCalledWith("preview_sync", {
      input: { direction: "pull" },
    });

    invoke.mockResolvedValueOnce({
      previewId: "preview-sync-1",
      direction: "push",
      affectedPaths: ["/tmp/assets"],
      committed: true,
      pushed: true,
      pulled: false,
      warnings: [],
      journalPath: "/tmp/journal",
    });
    await api.syncApply({
      previewId: "preview-sync-1",
      previewGeneratedAtEpochSeconds: 100,
      request: { direction: "push" },
    });
    expect(invoke).toHaveBeenLastCalledWith("sync_apply", {
      input: {
        previewId: "preview-sync-1",
        previewGeneratedAtEpochSeconds: 100,
        request: { direction: "push" },
      },
    });
  });

  it("uses shared-core discovery and canonical import command envelopes", async () => {
    const api = await import("./data-api");
    const scope = { kind: "project", projectPath: "/tmp/project" } as const;

    invoke.mockResolvedValueOnce({ sources: [], warnings: [] });
    await api.discoverRuntimeSources(scope);
    expect(invoke).toHaveBeenLastCalledWith("discover_runtime_sources", {
      input: scope,
    });

    const previewRequest = {
      scope,
      sourceId: "codex:project:skill:abc:review",
      resolution: { kind: "overwrite" },
    } as const;
    invoke.mockResolvedValueOnce({
      previewId: "import-1",
      sourceId: previewRequest.sourceId,
    });
    await api.canonicalImportPreview(previewRequest);
    expect(invoke).toHaveBeenLastCalledWith("canonical_import_preview", {
      input: previewRequest,
    });

    const applyRequest = {
      previewId: "import-1",
      previewGeneratedAtEpochSeconds: 123,
      request: previewRequest,
    };
    invoke.mockResolvedValueOnce({
      previewId: "import-1",
      assetId: "skill:review",
      status: "imported",
      affectedPaths: [],
    });
    await api.canonicalImportApply(applyRequest);
    expect(invoke).toHaveBeenLastCalledWith("canonical_import_apply", {
      input: applyRequest,
    });
  });

  it("uses target IDs for shared-core mount commands", async () => {
    const api = await import("./data-api");

    invoke.mockResolvedValueOnce([]);
    await api.listMountTargets();
    expect(invoke).toHaveBeenLastCalledWith("list_mount_targets");

    const previewRequest = {
      assetId: "skill:review",
      targetId: "claude-user-skills",
    };
    invoke.mockResolvedValueOnce({
      previewId: "mount-1",
      targetId: "claude-user-skills",
    });
    await api.canonicalMountPreview(previewRequest);
    expect(invoke).toHaveBeenLastCalledWith("canonical_mount_preview", {
      input: previewRequest,
    });

    const applyRequest = {
      previewId: "mount-1",
      previewGeneratedAtEpochSeconds: 123,
      request: previewRequest,
    };
    invoke.mockResolvedValueOnce({
      previewId: "mount-1",
      assetId: "skill:review",
      targetId: "claude-user-skills",
      mounted: true,
      affectedPaths: [],
      warnings: [],
    });
    await api.canonicalMountApply(applyRequest);
    expect(invoke).toHaveBeenLastCalledWith("canonical_mount_apply", {
      input: applyRequest,
    });
    expect(applyRequest.request).not.toHaveProperty("runtimePath");

    const unmountPreviewRequest = {
      assetId: "skill:review",
      targetId: "claude-user-skills",
    };
    invoke.mockResolvedValueOnce({ previewId: "unmount-1" });
    await api.canonicalUnmountPreview(unmountPreviewRequest);
    expect(invoke).toHaveBeenLastCalledWith("canonical_unmount_preview", {
      input: unmountPreviewRequest,
    });

    const unmountApplyRequest = {
      previewId: "unmount-1",
      previewGeneratedAtEpochSeconds: 124,
      request: unmountPreviewRequest,
    };
    invoke.mockResolvedValueOnce({
      previewId: "unmount-1",
      assetId: "skill:review",
      targetId: "claude-user-skills",
      unmounted: true,
      affectedPaths: [],
    });
    await api.canonicalUnmountApply(unmountApplyRequest);
    expect(invoke).toHaveBeenLastCalledWith("canonical_unmount_apply", {
      input: unmountApplyRequest,
    });

    const deletePreviewRequest = {
      assetId: "skill:review",
      mode: "unmount_all",
    } as const;
    invoke.mockResolvedValueOnce({ previewId: "delete-1" });
    await api.canonicalDeletePreview(deletePreviewRequest);
    expect(invoke).toHaveBeenLastCalledWith("canonical_delete_preview", {
      input: deletePreviewRequest,
    });

    const deleteApplyRequest = {
      previewId: "delete-1",
      previewGeneratedAtEpochSeconds: 125,
      request: deletePreviewRequest,
    };
    invoke.mockResolvedValueOnce({
      previewId: "delete-1",
      assetId: "skill:review",
      deleted: true,
      portableBackupId: "portable-1",
      localBackupId: "local-1",
      affectedPaths: [],
      journalPath: "/tmp/journal",
    });
    await api.canonicalDeleteApply(deleteApplyRequest);
    expect(invoke).toHaveBeenLastCalledWith("canonical_delete_apply", {
      input: deleteApplyRequest,
    });

    const adoptPreviewRequest = {
      scope: { kind: "user" },
      selections: [
        {
          sourceId: "claude:user:skill:abc:review",
          resolution: { kind: "unresolved" },
        },
      ],
    } satisfies AdoptPreviewRequest;
    invoke.mockResolvedValueOnce({ previewId: "adopt-1", items: [] });
    await api.previewAdopt(adoptPreviewRequest);
    expect(invoke).toHaveBeenLastCalledWith("preview_adopt", {
      input: adoptPreviewRequest,
    });

    const adoptApplyRequest = {
      previewId: "adopt-1",
      previewGeneratedAtEpochSeconds: 126,
      request: adoptPreviewRequest,
    };
    invoke.mockResolvedValueOnce({
      previewId: "adopt-1",
      items: [],
      affectedPaths: [],
      journalPath: "/tmp/adopt-journal",
    });
    await api.adoptApply(adoptApplyRequest);
    expect(invoke).toHaveBeenLastCalledWith("adopt_apply", {
      input: adoptApplyRequest,
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
    await expect(api.listCodexSkills()).resolves.toEqual({ skills: [], warnings: [] });
    await expect(api.listCodexMcpServers()).resolves.toEqual({ servers: [], warnings: [] });
    await expect(api.previewConflicts({ scope: { kind: "user" }, assetIds: [] })).resolves.toEqual([]);
    await expect(api.previewMount({
      assetId: "skill:review",
      target: { scope: "user", runtimePath: "~/.claude/skills/review", projectPath: null },
    })).resolves.toBeNull();
    await expect(api.previewImport({ scope: { kind: "user" }, assetIds: [], conflictResolutions: [] })).resolves.toMatchObject({
      canApply: false,
      warnings: ["Tauri runtime is unavailable; import preview skipped."],
    });
    await expect(api.previewSync({ direction: "push" })).resolves.toMatchObject({
      direction: "push",
      canApply: false,
      warnings: ["Tauri runtime is unavailable; sync preview skipped."],
    });
    await expect(api.syncApply({
      previewId: "preview-sync-1",
      previewGeneratedAtEpochSeconds: 100,
      request: { direction: "push" },
    })).rejects.toThrow("requires the Tauri runtime");
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
    expect(invoke).not.toHaveBeenCalled();
  });

  it("surfaces read failures in Tauri instead of masking them with mock data", async () => {
    const api = await import("./data-api");
    invoke.mockRejectedValue(new Error("command unavailable"));

    await expect(api.listAssets()).rejects.toThrow("command unavailable");
    await expect(api.gitStatus()).rejects.toThrow("command unavailable");
    await expect(api.listCodexSkills()).rejects.toThrow("command unavailable");
    await expect(api.listCodexMcpServers()).rejects.toThrow("command unavailable");
    await expect(api.settingsSave({ settings: savedSettings })).rejects.toThrow("command unavailable");
  });
});
