import { describe, expect, expectTypeOf, it } from "vitest";
import {
  APPEARANCE_THEMES,
  APPLY_MODES,
  APPLY_STEP_STATUSES,
  ASSET_STATUSES,
  ASSET_TYPES,
  CONFLICT_RESOLUTIONS,
  DENSITY_PREFERENCES,
  LOG_LEVELS,
  PLAN_STEP_KINDS,
  PROJECT_STATUSES,
  RISK_LEVELS,
  RUNTIME_SCOPES,
  type PreviewImportInput,
  type ScanScope,
  type GitStatus,
  type ImportApplyInput,
  type ApplyResult,
} from "./contracts";

describe("Tauri command contracts", () => {
  it("locks every enum wire value", () => {
    expect(ASSET_TYPES).toEqual(["skill", "command", "mcp"]);
    expect(ASSET_STATUSES).toEqual(["ready", "mounted", "unmounted", "conflict", "invalid"]);
    expect(PROJECT_STATUSES).toEqual(["ready", "changed", "needsSync", "invalid"]);
    expect(RUNTIME_SCOPES).toEqual(["user", "local", "project"]);
    expect(CONFLICT_RESOLUTIONS).toEqual(["skip", "rename", "overwrite"]);
    expect(PLAN_STEP_KINDS).toEqual(["check", "import", "mount", "compileMcp", "backup", "restore", "git", "settings"]);
    expect(RISK_LEVELS).toEqual(["none", "low", "medium", "high"]);
    expect(APPEARANCE_THEMES).toEqual(["system", "light", "dark"]);
    expect(DENSITY_PREFERENCES).toEqual(["compact", "comfortable"]);
    expect(LOG_LEVELS).toEqual(["error", "warn", "info", "debug"]);
    expect(APPLY_MODES).toEqual(["planOnly", "apply"]);
    expect(APPLY_STEP_STATUSES).toEqual(["pending", "skipped", "success", "failed"]);
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
      remote: null,
      clean: true,
      ahead: 0,
      behind: 0,
      changedFiles: [],
      conflicts: [],
      lastSyncedAt: null,
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
});
