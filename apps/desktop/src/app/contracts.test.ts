import { describe, expect, expectTypeOf, it } from "vitest";
import {
  APPEARANCE_THEMES,
  APPLY_MODES,
  APPLY_STEP_STATUSES,
  ASSET_STATUSES,
  ASSET_TYPES,
  DENSITY_PREFERENCES,
  DESKTOP_COMMAND_ERROR_CODES,
  LOG_LEVELS,
  PLAN_STEP_KINDS,
  PROJECT_STATUSES,
  RUNTIME_SCOPES,
  SYNC_DIRECTIONS,
  RUNTIME_PROVIDERS,
  RUNTIME_SOURCE_FORMATS,
  RUNTIME_SOURCE_SCOPES,
  CANONICAL_IMPORT_DISPOSITIONS,
  CONSISTENCY_REPAIR_ACTIONS,
  type GitStatus,
  type ApplyResult,
  type PreviewSyncInput,
  type SyncApplyInput,
  type SyncPreview,
  type BackupSummary,
  type BackupDeletePreview,
} from "./contracts";

describe("Tauri command contracts", () => {
  it("locks every enum wire value", () => {
    expect(ASSET_TYPES).toEqual(["skill", "command", "mcp"]);
    expect(ASSET_STATUSES).toEqual(["ready", "mounted", "unmounted", "conflict", "invalid"]);
    expect(PROJECT_STATUSES).toEqual([
      "ready",
      "unchecked",
      "needs_attention",
      "missing_path",
      "invalid",
    ]);
    expect(RUNTIME_SCOPES).toEqual(["user", "local", "project"]);
    expect(PLAN_STEP_KINDS).toEqual(["check", "import", "mount", "compileMcp", "backup", "git", "settings"]);
    expect(APPEARANCE_THEMES).toEqual(["system", "light", "dark"]);
    expect(DENSITY_PREFERENCES).toEqual(["compact", "comfortable"]);
    expect(LOG_LEVELS).toEqual(["error", "warn", "info", "debug"]);
    expect(APPLY_MODES).toEqual(["planOnly", "apply"]);
    expect(APPLY_STEP_STATUSES).toEqual(["pending", "skipped", "success", "failed"]);
    expect(SYNC_DIRECTIONS).toEqual(["pull", "push"]);
    expect(DESKTOP_COMMAND_ERROR_CODES).toEqual([
      "environmentUnavailable",
      "stalePreview",
      "validationFailed",
      "notInitialized",
      "operationBlocked",
      "notFound",
      "operationFailed",
    ]);
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
    expect(CONSISTENCY_REPAIR_ACTIONS).toEqual([
      "remove_missing_registry_record",
      "register_unregistered_content",
      "delete_unregistered_content",
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

  it("binds backup deletion to an explicit preview without adding a Restore contract", () => {
    const preview = {
      previewId: "backup-delete-1",
      entryId: "local:one",
      backupId: "one",
      class: "local",
      backupPath: "~/.my-agent-assets/backups/local/one",
      sizeBytes: 120,
      entryCount: 1,
      sensitiveConfigRisk: false,
      plannedEffects: ["permanently delete backup directory"],
      warnings: ["high risk"],
      canApply: true,
      generatedAtEpochSeconds: 100,
      expiresAtEpochSeconds: 700,
    } satisfies BackupDeletePreview;

    expect(preview.previewId).toContain("backup-delete");
    expectTypeOf(preview).toMatchTypeOf<BackupDeletePreview>();
  });
});
