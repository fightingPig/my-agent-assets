import { invoke } from "@tauri-apps/api/core";
import { isTauriRuntime } from "../lib/platform";
import type {
  AssetSummary,
  DesktopSettings,
  GitStatus,
  ListAssetsInput,
  ProjectSummary,
  ScanAssetsInput,
  ScanResult,
} from "./contracts";

const fallbackSettings: DesktopSettings = {
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
};

const fallbackGitStatus: GitStatus = {
  repositoryPath: "~/.my-agent-assets",
  isRepository: false,
  statusMessage: "Tauri runtime is unavailable.",
  branch: "",
  remote: null,
  clean: true,
  ahead: 0,
  behind: 0,
  changedFiles: [],
  conflicts: [],
  lastSyncedAt: null,
};

export async function listAssets(input: ListAssetsInput = { assetType: null }): Promise<AssetSummary[]> {
  return invokeOrFallback("list_assets", { input }, []);
}

export async function listProjects(): Promise<ProjectSummary[]> {
  return invokeOrFallback("list_projects", undefined, []);
}

export async function gitStatus(): Promise<GitStatus> {
  return invokeOrFallback("git_status", undefined, fallbackGitStatus);
}

export async function settingsLoad(): Promise<DesktopSettings> {
  return invokeOrFallback("settings_load", undefined, fallbackSettings);
}

export async function scanAssets(input: ScanAssetsInput): Promise<ScanResult> {
  return invokeOrFallback("scan_assets", { input }, {
    scope: input.scope,
    scannedAt: new Date(0).toISOString(),
    assets: [],
    counts: { total: 0, skills: 0, commands: 0, mcps: 0 },
    conflictCount: 0,
    warnings: ["Tauri runtime is unavailable; scan skipped."],
  });
}

async function invokeOrFallback<T>(
  command: string,
  args: Record<string, unknown> | undefined,
  fallback: T,
): Promise<T> {
  if (!isTauriRuntime()) return fallback;

  try {
    return args === undefined ? await invoke<T>(command) : await invoke<T>(command, args);
  } catch {
    return fallback;
  }
}
