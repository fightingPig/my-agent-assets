import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DashboardPage } from "./DashboardPage";

const {
  listAssets,
  listProjects,
  gitStatus,
  recoveryStatus,
  doctorReport,
  consistencyRepairPreview,
  consistencyRepairApply,
  initializationPreview,
  initializationApply,
} = vi.hoisted(() => ({
  listAssets: vi.fn(),
  listProjects: vi.fn(),
  gitStatus: vi.fn(),
  recoveryStatus: vi.fn(),
  doctorReport: vi.fn(),
  consistencyRepairPreview: vi.fn(),
  consistencyRepairApply: vi.fn(),
  initializationPreview: vi.fn(),
  initializationApply: vi.fn(),
}));

vi.mock("../app/data-api", () => ({
  listAssets,
  listProjects,
  gitStatus,
  recoveryStatus,
  doctorReport,
  consistencyRepairPreview,
  consistencyRepairApply,
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
    doctorReport.mockResolvedValue({
      assetCenterPath: "/tmp/home/.my-agent-assets",
      initialized: true,
      checks: [],
      contentDiagnostics: [],
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

  it("only applies a registry repair after an explicit high-risk preview", async () => {
    recoveryStatus.mockResolvedValue({
      writesBlocked: false,
      journals: [],
      recentRecoveries: [],
      message: "没有未完成事务。",
    });
    doctorReport.mockResolvedValue({
      assetCenterPath: "/tmp/home/.my-agent-assets",
      initialized: true,
      checks: [],
      contentDiagnostics: [{
        assetId: "skill:orphan",
        assetType: "skill",
        name: "orphan",
        path: "/tmp/home/.my-agent-assets/assets/skills/orphan",
        state: "unregistered",
        message: "canonical content exists without an assets.yaml record",
      }],
    });
    consistencyRepairPreview.mockResolvedValue({
      previewId: "repair-1",
      request: { assetId: "skill:orphan", action: "register_unregistered_content" },
      diagnostic: { assetId: "skill:orphan", state: "unregistered" },
      plannedEffects: ["register content"],
      warnings: ["high-risk"],
      canApply: true,
      generatedAtEpochSeconds: 100,
      expiresAtEpochSeconds: 700,
    });
    consistencyRepairApply.mockResolvedValue({
      previewId: "repair-1",
      assetId: "skill:orphan",
      action: "register_unregistered_content",
      affectedPaths: ["/tmp/home/.my-agent-assets/assets.yaml"],
      journalPath: "/tmp/home/.my-agent-assets/operations/repair.yaml",
    });

    render(<DashboardPage appInfo={{
      name: "My Agent Assets",
      version: "0.1.0",
      platform: "macos",
      arch: "aarch64",
      backendReady: true,
    }} />);

    expect(await screen.findByText("检测到资产索引与 canonical 内容不一致")).toBeInTheDocument();
    fireEvent.click(screen.getAllByRole("button", { name: "预览重新登记" }).at(-1)!);
    await waitFor(() => expect(consistencyRepairPreview).toHaveBeenCalledWith({
      assetId: "skill:orphan",
      action: "register_unregistered_content",
    }));
    expect(consistencyRepairApply).not.toHaveBeenCalled();
    fireEvent.click((await screen.findAllByRole("button", { name: "确认修复" })).at(-1)!);
    await waitFor(() => expect(consistencyRepairApply).toHaveBeenCalledWith({
      previewId: "repair-1",
      previewGeneratedAtEpochSeconds: 100,
      request: { assetId: "skill:orphan", action: "register_unregistered_content" },
    }));
  });
});
