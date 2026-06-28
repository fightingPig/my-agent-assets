export const ASSET_PROVIDERS = ["claude", "codex"] as const;

export type AssetProvider = (typeof ASSET_PROVIDERS)[number];

export const providerLabels: Record<AssetProvider, string> = {
  claude: "Claude Code",
  codex: "Codex",
};
