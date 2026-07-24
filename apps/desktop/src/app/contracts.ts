export const ASSET_TYPES = ["skill", "command", "mcp"] as const;
export type AssetType = (typeof ASSET_TYPES)[number];

export const ASSET_STATUSES = ["ready", "mounted", "unmounted", "conflict", "invalid"] as const;
export type AssetStatus = (typeof ASSET_STATUSES)[number];

export const PROJECT_STATUSES = ["ready", "changed", "needsSync", "invalid"] as const;
export type ProjectStatus = (typeof PROJECT_STATUSES)[number];

export const RUNTIME_SCOPES = ["user", "local", "project"] as const;
export type RuntimeScope = (typeof RUNTIME_SCOPES)[number];

export const PLAN_STEP_KINDS = ["check", "import", "mount", "compileMcp", "backup", "git", "settings"] as const;
export type PlanStepKind = (typeof PLAN_STEP_KINDS)[number];

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

export const DESKTOP_COMMAND_ERROR_CODES = [
  "environmentUnavailable",
  "stalePreview",
  "validationFailed",
  "notInitialized",
  "operationBlocked",
  "notFound",
  "operationFailed",
] as const;
export type DesktopCommandErrorCode = (typeof DESKTOP_COMMAND_ERROR_CODES)[number];

export type DesktopCommandError = {
  code: DesktopCommandErrorCode;
  message: string;
  parameters: Record<string, string>;
};

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

export type CanonicalAssetContent = {
  assetId: string;
  assetType: AssetType;
  canonicalPath: string;
  contentPath: string;
  content: string;
  truncated: boolean;
};

export type AssetOpenInput = {
  assetId: string;
  action: "reveal" | "open_external";
};

export type AssetOpenResult = {
  assetId: string;
  path: string;
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

export type ProjectSaveRequest = {
  id?: string;
  name: string;
  title: string;
  path: string;
  description: string;
};

export type ProjectRemoveRequest = {
  id: string;
};

export type ManagedProject = {
  id: string;
  name: string;
  title: string;
  path: string;
  description: string;
};

export type ProjectChangePreview = {
  previewId: string;
  operation: "save" | "remove";
  project?: ManagedProject;
  affectedPaths: string[];
  migratedTargetIds: string[];
  blockingBindings: string[];
  warnings: string[];
  canApply: boolean;
  generatedAtEpochSeconds: number;
  expiresAtEpochSeconds: number;
};

export type ProjectSaveApplyRequest = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
  request: ProjectSaveRequest;
};

export type ProjectRemoveApplyRequest = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
  request: ProjectRemoveRequest;
};

export type ProjectChangeResult = {
  previewId: string;
  operation: "save" | "remove";
  projectId: string;
  registryPath: string;
  affectedPaths: string[];
};

export type ApplyStepResult = {
  stepId: string;
  kind: PlanStepKind;
  label: string;
  status: ApplyStepStatus;
  message: string;
  affectedPaths: string[];
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

export type BackupRevealInput = {
  entryId: string;
};

export type BackupRevealResult = {
  manifestPath: string;
};

export type BackupDeletePreviewRequest = {
  entryId: string;
};

export type BackupDeletePreview = {
  previewId: string;
  entryId: string;
  backupId: string;
  class: "portable" | "local" | "legacy";
  backupPath: string;
  sizeBytes: number;
  entryCount: number;
  sensitiveConfigRisk: boolean;
  plannedEffects: string[];
  warnings: string[];
  canApply: boolean;
  generatedAtEpochSeconds: number;
  expiresAtEpochSeconds: number;
};

export type BackupDeleteApplyRequest = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
  request: BackupDeletePreviewRequest;
};

export type BackupDeleteApplyResult = {
  previewId: string;
  entryId: string;
  deleted: boolean;
  affectedPaths: string[];
  warnings: string[];
  journalPath: string;
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

export type OperationJournalStatus =
  | "started"
  | "rollback_required"
  | "recovered"
  | "completed";

export type OperationJournalSummary = {
  schemaVersion: number;
  operationId: string;
  operationKind: string;
  status: OperationJournalStatus;
  createdAtEpochSeconds: number;
  completedSteps: string[];
  recoveryMessage?: string;
  recoveredAtEpochSeconds?: number;
};

export type RecoveryStatus = {
  writesBlocked: boolean;
  journals: OperationJournalSummary[];
  recentRecoveries: OperationJournalSummary[];
  message: string;
};

export type AuditOutcome = "completed" | "rollback_required" | "recovered";

export type AuditLogEntry = {
  schemaVersion: number;
  occurredAtEpochSeconds: number;
  operationType: string;
  outcome: AuditOutcome;
};

export type CanonicalContentState = "ready" | "missing_content" | "unregistered" | "invalid_content";

export type ContentDiagnostic = {
  assetId: string;
  assetType: AssetType;
  name: string;
  path: string;
  state: CanonicalContentState;
  message?: string;
};

export type DoctorCheck = {
  id: string;
  label: string;
  status: "ok" | "warning" | "error";
  message: string;
};

export type DoctorReport = {
  assetCenterPath: string;
  initialized: boolean;
  checks: DoctorCheck[];
  contentDiagnostics: ContentDiagnostic[];
};

export const CONSISTENCY_REPAIR_ACTIONS = [
  "remove_missing_registry_record",
  "register_unregistered_content",
  "delete_unregistered_content",
] as const;
export type ConsistencyRepairAction = (typeof CONSISTENCY_REPAIR_ACTIONS)[number];

export type ConsistencyRepairPreviewRequest = {
  assetId: string;
  action: ConsistencyRepairAction;
};

export type ConsistencyRepairPreview = {
  previewId: string;
  request: ConsistencyRepairPreviewRequest;
  diagnostic: ContentDiagnostic;
  plannedEffects: string[];
  warnings: string[];
  canApply: boolean;
  generatedAtEpochSeconds: number;
  expiresAtEpochSeconds: number;
};

export type ConsistencyRepairApplyRequest = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
  request: ConsistencyRepairPreviewRequest;
};

export type ConsistencyRepairApplyResult = {
  previewId: string;
  assetId: string;
  action: ConsistencyRepairAction;
  affectedPaths: string[];
  journalPath: string;
};

export type DiagnosticExportFileKind = "audit_log" | "status_summary" | "version_metadata";

export type DiagnosticExportFile = {
  logicalPath: string;
  kind: DiagnosticExportFileKind;
};

export type DiagnosticExportPreview = {
  previewId: string;
  packagePath: string;
  includedFiles: DiagnosticExportFile[];
  warnings: string[];
  canApply: boolean;
  generatedAtEpochSeconds: number;
  expiresAtEpochSeconds: number;
};

export type DiagnosticExportApplyRequest = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
};

export type DiagnosticExportApplyResult = {
  previewId: string;
  packagePath: string;
  journalPath: string;
};

export type InitializationPreview = {
  previewId: string;
  assetCenterPath: string;
  plannedPaths: string[];
  warnings: string[];
  alreadyInitialized: boolean;
  canApply: boolean;
  generatedAtEpochSeconds: number;
  expiresAtEpochSeconds: number;
};

export type InitializationApplyInput = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
};

export type InitializationApplyResult = {
  previewId: string;
  assetCenterPath: string;
  created: boolean;
  createdPaths: string[];
};

export type SyncPreview = {
  previewId: string;
  direction: SyncDirection;
  status: GitStatus;
  repositoryVisibility: "private" | "public" | "internal" | "unknown";
  allowPublicRemotePush: boolean;
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
  backupWarningThresholdBytes: number;
  planOnlyByDefault: boolean;
  gitDefaultBranch: string;
  gitRemote: string;
  allowPublicRemotePush: boolean;
  appearanceTheme: AppearanceTheme;
  density: DensityPreference;
  logLevel: LogLevel;
  logRetentionDays: number;
  cliPath: string;
};

export type ListAssetsInput = { assetType: AssetType | null };
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
  contentDiagnostics?: ContentDiagnostic[];
  journalPath: string;
};
export type SettingsSaveInput = { settings: DesktopSettings };
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

export type TargetRegistrationPreviewRequest = {
  id: string;
  kind: MountTargetKind;
  location: string;
};

export type TargetRemovalPreviewRequest = {
  targetId: string;
};

export type TargetChangePreview = {
  previewId: string;
  operation: "add" | "remove";
  target: RegisteredMountTarget;
  affectedPaths: string[];
  blockingBindings: string[];
  warnings: string[];
  canApply: boolean;
  generatedAtEpochSeconds: number;
  expiresAtEpochSeconds: number;
};

export type TargetRegistrationApplyRequest = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
  request: TargetRegistrationPreviewRequest;
};

export type TargetRemovalApplyRequest = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
  request: TargetRemovalPreviewRequest;
};

export type TargetChangeResult = {
  previewId: string;
  operation: "add" | "remove";
  targetId: string;
  registryPath: string;
  backupPath: string;
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

export type McpTransport = "stdio" | "http" | "sse";

export type CanonicalMcpSpec = {
  type?: McpTransport;
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
  url?: string;
  headers?: Record<string, string>;
  [key: string]: unknown;
};

export type CanonicalMcp = {
  schemaVersion: 1;
  name: string;
  spec: CanonicalMcpSpec;
  providerExtensions: Record<string, unknown>;
};

export type McpSavePreviewRequest = {
  assetId?: string;
  canonical: CanonicalMcp;
  title?: string;
  description?: string;
};

export type McpTargetCompatibility = {
  targetId: string;
  compatible: boolean;
  warnings: string[];
  blockedReason?: string;
};

export type McpSavePreview = {
  previewId: string;
  operation: "create" | "edit";
  assetId: string;
  canonicalPath: string;
  registryPath: string;
  outOfSyncTargetIds: string[];
  targetCompatibility: McpTargetCompatibility[];
  plannedEffects: string[];
  warnings: string[];
  canApply: boolean;
  generatedAtEpochSeconds: number;
  expiresAtEpochSeconds: number;
};

export type McpSaveApplyRequest = {
  previewId: string;
  previewGeneratedAtEpochSeconds: number;
  request: McpSavePreviewRequest;
};

export type McpSaveApplyResult = {
  previewId: string;
  operation: "create" | "edit";
  assetId: string;
  canonicalPath: string;
  outOfSyncTargetIds: string[];
  affectedPaths: string[];
};

export type McpBindingStatus = "mounted" | "out_of_sync" | "orphaned";

export type McpAssetDefinition = {
  assetId: string;
  canonical: CanonicalMcp;
  title?: string;
  description?: string;
  bindings: {
    targetId: string;
    status: McpBindingStatus;
    lastSyncedAt?: string;
  }[];
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
  removeMcpTargetEntries: boolean;
};

export type CanonicalDeleteBindingImpact = {
  targetId: string;
  targetPath: string;
  canUnmount: boolean;
  willRemoveTargetEntry: boolean;
  warnings: string[];
};

export type CanonicalDeletePreview = {
  previewId: string;
  assetId: string;
  canonicalPath: string;
  removeMcpTargetEntries: boolean;
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
