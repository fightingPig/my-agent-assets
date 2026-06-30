import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  AssetSummary,
  BatchImportPreview,
  CanonicalMountPreview,
  DesktopSettings,
  GitStatus,
  ProjectSummary,
} from "../app/contracts";
import { BackupRestorePage } from "./BackupRestorePage";
import { AssetDetailPage } from "./AssetDetailPage";
import { CommandsListPage } from "./CommandsListPage";
import { ConflictResolverPage } from "./ConflictResolverPage";
import { McpServersListPage } from "./McpServersListPage";
import { MountManagerPage } from "./MountManagerPage";
import { ProjectsListPage } from "./ProjectsListPage";
import { ProjectDetailPage } from "./ProjectDetailPage";
import { staticProjects } from "./project-data";
import { ScanImportPage } from "./ScanImportPage";
import { SettingsPage } from "./SettingsPage";
import { SkillsListPage } from "./SkillsListPage";
import { SyncPage } from "./SyncPage";

const {
  listAssets,
  listProjects,
  listBackups,
  gitStatus,
  settingsLoad,
  settingsSave,
  previewSync,
  syncApply,
  listMountTargets,
  canonicalMountPreview,
  canonicalMountApply,
  discoverRuntimeSources,
  canonicalBatchImportPreview,
  canonicalBatchImportApply,
  previewAdopt,
  adoptApply,
} = vi.hoisted(() => ({
  listAssets: vi.fn(),
  listProjects: vi.fn(),
  listBackups: vi.fn(),
  gitStatus: vi.fn(),
  settingsLoad: vi.fn(),
  settingsSave: vi.fn(),
  previewSync: vi.fn(),
  syncApply: vi.fn(),
  listMountTargets: vi.fn(),
  canonicalMountPreview: vi.fn(),
  canonicalMountApply: vi.fn(),
  discoverRuntimeSources: vi.fn(),
  canonicalBatchImportPreview: vi.fn(),
  canonicalBatchImportApply: vi.fn(),
  previewAdopt: vi.fn(),
  adoptApply: vi.fn(),
}));

vi.mock("../app/data-api", () => ({
  listAssets,
  listProjects,
  listBackups,
  gitStatus,
  settingsLoad,
  settingsSave,
  previewSync,
  syncApply,
  listMountTargets,
  canonicalMountPreview,
  canonicalMountApply,
  discoverRuntimeSources,
  canonicalBatchImportPreview,
  canonicalBatchImportApply,
  previewAdopt,
  adoptApply,
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

beforeEach(() => {
  listAssets.mockResolvedValue([assetFixture("skill:review", "review", "skill")]);
  listProjects.mockResolvedValue([{
    id: "/tmp/project-a",
    name: "project-a",
    title: "Project A",
    path: "~/workspace/project-a",
    status: "ready",
    description: "Local project fixture",
    updatedAt: "2026-06-28T08:00:00Z",
    assetCounts: { total: 1, skills: 1, commands: 0, mcps: 0 },
    mounts: ["review"],
  } satisfies ProjectSummary]);
  listMountTargets.mockResolvedValue([
    {
      id: "claude-user-skills",
      kind: "claude_user_skills",
      provider: "claude_code",
      accepts: ["skill"],
      adapter: "symlink_directory",
      scope: "user",
      path: "/tmp/home/.claude/skills",
      providerState: "initialized",
      status: "ready",
    },
    {
      id: "project-a-skills",
      kind: "claude_project_skills",
      provider: "claude_code",
      accepts: ["skill"],
      adapter: "symlink_directory",
      scope: "project",
      path: "/tmp/project-a/.claude/skills",
      projectPath: "/tmp/project-a",
      providerState: "initialized",
      status: "ready",
    },
  ]);
  canonicalMountPreview.mockResolvedValue(canonicalMountPreviewFixture());
  canonicalMountApply.mockResolvedValue({
    previewId: "mount:skill-review",
    assetId: "skill:review",
    targetId: "claude-user-skills",
    mounted: true,
    backupId: "mount-backup-1",
    affectedPaths: ["/tmp/home/.claude/skills/review"],
    warnings: [],
  });
  discoverRuntimeSources.mockResolvedValue({ sources: [], warnings: [] });
  canonicalBatchImportPreview.mockResolvedValue(batchImportPreviewFixture());
  canonicalBatchImportApply.mockResolvedValue({
    previewId: "batch-import:test",
    items: [],
    affectedPaths: [],
    journalPath: "/tmp/batch-import-journal",
  });
  previewAdopt.mockResolvedValue({
    previewId: "adopt:test",
    items: [],
    importPlan: [],
    mountPlan: [],
    backupPlan: [],
    warnings: [],
    canApply: false,
    generatedAtEpochSeconds: 100,
    expiresAtEpochSeconds: 400,
  });
  adoptApply.mockResolvedValue({
    previewId: "adopt:test",
    items: [],
    affectedPaths: [],
    journalPath: "/tmp/adopt-journal",
  });
  listBackups.mockResolvedValue([{
    id: "backup-20260621-1842",
    label: "扫描导入前",
    createdAt: "2026-06-21T18:42:00Z",
    sizeBytes: 24 * 1024,
    entryCount: 2,
  }]);
  gitStatus.mockResolvedValue(gitStatusFixture());
  settingsLoad.mockResolvedValue(settingsFixture());
  settingsSave.mockImplementation(async ({ settings }) => settings);
  previewSync.mockResolvedValue({
    previewId: "preview:sync:push",
    direction: "push",
    status: gitStatusFixture({ isRepository: true }),
    repositoryVisibility: "private",
    plannedEffects: ["stage canonical whitelist", "git push origin main"],
    warnings: [],
    backupRequired: false,
    canApply: true,
    generatedAtEpochSeconds: 100,
    expiresAtEpochSeconds: 700,
  });
  syncApply.mockResolvedValue({
    previewId: "preview:sync:push",
    direction: "push",
    affectedPaths: ["~/.my-agent-assets"],
    committed: true,
    pushed: true,
    pulled: false,
    warnings: [],
    journalPath: "/tmp/sync-journal",
  });
});

describe("read-only UI integration", () => {
  it("feeds real read-only Skills, Commands, and MCP assets into Asset Center pages", async () => {
    listAssets
      .mockResolvedValueOnce([assetFixture("skill:real-review", "real-review", "skill")])
      .mockResolvedValueOnce([assetFixture("command:real-build", "real-build", "command")])
      .mockResolvedValueOnce([assetFixture("mcp:LocalFS", "LocalFS", "mcp")]);

    const { container, rerender } = render(<SkillsListPage />);
    expect(await screen.findByRole("option", { name: "real-review" })).toBeInTheDocument();
    expect(container.textContent).toContain("只读真实数据");
    expect(listAssets).toHaveBeenLastCalledWith({ assetType: "skill" });

    rerender(<CommandsListPage />);
    expect(await screen.findByRole("option", { name: "real-build" })).toBeInTheDocument();
    expect(listAssets).toHaveBeenLastCalledWith({ assetType: "command" });

    rerender(<McpServersListPage />);
    expect(await screen.findByRole("option", { name: "LocalFS" })).toBeInTheDocument();
    expect(listAssets).toHaveBeenLastCalledWith({ assetType: "mcp" });
  });

  it("renders real Codex Skills and MCP fields without Claude demo rows", async () => {
    discoverRuntimeSources.mockResolvedValue({
      sources: [{
        sourceId: "codex:skill:codex-review",
        provider: "codex",
        sourcePath: "/tmp/home/.agents/skills/codex-review",
        assetKind: "skill",
        assetName: "codex-review",
        sourceFormat: "skill_directory",
        scope: "user",
        isManaged: false,
        isSymlink: false,
        warnings: [],
        eligibleImport: true,
        eligibleAdopt: true,
      }, {
        sourceId: "codex:mcp:local-files",
        provider: "codex",
        sourcePath: "/tmp/home/.codex/config.toml",
        configPath: "/tmp/home/.codex/config.toml",
        assetKind: "mcp",
        assetName: "local-files",
        sourceFormat: "codex_toml",
        scope: "user",
        isManaged: false,
        isSymlink: false,
        warnings: [],
        eligibleImport: true,
        eligibleAdopt: false,
      }],
      warnings: [],
    });

    const { rerender } = render(<SkillsListPage provider="codex" />);
    expect(await screen.findByRole("option", { name: "codex-review" })).toBeInTheDocument();
    expect(screen.getByText("skill_directory")).toBeInTheDocument();
    expect(screen.getByText("可导入")).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "review" })).not.toBeInTheDocument();
    expect(discoverRuntimeSources).toHaveBeenCalledWith({ kind: "user" });

    rerender(<McpServersListPage provider="codex" />);
    expect(await screen.findByRole("option", { name: "local-files" })).toBeInTheDocument();
    expect(screen.getAllByText(/codex_toml/).length).toBeGreaterThan(0);
    expect(discoverRuntimeSources).toHaveBeenCalledWith({ kind: "user" });
  });

  it("shows provider-specific empty states for empty Codex discovery", async () => {
    const { rerender } = render(<SkillsListPage provider="codex" />);
    expect(await screen.findByText("未发现 Codex Skills")).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "review" })).not.toBeInTheDocument();

    rerender(<McpServersListPage provider="codex" />);
    expect(await screen.findByText("未发现 Codex MCP Servers")).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "PostgreSQL" })).not.toBeInTheDocument();
  });

  it("shows an empty state instead of static assets when read-only assets are empty", async () => {
    listAssets.mockResolvedValue([]);

    const { container } = render(<SkillsListPage />);
    expect(await screen.findByText("未发现 Skills")).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "review" })).not.toBeInTheDocument();
    expect(container.textContent).toContain("未发现本地数据");
  });

  it("feeds read-only projects while preserving selection and disabled actions", async () => {
    listProjects.mockResolvedValue([
      {
        id: "/tmp/local-app",
        name: "local-app",
        title: "Local App",
        path: "/tmp/local-app",
        status: "changed",
        description: "Read-only project",
        updatedAt: "2026-06-25T09:00:00Z",
        assetCounts: { total: 3, skills: 1, commands: 1, mcps: 1 },
        mounts: ["review"],
      } satisfies ProjectSummary,
    ]);

    const { container } = render(<ProjectsListPage />);

    const row = await screen.findByRole("option", { name: "local-app" });
    expect(row).toHaveAttribute("aria-selected", "true");
    expect(screen.getByText("/tmp/local-app")).toBeInTheDocument();
    expect(container.textContent).toContain("只读真实数据");
    expect(screen.getByRole("button", { name: "扫描项目" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "管理挂载" })).toBeDisabled();
  });

  it("shows an empty state instead of static projects when listProjects is empty", async () => {
    listProjects.mockResolvedValue([]);

    const { container } = render(<ProjectsListPage />);

    expect(await screen.findByText("未发现本地项目")).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "project-a" })).not.toBeInTheDocument();
    expect(container.textContent).toContain("未发现本地项目");
  });

  it("feeds read-only backup history with manifest metadata and manual restore guidance", async () => {
    listBackups.mockResolvedValue([
      {
        id: "restore-20260627",
        label: "Restore fixture backup",
        createdAt: "2026-06-27T10:00:00Z",
        sizeBytes: 2048,
        entryCount: 2,
        manifestPath: "/tmp/backups/restore-20260627/manifest.json",
        runtimeRoot: "/tmp/home",
        affectedPaths: ["/tmp/restore/a", "/tmp/restore/b"],
      },
    ]);

    render(<BackupRestorePage />);

    const backupRow = await screen.findByRole("option", { name: "restore-20260627" });
    expect(backupRow).toBeInTheDocument();
    expect(screen.getAllByText("Restore fixture backup").length).toBeGreaterThan(0);
    expect(backupRow).toHaveTextContent("2.0 KB");
    expect(screen.getByText("/tmp/backups/restore-20260627/manifest.json")).toBeInTheDocument();
    expect(await screen.findByText("/tmp/restore/a")).toBeInTheDocument();
    expect(screen.getByText("手动恢复说明")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /恢复/ })).not.toBeInTheDocument();
  });

  it("displays read-only GitStatus fields and keeps Pull and Push disabled", async () => {
    gitStatus.mockResolvedValue(gitStatusFixture({
      repositoryPath: "/tmp/assets",
      isRepository: true,
      statusMessage: "Git repository has no upstream.",
      branch: "feature/assets",
      remoteName: "origin",
      clean: false,
      ahead: 4,
      behind: 2,
      changedFiles: ["assets.yaml"],
      conflicts: ["mounts.yaml"],
      syncableChanges: ["assets.yaml"],
      blockedChanges: ["mounts.yaml"],
    }));

    render(<SyncPage />);

    expect(await screen.findByText("/tmp/assets")).toBeInTheDocument();
    expect(screen.getByText("feature/assets")).toBeInTheDocument();
    expect(screen.getAllByText("origin").length).toBeGreaterThan(0);
    expect(screen.getByText("4 commits")).toBeInTheDocument();
    expect(screen.getByText("2 commits")).toBeInTheDocument();
    expect(screen.getAllByText("Git repository has no upstream.").length).toBeGreaterThan(0);
    expect(screen.getByText("mounts.yaml", { exact: false })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "预览 Pull" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "预览 Push" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "执行 Push" })).toBeDisabled();
  });

  it("generates a Sync plan and confirms Push without typed input", async () => {
    render(<SyncPage />);

    await waitFor(() => expect(gitStatus).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "预览 Push" }));

    await waitFor(() => expect(previewSync).toHaveBeenCalledWith({ direction: "push" }));
    expect(await screen.findByText(/git push origin main/)).toBeInTheDocument();
    expect(screen.getByText("计划方向")).toBeInTheDocument();
    expect(screen.getByText("计划可执行")).toBeInTheDocument();
    const pushButton = screen.getByRole("button", { name: "执行 Push" });
    expect(pushButton).toBeEnabled();
    expect(screen.queryByPlaceholderText("APPLY")).not.toBeInTheDocument();
    fireEvent.click(pushButton);

    await waitFor(() => expect(syncApply).toHaveBeenCalledWith({
      previewId: "preview:sync:push",
      previewGeneratedAtEpochSeconds: 100,
      request: { direction: "push" },
    }));
    expect(await screen.findByText(/执行完成/)).toBeInTheDocument();
    await waitFor(() => expect(gitStatus).toHaveBeenCalledTimes(2));
  });

  it("displays loaded settings and saves edited values through the settings command", async () => {
    settingsLoad.mockResolvedValue(settingsFixture({
      assetCenterPath: "/tmp/assets",
      scanRoots: ["/tmp/workspace", "/tmp/code"],
      gitDefaultBranch: "trunk",
      gitRemote: "upstream",
      cliPath: "/tmp/maa",
    }));

    render(<SettingsPage />);

    const assetCenter = await screen.findByDisplayValue("/tmp/assets");
    expect(assetCenter).toBeInTheDocument();
    expect(screen.getByDisplayValue("/tmp/workspace, /tmp/code")).toBeInTheDocument();
    expect(screen.getByDisplayValue("trunk")).toBeInTheDocument();
    expect(screen.getByDisplayValue("upstream")).toBeInTheDocument();
    expect(screen.getByDisplayValue("/tmp/maa")).toBeInTheDocument();

    expect(assetCenter).toHaveAttribute("readonly");
    fireEvent.click(screen.getByRole("button", { name: "保存设置" }));

    await waitFor(() => expect(settingsSave).toHaveBeenCalledWith({
      settings: expect.objectContaining({
        assetCenterPath: "/tmp/assets",
        gitDefaultBranch: "trunk",
        gitRemote: "upstream",
      }),
    }));
    await waitFor(() => expect(settingsLoad).toHaveBeenCalledTimes(2));
    expect(screen.getByText("设置已写入本地配置，并已从后端重新读取确认。")).toBeInTheDocument();
  });

  it("shows settings save failures and never reports a successful save", async () => {
    settingsSave.mockRejectedValue(new Error("permission denied"));

    render(<SettingsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "保存设置" }));

    expect(await screen.findByText(/保存失败：permission denied/)).toBeInTheDocument();
    expect(screen.queryByText("设置已写入本地配置，并已从后端重新读取确认。")).not.toBeInTheDocument();
    expect(settingsLoad).toHaveBeenCalledTimes(1);
  });

  it("blocks Scan Import when the backend reports unresolved conflicts", async () => {
    const onOpenConflicts = vi.fn();
    discoverRuntimeSources.mockResolvedValue(discoveryFixture("source:review", "review"));
    canonicalBatchImportPreview.mockResolvedValue(batchImportPreviewFixture({
      items: [{
        ...canonicalImportItemFixture("source:review", "skill:review"),
        disposition: "conflict",
        canApply: false,
        conflict: {
          assetId: "skill:review",
          reason: "same name",
          existingContent: "# Existing",
          incomingContent: "# Incoming",
          rawSource: "# Incoming",
        },
      }],
      canApply: false,
    }));

    render(<ScanImportPage onOpenConflicts={onOpenConflicts} />);

    await waitFor(() => expect(discoverRuntimeSources).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "生成导入计划" }));
    expect(await screen.findByText("发现 1 项内容冲突")).toBeInTheDocument();
    expect(screen.getByText(/请逐项选择跳过、重命名或覆盖/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "确认导入" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "处理冲突" }));
    expect(onOpenConflicts).toHaveBeenCalledWith(expect.objectContaining({
      scope: { kind: "user" },
      preview: expect.objectContaining({ previewId: "batch-import:test" }),
    }));
    expect(canonicalBatchImportApply).not.toHaveBeenCalled();
  });

  it("calls unified discovery for explicit scopes and keeps import disabled before preview", async () => {
    discoverRuntimeSources.mockResolvedValue(discoveryFixture("source:live-scan", "live-scan"));

    render(<ScanImportPage />);

    await waitFor(() => expect(discoverRuntimeSources).toHaveBeenCalledWith({ kind: "user" }));
    expect(await screen.findByText("live-scan")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /项目级/ }));
    await waitFor(() => expect(discoverRuntimeSources).toHaveBeenLastCalledWith(
      { kind: "project", projectPath: "~/workspace/project-a" },
    ));

    fireEvent.click(screen.getByRole("button", { name: /自定义路径/ }));
    await waitFor(() => expect(discoverRuntimeSources).toHaveBeenLastCalledWith({
      kind: "custom",
      path: "~/code/design-system/.agents/skills",
      assetKind: "skill",
      sourceFormat: "skill_directory",
    }));

    expect(screen.getByRole("button", { name: "确认导入" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "保存扫描预览" })).toBeDisabled();
  });

  it("generates an atomic batch import preview before enabling confirmation", async () => {
    discoverRuntimeSources.mockResolvedValue(discoveryFixture("source:live-scan", "live-scan"));
    canonicalBatchImportPreview.mockResolvedValue(batchImportPreviewFixture({
      items: [canonicalImportItemFixture("source:live-scan", "skill:live-scan")],
    }));

    render(<ScanImportPage />);

    await waitFor(() => expect(discoverRuntimeSources).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "生成导入计划" }));

    await waitFor(() => expect(canonicalBatchImportPreview).toHaveBeenCalledWith({
      scope: { kind: "user" },
      selections: [{
        sourceId: "source:live-scan",
        resolution: { kind: "unresolved" },
      }],
    }));
    expect(await screen.findByText(/skill:live-scan：新增/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "确认导入" })).toBeEnabled();
  });

  it("executes atomic batch import after preview without typed input", async () => {
    discoverRuntimeSources.mockResolvedValue(discoveryFixture("source:live-scan", "live-scan"));
    canonicalBatchImportPreview.mockResolvedValue(batchImportPreviewFixture({
      items: [canonicalImportItemFixture("source:live-scan", "skill:live-scan")],
    }));
    canonicalBatchImportApply.mockResolvedValue({
      previewId: "batch-import:test",
      items: [{
        previewId: "import:live-scan",
        assetId: "skill:live-scan",
        status: "imported",
        affectedPaths: ["/tmp/assets/skills/live-scan"],
      }],
      affectedPaths: ["/tmp/assets/skills/live-scan"],
      journalPath: "/tmp/batch-import-journal",
    });

    render(<ScanImportPage />);

    await waitFor(() => expect(discoverRuntimeSources).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "生成导入计划" }));
    await waitFor(() => expect(canonicalBatchImportPreview).toHaveBeenCalled());

    const applyButton = screen.getByRole("button", { name: "确认导入" });
    expect(applyButton).toBeEnabled();
    expect(screen.queryByPlaceholderText("APPLY")).not.toBeInTheDocument();
    fireEvent.click(applyButton);

    await waitFor(() => expect(canonicalBatchImportApply).toHaveBeenLastCalledWith({
      previewId: "batch-import:test",
      previewGeneratedAtEpochSeconds: 100,
      request: {
        scope: { kind: "user" },
        selections: [{
          sourceId: "source:live-scan",
          resolution: { kind: "unresolved" },
        }],
      },
    }));
    expect(await screen.findByText(/执行完成/)).toBeInTheDocument();
    await waitFor(() => expect(discoverRuntimeSources).toHaveBeenCalledTimes(2));
  });

  it("uses the backend-composed adopt workflow instead of chaining import and mount", async () => {
    discoverRuntimeSources.mockResolvedValue(discoveryFixture("source:live-scan", "live-scan"));
    previewAdopt.mockResolvedValue({
      previewId: "adopt:test",
      items: [],
      importPlan: ["import skill:live-scan"],
      mountPlan: ["mount through claude-user-skills"],
      backupPlan: ["backup runtime source"],
      warnings: [],
      canApply: true,
      generatedAtEpochSeconds: 100,
      expiresAtEpochSeconds: 400,
    });
    adoptApply.mockResolvedValue({
      previewId: "adopt:test",
      items: [{
        sourceId: "source:live-scan",
        assetId: "skill:live-scan",
        targetId: "claude-user-skills",
        imported: true,
        mounted: true,
      }],
      affectedPaths: ["/tmp/home/.claude/skills/live-scan"],
      journalPath: "/tmp/adopt-journal",
    });

    render(<ScanImportPage />);
    await waitFor(() => expect(discoverRuntimeSources).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "生成接管计划" }));
    await waitFor(() => expect(previewAdopt).toHaveBeenCalledWith({
      scope: { kind: "user" },
      selections: [{
        sourceId: "source:live-scan",
        resolution: { kind: "unresolved" },
      }],
    }));
    const applyButton = screen.getByRole("button", { name: "导入并接管" });
    expect(applyButton).toBeEnabled();
    fireEvent.click(applyButton);
    await waitFor(() => expect(adoptApply).toHaveBeenCalledWith({
      previewId: "adopt:test",
      previewGeneratedAtEpochSeconds: 100,
      request: {
        scope: { kind: "user" },
        selections: [{
          sourceId: "source:live-scan",
          resolution: { kind: "unresolved" },
        }],
      },
    }));
    expect(canonicalBatchImportApply).not.toHaveBeenCalled();
    expect(canonicalMountApply).not.toHaveBeenCalled();
  });

  it("renders target-registry mount preview and conflict data", async () => {
    canonicalMountPreview.mockResolvedValue(canonicalMountPreviewFixture({
      plannedEffects: ["预览资产来源", "预览目标挂载"],
      warnings: ["Preview mount warning"],
    }));
    const { rerender } = render(<MountManagerPage />);
    await waitFor(() => expect(canonicalMountPreview).toHaveBeenCalled());
    expect(await screen.findByText("预览资产来源")).toBeInTheDocument();
    expect(screen.getByText("Preview mount warning")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "刷新挂载计划" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "确认挂载" })).toBeEnabled();

    rerender(<ConflictResolverPage demoMode />);
    fireEvent.click(screen.getByRole("option", { name: "review" }));
    await waitFor(() => expect(screen.getByText(/检查架构、性能和安全边界/)).toBeInTheDocument());
    expect(screen.getByRole("button", { name: "执行冲突处理" })).toBeDisabled();
  });

  it("refreshes the targetId-only mount preview without calling apply", async () => {
    render(<MountManagerPage />);

    await waitFor(() => expect(canonicalMountPreview).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: /project-a-skills/ }));
    await waitFor(() => expect(canonicalMountPreview).toHaveBeenLastCalledWith({
      assetId: "skill:review",
      targetId: "project-a-skills",
    }));
    fireEvent.click(screen.getByRole("button", { name: "刷新挂载计划" }));
    await waitFor(() => expect(canonicalMountPreview.mock.calls.length).toBeGreaterThanOrEqual(3));
    expect(canonicalMountApply).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "确认挂载" })).toBeEnabled();
  });

  it("executes targetId-only mount apply without typed input", async () => {
    render(<MountManagerPage />);

    await waitFor(() => expect(canonicalMountPreview).toHaveBeenCalled());

    const mountButton = screen.getByRole("button", { name: "确认挂载" });
    expect(mountButton).toBeEnabled();
    expect(screen.queryByPlaceholderText("APPLY")).not.toBeInTheDocument();
    fireEvent.click(mountButton);

    await waitFor(() => expect(canonicalMountApply).toHaveBeenLastCalledWith({
      previewId: "mount:skill-review",
      previewGeneratedAtEpochSeconds: 100,
      request: {
        assetId: "skill:review",
        targetId: "claude-user-skills",
      },
    }));
    expect(await screen.findByText(/执行完成/)).toBeInTheDocument();
    await waitFor(() => expect(canonicalMountPreview.mock.calls.length).toBeGreaterThanOrEqual(2));
  });

  it("resolves canonical Scan conflicts through atomic batch preview and apply", async () => {
    const conflictItem = {
      ...canonicalImportItemFixture("source:review", "skill:review"),
      disposition: "conflict" as const,
      canApply: false,
      conflict: {
        assetId: "skill:review",
        reason: "same name",
        existingContent: "# Existing",
        incomingContent: "# Incoming",
        rawSource: "# Incoming",
      },
    };
    const context = {
      scope: { kind: "user" as const },
      preview: batchImportPreviewFixture({
        items: [conflictItem],
        canApply: false,
      }),
    };
    canonicalBatchImportPreview.mockResolvedValue(batchImportPreviewFixture({
      previewId: "batch-import:resolved",
      items: [{
        ...conflictItem,
        disposition: "rename",
        destinationName: "review-imported",
        canApply: true,
      }],
      canApply: true,
    }));
    canonicalBatchImportApply.mockResolvedValue({
      previewId: "batch-import:resolved",
      items: [{
        previewId: "import:review",
        assetId: "skill:review-imported",
        status: "imported",
        affectedPaths: ["/tmp/assets/skills/review-imported"],
      }],
      affectedPaths: ["/tmp/assets/skills/review-imported"],
      journalPath: "/tmp/conflict-journal",
    });

    render(<ConflictResolverPage context={context} />);

    expect(screen.getByText("# Existing")).toBeInTheDocument();
    expect(screen.getByText("# Incoming")).toBeInTheDocument();
    expect(screen.getByText(/review 将被跳过/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /重命名.*以新名称导入当前内容/ }));
    expect(screen.getByText(/review 将以 review-imported 导入/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "生成处理计划" }));
    await waitFor(() => expect(canonicalBatchImportPreview).toHaveBeenCalledWith({
      scope: { kind: "user" },
      selections: [{
        sourceId: "source:review",
        resolution: { kind: "rename", newName: "review-imported" },
      }],
    }));
    const applyButton = screen.getByRole("button", { name: "执行冲突处理" });
    expect(applyButton).toBeEnabled();
    expect(screen.queryByPlaceholderText("APPLY")).not.toBeInTheDocument();
    fireEvent.click(applyButton);
    await waitFor(() => expect(canonicalBatchImportApply).toHaveBeenCalledWith({
      previewId: "batch-import:resolved",
      previewGeneratedAtEpochSeconds: 100,
      request: {
        scope: { kind: "user" },
        selections: [{
          sourceId: "source:review",
          resolution: { kind: "rename", newName: "review-imported" },
        }],
      },
    }));
    expect(await screen.findByText(/执行完成/)).toBeInTheDocument();
  });

  it("uses selected real asset data for detail mount preview, apply, and refresh", async () => {
    const asset = assetFixture("skill:real-review", "real-review", "skill");
    listAssets.mockResolvedValue([{ ...asset, mountTargets: ["/tmp/home/.claude/skills/real-review.md"] }]);
    canonicalMountPreview.mockResolvedValue(canonicalMountPreviewFixture({
      previewId: "preview:mount:real-review",
      assetId: asset.id,
      targetId: "claude-user-skills",
      affectedTargetPath: "/tmp/home/.claude/skills/real-review",
    }));

    render(<AssetDetailPage detail={{
      assetId: asset.id,
      assetType: "skill",
      name: asset.name,
      title: "Real Review",
      summary: "Real selected asset",
      status: "可用",
      statusTone: "success",
      typeLabel: "Skill",
      category: "本地 Skill",
      sourcePath: "/tmp/assets/skills/real-review.md",
      scope: "资产中心",
      updated: "刚刚",
      mountTargets: [],
      previewLabel: "SKILL.md 内容预览",
      preview: "# Real Review",
    }} />);

    expect(screen.getByText("Real selected asset")).toBeInTheDocument();
    await waitFor(() => expect(canonicalMountPreview).toHaveBeenCalledWith({
      assetId: "skill:real-review",
      targetId: "claude-user-skills",
    }));
    expect(screen.getByRole("button", { name: "确认挂载" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "确认挂载" }));
    await waitFor(() => expect(canonicalMountApply).toHaveBeenLastCalledWith({
      previewId: "preview:mount:real-review",
      previewGeneratedAtEpochSeconds: 100,
      request: {
        assetId: "skill:real-review",
        targetId: "claude-user-skills",
      },
    }));
    await waitFor(() => expect(listAssets).toHaveBeenCalledWith({ assetType: "skill" }));
    expect(await screen.findByText("/tmp/home/.claude/skills/real-review.md")).toBeInTheDocument();
  });

  it("uses selected real project data for project mount preview, apply, and refresh", async () => {
    const project = staticProjects[0];
    const asset = assetFixture("skill:review", "review", "skill");
    listAssets.mockResolvedValue([asset]);
    listProjects.mockResolvedValue([projectFixture({
      id: project.id,
      name: project.name,
      path: project.path,
      mounts: ["review"],
      assetCounts: { total: 1, skills: 1, commands: 0, mcps: 0 },
    })]);
    listMountTargets.mockResolvedValue([{
      id: "project-a-skills",
      kind: "claude_project_skills",
      provider: "claude_code",
      accepts: ["skill"],
      adapter: "symlink_directory",
      scope: "project",
      path: "~/workspace/project-a/.claude/skills",
      projectPath: "~/workspace/project-a",
      providerState: "initialized",
      status: "ready",
    }]);
    canonicalMountPreview.mockResolvedValue(canonicalMountPreviewFixture({
      previewId: "preview:mount:project-detail",
      assetId: asset.id,
      targetId: "project-a-skills",
      affectedTargetPath: "~/workspace/project-a/.claude/skills/review",
    }));

    render(<ProjectDetailPage detail={project} />);

    await waitFor(() => expect(canonicalMountPreview).toHaveBeenCalledWith({
      assetId: "skill:review",
      targetId: "project-a-skills",
    }));
    expect(screen.getByRole("button", { name: "确认项目挂载" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "确认项目挂载" }));
    await waitFor(() => expect(canonicalMountApply).toHaveBeenLastCalledWith({
      previewId: "preview:mount:project-detail",
      previewGeneratedAtEpochSeconds: 100,
      request: {
        assetId: "skill:review",
        targetId: "project-a-skills",
      },
    }));
    await waitFor(() => expect(listProjects).toHaveBeenCalled());
  });

  it("does not call apply command wrappers from Scan Import preview", async () => {
    render(<ScanImportPage />);
    await waitFor(() => expect(discoverRuntimeSources).toHaveBeenCalled());
    expect(canonicalBatchImportPreview).not.toHaveBeenCalled();
    expect(canonicalBatchImportApply).not.toHaveBeenCalled();
  });
});

function assetFixture(id: string, name: string, assetType: AssetSummary["assetType"]): AssetSummary {
  return {
    id,
    name,
    title: `${name} title`,
    assetType,
    status: "ready",
    category: "local",
    description: `${name} summary`,
    sourcePath: `/tmp/assets/${name}`,
    scope: "local",
    updatedAt: "2026-06-25T08:00:00Z",
    mountTargets: ["/tmp/runtime"],
  };
}

function projectFixture(overrides: Partial<ProjectSummary> = {}): ProjectSummary {
  return {
    id: "/tmp/project-a",
    name: "project-a",
    title: "Project A",
    path: "/tmp/project-a",
    status: "ready",
    description: "Local project",
    updatedAt: "2026-06-27T00:00:00Z",
    assetCounts: { total: 0, skills: 0, commands: 0, mcps: 0 },
    mounts: [],
    ...overrides,
  };
}

function gitStatusFixture(overrides: Partial<GitStatus> = {}): GitStatus {
  return {
    repositoryPath: "~/.my-agent-assets",
    isRepository: false,
    statusMessage: "Asset center directory does not exist.",
    branch: "",
    remoteName: "origin",
    clean: true,
    ahead: 0,
    behind: 0,
    changedFiles: [],
    conflicts: [],
    syncableChanges: [],
    blockedChanges: [],
    lastSyncedAt: null,
    ...overrides,
  };
}

function settingsFixture(overrides: Partial<DesktopSettings> = {}): DesktopSettings {
  return {
    assetCenterPath: "~/.my-agent-assets",
    scanRoots: ["~/.claude", "~/workspace", "~/code"],
    maxDepth: 5,
    backupBeforeApply: true,
    planOnlyByDefault: true,
    gitDefaultBranch: "main",
    gitRemote: "origin",
    appearanceTheme: "system",
    density: "compact",
    logLevel: "info",
    logRetentionDays: 14,
    cliPath: "maa",
    ...overrides,
  };
}

function discoveryFixture(sourceId: string, name: string) {
  return {
    sources: [{
      sourceId,
      provider: "claude_code" as const,
      sourcePath: `/tmp/home/.claude/skills/${name}`,
      assetKind: "skill" as const,
      assetName: name,
      sourceFormat: "skill_directory" as const,
      scope: "user" as const,
      isManaged: false,
      isSymlink: false,
      warnings: [],
      eligibleImport: true,
      eligibleAdopt: true,
    }],
    warnings: [],
  };
}

function canonicalImportItemFixture(sourceId: string, assetId: string) {
  const name = assetId.split(":")[1];
  return {
    previewId: `import:${name}`,
    sourceId,
    assetId,
    assetType: "skill" as const,
    sourceName: name,
    destinationName: name,
    sourcePath: `/tmp/home/.claude/skills/${name}`,
    destinationPath: `/tmp/assets/skills/${name}`,
    disposition: "create" as const,
    warnings: [],
    canApply: true,
    generatedAtEpochSeconds: 100,
    expiresAtEpochSeconds: 400,
  };
}

function batchImportPreviewFixture(
  overrides: Partial<BatchImportPreview> = {},
): BatchImportPreview {
  return {
    previewId: "batch-import:test",
    items: [],
    warnings: [],
    canApply: true,
    generatedAtEpochSeconds: 100,
    expiresAtEpochSeconds: 400,
    ...overrides,
  };
}

function canonicalMountPreviewFixture(
  overrides: Partial<CanonicalMountPreview> = {},
): CanonicalMountPreview {
  return {
    previewId: "mount:skill-review",
    assetId: "skill:review",
    targetId: "claude-user-skills",
    canonicalPath: "/tmp/home/.my-agent-assets/assets/skills/review",
    affectedTargetPath: "/tmp/home/.claude/skills/review",
    compatible: true,
    adapter: "symlink_directory",
    disposition: "create_link",
    plannedEffects: ["预览挂载计划"],
    warnings: [],
    backupRequired: true,
    canApply: true,
    generatedAtEpochSeconds: 100,
    expiresAtEpochSeconds: 400,
    ...overrides,
  };
}
