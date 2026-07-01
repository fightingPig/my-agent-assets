import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DashboardPage } from "./DashboardPage";

const {
  listAssets,
  listProjects,
  gitStatus,
  recoveryStatus,
  initializationPreview,
  initializationApply,
} = vi.hoisted(() => ({
  listAssets: vi.fn(),
  listProjects: vi.fn(),
  gitStatus: vi.fn(),
  recoveryStatus: vi.fn(),
  initializationPreview: vi.fn(),
  initializationApply: vi.fn(),
}));

vi.mock("../app/data-api", () => ({
  listAssets,
  listProjects,
  gitStatus,
  recoveryStatus,
  initializationPreview,
  initializationApply,
}));

describe("Dashboard recovery status", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listAssets.mockResolvedValue([]);
    listProjects.mockResolvedValue([]);
    gitStatus.mockResolvedValue({
      repositoryPath: "/tmp/home/.my-agent-assets",
      isRepository: true,
      statusMessage: "Repository ready.",
      branch: "main",
      remoteName: "origin",
      clean: true,
      ahead: 0,
      behind: 0,
      changedFiles: [],
      conflicts: [],
      syncableChanges: [],
      blockedChanges: [],
      lastSyncedAt: null,
    });
    initializationPreview.mockResolvedValue({
      previewId: "init-ready",
      assetCenterPath: "/tmp/home/.my-agent-assets",
      plannedPaths: [],
      warnings: [],
      alreadyInitialized: true,
      canApply: true,
      generatedAtEpochSeconds: 100,
      expiresAtEpochSeconds: 700,
    });
  });

  it("shows incomplete journals as a write-blocking system state", async () => {
    recoveryStatus.mockResolvedValue({
      writesBlocked: true,
      journals: [{
        schemaVersion: 1,
        operationId: "mount-interrupted-1",
        operationKind: "mount",
        status: "rollback_required",
        createdAtEpochSeconds: 100,
        completedSteps: ["backup_created"],
        recoveryMessage: "runtime rollback incomplete",
      }],
      recentRecoveries: [],
      message: "检测到 1 个未完成事务；新的写操作已阻止，等待安全恢复。",
    });

    render(<DashboardPage appInfo={{
      name: "My Agent Assets",
      version: "0.1.0",
      platform: "macos",
      arch: "aarch64",
      backendReady: true,
    }} />);

    await waitFor(() => expect(recoveryStatus).toHaveBeenCalledTimes(1));
    expect(screen.getByText("事务恢复")).toBeInTheDocument();
    expect(screen.getByText("写入已阻止")).toBeInTheDocument();
    expect(screen.getByText(/检测到 1 个未完成事务/)).toBeInTheDocument();
  });

  it("requires preview and explicit confirmation before initialization apply", async () => {
    recoveryStatus.mockResolvedValue({
      writesBlocked: false,
      journals: [],
      recentRecoveries: [],
      message: "没有未完成事务。",
    });
    const preview = {
      previewId: "init-abc",
      assetCenterPath: "/tmp/home/.my-agent-assets",
      plannedPaths: ["/tmp/home/.my-agent-assets", "/tmp/home/.my-agent-assets/assets"],
      warnings: [],
      alreadyInitialized: false,
      canApply: true,
      generatedAtEpochSeconds: 100,
      expiresAtEpochSeconds: 700,
    };
    initializationPreview.mockResolvedValue(preview);
    initializationApply.mockResolvedValue({
      previewId: preview.previewId,
      assetCenterPath: preview.assetCenterPath,
      created: true,
      createdPaths: preview.plannedPaths,
    });

    render(<DashboardPage appInfo={{
      name: "My Agent Assets",
      version: "0.1.0",
      platform: "macos",
      arch: "aarch64",
      backendReady: true,
    }} />);

    expect(await screen.findByText("资产中心尚未初始化")).toBeInTheDocument();
    expect(initializationApply).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "预览初始化" }));
    expect(await screen.findByText(/将创建 2 个目录或文件/)).toBeInTheDocument();
    expect(initializationApply).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "确认初始化" }));
    await waitFor(() => expect(initializationApply).toHaveBeenCalledWith({
      previewId: "init-abc",
      previewGeneratedAtEpochSeconds: 100,
    }));
  });
});
