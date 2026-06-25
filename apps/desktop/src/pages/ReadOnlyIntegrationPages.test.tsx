import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AssetSummary, DesktopSettings, GitStatus, ProjectSummary, ScanResult } from "../app/contracts";
import { CommandsListPage } from "./CommandsListPage";
import { McpServersListPage } from "./McpServersListPage";
import { ProjectsListPage } from "./ProjectsListPage";
import { ScanImportPage } from "./ScanImportPage";
import { SettingsPage } from "./SettingsPage";
import { SkillsListPage } from "./SkillsListPage";
import { SyncPage } from "./SyncPage";

const { listAssets, listProjects, gitStatus, settingsLoad, scanAssets } = vi.hoisted(() => ({
  listAssets: vi.fn(),
  listProjects: vi.fn(),
  gitStatus: vi.fn(),
  settingsLoad: vi.fn(),
  scanAssets: vi.fn(),
}));

vi.mock("../app/data-api", () => ({
  listAssets,
  listProjects,
  gitStatus,
  settingsLoad,
  scanAssets,
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

beforeEach(() => {
  listAssets.mockResolvedValue([]);
  listProjects.mockResolvedValue([]);
  gitStatus.mockResolvedValue(gitStatusFixture());
  settingsLoad.mockResolvedValue(settingsFixture());
  scanAssets.mockResolvedValue(scanResultFixture([]));
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
    expect(screen.getByRole("button", { name: "Pull" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Push" })).toBeDisabled();
  });

  it("displays loaded read-only settings without enabling save behavior", async () => {
    settingsLoad.mockResolvedValue(settingsFixture({
      assetCenterPath: "/tmp/assets",
      scanRoots: ["/tmp/workspace", "/tmp/code"],
      gitDefaultBranch: "trunk",
      gitRemote: "upstream",
      cliPath: "/tmp/maa",
    }));

    render(<SettingsPage />);

    expect(await screen.findByDisplayValue("/tmp/assets")).toBeInTheDocument();
    expect(screen.getByDisplayValue("/tmp/workspace, /tmp/code")).toBeInTheDocument();
    expect(screen.getByDisplayValue("trunk")).toBeInTheDocument();
    expect(screen.getByDisplayValue("upstream")).toBeInTheDocument();
    expect(screen.getByDisplayValue("/tmp/maa")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "保存设置" })).toBeDisabled();
  });

  it("calls scanAssets for the selected scope and keeps import disabled", async () => {
    scanAssets.mockResolvedValue(scanResultFixture([
      assetFixture("skill:live-scan", "live-scan", "skill"),
    ]));

    render(<ScanImportPage />);

    await waitFor(() => expect(scanAssets).toHaveBeenCalledWith({ scope: { kind: "user" } }));
    expect(await screen.findByText("live-scan")).toBeInTheDocument();

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
