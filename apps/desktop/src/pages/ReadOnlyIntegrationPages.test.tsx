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
import { CommandsListPage } from "./CommandsListPage";
import { ConflictResolverPage } from "./ConflictResolverPage";
import { McpServersListPage } from "./McpServersListPage";
import { MountManagerPage } from "./MountManagerPage";
import { ProjectsListPage } from "./ProjectsListPage";
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
  importApply,
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
  importApply: vi.fn(),
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
  importApply,
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
  importApply.mockResolvedValue({
    mode: "planOnly",
    ok: true,
    previewId: "import-plan:user:skill:live-scan",
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
  previewMount.mockResolvedValue(mountPreviewFixture());
  previewConflicts.mockResolvedValue([]);
  previewRestore.mockResolvedValue(restorePreviewFixture("backup-20260621-1842"));
  mountApply.mockResolvedValue({
    mode: "planOnly",
    ok: true,
    previewId: "mount-plan:skill:review:project-a",
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
    previewId: "restore-plan:backup-20260621-1842",
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
    expect(screen.getByRole("button", { name: "Pull" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Push" })).toBeDisabled();
  });

  it("generates a preview-only Sync plan without enabling Pull or Push", async () => {
    render(<SyncPage />);

    await waitFor(() => expect(gitStatus).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "预览 Push" }));

    await waitFor(() => expect(previewSync).toHaveBeenCalledWith({ direction: "push" }));
    expect(await screen.findByText(/生成 Push 计划/)).toBeInTheDocument();
    expect(screen.getByText("计划方向")).toBeInTheDocument();
    expect(screen.getByText("计划可执行")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Pull" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Push" })).toBeDisabled();
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
      previewId: "import-plan:user:skill:live-scan",
      mode: "planOnly",
      scope: { kind: "user" },
      assetIds: ["skill:live-scan"],
      conflictResolutions: [],
      backupBeforeApply: true,
    }));
    expect(await screen.findByText(/Plan-only mode: 1 asset would be imported/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "确认导入" })).toBeDisabled();
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
    expect(screen.getByRole("button", { name: "覆盖" })).toBeDisabled();

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
      previewId: "restore-plan:backup-20260621-1842",
      mode: "planOnly",
      backupId: "backup-20260621-1842",
      backupBeforeRestore: true,
    }));
    expect(await screen.findByText(/Plan-only mode/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "恢复此备份" })).toBeDisabled();
  });

  it("generates a plan-only mount apply preview without enabling mount execution", async () => {
    render(<MountManagerPage />);

    await waitFor(() => expect(previewMount).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "生成挂载计划" }));

    await waitFor(() => expect(mountApply).toHaveBeenCalledWith({
      previewId: "mount-plan:skill:review:project-a",
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

  it("updates Conflict Resolver local resolution preview without calling apply wrappers", async () => {
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

    expect(screen.getByRole("button", { name: "跳过" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "重命名" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "覆盖" })).toBeDisabled();
    expect(importApply).not.toHaveBeenCalled();
    expect(mountApply).not.toHaveBeenCalled();
    expect(restoreApply).not.toHaveBeenCalled();
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
