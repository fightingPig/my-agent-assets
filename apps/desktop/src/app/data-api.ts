import { invoke } from "@tauri-apps/api/core";
import { isTauriRuntime } from "../lib/platform";
import type {
  AssetSummary,
  BackupSummary,
  ConflictPreview,
  ConflictApplyInput,
  CodexDiscoveryInput,
  CodexMcpListResult,
  CodexSkillListResult,
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
  PreviewSyncInput,
  ProjectSummary,
  ScanAssetsInput,
  ScanResult,
  SettingsSaveInput,
  SyncApplyInput,
  SyncApplyResult,
  SyncPreview,
  RuntimeDiscoveryScope,
  RuntimeDiscoveryResult,
  CanonicalImportPreviewRequest,
  CanonicalImportPreview,
  CanonicalImportApplyRequest,
  CanonicalImportApplyResult,
  RegisteredMountTarget,
  TargetRegistrationPreviewRequest,
  TargetRegistrationApplyRequest,
  TargetRemovalPreviewRequest,
  TargetRemovalApplyRequest,
  TargetChangePreview,
  TargetChangeResult,
  CanonicalMountPreviewRequest,
  CanonicalMountPreview,
  CanonicalMountApplyRequest,
  CanonicalMountApplyResult,
  CanonicalUnmountPreviewRequest,
  CanonicalUnmountPreview,
  CanonicalUnmountApplyRequest,
  CanonicalUnmountApplyResult,
  CanonicalDeletePreviewRequest,
  CanonicalDeletePreview,
  CanonicalDeleteApplyRequest,
  CanonicalDeleteApplyResult,
  AdoptPreviewRequest,
  AdoptPreview,
  AdoptApplyRequest,
  AdoptApplyResult,
  BatchImportPreviewRequest,
  BatchImportPreview,
  BatchImportApplyRequest,
  BatchImportApplyResult,
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
  remoteName: "origin",
  clean: true,
  ahead: 0,
  behind: 0,
  changedFiles: [],
  conflicts: [],
  syncableChanges: [],
  blockedChanges: [],
  lastSyncedAt: null,
};

export async function listAssets(input: ListAssetsInput = { assetType: null }): Promise<AssetSummary[]> {
  const assets = await invokeRead<unknown>("list_assets", { input }, []);
  return Array.isArray(assets) ? assets as AssetSummary[] : [];
}

export async function listProjects(): Promise<ProjectSummary[]> {
  const projects = await invokeRead<unknown>("list_projects", undefined, []);
  return Array.isArray(projects) ? projects as ProjectSummary[] : [];
}

export async function listBackups(): Promise<BackupSummary[]> {
  const backups = await invokeRead<unknown>("list_backups", undefined, []);
  return Array.isArray(backups) ? backups as BackupSummary[] : [];
}

export async function gitStatus(): Promise<GitStatus> {
  const status = await invokeRead<unknown>("git_status", undefined, fallbackGitStatus);
  return isRecord(status) && typeof status.repositoryPath === "string" ? status as GitStatus : fallbackGitStatus;
}

export async function settingsLoad(): Promise<DesktopSettings> {
  const settings = await invokeRead<unknown>("settings_load", undefined, fallbackSettings);
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
  const result = await invokeRead<unknown>("scan_assets", { input }, fallback);
  return isRecord(result) && Array.isArray(result.assets) && isRecord(result.counts) ? result as ScanResult : fallback;
}

export async function discoverRuntimeSources(
  input: RuntimeDiscoveryScope,
): Promise<RuntimeDiscoveryResult> {
  const fallback: RuntimeDiscoveryResult = {
    sources: [],
    warnings: ["Tauri runtime is unavailable; runtime discovery skipped."],
  };
  const result = await invokeRead<unknown>(
    "discover_runtime_sources",
    { input },
    fallback,
  );
  return isRecord(result) &&
    Array.isArray(result.sources) &&
    Array.isArray(result.warnings)
    ? (result as RuntimeDiscoveryResult)
    : fallback;
}

export async function canonicalImportPreview(
  input: CanonicalImportPreviewRequest,
): Promise<CanonicalImportPreview> {
  const fallback: CanonicalImportPreview = {
    previewId: "canonical-import-unavailable",
    sourceId: input.sourceId,
    assetId: "",
    assetType: "skill",
    sourceName: "",
    destinationName: "",
    sourcePath: "",
    destinationPath: "",
    disposition: "conflict",
    warnings: ["Tauri runtime is unavailable; canonical import preview skipped."],
    canApply: false,
    generatedAtEpochSeconds: 0,
    expiresAtEpochSeconds: 0,
  };
  const result = await invokeOrFallback<unknown>(
    "canonical_import_preview",
    { input },
    fallback,
  );
  return isRecord(result) &&
    typeof result.previewId === "string" &&
    typeof result.sourceId === "string"
    ? (result as CanonicalImportPreview)
    : fallback;
}

export async function canonicalImportApply(
  input: CanonicalImportApplyRequest,
): Promise<CanonicalImportApplyResult> {
  if (!isTauriRuntime()) {
    throw new Error("canonical_import_apply requires the Tauri runtime.");
  }
  const result = await invoke<unknown>("canonical_import_apply", { input });
  if (
    !isRecord(result) ||
    typeof result.previewId !== "string" ||
    !Array.isArray(result.affectedPaths)
  ) {
    throw new Error("canonical_import_apply returned an invalid response.");
  }
  return result as CanonicalImportApplyResult;
}

export async function listMountTargets(): Promise<RegisteredMountTarget[]> {
  const result = await invokeRead<unknown>("list_mount_targets", undefined, []);
  return Array.isArray(result) ? (result as RegisteredMountTarget[]) : [];
}

export async function targetRegistrationPreview(
  input: TargetRegistrationPreviewRequest,
): Promise<TargetChangePreview> {
  return invokeTargetPreview("target_registration_preview", input);
}

export async function targetRegistrationApply(
  input: TargetRegistrationApplyRequest,
): Promise<TargetChangeResult> {
  return invokeTargetApply("target_registration_apply", input);
}

export async function targetRemovalPreview(
  input: TargetRemovalPreviewRequest,
): Promise<TargetChangePreview> {
  return invokeTargetPreview("target_removal_preview", input);
}

export async function targetRemovalApply(
  input: TargetRemovalApplyRequest,
): Promise<TargetChangeResult> {
  return invokeTargetApply("target_removal_apply", input);
}

export async function canonicalMountPreview(
  input: CanonicalMountPreviewRequest,
): Promise<CanonicalMountPreview> {
  const fallback: CanonicalMountPreview = {
    previewId: "canonical-mount-unavailable",
    assetId: input.assetId,
    targetId: input.targetId,
    canonicalPath: "",
    affectedTargetPath: "",
    compatible: false,
    adapter: "symlink_directory",
    unsupportedReason: "Tauri runtime is unavailable.",
    disposition: "blocked",
    plannedEffects: [],
    warnings: ["Tauri runtime is unavailable; canonical mount preview skipped."],
    backupRequired: false,
    canApply: false,
    generatedAtEpochSeconds: 0,
    expiresAtEpochSeconds: 0,
  };
  const result = await invokeOrFallback<unknown>(
    "canonical_mount_preview",
    { input },
    fallback,
  );
  return isRecord(result) &&
    typeof result.previewId === "string" &&
    typeof result.targetId === "string"
    ? (result as CanonicalMountPreview)
    : fallback;
}

export async function canonicalMountApply(
  input: CanonicalMountApplyRequest,
): Promise<CanonicalMountApplyResult> {
  if (!isTauriRuntime()) {
    throw new Error("canonical_mount_apply requires the Tauri runtime.");
  }
  const result = await invoke<unknown>("canonical_mount_apply", { input });
  if (
    !isRecord(result) ||
    typeof result.previewId !== "string" ||
    !Array.isArray(result.affectedPaths)
  ) {
    throw new Error("canonical_mount_apply returned an invalid response.");
  }
  return result as CanonicalMountApplyResult;
}

export async function canonicalUnmountPreview(
  input: CanonicalUnmountPreviewRequest,
): Promise<CanonicalUnmountPreview> {
  const fallback: CanonicalUnmountPreview = {
    previewId: "canonical-unmount-unavailable",
    assetId: input.assetId,
    targetId: input.targetId,
    affectedTargetPath: "",
    plannedEffects: [],
    warnings: ["Tauri runtime is unavailable; canonical unmount preview skipped."],
    backupRequired: false,
    canApply: false,
    generatedAtEpochSeconds: 0,
    expiresAtEpochSeconds: 0,
  };
  const result = await invokeOrFallback<unknown>(
    "canonical_unmount_preview",
    { input },
    fallback,
  );
  return isRecord(result) && typeof result.previewId === "string"
    ? (result as CanonicalUnmountPreview)
    : fallback;
}

export async function canonicalUnmountApply(
  input: CanonicalUnmountApplyRequest,
): Promise<CanonicalUnmountApplyResult> {
  if (!isTauriRuntime()) {
    throw new Error("canonical_unmount_apply requires the Tauri runtime.");
  }
  const result = await invoke<unknown>("canonical_unmount_apply", { input });
  if (
    !isRecord(result) ||
    typeof result.previewId !== "string" ||
    !Array.isArray(result.affectedPaths)
  ) {
    throw new Error("canonical_unmount_apply returned an invalid response.");
  }
  return result as CanonicalUnmountApplyResult;
}

export async function canonicalDeletePreview(
  input: CanonicalDeletePreviewRequest,
): Promise<CanonicalDeletePreview> {
  const fallback: CanonicalDeletePreview = {
    previewId: "canonical-delete-unavailable",
    assetId: input.assetId,
    canonicalPath: "",
    bindings: [],
    plannedEffects: [],
    warnings: ["Tauri runtime is unavailable; canonical delete preview skipped."],
    backupRequired: true,
    canApply: false,
    generatedAtEpochSeconds: 0,
    expiresAtEpochSeconds: 0,
  };
  const result = await invokeOrFallback<unknown>(
    "canonical_delete_preview",
    { input },
    fallback,
  );
  return isRecord(result) && typeof result.previewId === "string"
    ? (result as CanonicalDeletePreview)
    : fallback;
}

export async function canonicalDeleteApply(
  input: CanonicalDeleteApplyRequest,
): Promise<CanonicalDeleteApplyResult> {
  if (!isTauriRuntime()) {
    throw new Error("canonical_delete_apply requires the Tauri runtime.");
  }
  const result = await invoke<unknown>("canonical_delete_apply", { input });
  if (
    !isRecord(result) ||
    typeof result.previewId !== "string" ||
    !Array.isArray(result.affectedPaths)
  ) {
    throw new Error("canonical_delete_apply returned an invalid response.");
  }
  return result as CanonicalDeleteApplyResult;
}

export async function previewAdopt(
  input: AdoptPreviewRequest,
): Promise<AdoptPreview> {
  const fallback: AdoptPreview = {
    previewId: "adopt-unavailable",
    items: [],
    importPlan: [],
    mountPlan: [],
    backupPlan: [],
    warnings: ["Tauri runtime is unavailable; adopt preview skipped."],
    canApply: false,
    generatedAtEpochSeconds: 0,
    expiresAtEpochSeconds: 0,
  };
  const result = await invokeOrFallback<unknown>(
    "preview_adopt",
    { input },
    fallback,
  );
  return isRecord(result) &&
    typeof result.previewId === "string" &&
    Array.isArray(result.items)
    ? (result as AdoptPreview)
    : fallback;
}

export async function adoptApply(
  input: AdoptApplyRequest,
): Promise<AdoptApplyResult> {
  if (!isTauriRuntime()) {
    throw new Error("adopt_apply requires the Tauri runtime.");
  }
  const result = await invoke<unknown>("adopt_apply", { input });
  if (
    !isRecord(result) ||
    typeof result.previewId !== "string" ||
    !Array.isArray(result.items) ||
    !Array.isArray(result.affectedPaths)
  ) {
    throw new Error("adopt_apply returned an invalid response.");
  }
  return result as AdoptApplyResult;
}

export async function canonicalBatchImportPreview(
  input: BatchImportPreviewRequest,
): Promise<BatchImportPreview> {
  const fallback: BatchImportPreview = {
    previewId: "batch-import-unavailable",
    items: [],
    warnings: ["Tauri runtime is unavailable; batch import preview skipped."],
    canApply: false,
    generatedAtEpochSeconds: 0,
    expiresAtEpochSeconds: 0,
  };
  const result = await invokeOrFallback<unknown>(
    "canonical_batch_import_preview",
    { input },
    fallback,
  );
  return isRecord(result) &&
    typeof result.previewId === "string" &&
    Array.isArray(result.items)
    ? (result as BatchImportPreview)
    : fallback;
}

export async function canonicalBatchImportApply(
  input: BatchImportApplyRequest,
): Promise<BatchImportApplyResult> {
  if (!isTauriRuntime()) {
    throw new Error("canonical_batch_import_apply requires the Tauri runtime.");
  }
  const result = await invoke<unknown>(
    "canonical_batch_import_apply",
    { input },
  );
  if (
    !isRecord(result) ||
    typeof result.previewId !== "string" ||
    !Array.isArray(result.items) ||
    !Array.isArray(result.affectedPaths)
  ) {
    throw new Error("canonical_batch_import_apply returned an invalid response.");
  }
  return result as BatchImportApplyResult;
}

export async function listCodexSkills(
  input: CodexDiscoveryInput = { projectPath: null },
): Promise<CodexSkillListResult> {
  const fallback: CodexSkillListResult = { skills: [], warnings: [] };
  const result = await invokeRead<unknown>("list_codex_skills", { input }, fallback);
  return isRecord(result) && Array.isArray(result.skills) && Array.isArray(result.warnings)
    ? result as CodexSkillListResult
    : fallback;
}

export async function listCodexMcpServers(
  input: CodexDiscoveryInput = { projectPath: null },
): Promise<CodexMcpListResult> {
  const fallback: CodexMcpListResult = { servers: [], warnings: [] };
  const result = await invokeRead<unknown>("list_codex_mcp_servers", { input }, fallback);
  return isRecord(result) && Array.isArray(result.servers) && Array.isArray(result.warnings)
    ? result as CodexMcpListResult
    : fallback;
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

export async function previewSync(input: PreviewSyncInput): Promise<SyncPreview> {
  const fallback: SyncPreview = {
    previewId: `preview:sync:${input.direction}`,
    direction: input.direction,
    status: fallbackGitStatus,
    repositoryVisibility: "unknown",
    plannedEffects: [],
    warnings: ["Tauri runtime is unavailable; sync preview skipped."],
    backupRequired: input.direction === "pull",
    canApply: false,
    generatedAtEpochSeconds: 0,
    expiresAtEpochSeconds: 0,
  };
  const result = await invokeOrFallback<unknown>("preview_sync", { input }, fallback);
  return isRecord(result) && typeof result.previewId === "string" && Array.isArray(result.plannedEffects) && Array.isArray(result.warnings)
    ? result as SyncPreview
    : fallback;
}

export async function syncApply(input: SyncApplyInput): Promise<SyncApplyResult> {
  if (!isTauriRuntime()) {
    throw new Error("sync_apply requires the Tauri runtime.");
  }
  const result = await invoke<unknown>("sync_apply", { input });
  if (
    !isRecord(result) ||
    typeof result.previewId !== "string" ||
    !Array.isArray(result.affectedPaths)
  ) {
    throw new Error("sync_apply returned an invalid response.");
  }
  return result as SyncApplyResult;
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

async function invokeRead<T>(
  command: string,
  args: Record<string, unknown> | undefined,
  browserFallback: T,
): Promise<T> {
  if (!isTauriRuntime()) return browserFallback;
  return args === undefined ? invoke<T>(command) : invoke<T>(command, args);
}

async function invokeTargetPreview(
  command: "target_registration_preview" | "target_removal_preview",
  input: TargetRegistrationPreviewRequest | TargetRemovalPreviewRequest,
): Promise<TargetChangePreview> {
  if (!isTauriRuntime()) {
    throw new Error(`${command} requires the Tauri runtime.`);
  }
  const result = await invoke<unknown>(command, { input });
  if (
    !isRecord(result) ||
    typeof result.previewId !== "string" ||
    !isRecord(result.target) ||
    !Array.isArray(result.affectedPaths)
  ) {
    throw new Error(`${command} returned an invalid response.`);
  }
  return result as TargetChangePreview;
}

async function invokeTargetApply(
  command: "target_registration_apply" | "target_removal_apply",
  input: TargetRegistrationApplyRequest | TargetRemovalApplyRequest,
): Promise<TargetChangeResult> {
  if (!isTauriRuntime()) {
    throw new Error(`${command} requires the Tauri runtime.`);
  }
  const result = await invoke<unknown>(command, { input });
  if (
    !isRecord(result) ||
    typeof result.previewId !== "string" ||
    typeof result.targetId !== "string" ||
    typeof result.registryPath !== "string"
  ) {
    throw new Error(`${command} returned an invalid response.`);
  }
  return result as TargetChangeResult;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
