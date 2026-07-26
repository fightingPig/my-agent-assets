import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { McpServersListPage } from "./McpServersListPage";

const {
  listAssets,
  canonicalAssetContent,
  canonicalMcpGet,
  canonicalMcpSavePreview,
  canonicalMcpSaveApply,
  canonicalMountPreview,
  canonicalMountApply,
  canonicalDeletePreview,
  canonicalDeleteApply,
} = vi.hoisted(() => ({
  listAssets: vi.fn(),
  canonicalAssetContent: vi.fn(),
  canonicalMcpGet: vi.fn(),
  canonicalMcpSavePreview: vi.fn(),
  canonicalMcpSaveApply: vi.fn(),
  canonicalMountPreview: vi.fn(),
  canonicalMountApply: vi.fn(),
  canonicalDeletePreview: vi.fn(),
  canonicalDeleteApply: vi.fn(),
}));

vi.mock("../app/data-api", () => ({
  listAssets,
  canonicalAssetContent,
  canonicalMcpGet,
  canonicalMcpSavePreview,
  canonicalMcpSaveApply,
  canonicalMountPreview,
  canonicalMountApply,
  canonicalDeletePreview,
  canonicalDeleteApply,
}));

describe("MCP canonical management", () => {
  afterEach(cleanup);

  beforeEach(() => {
    vi.clearAllMocks();
    listAssets.mockResolvedValue([{
      id: "mcp:filesystem",
      name: "filesystem",
      title: "Filesystem",
      assetType: "mcp",
      status: "mounted",
      category: "资产中心",
      description: "Local files",
      sourcePath: "/tmp/home/.my-agent-assets/assets/mcps/filesystem.json",
      scope: "local",
      updatedAt: "2026-07-01T10:00:00Z",
      mountTargets: ["/tmp/home/.claude.json"],
    }]);
    canonicalAssetContent.mockResolvedValue({
      assetId: "mcp:filesystem",
      assetType: "mcp",
      canonicalPath: "/tmp/filesystem.json",
      contentPath: "/tmp/filesystem.json",
      content: "{\"schemaVersion\":1}",
      truncated: false,
    });
    canonicalMcpGet.mockResolvedValue({
      assetId: "mcp:filesystem",
      title: "Filesystem",
      description: "Local files",
      canonical: {
        schemaVersion: 1,
        name: "filesystem",
        spec: { type: "stdio", command: "npx", args: ["server-files"] },
        providerExtensions: {},
      },
      bindings: [{
        targetId: "claude-user-mcp",
        status: "out_of_sync",
      }],
    });
    canonicalMcpSavePreview.mockResolvedValue({
      previewId: "mcp-save-1",
      operation: "edit",
      assetId: "mcp:filesystem",
      canonicalPath: "/tmp/home/.my-agent-assets/assets/mcps/filesystem.json",
      registryPath: "/tmp/home/.my-agent-assets/assets.yaml",
      outOfSyncTargetIds: ["claude-user-mcp"],
      targetCompatibility: [{
        targetId: "claude-user-mcp",
        compatible: true,
        warnings: [],
      }],
      plannedEffects: ["write canonical MCP definition"],
      warnings: [],
      canApply: true,
      generatedAtEpochSeconds: 100,
      expiresAtEpochSeconds: 400,
    });
    canonicalMcpSaveApply.mockResolvedValue({
      previewId: "mcp-save-1",
      operation: "edit",
      assetId: "mcp:filesystem",
      canonicalPath: "/tmp/home/.my-agent-assets/assets/mcps/filesystem.json",
      outOfSyncTargetIds: ["claude-user-mcp"],
      affectedPaths: [],
    });
    canonicalMountPreview.mockResolvedValue({
      previewId: "mount-1",
      assetId: "mcp:filesystem",
      targetId: "claude-user-mcp",
      canonicalPath: "/tmp/canonical.json",
      affectedTargetPath: "/tmp/home/.claude.json",
      compatible: true,
      adapter: "json_mcp_patch",
      disposition: "compile_mcp",
      plannedEffects: ["patch only selected MCP entry"],
      warnings: [],
      backupRequired: true,
      canApply: true,
      generatedAtEpochSeconds: 200,
      expiresAtEpochSeconds: 500,
    });
    canonicalMountApply.mockResolvedValue({
      previewId: "mount-1",
      assetId: "mcp:filesystem",
      targetId: "claude-user-mcp",
      mounted: true,
      affectedPaths: ["/tmp/home/.claude.json"],
      warnings: [],
    });
    canonicalDeletePreview.mockImplementation(async (input: { removeMcpTargetEntries: boolean }) => ({
      previewId: "delete-mcp-1",
      assetId: "mcp:filesystem",
      canonicalPath: "/tmp/home/.my-agent-assets/assets/mcps/filesystem.json",
      removeMcpTargetEntries: input.removeMcpTargetEntries,
      bindings: [{
        targetId: "claude-user-mcp",
        targetPath: "/tmp/home/.claude.json",
        canUnmount: input.removeMcpTargetEntries,
        willRemoveTargetEntry: input.removeMcpTargetEntries,
        warnings: [],
      }],
      plannedEffects: input.removeMcpTargetEntries
        ? ["unmount claude-user-mcp"]
        : ["preserve target live config"],
      warnings: [],
      backupRequired: true,
      canApply: true,
      generatedAtEpochSeconds: 100,
      expiresAtEpochSeconds: 400,
    }));
    canonicalDeleteApply.mockResolvedValue({
      previewId: "delete-mcp-1",
      assetId: "mcp:filesystem",
      deleted: true,
      portableBackupId: "portable-1",
      localBackupId: "local-1",
      affectedPaths: [],
      journalPath: "/tmp/delete-journal",
    });
  });

  it("requires preview then ordinary confirmation to edit canonical data", async () => {
    render(<McpServersListPage />);
    await screen.findByRole("option", { name: "filesystem" });
    fireEvent.click(screen.getByRole("button", { name: "编辑配置" }));
    expect(await screen.findByRole("heading", { name: "mcp:filesystem" })).toBeInTheDocument();
    expect(screen.getByDisplayValue("filesystem")).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "生成保存预览" }));
    await waitFor(() => expect(canonicalMcpSavePreview).toHaveBeenCalled());
    expect(canonicalMcpSaveApply).not.toHaveBeenCalled();

    fireEvent.click(await screen.findByRole("button", { name: "确认保存" }));
    await waitFor(() => expect(canonicalMcpSaveApply).toHaveBeenCalledWith({
      previewId: "mcp-save-1",
      previewGeneratedAtEpochSeconds: 100,
      request: expect.objectContaining({
        assetId: "mcp:filesystem",
        canonical: expect.objectContaining({ name: "filesystem" }),
      }),
    }));
  });

  it("uses explicit mount preview/apply to sync an out-of-sync target", async () => {
    render(<McpServersListPage />);
    await screen.findByRole("option", { name: "filesystem" });
    fireEvent.click(screen.getByRole("button", { name: "编辑配置" }));
    await screen.findByText("out_of_sync");

    fireEvent.click(screen.getByRole("button", { name: "生成同步预览" }));
    await waitFor(() => expect(canonicalMountPreview).toHaveBeenCalledWith({
      assetId: "mcp:filesystem",
      targetId: "claude-user-mcp",
    }));
    expect(canonicalMountApply).not.toHaveBeenCalled();

    fireEvent.click(await screen.findByRole("button", { name: "确认同步" }));
    await waitFor(() => expect(canonicalMountApply).toHaveBeenCalledWith({
      previewId: "mount-1",
      previewGeneratedAtEpochSeconds: 200,
      request: {
        assetId: "mcp:filesystem",
        targetId: "claude-user-mcp",
      },
    }));
  });

  it("preserves MCP target live config by default and requires an explicit opt-in to remove it", async () => {
    render(<McpServersListPage />);
    await screen.findByRole("option", { name: "filesystem" });

    fireEvent.click(screen.getByRole("button", { name: "删除 MCP" }));
    await waitFor(() => expect(canonicalDeletePreview).toHaveBeenLastCalledWith({
      assetId: "mcp:filesystem",
      mode: "require_unmounted",
      removeMcpTargetEntries: false,
    }));
    expect(screen.getByText(/Target live config 将被保留/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("checkbox", { name: /同时从已启用 Target 配置中移除/ }));
    await waitFor(() => expect(canonicalDeletePreview).toHaveBeenLastCalledWith({
      assetId: "mcp:filesystem",
      mode: "unmount_all",
      removeMcpTargetEntries: true,
    }));
    expect(screen.getByText(/将删除 canonical MCP 和列出的 Target live config entry/)).toBeInTheDocument();
  });
});
