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
  const assets = await invokeOrFallback<unknown>("list_assets", { input }, []);
  return Array.isArray(assets) ? assets as AssetSummary[] : [];
}

export async function listProjects(): Promise<ProjectSummary[]> {
  const projects = await invokeOrFallback<unknown>("list_projects", undefined, []);
  return Array.isArray(projects) ? projects as ProjectSummary[] : [];
}

export async function gitStatus(): Promise<GitStatus> {
  const status = await invokeOrFallback<unknown>("git_status", undefined, fallbackGitStatus);
  return isRecord(status) && typeof status.repositoryPath === "string" ? status as GitStatus : fallbackGitStatus;
}

export async function settingsLoad(): Promise<DesktopSettings> {
  const settings = await invokeOrFallback<unknown>("settings_load", undefined, fallbackSettings);
  return isRecord(settings) && typeof settings.assetCenterPath === "string" ? settings as DesktopSettings : fallbackSettings;
}

export async function scanAssets(input: ScanAssetsInput): Promise<ScanResult> {
  const fallback = {
    scope: input.scope,
    scannedAt: new Date(0).toISOString(),
    assets: [],
    counts: { total: 0, skills: 0, commands: 0, mcps: 0 },
    conflictCount: 0,
    warnings: ["Tauri runtime is unavailable; scan skipped."],
  };
  const result = await invokeOrFallback<unknown>("scan_assets", { input }, fallback);
  return isRecord(result) && Array.isArray(result.assets) && isRecord(result.counts) ? result as ScanResult : fallback;
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
