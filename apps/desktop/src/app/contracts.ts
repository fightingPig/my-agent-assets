export const ASSET_TYPES = ["skill", "command", "mcp"] as const;
export type AssetType = (typeof ASSET_TYPES)[number];

export const ASSET_STATUSES = ["ready", "mounted", "unmounted", "conflict", "invalid"] as const;
export type AssetStatus = (typeof ASSET_STATUSES)[number];

export const PROJECT_STATUSES = ["ready", "changed", "needsSync", "invalid"] as const;
export type ProjectStatus = (typeof PROJECT_STATUSES)[number];

export const RUNTIME_SCOPES = ["user", "local", "project"] as const;
export type RuntimeScope = (typeof RUNTIME_SCOPES)[number];

export const CODEX_SCOPES = ["global", "project", "system"] as const;
export type CodexScope = (typeof CODEX_SCOPES)[number];

export const CODEX_MCP_TRANSPORTS = ["stdio", "streamableHttp", "unknown"] as const;
export type CodexMcpTransport = (typeof CODEX_MCP_TRANSPORTS)[number];

export const CONFLICT_RESOLUTIONS = ["skip", "rename", "overwrite"] as const;
export type ConflictResolution = (typeof CONFLICT_RESOLUTIONS)[number];

export const PLAN_STEP_KINDS = ["check", "import", "mount", "compileMcp", "backup", "restore", "git", "settings"] as const;
export type PlanStepKind = (typeof PLAN_STEP_KINDS)[number];

export const RISK_LEVELS = ["none", "low", "medium", "high"] as const;
export type RiskLevel = (typeof RISK_LEVELS)[number];

export const APPEARANCE_THEMES = ["system", "light", "dark"] as const;
export type AppearanceTheme = (typeof APPEARANCE_THEMES)[number];

export const DENSITY_PREFERENCES = ["compact", "comfortable"] as const;
export type DensityPreference = (typeof DENSITY_PREFERENCES)[number];

export const LOG_LEVELS = ["error", "warn", "info", "debug"] as const;
export type LogLevel = (typeof LOG_LEVELS)[number];

export const APPLY_MODES = ["planOnly", "apply"] as const;
export type ApplyMode = (typeof APPLY_MODES)[number];

export const APPLY_STEP_STATUSES = ["pending", "skipped", "success", "failed"] as const;
export type ApplyStepStatus = (typeof APPLY_STEP_STATUSES)[number];

export const SYNC_DIRECTIONS = ["pull", "push"] as const;
export type SyncDirection = (typeof SYNC_DIRECTIONS)[number];

export type AppInfo = {
  name: string;
  version: string;
  platform: string;
  arch: string;
  backendReady: boolean;
};

export type AssetCounts = {
  total: number;
  skills: number;
  commands: number;
  mcps: number;
};

export type AssetSummary = {
  id: string;
  name: string;
  title: string;
  assetType: AssetType;
  status: AssetStatus;
  category: string;
  description: string;
  sourcePath: string;
  scope: RuntimeScope | null;
  updatedAt: string | null;
  mountTargets: string[];
};

export type ProjectSummary = {
  id: string;
  name: string;
  title: string;
  path: string;
  status: ProjectStatus;
  description: string;
  updatedAt: string | null;
  assetCounts: AssetCounts;
  mounts: string[];
};

export type CodexDiscoveryInput = {
  projectPath: string | null;
};

export type CodexSkillSummary = {
  id: string;
  name: string;
  description: string;
  scope: CodexScope;
  path: string;
  status: AssetStatus;
  hasScripts: boolean;
  hasReferences: boolean;
  hasAssets: boolean;
  hasOpenaiMetadata: boolean;
  symlinkTarget: string | null;
  updatedAt: string | null;
  warnings: string[];
};

export type CodexSkillListResult = {
  skills: CodexSkillSummary[];
  warnings: string[];
};

export type CodexMcpServerSummary = {
  id: string;
  name: string;
  scope: CodexScope;
  configPath: string;
  transport: CodexMcpTransport;
  command: string | null;
  args: string[];
  url: string | null;
  enabled: boolean;
  enabledTools: string[];
  disabledTools: string[];
  approvalMode: string | null;
  warnings: string[];
};

export type CodexMcpListResult = {
  servers: CodexMcpServerSummary[];
  warnings: string[];
};

export type ScanScope =
  | { kind: "user" }
  | { kind: "project"; projectPath: string }
  | { kind: "custom"; path: string };

export type PlanStep = {
  id: string;
  kind: PlanStepKind;
  label: string;
  description: string;
  risk: RiskLevel;
};

export type ApplyStepResult = {
  stepId: string;
  kind: PlanStepKind;
  label: string;
  status: ApplyStepStatus;
  message: string;
  affectedPaths: string[];
};

export type ConflictResolutionChoice = {
  conflictId: string;
  resolution: ConflictResolution;
  renameTo: string | null;
};

export type ScanResult = {
  scope: ScanScope;
  scannedAt: string;
  assets: AssetSummary[];
  counts: AssetCounts;
  conflictCount: number;
  warnings: string[];
};

export type ConflictPreview = {
  id: string;
  assetId: string;
  assetType: AssetType;
  name: string;
  reason: string;
  existingContent: string;
  incomingContent: string;
  allowedResolutions: ConflictResolution[];
};

export type ImportPreview = {
  previewId: string;
  scope: ScanScope;
  assets: AssetSummary[];
  conflicts: ConflictPreview[];
  steps: PlanStep[];
  warnings: string[];
  canApply: boolean;
};

export type MountTarget = {
  scope: RuntimeScope;
  runtimePath: string;
  projectPath: string | null;
};

export type MountPreview = {
  previewId: string;
  asset: AssetSummary;
  target: MountTarget;
  steps: PlanStep[];
  warnings: string[];
  backupRequired: boolean;
  canApply: boolean;
};

export type BackupSummary = {
  id: string;
  backupId?: string;
  label: string;
  class?: "portable" | "local" | "legacy";
  operation?: string;
  createdAt?: string;
  createdAtEpochSeconds?: number;
  sizeBytes: number;
  entryCount: number;
  manifestPath?: string;
  runtimeRoot?: string;
  affectedPaths?: string[];
  sensitiveConfigRisk?: boolean;
  manualRestoreOnly?: boolean;
  warnings?: string[];
};

export type BackupManifestSummary = BackupSummary & {
  manifestPath: string;
  runtimeRoot: string;
  affectedPaths: string[];
};

export type ApplyResult = {
  mode: ApplyMode;
  ok: boolean;
  previewId: string;
  backup: BackupManifestSummary | null;
  steps: ApplyStepResult[];
  warnings: string[];
  errors: string[];
};

export type GitStatus = {
  repositoryPath: string;
  isRepository: boolean;
  statusMessage: string;
  branch: string;
  remoteName: string;
  remoteIdentity?: string;
  upstream?: string;
  clean: boolean;
  ahead: number;
  behind: number;
  changedFiles: string[];
  conflicts: string[];
  syncableChanges: string[];
  blockedChanges: string[];
  lastSyncedAt?: string | null;
};

export type SyncPreview = {
  previewId: string;
  direction: SyncDirection;
  status: GitStatus;
  repositoryVisibility: "private" | "public" | "internal" | "unknown";
  plannedEffects: string[];
  warnings: string[];
  backupRequired: boolean;
  canApply: boolean;
  generatedAtEpochSeconds: number;
  expiresAtEpochSeconds: number;
};

export type DesktopSettings = {
  assetCenterPath: string;
  scanRoots: string[];
  maxDepth: number;
  backupBeforeApply: boolean;
  planOnlyByDefault: boolean;
  gitDefaultBranch: string;
  gitRemote: string;
  appearanceTheme: AppearanceTheme;
  density: DensityPreference;
  logLevel: LogLevel;
  logRetentionDays: number;
  cliPath: string;
};

export type ScanAssetsInput = { scope: ScanScope };
export type PreviewImportInput = {
  scope: ScanScope;
  assetIds: string[];
  conflictResolutions: ConflictResolutionChoice[];
};
export type ListAssetsInput = { assetType: AssetType | null };
export type PreviewMountInput = { assetId: string; target: MountTarget };
export type PreviewConflictsInput = { scope: ScanScope; assetIds: string[] };
export type PreviewSyncInput = { direction: SyncDirection };
export type SyncApplyInput = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
  request: PreviewSyncInput;
};
export type SyncApplyResult = {
  previewId: string;
  direction: SyncDirection;
  affectedPaths: string[];
  backupId?: string;
  committed: boolean;
  pushed: boolean;
  pulled: boolean;
  warnings: string[];
  journalPath: string;
};
export type SettingsSaveInput = { settings: DesktopSettings };
export type ImportApplyInput = {
  previewId: string;
  mode: ApplyMode;
  scope: ScanScope;
  assetIds: string[];
  conflictResolutions: ConflictResolutionChoice[];
  backupBeforeApply: boolean;
};
export type ConflictApplyInput = {
  previewId: string;
  mode: ApplyMode;
  scope: ScanScope;
  assetIds: string[];
  conflictResolutions: ConflictResolutionChoice[];
  backupBeforeApply: boolean;
};
export type MountApplyInput = {
  previewId: string;
  mode: ApplyMode;
  assetId: string;
  target: MountTarget;
  backupBeforeApply: boolean;
};

export const RUNTIME_PROVIDERS = ["claude_code", "codex", "custom"] as const;
export type RuntimeProvider = (typeof RUNTIME_PROVIDERS)[number];
export const RUNTIME_SOURCE_FORMATS = [
  "skill_directory",
  "markdown",
  "claude_mcp_json",
  "codex_mcp_toml",
] as const;
export type RuntimeSourceFormat = (typeof RUNTIME_SOURCE_FORMATS)[number];
export const RUNTIME_SOURCE_SCOPES = ["user", "project", "custom"] as const;
export type RuntimeSourceScope = (typeof RUNTIME_SOURCE_SCOPES)[number];

export type RuntimeDiscoveryScope =
  | { kind: "user" }
  | { kind: "project"; projectPath: string }
  | {
      kind: "custom";
      path: string;
      assetKind: AssetType;
      sourceFormat: RuntimeSourceFormat;
    };

export type DiscoveredRuntimeSource = {
  sourceId: string;
  provider: RuntimeProvider;
  sourcePath: string;
  configPath?: string;
  assetKind: AssetType;
  assetName: string;
  sourceFormat: RuntimeSourceFormat;
  scope: RuntimeSourceScope;
  isManaged: boolean;
  isSymlink: boolean;
  symlinkTarget?: string;
  warnings: string[];
  eligibleImport: boolean;
  eligibleAdopt: boolean;
};

export type RuntimeDiscoveryResult = {
  sources: DiscoveredRuntimeSource[];
  warnings: string[];
};

export type CanonicalImportResolution =
  | { kind: "unresolved" }
  | { kind: "skip" }
  | { kind: "overwrite" }
  | { kind: "rename"; newName: string };

export const CANONICAL_IMPORT_DISPOSITIONS = [
  "create",
  "conflict",
  "skip",
  "overwrite",
  "rename",
  "unchanged",
] as const;
export type CanonicalImportDisposition =
  (typeof CANONICAL_IMPORT_DISPOSITIONS)[number];

export type CanonicalImportPreviewRequest = {
  scope: RuntimeDiscoveryScope;
  sourceId: string;
  resolution: CanonicalImportResolution;
};

export type CanonicalImportConflict = {
  assetId: string;
  reason: string;
  existingContent: string;
  incomingContent: string;
  rawSource: string;
};

export type CanonicalImportPreview = {
  previewId: string;
  sourceId: string;
  assetId: string;
  assetType: AssetType;
  sourceName: string;
  destinationName: string;
  sourcePath: string;
  destinationPath: string;
  disposition: CanonicalImportDisposition;
  conflict?: CanonicalImportConflict;
  warnings: string[];
  canApply: boolean;
  generatedAtEpochSeconds: number;
  expiresAtEpochSeconds: number;
};

export type CanonicalImportApplyRequest = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
  request: CanonicalImportPreviewRequest;
};

export type CanonicalImportApplyResult = {
  previewId: string;
  assetId: string;
  status: "imported" | "skipped" | "unchanged";
  backupId?: string;
  affectedPaths: string[];
};

export type MountTargetKind =
  | "claude_user_skills"
  | "claude_project_skills"
  | "codex_user_skills"
  | "codex_project_skills"
  | "custom_skill_directory"
  | "claude_user_commands"
  | "claude_project_commands"
  | "custom_command_directory"
  | "claude_user_mcp_json"
  | "claude_local_mcp_json"
  | "claude_project_mcp_json"
  | "codex_user_mcp_toml"
  | "codex_project_mcp_toml"
  | "custom_claude_mcp_json"
  | "custom_codex_mcp_toml";

export type MountAdapter =
  | "symlink_directory"
  | "symlink_file"
  | "windows_directory_junction"
  | "json_mcp_patch"
  | "toml_mcp_patch";

export type RegisteredMountTarget = {
  id: string;
  kind: MountTargetKind;
  provider: RuntimeProvider;
  accepts: AssetType[];
  adapter: MountAdapter;
  scope: "user" | "local" | "project" | "custom";
  path: string;
  projectPath?: string;
  providerState:
    | "not_installed"
    | "installed_not_initialized"
    | "initialized";
  status: "ready" | "blocked" | "invalid";
};

export type CanonicalMountPreviewRequest = {
  assetId: string;
  targetId: string;
};

export type CanonicalMountPreview = {
  previewId: string;
  assetId: string;
  targetId: string;
  canonicalPath: string;
  affectedTargetPath: string;
  compatible: boolean;
  adapter: MountAdapter;
  unsupportedReason?: string;
  disposition:
    | "create_link"
    | "replace_runtime_path"
    | "compile_mcp"
    | "already_mounted"
    | "blocked";
  plannedEffects: string[];
  warnings: string[];
  backupRequired: boolean;
  canApply: boolean;
  generatedAtEpochSeconds: number;
  expiresAtEpochSeconds: number;
};

export type CanonicalMountApplyRequest = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
  request: CanonicalMountPreviewRequest;
};

export type CanonicalMountApplyResult = {
  previewId: string;
  assetId: string;
  targetId: string;
  mounted: boolean;
  backupId?: string;
  affectedPaths: string[];
  warnings: string[];
};

export type CanonicalUnmountPreviewRequest = {
  assetId: string;
  targetId: string;
};

export type CanonicalUnmountPreview = {
  previewId: string;
  assetId: string;
  targetId: string;
  affectedTargetPath: string;
  plannedEffects: string[];
  warnings: string[];
  backupRequired: boolean;
  canApply: boolean;
  generatedAtEpochSeconds: number;
  expiresAtEpochSeconds: number;
};

export type CanonicalUnmountApplyRequest = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
  request: CanonicalUnmountPreviewRequest;
};

export type CanonicalUnmountApplyResult = {
  previewId: string;
  assetId: string;
  targetId: string;
  unmounted: boolean;
  backupId?: string;
  affectedPaths: string[];
};

export type CanonicalDeletePreviewRequest = {
  assetId: string;
  mode: "require_unmounted" | "unmount_all";
};

export type CanonicalDeleteBindingImpact = {
  targetId: string;
  targetPath: string;
  canUnmount: boolean;
  warnings: string[];
};

export type CanonicalDeletePreview = {
  previewId: string;
  assetId: string;
  canonicalPath: string;
  bindings: CanonicalDeleteBindingImpact[];
  plannedEffects: string[];
  warnings: string[];
  backupRequired: boolean;
  canApply: boolean;
  generatedAtEpochSeconds: number;
  expiresAtEpochSeconds: number;
};

export type CanonicalDeleteApplyRequest = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
  request: CanonicalDeletePreviewRequest;
};

export type CanonicalDeleteApplyResult = {
  previewId: string;
  assetId: string;
  deleted: boolean;
  portableBackupId: string;
  localBackupId: string;
  affectedPaths: string[];
  journalPath: string;
};

export type AdoptSelection = {
  sourceId: string;
  resolution: CanonicalImportResolution;
};

export type AdoptPreviewRequest = {
  scope: RuntimeDiscoveryScope;
  selections: AdoptSelection[];
};

export type AdoptItemPreview = {
  sourceId: string;
  importPlan: CanonicalImportPreview;
  targetId?: string;
  targetPath?: string;
  backupRequired: boolean;
  warnings: string[];
  canApply: boolean;
};

export type AdoptPreview = {
  previewId: string;
  items: AdoptItemPreview[];
  importPlan: string[];
  mountPlan: string[];
  backupPlan: string[];
  warnings: string[];
  canApply: boolean;
  generatedAtEpochSeconds: number;
  expiresAtEpochSeconds: number;
};

export type AdoptApplyRequest = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
  request: AdoptPreviewRequest;
};

export type AdoptApplyResult = {
  previewId: string;
  items: Array<{
    sourceId: string;
    assetId: string;
    targetId?: string;
    imported: boolean;
    mounted: boolean;
  }>;
  affectedPaths: string[];
  journalPath: string;
};

export type BatchImportSelection = {
  sourceId: string;
  resolution: CanonicalImportResolution;
};

export type BatchImportPreviewRequest = {
  scope: RuntimeDiscoveryScope;
  selections: BatchImportSelection[];
};

export type BatchImportPreview = {
  previewId: string;
  items: CanonicalImportPreview[];
  warnings: string[];
  canApply: boolean;
  generatedAtEpochSeconds: number;
  expiresAtEpochSeconds: number;
};

export type BatchImportApplyRequest = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
  request: BatchImportPreviewRequest;
};

export type BatchImportApplyResult = {
  previewId: string;
  items: CanonicalImportApplyResult[];
  affectedPaths: string[];
  journalPath: string;
};
