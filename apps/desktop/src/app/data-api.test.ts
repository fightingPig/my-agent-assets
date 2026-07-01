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

    invoke.mockResolvedValueOnce({
      manifestPath: "/tmp/backups/local/one/manifest.yaml",
    });
    await api.revealBackupManifest({ entryId: "local:one" });
    expect(invoke).toHaveBeenLastCalledWith("reveal_backup_manifest", {
      input: { entryId: "local:one" },
    });

    invoke.mockResolvedValueOnce({} satisfies Partial<GitStatus>);
    await api.gitStatus();
    expect(invoke).toHaveBeenLastCalledWith("git_status");

    invoke.mockResolvedValueOnce({
      writesBlocked: false,
      journals: [],
      message: "没有未完成事务。",
    });
    await api.recoveryStatus();
    expect(invoke).toHaveBeenLastCalledWith("recovery_status");

    invoke.mockResolvedValueOnce({
      previewId: "init-1",
      assetCenterPath: "/tmp/home/.my-agent-assets",
      plannedPaths: [],
      warnings: [],
    });
    await api.initializationPreview();
    expect(invoke).toHaveBeenLastCalledWith("initialization_preview");

    invoke.mockResolvedValueOnce({
      previewId: "init-1",
      assetCenterPath: "/tmp/home/.my-agent-assets",
      created: true,
      createdPaths: [],
    });
    await api.initializationApply({
      previewId: "init-1",
      previewGeneratedAtEpochSeconds: 100,
    });
    expect(invoke).toHaveBeenLastCalledWith("initialization_apply", {
      input: {
        previewId: "init-1",
        previewGeneratedAtEpochSeconds: 100,
      },
    });

    invoke.mockResolvedValueOnce({ assetCenterPath: "~/.my-agent-assets" });
    await api.settingsLoad();
    expect(invoke).toHaveBeenLastCalledWith("settings_load");

    invoke.mockResolvedValueOnce(savedSettings);
    await api.settingsSave({ settings: savedSettings });
    expect(invoke).toHaveBeenLastCalledWith("settings_save", {
      input: { settings: savedSettings },
    });

  });

  it("calls preview-only command names with the expected input envelope", async () => {
    const api = await import("./data-api");

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

    invoke.mockResolvedValueOnce({
      assetId: "mcp:filesystem",
      canonical: {
        schemaVersion: 1,
        name: "filesystem",
        spec: { type: "stdio", command: "npx" },
        providerExtensions: {},
      },
      bindings: [],
    });
    await api.canonicalMcpGet("mcp:filesystem");
    expect(invoke).toHaveBeenLastCalledWith("canonical_mcp_get", {
      input: { assetId: "mcp:filesystem" },
    });

    const mcpSaveRequest = {
      canonical: {
        schemaVersion: 1 as const,
        name: "filesystem",
        spec: { type: "stdio" as const, command: "npx" },
        providerExtensions: {},
      },
    };
    invoke.mockResolvedValueOnce({
      previewId: "mcp-save-1",
      assetId: "mcp:filesystem",
      plannedEffects: [],
    });
    await api.canonicalMcpSavePreview(mcpSaveRequest);
    expect(invoke).toHaveBeenLastCalledWith("canonical_mcp_save_preview", {
      input: mcpSaveRequest,
    });

    const mcpSaveApplyRequest = {
      previewId: "mcp-save-1",
      previewGeneratedAtEpochSeconds: 123,
      request: mcpSaveRequest,
    };
    invoke.mockResolvedValueOnce({
      previewId: "mcp-save-1",
      assetId: "mcp:filesystem",
      affectedPaths: [],
    });
    await api.canonicalMcpSaveApply(mcpSaveApplyRequest);
    expect(invoke).toHaveBeenLastCalledWith("canonical_mcp_save_apply", {
      input: mcpSaveApplyRequest,
    });

    const registrationRequest = {
      id: "project-a-skills",
      kind: "claude_project_skills",
      location: "~/workspace/project-a",
    } as const;
    invoke.mockResolvedValueOnce({
      previewId: "target-add-1",
      target: { id: registrationRequest.id },
      affectedPaths: ["/tmp/targets.yaml"],
    });
    await api.targetRegistrationPreview(registrationRequest);
    expect(invoke).toHaveBeenLastCalledWith("target_registration_preview", {
      input: registrationRequest,
    });

    const registrationApplyRequest = {
      previewId: "target-add-1",
      previewGeneratedAtEpochSeconds: 125,
      request: registrationRequest,
    };
    invoke.mockResolvedValueOnce({
      previewId: "target-add-1",
      operation: "add",
      targetId: registrationRequest.id,
      registryPath: "/tmp/targets.yaml",
      backupPath: "/tmp/backup/targets.yaml",
    });
    await api.targetRegistrationApply(registrationApplyRequest);
    expect(invoke).toHaveBeenLastCalledWith("target_registration_apply", {
      input: registrationApplyRequest,
    });

    const removalRequest = { targetId: registrationRequest.id };
    invoke.mockResolvedValueOnce({
      previewId: "target-remove-1",
      target: { id: registrationRequest.id },
      affectedPaths: ["/tmp/targets.yaml"],
    });
    await api.targetRemovalPreview(removalRequest);
    expect(invoke).toHaveBeenLastCalledWith("target_removal_preview", {
      input: removalRequest,
    });

    const removalApplyRequest = {
      previewId: "target-remove-1",
      previewGeneratedAtEpochSeconds: 126,
      request: removalRequest,
    };
    invoke.mockResolvedValueOnce({
      previewId: "target-remove-1",
      operation: "remove",
      targetId: registrationRequest.id,
      registryPath: "/tmp/targets.yaml",
      backupPath: "/tmp/backup/targets.yaml",
    });
    await api.targetRemovalApply(removalApplyRequest);
    expect(invoke).toHaveBeenLastCalledWith("target_removal_apply", {
      input: removalApplyRequest,
    });

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
    expect(invoke).not.toHaveBeenCalled();
  });

  it("surfaces read failures in Tauri instead of masking them with mock data", async () => {
    const api = await import("./data-api");
    invoke.mockRejectedValue(new Error("command unavailable"));

    await expect(api.listAssets()).rejects.toThrow("command unavailable");
    await expect(api.gitStatus()).rejects.toThrow("command unavailable");
    await expect(api.settingsSave({ settings: savedSettings })).rejects.toThrow("command unavailable");
  });
});
