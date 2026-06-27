import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  AssetSummary,
  ConflictPreview,
  DesktopSettings,
  GitStatus,
  ImportPreview,
  MountPreview,
  ProjectSummary,
  RestorePreview,
  ScanResult,
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
  scanAssets,
  previewImport,
  previewSync,
  syncApply,
  importApply,
  conflictApply,
  previewMount,
  previewConflicts,
  previewRestore,
  mountApply,
  restoreApply,
} = vi.hoisted(() => ({
  listAssets: vi.fn(),
  listProjects: vi.fn(),
  listBackups: vi.fn(),
  gitStatus: vi.fn(),
  settingsLoad: vi.fn(),
  settingsSave: vi.fn(),
  scanAssets: vi.fn(),
  previewImport: vi.fn(),
  previewSync: vi.fn(),
  syncApply: vi.fn(),
  importApply: vi.fn(),
  conflictApply: vi.fn(),
  previewMount: vi.fn(),
  previewConflicts: vi.fn(),
  previewRestore: vi.fn(),
  mountApply: vi.fn(),
  restoreApply: vi.fn(),
}));

vi.mock("../app/data-api", () => ({
  listAssets,
  listProjects,
  listBackups,
  gitStatus,
  settingsLoad,
  settingsSave,
  scanAssets,
  previewImport,
  previewSync,
  syncApply,
  importApply,
  conflictApply,
  previewMount,
  previewConflicts,
  previewRestore,
  mountApply,
  restoreApply,
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

beforeEach(() => {
  listAssets.mockResolvedValue([]);
  listProjects.mockResolvedValue([]);
  listBackups.mockResolvedValue([]);
  gitStatus.mockResolvedValue(gitStatusFixture());
  settingsLoad.mockResolvedValue(settingsFixture());
  settingsSave.mockImplementation(async ({ settings }) => settings);
  scanAssets.mockResolvedValue(scanResultFixture([]));
  previewImport.mockResolvedValue(importPreviewFixture([]));
  previewSync.mockResolvedValue({
    previewId: "preview:sync:push",
    direction: "push",
    repositoryPath: "~/.my-agent-assets",
    branch: "main",
    remote: "origin/main",
    steps: [
      { id: "check-git-repository", kind: "check", label: "校验本地 Git 仓库", description: "Repository ready.", risk: "none" },
      { id: "preview-git-sync", kind: "git", label: "生成 Push 计划", description: "No git push is executed.", risk: "medium" },
    ],
    warnings: ["Preview only: no git pull, push, or fetch is executed."],
    canApply: true,
  });
  syncApply.mockResolvedValue({
    mode: "apply",
    ok: true,
    previewId: "preview:sync:push",
    backup: null,
    steps: [
      {
        stepId: "git-sync",
        kind: "git",
        label: "执行 Push",
        status: "success",
        message: "git push completed.",
        affectedPaths: ["~/.my-agent-assets"],
      },
    ],
    warnings: [],
    errors: [],
  });
  importApply.mockResolvedValue({
    mode: "planOnly",
    ok: true,
    previewId: "preview:import:test",
    backup: null,
    steps: [
      {
        stepId: "plan-import-skill-live-scan",
        kind: "import",
        label: "预览导入",
        status: "skipped",
        message: "Plan-only mode: 1 asset would be imported.",
        affectedPaths: ["~/.my-agent-assets/assets/skills/live-scan"],
      },
    ],
    warnings: [],
    errors: [],
  });
  conflictApply.mockResolvedValue({
    mode: "planOnly",
    ok: true,
    previewId: "preview:import:test",
    backup: null,
    steps: [
      {
        stepId: "plan-conflict-review",
        kind: "import",
        label: "预览冲突处理",
        status: "skipped",
        message: "Plan-only mode: no files were written.",
        affectedPaths: [],
      },
    ],
    warnings: [],
    errors: [],
  });
  previewMount.mockResolvedValue(mountPreviewFixture());
  previewConflicts.mockResolvedValue([]);
  previewRestore.mockResolvedValue(restorePreviewFixture("backup-20260621-1842"));
  mountApply.mockResolvedValue({
    mode: "planOnly",
    ok: true,
    previewId: "preview:mount:skill-review-project-a",
    backup: null,
    steps: [
      {
        stepId: "plan-mount-skill-review",
        kind: "mount",
        label: "预览挂载",
        status: "skipped",
        message: "Plan-only mode: no symlink was created.",
        affectedPaths: ["~/workspace/project-a/.claude/skills/review"],
      },
    ],
    warnings: [],
    errors: [],
  });
  restoreApply.mockResolvedValue({
    mode: "planOnly",
    ok: true,
    previewId: "preview:restore:backup-20260621-1842",
    backup: null,
    steps: [
      {
        stepId: "plan-restore-backup-20260621-1842",
        kind: "restore",
        label: "预览恢复",
        status: "skipped",
        message: "Plan-only mode: 2 backup entries would be restored.",
        affectedPaths: ["/tmp/restore/one", "/tmp/restore/two"],
      },
    ],
    warnings: [],
    errors: [],
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

  it("falls back to static Asset Center data when read-only assets are empty", async () => {
    listAssets.mockResolvedValue([]);

    const { container } = render(<SkillsListPage />);
    expect(await screen.findByRole("option", { name: "review" })).toBeInTheDocument();
    expect(container.textContent).toContain("静态预览");
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

  it("falls back to static projects when listProjects is empty", async () => {
    listProjects.mockResolvedValue([]);

    const { container } = render(<ProjectsListPage />);

    expect(await screen.findByRole("option", { name: "project-a" })).toBeInTheDocument();
    expect(container.textContent).toContain("静态预览");
  });

  it("feeds read-only backup manifests while preserving restore planning", async () => {
    listBackups.mockResolvedValue([
      {
        id: "restore-20260627",
        label: "Restore fixture backup",
        createdAt: "2026-06-27T10:00:00Z",
        sizeBytes: 2048,
        entryCount: 2,
      },
    ]);
    previewRestore.mockResolvedValue(restorePreviewFixture("restore-20260627", {
      affectedPaths: ["/tmp/restore/a", "/tmp/restore/b"],
    }));

    render(<BackupRestorePage />);

    const backupRow = await screen.findByRole("option", { name: "restore-20260627" });
    expect(backupRow).toBeInTheDocument();
    expect(screen.getAllByText("Restore fixture backup").length).toBeGreaterThan(0);
    expect(backupRow).toHaveTextContent("2.0 KB");
    await waitFor(() => expect(previewRestore).toHaveBeenCalledWith({ backupId: "restore-20260627" }));
    expect(await screen.findByText("/tmp/restore/a")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "恢复此备份" })).toBeDisabled();
  });

  it("displays read-only GitStatus fields and keeps Pull and Push disabled", async () => {
    gitStatus.mockResolvedValue(gitStatusFixture({
      repositoryPath: "/tmp/assets",
      isRepository: true,
      statusMessage: "Git repository has no upstream.",
      branch: "feature/assets",
      remote: null,
      clean: false,
      ahead: 4,
      behind: 2,
      changedFiles: ["assets.yaml"],
      conflicts: ["mounts.yaml"],
    }));

    render(<SyncPage />);

    expect(await screen.findByText("/tmp/assets")).toBeInTheDocument();
    expect(screen.getByText("feature/assets")).toBeInTheDocument();
    expect(screen.getByText("未设置")).toBeInTheDocument();
    expect(screen.getByText("4 commits")).toBeInTheDocument();
    expect(screen.getByText("2 commits")).toBeInTheDocument();
    expect(screen.getAllByText("Git repository has no upstream.").length).toBeGreaterThan(0);
    expect(screen.getByText("mounts.yaml", { exact: false })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "预览 Pull" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "预览 Push" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "执行 Push" })).toBeDisabled();
  });

  it("generates a Sync plan and requires typed confirmation before executing Push", async () => {
    render(<SyncPage />);

    await waitFor(() => expect(gitStatus).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "预览 Push" }));

    await waitFor(() => expect(previewSync).toHaveBeenCalledWith({ direction: "push" }));
    expect(await screen.findByText(/生成 Push 计划/)).toBeInTheDocument();
    expect(screen.getByText("计划方向")).toBeInTheDocument();
    expect(screen.getByText("计划可执行")).toBeInTheDocument();
    const pushButton = screen.getByRole("button", { name: "执行 Push" });
    expect(pushButton).toBeDisabled();
    fireEvent.change(screen.getByPlaceholderText("APPLY"), { target: { value: "APPLY" } });
    expect(pushButton).toBeEnabled();
    fireEvent.click(pushButton);

    await waitFor(() => expect(syncApply).toHaveBeenCalledWith({
      previewId: "preview:sync:push",
      mode: "apply",
      direction: "push",
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

    fireEvent.change(assetCenter, { target: { value: "/tmp/edited-assets" } });
    fireEvent.click(screen.getByRole("button", { name: "保存设置" }));

    await waitFor(() => expect(settingsSave).toHaveBeenCalledWith({
      settings: expect.objectContaining({
        assetCenterPath: "/tmp/edited-assets",
        gitDefaultBranch: "trunk",
        gitRemote: "upstream",
      }),
    }));
    await waitFor(() => expect(settingsLoad).toHaveBeenCalledTimes(2));
    expect(screen.getByText("设置已写入本地配置，并已从后端重新读取确认。")).toBeInTheDocument();
  });

  it("calls scanAssets for the selected scope and keeps import disabled", async () => {
    scanAssets.mockResolvedValue(scanResultFixture([
      assetFixture("skill:live-scan", "live-scan", "skill"),
    ]));
    previewImport.mockResolvedValue(importPreviewFixture([
      assetFixture("skill:live-scan", "live-scan", "skill"),
    ]));

    render(<ScanImportPage />);

    await waitFor(() => expect(scanAssets).toHaveBeenCalledWith({ scope: { kind: "user" } }));
    await waitFor(() => expect(previewImport).toHaveBeenCalledWith({
      scope: { kind: "user" },
      assetIds: ["skill:live-scan"],
      conflictResolutions: [],
    }));
    expect(await screen.findByText("live-scan")).toBeInTheDocument();
    expect(await screen.findByText(/预览导入选择/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /项目级/ }));
    await waitFor(() => expect(scanAssets).toHaveBeenLastCalledWith({
      scope: { kind: "project", projectPath: "~/workspace/project-a" },
    }));

    fireEvent.click(screen.getByRole("button", { name: /自定义路径/ }));
    await waitFor(() => expect(scanAssets).toHaveBeenLastCalledWith({
      scope: { kind: "custom", path: "~/code/design-system" },
    }));

    expect(screen.getByRole("button", { name: "确认导入" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "保存扫描预览" })).toBeDisabled();
  });

  it("generates a plan-only import apply preview without enabling import execution", async () => {
    scanAssets.mockResolvedValue(scanResultFixture([
      assetFixture("skill:live-scan", "live-scan", "skill"),
    ]));
    previewImport.mockResolvedValue(importPreviewFixture([
      assetFixture("skill:live-scan", "live-scan", "skill"),
    ]));

    render(<ScanImportPage />);

    await waitFor(() => expect(previewImport).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "生成导入计划" }));

    await waitFor(() => expect(importApply).toHaveBeenCalledWith({
      previewId: "preview:import:test",
      mode: "planOnly",
      scope: { kind: "user" },
      assetIds: ["skill:live-scan"],
      conflictResolutions: [],
      backupBeforeApply: true,
    }));
    expect(await screen.findByText(/Plan-only mode: 1 asset would be imported/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "确认导入" })).toBeDisabled();
  });

  it("requires typed confirmation before executing import apply", async () => {
    scanAssets.mockResolvedValue(scanResultFixture([
      assetFixture("skill:live-scan", "live-scan", "skill"),
    ]));
    previewImport.mockResolvedValue(importPreviewFixture([
      assetFixture("skill:live-scan", "live-scan", "skill"),
    ]));

    render(<ScanImportPage />);

    await waitFor(() => expect(previewImport).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "生成导入计划" }));
    await waitFor(() => expect(importApply).toHaveBeenCalledWith(expect.objectContaining({ mode: "planOnly" })));

    const applyButton = screen.getByRole("button", { name: "确认导入" });
    expect(applyButton).toBeDisabled();
    fireEvent.change(screen.getByPlaceholderText("APPLY"), { target: { value: "APPLY" } });
    expect(applyButton).toBeEnabled();
    fireEvent.click(applyButton);

    await waitFor(() => expect(importApply).toHaveBeenLastCalledWith({
      previewId: "preview:import:test",
      mode: "apply",
      scope: { kind: "user" },
      assetIds: ["skill:live-scan"],
      conflictResolutions: [],
      backupBeforeApply: true,
    }));
    expect(await screen.findByText(/执行完成/)).toBeInTheDocument();
    await waitFor(() => expect(scanAssets).toHaveBeenCalledTimes(2));
  });

  it("renders preview-only mount, conflict, and restore data while keeping actions disabled", async () => {
    previewMount.mockResolvedValue(mountPreviewFixture({
      steps: [
        { id: "check", kind: "check", label: "预览资产来源", description: "check", risk: "none" },
        { id: "mount", kind: "mount", label: "预览目标挂载", description: "mount", risk: "medium" },
      ],
      warnings: ["Preview mount warning"],
    }));
    previewConflicts.mockResolvedValue([
      conflictPreviewFixture("conflict:skill:review", "review", "skill"),
    ]);
    previewRestore.mockResolvedValue(restorePreviewFixture("backup-20260621-1842", {
      affectedPaths: ["/tmp/restore/one", "/tmp/restore/two"],
      steps: [
        { id: "restore", kind: "restore", label: "预览恢复影响", description: "restore", risk: "high" },
      ],
      warnings: ["Preview restore warning"],
    }));

    const { rerender } = render(<MountManagerPage />);
    await waitFor(() => expect(previewMount).toHaveBeenCalled());
    expect(await screen.findByText("预览资产来源")).toBeInTheDocument();
    expect(screen.getByText("Preview mount warning")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "生成挂载计划" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "确认挂载" })).toBeDisabled();

    rerender(<ConflictResolverPage />);
    await waitFor(() => expect(screen.getByText("Incoming preview content for review")).toBeInTheDocument());
    expect(screen.getByRole("button", { name: "执行冲突处理" })).toBeDisabled();

    rerender(<BackupRestorePage />);
    expect(await screen.findByText("/tmp/restore/one")).toBeInTheDocument();
    expect(screen.getByText("预览恢复影响")).toBeInTheDocument();
    expect(screen.getByText("Preview restore warning")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "生成恢复计划" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "恢复此备份" })).toBeDisabled();
  });

  it("generates a plan-only restore apply preview without enabling restore execution", async () => {
    render(<BackupRestorePage />);

    await screen.findByText("backup-20260621-1842");
    fireEvent.click(screen.getByRole("button", { name: "生成恢复计划" }));

    await waitFor(() => expect(restoreApply).toHaveBeenCalledWith({
      previewId: "preview:restore:backup-20260621-1842",
      mode: "planOnly",
      backupId: "backup-20260621-1842",
      backupBeforeRestore: true,
    }));
    expect(await screen.findByText(/Plan-only mode/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "恢复此备份" })).toBeDisabled();
  });

  it("requires typed confirmation before executing restore apply", async () => {
    render(<BackupRestorePage />);

    await screen.findByText("backup-20260621-1842");
    fireEvent.click(screen.getByRole("button", { name: "生成恢复计划" }));
    await waitFor(() => expect(restoreApply).toHaveBeenCalledWith(expect.objectContaining({ mode: "planOnly" })));

    const restoreButton = screen.getByRole("button", { name: "恢复此备份" });
    expect(restoreButton).toBeDisabled();
    fireEvent.change(screen.getByPlaceholderText("APPLY"), { target: { value: "APPLY" } });
    expect(restoreButton).toBeEnabled();
    fireEvent.click(restoreButton);

    await waitFor(() => expect(restoreApply).toHaveBeenLastCalledWith({
      previewId: "preview:restore:backup-20260621-1842",
      mode: "apply",
      backupId: "backup-20260621-1842",
      backupBeforeRestore: true,
    }));
    expect(await screen.findByText(/执行完成/)).toBeInTheDocument();
    await waitFor(() => expect(listBackups).toHaveBeenCalledTimes(2));
  });

  it("generates a plan-only mount apply preview without enabling mount execution", async () => {
    render(<MountManagerPage />);

    await waitFor(() => expect(previewMount).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "生成挂载计划" }));

    await waitFor(() => expect(mountApply).toHaveBeenCalledWith({
      previewId: "preview:mount:skill-review-project-a",
      mode: "planOnly",
      assetId: "skill:review",
      target: {
        scope: "project",
        runtimePath: "~/workspace/project-a/.claude/skills/review",
        projectPath: "~/workspace/project-a",
      },
      backupBeforeApply: true,
    }));
    expect(await screen.findByText(/Plan-only mode: no symlink was created/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "确认挂载" })).toBeDisabled();
  });

  it("requires typed confirmation before executing mount apply", async () => {
    render(<MountManagerPage />);

    await waitFor(() => expect(previewMount).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "生成挂载计划" }));
    await waitFor(() => expect(mountApply).toHaveBeenCalledWith(expect.objectContaining({ mode: "planOnly" })));

    const mountButton = screen.getByRole("button", { name: "确认挂载" });
    expect(mountButton).toBeDisabled();
    fireEvent.change(screen.getByPlaceholderText("APPLY"), { target: { value: "APPLY" } });
    expect(mountButton).toBeEnabled();
    fireEvent.click(mountButton);

    await waitFor(() => expect(mountApply).toHaveBeenLastCalledWith({
      previewId: "preview:mount:skill-review-project-a",
      mode: "apply",
      assetId: "skill:review",
      target: {
        scope: "project",
        runtimePath: "~/workspace/project-a/.claude/skills/review",
        projectPath: "~/workspace/project-a",
      },
      backupBeforeApply: true,
    }));
    expect(await screen.findByText(/执行完成/)).toBeInTheDocument();
    await waitFor(() => expect(previewMount).toHaveBeenCalledTimes(2));
  });

  it("updates Conflict Resolver decisions and requires a plan plus typed confirmation", async () => {
    previewConflicts.mockResolvedValue([
      conflictPreviewFixture("conflict:skill:review", "review", "skill"),
    ]);

    render(<ConflictResolverPage />);

    await waitFor(() => expect(screen.getByText("Incoming preview content for review")).toBeInTheDocument());
    expect(screen.getByText(/review 将被跳过/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /覆盖.*使用扫描结果替换现有内容/ }));
    expect(screen.getByText(/review 将在未来确认导入时覆盖资产中心内容/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /重命名.*以新名称导入当前内容/ }));
    expect(screen.getByText(/review 将以新名称导入/)).toBeInTheDocument();
    expect(screen.getByText(/新名称：review-imported/)).toBeInTheDocument();

    await waitFor(() => expect(previewImport).toHaveBeenLastCalledWith({
      scope: { kind: "user" },
      assetIds: ["skill:review"],
      conflictResolutions: [{
        conflictId: "conflict:skill:review",
        resolution: "rename",
        renameTo: "review-imported",
      }],
    }));

    fireEvent.click(screen.getByRole("button", { name: "生成处理计划" }));
    await waitFor(() => expect(conflictApply).toHaveBeenCalledWith(expect.objectContaining({
      mode: "planOnly",
      assetIds: ["skill:review"],
    })));
    const applyButton = screen.getByRole("button", { name: "执行冲突处理" });
    expect(applyButton).toBeDisabled();
    fireEvent.change(screen.getByPlaceholderText("APPLY"), { target: { value: "APPLY" } });
    expect(applyButton).toBeEnabled();
    fireEvent.click(applyButton);
    await waitFor(() => expect(conflictApply).toHaveBeenLastCalledWith(expect.objectContaining({
      mode: "apply",
      assetIds: ["skill:review"],
    })));
    expect(importApply).not.toHaveBeenCalled();
    expect(mountApply).not.toHaveBeenCalled();
    expect(restoreApply).not.toHaveBeenCalled();
  });

  it("uses selected real asset data for detail mount preview, apply, and refresh", async () => {
    const asset = assetFixture("skill:real-review", "real-review", "skill");
    listAssets.mockResolvedValue([{ ...asset, mountTargets: ["/tmp/home/.claude/skills/real-review.md"] }]);
    previewMount.mockResolvedValue(mountPreviewFixture({
      previewId: "preview:mount:real-review",
      asset,
      target: {
        scope: "user",
        runtimePath: "~/.claude/skills/real-review.md",
        projectPath: null,
      },
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
    await waitFor(() => expect(previewMount).toHaveBeenCalledWith({
      assetId: "skill:real-review",
      target: {
        scope: "user",
        runtimePath: "~/.claude/skills/real-review.md",
        projectPath: null,
      },
    }));
    fireEvent.click(screen.getByRole("button", { name: "生成挂载计划" }));
    await waitFor(() => expect(mountApply).toHaveBeenCalledWith(expect.objectContaining({ mode: "planOnly" })));
    fireEvent.change(screen.getByPlaceholderText("APPLY"), { target: { value: "APPLY" } });
    fireEvent.click(screen.getByRole("button", { name: "确认挂载" }));
    await waitFor(() => expect(mountApply).toHaveBeenLastCalledWith(expect.objectContaining({ mode: "apply" })));
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
    previewMount.mockResolvedValue(mountPreviewFixture({
      previewId: "preview:mount:project-detail",
      asset,
      target: {
        scope: "project",
        runtimePath: "~/workspace/project-a/.claude/skills/review",
        projectPath: "~/workspace/project-a",
      },
    }));

    render(<ProjectDetailPage detail={project} />);

    await waitFor(() => expect(previewMount).toHaveBeenCalledWith({
      assetId: "skill:review",
      target: {
        scope: "project",
        runtimePath: "~/workspace/project-a/.claude/skills/review",
        projectPath: "~/workspace/project-a",
      },
    }));
    fireEvent.click(screen.getByRole("button", { name: "生成挂载计划" }));
    await waitFor(() => expect(mountApply).toHaveBeenCalledWith(expect.objectContaining({ mode: "planOnly" })));
    fireEvent.change(screen.getByPlaceholderText("APPLY"), { target: { value: "APPLY" } });
    fireEvent.click(screen.getByRole("button", { name: "确认项目挂载" }));
    await waitFor(() => expect(mountApply).toHaveBeenLastCalledWith(expect.objectContaining({ mode: "apply" })));
    await waitFor(() => expect(listProjects).toHaveBeenCalled());
  });

  it("does not call apply command wrappers from Scan Import preview", async () => {
    render(<ScanImportPage />);
    await waitFor(() => expect(scanAssets).toHaveBeenCalled());
    expect(previewImport).not.toHaveBeenCalled();
    expect(importApply).not.toHaveBeenCalled();
    expect(mountApply).not.toHaveBeenCalled();
    expect(restoreApply).not.toHaveBeenCalled();
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
    remote: null,
    clean: true,
    ahead: 0,
    behind: 0,
    changedFiles: [],
    conflicts: [],
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

function scanResultFixture(assets: AssetSummary[]): ScanResult {
  return {
    scope: { kind: "user" },
    scannedAt: "2026-06-25T08:00:00Z",
    assets,
    counts: {
      total: assets.length,
      skills: assets.filter((asset) => asset.assetType === "skill").length,
      commands: assets.filter((asset) => asset.assetType === "command").length,
      mcps: assets.filter((asset) => asset.assetType === "mcp").length,
    },
    conflictCount: 0,
    warnings: [],
  };
}

function importPreviewFixture(assets: AssetSummary[]): ImportPreview {
  return {
    previewId: "preview:import:test",
    scope: { kind: "user" },
    assets,
    conflicts: [],
    steps: [
      { id: "preview-import", kind: "import", label: "预览导入选择", description: "No write", risk: "low" },
    ],
    warnings: ["Preview only: no files will be written."],
    canApply: true,
  };
}

function mountPreviewFixture(overrides: Partial<MountPreview> = {}): MountPreview {
  return {
    previewId: "preview:mount:skill-review-project-a",
    asset: assetFixture("skill:review", "review", "skill"),
    target: { scope: "project", runtimePath: "~/workspace/project-a/.claude/skills/review", projectPath: "~/workspace/project-a" },
    steps: [
      { id: "preview-mount", kind: "mount", label: "预览挂载计划", description: "No write", risk: "medium" },
    ],
    warnings: ["Preview only: no runtime path will be changed."],
    backupRequired: true,
    canApply: true,
    ...overrides,
  };
}

function conflictPreviewFixture(id: string, name: string, assetType: ConflictPreview["assetType"]): ConflictPreview {
  return {
    id,
    assetId: `${assetType}:${name}`,
    assetType,
    name,
    reason: "同名资产预览冲突",
    existingContent: `Existing preview content for ${name}`,
    incomingContent: `Incoming preview content for ${name}`,
    allowedResolutions: ["skip", "rename", "overwrite"],
  };
}

function restorePreviewFixture(backupId: string, overrides: Partial<RestorePreview> = {}): RestorePreview {
  return {
    previewId: `preview:restore:${backupId}`,
    backup: { id: backupId, label: `Restore preview for ${backupId}`, createdAt: "preview-only", sizeBytes: 0, entryCount: 3 },
    affectedPaths: [`backups/${backupId}/manifest.json`, "~/.claude/skills/review"],
    steps: [
      { id: "preview-restore", kind: "restore", label: "预览恢复影响", description: "No write", risk: "high" },
    ],
    warnings: ["Preview only: restore is not executed."],
    backupBeforeRestore: true,
    canApply: true,
    ...overrides,
  };
}
