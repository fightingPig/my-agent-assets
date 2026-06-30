import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TargetRegistryPanel } from "./TargetRegistryPanel";

const {
  listMountTargets,
  targetRegistrationPreview,
  targetRegistrationApply,
  targetRemovalPreview,
  targetRemovalApply,
} = vi.hoisted(() => ({
  listMountTargets: vi.fn(),
  targetRegistrationPreview: vi.fn(),
  targetRegistrationApply: vi.fn(),
  targetRemovalPreview: vi.fn(),
  targetRemovalApply: vi.fn(),
}));

vi.mock("../../app/data-api", () => ({
  listMountTargets,
  targetRegistrationPreview,
  targetRegistrationApply,
  targetRemovalPreview,
  targetRemovalApply,
}));

describe("TargetRegistryPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listMountTargets.mockResolvedValue([]);
    targetRegistrationPreview.mockResolvedValue({
      previewId: "target-add-1",
      operation: "add",
      target: {
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
      affectedPaths: ["/tmp/targets.yaml", "/tmp/project-a/.claude/skills"],
      blockingBindings: [],
      warnings: [],
      canApply: true,
      generatedAtEpochSeconds: 100,
      expiresAtEpochSeconds: 400,
    });
    targetRegistrationApply.mockResolvedValue({
      previewId: "target-add-1",
      operation: "add",
      targetId: "project-a-skills",
      registryPath: "/tmp/targets.yaml",
      backupPath: "/tmp/backups/targets.yaml",
    });
  });

  it("previews and confirms registration without deriving runtime paths in React", async () => {
    render(<TargetRegistryPanel />);
    fireEvent.change(screen.getByLabelText("目标 ID"), {
      target: { value: "project-a-skills" },
    });
    fireEvent.change(screen.getByLabelText("项目根目录"), {
      target: { value: "/tmp/project-a" },
    });
    fireEvent.click(screen.getByRole("button", { name: "预览注册" }));

    await waitFor(() => expect(targetRegistrationPreview).toHaveBeenCalledWith({
      id: "project-a-skills",
      kind: "claude_project_skills",
      location: "/tmp/project-a",
    }));
    expect(targetRegistrationPreview.mock.calls[0][0]).not.toHaveProperty("runtimePath");

    fireEvent.click(await screen.findByRole("button", { name: "确认注册目标" }));
    await waitFor(() => expect(targetRegistrationApply).toHaveBeenCalledWith({
      previewId: "target-add-1",
      previewGeneratedAtEpochSeconds: 100,
      request: {
        id: "project-a-skills",
        kind: "claude_project_skills",
        location: "/tmp/project-a",
      },
    }));
  });

  it("keeps built-in user targets non-removable and previews custom removal", async () => {
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
        id: "custom-skills",
        kind: "custom_skill_directory",
        provider: "custom",
        accepts: ["skill"],
        adapter: "symlink_directory",
        scope: "custom",
        path: "/tmp/custom/skills",
        providerState: "initialized",
        status: "ready",
      },
    ]);
    targetRemovalPreview.mockResolvedValue({
      previewId: "target-remove-1",
      operation: "remove",
      target: {
        id: "custom-skills",
        kind: "custom_skill_directory",
        provider: "custom",
        accepts: ["skill"],
        adapter: "symlink_directory",
        scope: "custom",
        path: "/tmp/custom/skills",
        providerState: "initialized",
        status: "ready",
      },
      affectedPaths: ["/tmp/targets.yaml"],
      blockingBindings: [],
      warnings: [],
      canApply: true,
      generatedAtEpochSeconds: 101,
      expiresAtEpochSeconds: 401,
    });
    targetRemovalApply.mockResolvedValue({
      previewId: "target-remove-1",
      operation: "remove",
      targetId: "custom-skills",
      registryPath: "/tmp/targets.yaml",
      backupPath: "/tmp/backups/targets.yaml",
    });

    render(<TargetRegistryPanel />);
    expect(await screen.findByRole("button", { name: "移除目标 claude-user-skills" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "移除目标 custom-skills" }));
    await waitFor(() => expect(targetRemovalPreview).toHaveBeenCalledWith({
      targetId: "custom-skills",
    }));
    fireEvent.click(await screen.findByRole("button", { name: "确认移除目标" }));
    await waitFor(() => expect(targetRemovalApply).toHaveBeenCalled());
  });
});
