export const ASSET_TYPES = ["skill", "command", "mcp"] as const;
export type AssetType = (typeof ASSET_TYPES)[number];

export const ASSET_STATUSES = ["ready", "mounted", "unmounted", "conflict", "invalid"] as const;
export type AssetStatus = (typeof ASSET_STATUSES)[number];

export const PROJECT_STATUSES = ["ready", "changed", "needsSync", "invalid"] as const;
export type ProjectStatus = (typeof PROJECT_STATUSES)[number];

export const RUNTIME_SCOPES = ["user", "local", "project"] as const;
export type RuntimeScope = (typeof RUNTIME_SCOPES)[number];

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
  asset: AssetSummary;
  target: MountTarget;
  steps: PlanStep[];
  warnings: string[];
  backupRequired: boolean;
  canApply: boolean;
};

export type BackupSummary = {
  id: string;
  label: string;
  createdAt: string;
  sizeBytes: number;
  entryCount: number;
};

export type RestorePreview = {
  backup: BackupSummary;
  affectedPaths: string[];
  steps: PlanStep[];
  warnings: string[];
  backupBeforeRestore: boolean;
  canApply: boolean;
};

export type GitStatus = {
  repositoryPath: string;
  isRepository: boolean;
  statusMessage: string;
  branch: string;
  remote: string | null;
  clean: boolean;
  ahead: number;
  behind: number;
  changedFiles: string[];
  conflicts: string[];
  lastSyncedAt: string | null;
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
export type PreviewRestoreInput = { backupId: string };
export type SettingsSaveInput = { settings: DesktopSettings };
