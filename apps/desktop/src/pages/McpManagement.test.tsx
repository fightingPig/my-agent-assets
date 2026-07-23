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
} = vi.hoisted(() => ({
  listAssets: vi.fn(),
  canonicalAssetContent: vi.fn(),
  canonicalMcpGet: vi.fn(),
  canonicalMcpSavePreview: vi.fn(),
  canonicalMcpSaveApply: vi.fn(),
  canonicalMountPreview: vi.fn(),
  canonicalMountApply: vi.fn(),
}));

vi.mock("../app/data-api", () => ({
  listAssets,
  canonicalAssetContent,
  canonicalMcpGet,
  canonicalMcpSavePreview,
  canonicalMcpSaveApply,
  canonicalMountPreview,
  canonicalMountApply,
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

  it("shows mount state but keeps target synchronization in Mount Manager", async () => {
    render(<McpServersListPage />);
    await screen.findByRole("option", { name: "filesystem" });
    fireEvent.click(screen.getByRole("button", { name: "编辑配置" }));
    await screen.findByText("out_of_sync");

    expect(screen.getByText("当前挂载")).toBeInTheDocument();
    expect(screen.getByText(/目标启用、同步和解除挂载统一在“挂载管理”中完成/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "生成同步预览" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "确认同步" })).not.toBeInTheDocument();
    expect(canonicalMountPreview).not.toHaveBeenCalled();
    expect(canonicalMountApply).not.toHaveBeenCalled();
  });
});
