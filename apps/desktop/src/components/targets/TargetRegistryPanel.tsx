import { FolderCog, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import {
  listMountTargets,
  targetRegistrationApply,
  targetRegistrationPreview,
  targetRemovalApply,
  targetRemovalPreview,
} from "../../app/data-api";
import type {
  ApplyResult,
  MountTargetKind,
  RegisteredMountTarget,
  TargetChangePreview,
  TargetRegistrationPreviewRequest,
  TargetRemovalPreviewRequest,
} from "../../app/contracts";
import { NO_DRAG_REGION_STYLE } from "../../lib/platform";
import { ApplyConfirmationPanel } from "../ui/ApplyConfirmationPanel";

const TARGET_KINDS: readonly { value: MountTargetKind; label: string; project: boolean }[] = [
  { value: "claude_project_skills", label: "Claude 项目 Skills", project: true },
  { value: "codex_project_skills", label: "Codex 项目 Skills", project: true },
  { value: "claude_project_commands", label: "Claude 项目 Commands", project: true },
  { value: "claude_project_mcp_json", label: "Claude 项目 MCP", project: true },
  { value: "codex_project_mcp_toml", label: "Codex 项目 MCP", project: true },
  { value: "custom_skill_directory", label: "自定义 Skill 目录", project: false },
  { value: "custom_command_directory", label: "Claude-compatible Command 目录", project: false },
  { value: "custom_claude_mcp_json", label: "自定义 Claude MCP JSON", project: false },
  { value: "custom_codex_mcp_toml", label: "自定义 Codex MCP TOML", project: false },
];

type PendingChange =
  | { kind: "register"; request: TargetRegistrationPreviewRequest; preview: TargetChangePreview }
  | { kind: "remove"; request: TargetRemovalPreviewRequest; preview: TargetChangePreview };

export function TargetRegistryPanel() {
  const [targets, setTargets] = useState<RegisteredMountTarget[]>([]);
  const [targetId, setTargetId] = useState("");
  const [targetKind, setTargetKind] = useState<MountTargetKind>("claude_project_skills");
  const [location, setLocation] = useState("~/workspace/project-a");
  const [pending, setPending] = useState<PendingChange | null>(null);
  const [result, setResult] = useState<ApplyResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [isApplying, setIsApplying] = useState(false);

  const refreshTargets = async () => {
    setTargets(await listMountTargets());
  };

  useEffect(() => {
    refreshTargets().catch((loadError) => setError(errorMessage(loadError)));
  }, []);

  const previewRegistration = async () => {
    const request = { id: targetId.trim(), kind: targetKind, location: location.trim() };
    setIsPreviewing(true);
    setError(null);
    setResult(null);
    try {
      setPending({ kind: "register", request, preview: await targetRegistrationPreview(request) });
    } catch (previewError) {
      setPending(null);
      setError(errorMessage(previewError));
    } finally {
      setIsPreviewing(false);
    }
  };

  const previewRemoval = async (targetIdToRemove: string) => {
    const request = { targetId: targetIdToRemove };
    setIsPreviewing(true);
    setError(null);
    setResult(null);
    try {
      setPending({ kind: "remove", request, preview: await targetRemovalPreview(request) });
    } catch (previewError) {
      setPending(null);
      setError(errorMessage(previewError));
    } finally {
      setIsPreviewing(false);
    }
  };

  const applyChange = async () => {
    if (!pending?.preview.canApply) return;
    setIsApplying(true);
    setError(null);
    try {
      const applyResult = pending.kind === "register"
        ? await targetRegistrationApply({
          previewId: pending.preview.previewId,
          previewGeneratedAtEpochSeconds: pending.preview.generatedAtEpochSeconds,
          request: pending.request,
        })
        : await targetRemovalApply({
          previewId: pending.preview.previewId,
          previewGeneratedAtEpochSeconds: pending.preview.generatedAtEpochSeconds,
          request: pending.request,
        });
      setResult(toApplyResult(applyResult, pending.kind));
      setPending(null);
      if (applyResult.operation === "add") {
        setTargetId("");
      }
      await refreshTargets();
    } catch (applyError) {
      setResult(null);
      setError(errorMessage(applyError));
    } finally {
      setIsApplying(false);
    }
  };

  const selectedKind = TARGET_KINDS.find((item) => item.value === targetKind) ?? TARGET_KINDS[0];

  return (
    <div className="target-registry-settings">
      <div className="section-heading">
        <div>
          <h4>运行目标注册</h4>
          <p>授权项目或自定义路径后，挂载操作只使用 targetId。</p>
        </div>
        <FolderCog size={16} />
      </div>
      <div className="settings-controls">
        <label>
          <span>目标 ID</span>
          <input
            data-no-drag="true"
            onChange={(event) => setTargetId(event.target.value)}
            placeholder="project-a-claude-skills"
            style={NO_DRAG_REGION_STYLE}
            value={targetId}
          />
        </label>
        <label>
          <span>目标类型</span>
          <select
            data-no-drag="true"
            onChange={(event) => {
              const kind = event.target.value as MountTargetKind;
              setTargetKind(kind);
              const option = TARGET_KINDS.find((item) => item.value === kind);
              setLocation(option?.project ? "~/workspace/project-a" : "~/custom/assets");
              setPending(null);
            }}
            style={NO_DRAG_REGION_STYLE}
            value={targetKind}
          >
            {TARGET_KINDS.map((kind) => <option key={kind.value} value={kind.value}>{kind.label}</option>)}
          </select>
        </label>
        <label>
          <span>{selectedKind.project ? "项目根目录" : "目标路径"}</span>
          <input
            data-no-drag="true"
            onChange={(event) => setLocation(event.target.value)}
            style={NO_DRAG_REGION_STYLE}
            value={location}
          />
        </label>
      </div>
      <div className="settings-actions">
        <button
          className="asset-secondary-action"
          data-no-drag="true"
          disabled={isPreviewing || !targetId.trim() || !location.trim()}
          onClick={previewRegistration}
          style={NO_DRAG_REGION_STYLE}
          type="button"
        >
          {isPreviewing ? "生成中" : "预览注册"}
        </button>
      </div>
      <div className="reference-list">
        {targets.map((target) => (
          <div key={target.id}>
            <FolderCog size={15} />
            <span>{target.id}</span>
            <small>{target.path} · {target.status}</small>
            <button
              aria-label={`移除目标 ${target.id}`}
              className="icon-button"
              data-no-drag="true"
              disabled={target.scope === "user" || isPreviewing}
              onClick={() => previewRemoval(target.id)}
              style={NO_DRAG_REGION_STYLE}
              title={target.scope === "user" ? "内置用户级目标不可在此移除" : "预览移除目标"}
              type="button"
            >
              <Trash2 size={14} />
            </button>
          </div>
        ))}
        {targets.length === 0 ? <p className="muted-text">暂无已授权运行目标。</p> : null}
      </div>
      {pending ? (
        <ApplyConfirmationPanel
          actionLabel={pending.kind === "register" ? "确认注册目标" : "确认移除目标"}
          canApply={pending.preview.canApply}
          description={`${pending.preview.affectedPaths.join("；")}。${pending.preview.warnings.join(" ")}`}
          isApplying={isApplying}
          onApply={applyChange}
          operationError={error}
          result={result}
          title={pending.kind === "register" ? "注册已授权运行目标" : "移除运行目标"}
        />
      ) : error || result ? (
        <p className={error ? "warning-text" : "success-text"} role="status">
          {error ?? result?.steps[0]?.message}
        </p>
      ) : null}
    </div>
  );
}

function toApplyResult(
  result: Awaited<ReturnType<typeof targetRegistrationApply>>,
  kind: PendingChange["kind"],
): ApplyResult {
  return {
    mode: "apply",
    ok: true,
    previewId: result.previewId,
    backup: null,
    steps: [{
      stepId: `target-${result.operation}`,
      kind: "settings",
      label: kind === "register" ? "注册目标" : "移除目标",
      status: "success",
      message: kind === "register" ? "运行目标已注册。" : "运行目标已移除。",
      affectedPaths: [result.registryPath, result.backupPath],
    }],
    warnings: [],
    errors: [],
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "运行目标操作失败。";
}
