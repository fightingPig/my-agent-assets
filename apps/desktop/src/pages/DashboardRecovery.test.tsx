import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DashboardPage } from "./DashboardPage";

const { listAssets, listProjects, gitStatus, recoveryStatus } = vi.hoisted(() => ({
  listAssets: vi.fn(),
  listProjects: vi.fn(),
  gitStatus: vi.fn(),
  recoveryStatus: vi.fn(),
}));

vi.mock("../app/data-api", () => ({
  listAssets,
  listProjects,
  gitStatus,
  recoveryStatus,
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
});
