import { invoke } from "@tauri-apps/api/core";
import { isTauriRuntime } from "../lib/platform";
import type {
  AssetSummary,
  BackupSummary,
  ConflictPreview,
  ConflictApplyInput,
  DesktopSettings,
  GitStatus,
  ApplyResult,
  ImportApplyInput,
  ImportPreview,
  ListAssetsInput,
  MountApplyInput,
  MountPreview,
  PreviewConflictsInput,
  PreviewImportInput,
  PreviewMountInput,
  PreviewRestoreInput,
  PreviewSyncInput,
  ProjectSummary,
  RestoreApplyInput,
  RestorePreview,
  ScanAssetsInput,
  ScanResult,
  SettingsSaveInput,
  SyncApplyInput,
  SyncPreview,
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

export async function listBackups(): Promise<BackupSummary[]> {
  const backups = await invokeOrFallback<unknown>("list_backups", undefined, []);
  return Array.isArray(backups) ? backups as BackupSummary[] : [];
}

export async function gitStatus(): Promise<GitStatus> {
  const status = await invokeOrFallback<unknown>("git_status", undefined, fallbackGitStatus);
  return isRecord(status) && typeof status.repositoryPath === "string" ? status as GitStatus : fallbackGitStatus;
}

export async function settingsLoad(): Promise<DesktopSettings> {
  const settings = await invokeOrFallback<unknown>("settings_load", undefined, fallbackSettings);
  return isRecord(settings) && typeof settings.assetCenterPath === "string" ? settings as DesktopSettings : fallbackSettings;
}

export async function settingsSave(input: SettingsSaveInput): Promise<DesktopSettings> {
  if (!isTauriRuntime()) return input.settings;
  const settings = await invoke<unknown>("settings_save", { input });
  if (!isRecord(settings) || typeof settings.assetCenterPath !== "string") {
    throw new Error("settings_save returned an invalid response.");
  }
  return settings as DesktopSettings;
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

export async function previewImport(input: PreviewImportInput): Promise<ImportPreview> {
  const fallback: ImportPreview = {
    previewId: "preview:import:unavailable",
    scope: input.scope,
    assets: [],
    conflicts: [],
    steps: [],
    warnings: ["Tauri runtime is unavailable; import preview skipped."],
    canApply: false,
  };
  const result = await invokeOrFallback<unknown>("preview_import", { input }, fallback);
  return isRecord(result) && typeof result.previewId === "string" && Array.isArray(result.steps) && Array.isArray(result.assets)
    ? result as ImportPreview
    : fallback;
}

export async function previewMount(input: PreviewMountInput): Promise<MountPreview | null> {
  const result = await invokeOrFallback<unknown>("preview_mount", { input }, null);
  return isRecord(result) && typeof result.previewId === "string" && isRecord(result.asset) && isRecord(result.target) && Array.isArray(result.steps)
    ? result as MountPreview
    : null;
}

export async function previewConflicts(input: PreviewConflictsInput): Promise<ConflictPreview[]> {
  const result = await invokeOrFallback<unknown>("preview_conflicts", { input }, []);
  return Array.isArray(result) ? result as ConflictPreview[] : [];
}

export async function previewRestore(input: PreviewRestoreInput): Promise<RestorePreview> {
  const fallbackBackup: BackupSummary = {
    id: input.backupId,
    label: `Restore preview for ${input.backupId}`,
    createdAt: "",
    sizeBytes: 0,
    entryCount: 0,
  };
  const fallback: RestorePreview = {
    previewId: `preview:restore:${input.backupId}`,
    backup: fallbackBackup,
    affectedPaths: [],
    steps: [],
    warnings: ["Tauri runtime is unavailable; restore preview skipped."],
    backupBeforeRestore: true,
    canApply: false,
  };
  const result = await invokeOrFallback<unknown>("preview_restore", { input }, fallback);
  return isRecord(result) && typeof result.previewId === "string" && isRecord(result.backup) && Array.isArray(result.affectedPaths)
    ? result as RestorePreview
    : fallback;
}

export async function previewSync(input: PreviewSyncInput): Promise<SyncPreview> {
  const fallback: SyncPreview = {
    previewId: `preview:sync:${input.direction}`,
    direction: input.direction,
    repositoryPath: fallbackGitStatus.repositoryPath,
    branch: fallbackGitStatus.branch,
    remote: fallbackGitStatus.remote,
    steps: [],
    warnings: ["Tauri runtime is unavailable; sync preview skipped."],
    canApply: false,
  };
  const result = await invokeOrFallback<unknown>("preview_sync", { input }, fallback);
  return isRecord(result) && typeof result.previewId === "string" && Array.isArray(result.steps) && Array.isArray(result.warnings)
    ? result as SyncPreview
    : fallback;
}

export async function syncApply(input: SyncApplyInput): Promise<ApplyResult> {
  const fallback: ApplyResult = {
    mode: input.mode,
    ok: false,
    previewId: input.previewId,
    backup: null,
    steps: [],
    warnings: ["Tauri runtime is unavailable; sync apply skipped."],
    errors: ["sync_apply could not run outside the Tauri runtime."],
  };
  const result = await invokeOrFallback<unknown>("sync_apply", { input }, fallback);
  return isRecord(result) && Array.isArray(result.steps) && Array.isArray(result.errors)
    ? result as ApplyResult
    : fallback;
}

export async function importApply(input: ImportApplyInput): Promise<ApplyResult> {
  const fallback: ApplyResult = {
    mode: input.mode,
    ok: false,
    previewId: input.previewId,
    backup: null,
    steps: [],
    warnings: ["Tauri runtime is unavailable; import apply skipped."],
    errors: ["import_apply could not run outside the Tauri runtime."],
  };
  const result = await invokeOrFallback<unknown>("import_apply", { input }, fallback);
  return isRecord(result) && Array.isArray(result.steps) && Array.isArray(result.errors)
    ? result as ApplyResult
    : fallback;
}

export async function conflictApply(input: ConflictApplyInput): Promise<ApplyResult> {
  const fallback: ApplyResult = {
    mode: input.mode,
    ok: false,
    previewId: input.previewId,
    backup: null,
    steps: [],
    warnings: [],
    errors: ["conflict_apply could not run outside the Tauri runtime."],
  };
  const result = await invokeOrFallback<unknown>("conflict_apply", { input }, fallback);
  return isRecord(result) && Array.isArray(result.steps) && Array.isArray(result.errors)
    ? result as ApplyResult
    : fallback;
}

export async function mountApply(input: MountApplyInput): Promise<ApplyResult> {
  const fallback: ApplyResult = {
    mode: input.mode,
    ok: false,
    previewId: input.previewId,
    backup: null,
    steps: [],
    warnings: ["Tauri runtime is unavailable; mount apply skipped."],
    errors: ["mount_apply could not run outside the Tauri runtime."],
  };
  const result = await invokeOrFallback<unknown>("mount_apply", { input }, fallback);
  return isRecord(result) && Array.isArray(result.steps) && Array.isArray(result.errors)
    ? result as ApplyResult
    : fallback;
}

export async function restoreApply(input: RestoreApplyInput): Promise<ApplyResult> {
  const fallback: ApplyResult = {
    mode: input.mode,
    ok: false,
    previewId: input.previewId,
    backup: null,
    steps: [],
    warnings: ["Tauri runtime is unavailable; restore apply skipped."],
    errors: ["restore_apply could not run outside the Tauri runtime."],
  };
  const result = await invokeOrFallback<unknown>("restore_apply", { input }, fallback);
  return isRecord(result) && Array.isArray(result.steps) && Array.isArray(result.errors)
    ? result as ApplyResult
    : fallback;
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
