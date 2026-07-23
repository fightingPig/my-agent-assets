import { FolderCog, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
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

const TARGET_KINDS: readonly { value: MountTargetKind; label: string; picker: "directory" | "json" | "toml" }[] = [
  { value: "custom_skill_directory", label: "自定义 Skill 目录", picker: "directory" },
  { value: "custom_command_directory", label: "Claude-compatible Command 目录", picker: "directory" },
  { value: "custom_claude_mcp_json", label: "自定义 Claude MCP JSON", picker: "json" },
  { value: "custom_codex_mcp_toml", label: "自定义 Codex MCP TOML", picker: "toml" },
];

type PendingChange =
  | { kind: "register"; request: TargetRegistrationPreviewRequest; preview: TargetChangePreview }
  | { kind: "remove"; request: TargetRemovalPreviewRequest; preview: TargetChangePreview };

export function TargetRegistryPanel() {
  const [targets, setTargets] = useState<RegisteredMountTarget[]>([]);
  const [targetKind, setTargetKind] = useState<MountTargetKind>("custom_skill_directory");
  const [location, setLocation] = useState("");
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

  const chooseLocation = async () => {
    const kind = TARGET_KINDS.find((item) => item.value === targetKind) ?? TARGET_KINDS[0];
    const selected = await open({
      directory: kind.picker === "directory",
      multiple: false,
      title: `选择${kind.label}`,
      filters: kind.picker === "json"
        ? [{ name: "JSON", extensions: ["json"] }]
        : kind.picker === "toml"
          ? [{ name: "TOML", extensions: ["toml"] }]
          : undefined,
    });
    if (typeof selected === "string") setLocation(selected);
  };

  const previewRegistration = async () => {
    const request = { id: targetIdFor(targetKind, location), kind: targetKind, location };
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
      if (applyResult.operation === "add") setLocation("");
      await refreshTargets();
    } catch (applyError) {
      setResult(null);
      setError(errorMessage(applyError));
    } finally {
      setIsApplying(false);
    }
  };

  const selectedKind = TARGET_KINDS.find((item) => item.value === targetKind) ?? TARGET_KINDS[0];
  const customTargets = targets.filter((target) => target.scope === "custom");

  return (
    <div className="target-registry-settings">
      <div className="section-heading">
        <div>
          <h4>高级自定义 Target</h4>
          <p>标准用户级和维护项目目标由系统派生。这里只登记额外目录或配置文件。</p>
        </div>
        <FolderCog size={16} />
      </div>
      <div className="settings-controls">
        <label>
          <span>目标类型</span>
          <select
            data-no-drag="true"
            onChange={(event) => {
              const kind = event.target.value as MountTargetKind;
              setTargetKind(kind);
              setLocation("");
              setPending(null);
            }}
            style={NO_DRAG_REGION_STYLE}
            value={targetKind}
          >
            {TARGET_KINDS.map((kind) => <option key={kind.value} value={kind.value}>{kind.label}</option>)}
          </select>
        </label>
        <label>
          <span>已选路径</span>
          <input data-no-drag="true" readOnly style={NO_DRAG_REGION_STYLE} value={location || "尚未选择"} />
        </label>
      </div>
      <div className="settings-actions">
        <button
          className="asset-secondary-action"
          data-no-drag="true"
          disabled={isPreviewing}
          onClick={() => void chooseLocation()}
          style={NO_DRAG_REGION_STYLE}
          type="button"
        >
          选择路径
        </button>
        <button
          className="asset-secondary-action"
          data-no-drag="true"
          disabled={isPreviewing || !location}
          onClick={() => void previewRegistration()}
          style={NO_DRAG_REGION_STYLE}
          type="button"
        >
          {isPreviewing ? "生成中" : "预览注册"}
        </button>
      </div>
      <div className="reference-list">
        {customTargets.map((target) => (
          <div key={target.id}>
            <FolderCog size={15} />
            <span>{target.id}</span>
            <small>{target.path} · {target.status}</small>
            <button
              aria-label={`移除目标 ${target.id}`}
              className="icon-button"
              data-no-drag="true"
              disabled={isPreviewing}
              onClick={() => previewRemoval(target.id)}
              style={NO_DRAG_REGION_STYLE}
              title="预览移除自定义 Target"
              type="button"
            >
              <Trash2 size={14} />
            </button>
          </div>
        ))}
        {customTargets.length === 0 ? <p className="muted-text">暂无高级自定义 Target。</p> : null}
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

function targetIdFor(kind: MountTargetKind, location: string) {
  const filename = location.split(/[\\/]/).filter(Boolean).at(-1) ?? "target";
  const safeName = filename
    .toLocaleLowerCase()
    .replace(/\.[^.]+$/, "")
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "") || "target";
  return `custom-${kind.replace(/^custom_/, "").replace(/_/g, "-")}-${safeName}`;
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

function errorMessage(_error: unknown) {
  return "运行目标操作未完成。请查看系统状态或导出诊断包后重试。";
}
