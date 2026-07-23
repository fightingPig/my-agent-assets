import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TargetRegistryPanel } from "./TargetRegistryPanel";

const {
  listMountTargets,
  targetRegistrationPreview,
  targetRegistrationApply,
  targetRemovalPreview,
  targetRemovalApply,
  openDialog,
} = vi.hoisted(() => ({
  listMountTargets: vi.fn(),
  targetRegistrationPreview: vi.fn(),
  targetRegistrationApply: vi.fn(),
  targetRemovalPreview: vi.fn(),
  targetRemovalApply: vi.fn(),
  openDialog: vi.fn(),
}));

vi.mock("../../app/data-api", () => ({
  listMountTargets,
  targetRegistrationPreview,
  targetRegistrationApply,
  targetRemovalPreview,
  targetRemovalApply,
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openDialog }));

describe("TargetRegistryPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listMountTargets.mockResolvedValue([]);
    targetRegistrationPreview.mockResolvedValue({
      previewId: "target-add-1",
      operation: "add",
      target: {
        id: "custom-skill-directory-skills",
        kind: "custom_skill_directory",
        provider: "custom",
        accepts: ["skill"],
        adapter: "symlink_directory",
        scope: "custom",
        path: "/tmp/custom/skills",
        providerState: "initialized",
        status: "ready",
      },
      affectedPaths: ["/tmp/targets.yaml", "/tmp/custom/skills"],
      blockingBindings: [],
      warnings: [],
      canApply: true,
      generatedAtEpochSeconds: 100,
      expiresAtEpochSeconds: 400,
    });
    targetRegistrationApply.mockResolvedValue({
      previewId: "target-add-1",
      operation: "add",
      targetId: "custom-skill-directory-skills",
      registryPath: "/tmp/targets.yaml",
      backupPath: "/tmp/backups/targets.yaml",
    });
  });

  it("previews and confirms registration without deriving runtime paths in React", async () => {
    openDialog.mockResolvedValue("/tmp/custom/skills");
    render(<TargetRegistryPanel />);
    fireEvent.click(screen.getByRole("button", { name: "选择路径" }));
    await waitFor(() => expect(openDialog).toHaveBeenCalledWith(expect.objectContaining({ directory: true })));
    fireEvent.click(screen.getByRole("button", { name: "预览注册" }));

    await waitFor(() => expect(targetRegistrationPreview).toHaveBeenCalledWith({
      id: "custom-skill-directory-skills",
      kind: "custom_skill_directory",
      location: "/tmp/custom/skills",
    }));
    expect(targetRegistrationPreview.mock.calls[0][0]).not.toHaveProperty("runtimePath");

    fireEvent.click(await screen.findByRole("button", { name: "确认注册目标" }));
    await waitFor(() => expect(targetRegistrationApply).toHaveBeenCalledWith({
      previewId: "target-add-1",
      previewGeneratedAtEpochSeconds: 100,
      request: {
        id: "custom-skill-directory-skills",
        kind: "custom_skill_directory",
        location: "/tmp/custom/skills",
      },
    }));
  });

  it("hides standard targets and previews custom target removal", async () => {
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
    expect(screen.queryByRole("button", { name: "移除目标 claude-user-skills" })).not.toBeInTheDocument();
    fireEvent.click(await screen.findByRole("button", { name: "移除目标 custom-skills" }));
    await waitFor(() => expect(targetRemovalPreview).toHaveBeenCalledWith({
      targetId: "custom-skills",
    }));
    fireEvent.click(await screen.findByRole("button", { name: "确认移除目标" }));
    await waitFor(() => expect(targetRemovalApply).toHaveBeenCalled());
  });
});
