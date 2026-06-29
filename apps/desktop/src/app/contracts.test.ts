import { describe, expect, expectTypeOf, it } from "vitest";
import {
  APPEARANCE_THEMES,
  APPLY_MODES,
  APPLY_STEP_STATUSES,
  ASSET_STATUSES,
  ASSET_TYPES,
  CONFLICT_RESOLUTIONS,
  CODEX_MCP_TRANSPORTS,
  CODEX_SCOPES,
  DENSITY_PREFERENCES,
  LOG_LEVELS,
  PLAN_STEP_KINDS,
  PROJECT_STATUSES,
  RISK_LEVELS,
  RUNTIME_SCOPES,
  SYNC_DIRECTIONS,
  RUNTIME_PROVIDERS,
  RUNTIME_SOURCE_FORMATS,
  RUNTIME_SOURCE_SCOPES,
  CANONICAL_IMPORT_DISPOSITIONS,
  type PreviewImportInput,
  type ScanScope,
  type GitStatus,
  type ImportApplyInput,
  type ApplyResult,
  type ConflictApplyInput,
  type PreviewSyncInput,
  type SyncApplyInput,
  type SyncPreview,
  type BackupSummary,
} from "./contracts";

describe("Tauri command contracts", () => {
  it("locks every enum wire value", () => {
    expect(ASSET_TYPES).toEqual(["skill", "command", "mcp"]);
    expect(ASSET_STATUSES).toEqual(["ready", "mounted", "unmounted", "conflict", "invalid"]);
    expect(PROJECT_STATUSES).toEqual(["ready", "changed", "needsSync", "invalid"]);
    expect(RUNTIME_SCOPES).toEqual(["user", "local", "project"]);
    expect(CODEX_SCOPES).toEqual(["global", "project", "system"]);
    expect(CODEX_MCP_TRANSPORTS).toEqual(["stdio", "streamableHttp", "unknown"]);
    expect(CONFLICT_RESOLUTIONS).toEqual(["skip", "rename", "overwrite"]);
    expect(PLAN_STEP_KINDS).toEqual(["check", "import", "mount", "compileMcp", "backup", "restore", "git", "settings"]);
    expect(RISK_LEVELS).toEqual(["none", "low", "medium", "high"]);
    expect(APPEARANCE_THEMES).toEqual(["system", "light", "dark"]);
    expect(DENSITY_PREFERENCES).toEqual(["compact", "comfortable"]);
    expect(LOG_LEVELS).toEqual(["error", "warn", "info", "debug"]);
    expect(APPLY_MODES).toEqual(["planOnly", "apply"]);
    expect(APPLY_STEP_STATUSES).toEqual(["pending", "skipped", "success", "failed"]);
    expect(SYNC_DIRECTIONS).toEqual(["pull", "push"]);
    expect(RUNTIME_PROVIDERS).toEqual(["claude_code", "codex", "custom"]);
    expect(RUNTIME_SOURCE_FORMATS).toEqual([
      "skill_directory",
      "markdown",
      "claude_mcp_json",
      "codex_mcp_toml",
    ]);
    expect(RUNTIME_SOURCE_SCOPES).toEqual(["user", "project", "custom"]);
    expect(CANONICAL_IMPORT_DISPOSITIONS).toEqual([
      "create",
      "conflict",
      "skip",
      "overwrite",
      "rename",
      "unchanged",
    ]);
  });

  it("keeps Sync preview contracts direction-bound and preview-only", () => {
    const input = { direction: "pull" } satisfies PreviewSyncInput;
    const preview = {
      previewId: "preview:sync:pull",
      direction: "pull",
      status: {
        repositoryPath: "~/.my-agent-assets",
        isRepository: true,
        statusMessage: "Git worktree is clean",
        branch: "main",
        remoteName: "origin",
        upstream: "origin/main",
        clean: true,
        ahead: 0,
        behind: 1,
        changedFiles: [],
        conflicts: [],
        syncableChanges: [],
        blockedChanges: [],
      },
      repositoryVisibility: "unknown",
      plannedEffects: ["run git pull --ff-only origin main"],
      warnings: [],
      backupRequired: true,
      canApply: true,
      generatedAtEpochSeconds: 100,
      expiresAtEpochSeconds: 700,
    } satisfies SyncPreview;

    expect(input.direction).toBe("pull");
    expect(preview.direction).toBe("pull");
    expect(preview.previewId).toBe("preview:sync:pull");
    expect(preview.plannedEffects[0]).toContain("pull --ff-only");
    expectTypeOf(input).toMatchTypeOf<PreviewSyncInput>();
    expectTypeOf(preview).toMatchTypeOf<SyncPreview>();
  });

  it("keeps ScanScope discriminated and PreviewImportInput self-contained", () => {
    const scopes = [
      { kind: "user" },
      { kind: "project", projectPath: "~/workspace/project-a" },
      { kind: "custom", path: "~/code" },
    ] satisfies ScanScope[];
    const projectScope = { kind: "project", projectPath: "~/workspace/project-a" } satisfies ScanScope;
    const input = {
      scope: projectScope,
      assetIds: ["skill:review", "mcp:PostgreSQL"],
      conflictResolutions: [
        { conflictId: "mcp:PostgreSQL", resolution: "rename", renameTo: "PostgreSQL-local" },
      ],
    } satisfies PreviewImportInput;

    expect(scopes).toEqual([
      { kind: "user" },
      { kind: "project", projectPath: "~/workspace/project-a" },
      { kind: "custom", path: "~/code" },
    ]);
    expect(input.scope).toEqual(scopes[1]);
    expect(input).not.toHaveProperty("scanId");
    expect(input).not.toHaveProperty("sessionId");
    expectTypeOf(input).toMatchTypeOf<PreviewImportInput>();
  });

  it("keeps GitStatus read-only repository fields explicit", () => {
    const status = {
      repositoryPath: "~/.my-agent-assets",
      isRepository: false,
      statusMessage: "Asset center directory does not exist.",
      branch: "",
      remoteName: "origin",
      clean: true,
      ahead: 0,
      behind: 0,
      changedFiles: [],
      conflicts: [],
      syncableChanges: [],
      blockedChanges: [],
    } satisfies GitStatus;

    expect(status.isRepository).toBe(false);
    expect(status.statusMessage).toBe("Asset center directory does not exist.");
    expectTypeOf(status).toMatchTypeOf<GitStatus>();
  });

  it("keeps apply inputs tied to a preview and explicit mode", () => {
    const input = {
      previewId: "preview-import-1",
      mode: "planOnly",
      scope: { kind: "user" },
      assetIds: ["skill:review"],
      conflictResolutions: [],
      backupBeforeApply: true,
    } satisfies ImportApplyInput;

    expect(input.previewId).toBe("preview-import-1");
    expect(input.mode).toBe("planOnly");
    expect(input.backupBeforeApply).toBe(true);
    expect(input).not.toHaveProperty("runtimePath");
    expectTypeOf(input).toMatchTypeOf<ImportApplyInput>();
  });

  it("keeps Sync apply tied to a timestamped preview request", () => {
    const input = {
      previewId: "preview:sync:push",
      previewGeneratedAtEpochSeconds: 100,
      request: { direction: "push" },
    } satisfies SyncApplyInput;

    expect(input.previewId).toBe("preview:sync:push");
    expect(input.request.direction).toBe("push");
    expectTypeOf(input).toMatchTypeOf<SyncApplyInput>();
  });

  it("keeps conflict apply tied to previewed per-asset decisions", () => {
    const input = {
      previewId: "preview:import:conflicts",
      mode: "apply",
      scope: { kind: "user" },
      assetIds: ["skill:review"],
      conflictResolutions: [{
        conflictId: "conflict:skill:review",
        resolution: "rename",
        renameTo: "review-imported",
      }],
      backupBeforeApply: true,
    } satisfies ConflictApplyInput;

    expect(input.conflictResolutions[0].resolution).toBe("rename");
    expectTypeOf(input).toMatchTypeOf<ConflictApplyInput>();
  });

  it("keeps ApplyResult explicit about backup, step outcomes, warnings, and errors", () => {
    const result = {
      mode: "planOnly",
      ok: true,
      previewId: "preview-mount-1",
      backup: null,
      steps: [
        {
          stepId: "check",
          kind: "check",
          label: "校验",
          status: "skipped",
          message: "Plan only",
          affectedPaths: [],
        },
      ],
      warnings: [],
      errors: [],
    } satisfies ApplyResult;

    expect(result.steps[0].status).toBe("skipped");
    expect(result.backup).toBeNull();
    expectTypeOf(result).toMatchTypeOf<ApplyResult>();
  });

  it("keeps backup history read-only and free of restore command contracts", () => {
    const backup = {
      id: "backup-1",
      label: "Import apply backup",
      createdAt: "2026-06-29T10:00:00Z",
      sizeBytes: 120,
      entryCount: 1,
      manifestPath: "~/.my-agent-assets/backups/backup-1/manifest.json",
      runtimeRoot: "~",
      affectedPaths: ["~/.claude/skills/review"],
    } satisfies BackupSummary;

    expect(backup.manifestPath).toContain("manifest.json");
    expect(backup.affectedPaths).toHaveLength(1);
    expectTypeOf(backup).toMatchTypeOf<BackupSummary>();
  });
});
