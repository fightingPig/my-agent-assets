/** Portable contracts for the reusable UI layer.
 *
 * Product adapters may pass richer DTOs as long as they satisfy these shapes.
 * Keeping them independent from Tauri DTOs lets another app reuse the visual
 * system without copying this project's data boundary.
 */
export type UiAssetType = "skill" | "command" | "mcp";
export type UiRuntimeProvider = "claude_code" | "codex" | "custom";
export type SupportedMountProvider = Extract<UiRuntimeProvider, "claude_code" | "codex">;

export type UiMountTarget = {
  id: string;
  provider: UiRuntimeProvider;
  accepts: UiAssetType[];
  scope: "user" | "local" | "project" | "custom";
  path: string;
  projectPath?: string;
  status: "ready" | "blocked" | "invalid";
};

export type UiMountBinding = {
  assetId: string;
  targetId: string;
  status: "mounted" | "out_of_sync" | "orphaned";
};
