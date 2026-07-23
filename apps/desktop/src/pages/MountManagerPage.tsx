import { AlertTriangle, Blocks, BookOpen, FolderKanban, Link2, TerminalSquare } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  canonicalMountApply,
  canonicalMountPreview,
  canonicalUnmountApply,
  canonicalUnmountPreview,
  listAssets,
  listMountBindings,
  listMountTargets,
} from "../app/data-api";
import type {
  ApplyResult,
  AssetSummary,
  CanonicalMountPreview,
  CanonicalMountPreviewRequest,
  CanonicalUnmountPreview,
  MountBindingSummary,
  RegisteredMountTarget,
} from "../app/contracts";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

type AssetOption = {
  id: string;
  assetId: string;
  assetType: AssetSummary["assetType"];
  type: "Skill" | "Command" | "MCP";
  detail: string;
  icon: typeof BookOpen;
  mountTargets: readonly string[];
};

type TargetOption = {
  id: string;
  detail: string;
  provider: RegisteredMountTarget["provider"];
  accepts: readonly AssetSummary["assetType"][];
  scope: RegisteredMountTarget["scope"];
  status: "ready" | "blocked" | "invalid";
};

type MountedEntry = {
  asset: AssetOption;
  target: TargetOption;
  runtimePath: string;
  status: MountBindingSummary["status"];
};

const demoAssets: readonly AssetOption[] = [
  { id: "review", assetId: "skill:review", assetType: "skill", type: "Skill", detail: "代码审查工作流", icon: BookOpen, mountTargets: ["~/workspace/project-a/.claude/skills/review"] },
  { id: "deploy-prod", assetId: "command:deploy-prod", assetType: "command", type: "Command", detail: "生产环境部署", icon: TerminalSquare, mountTargets: [] },
  { id: "PostgreSQL", assetId: "mcp:PostgreSQL", assetType: "mcp", type: "MCP", detail: "数据库访问", icon: Blocks, mountTargets: [] },
];
const demoTargets: readonly TargetOption[] = [
  { id: "project-a", detail: "~/workspace/project-a/.claude/skills", provider: "claude_code", accepts: ["skill", "command", "mcp"], scope: "project", status: "ready" },
  { id: "my-app", detail: "~/workspace/my-app/.claude/skills", provider: "claude_code", accepts: ["skill", "command", "mcp"], scope: "project", status: "ready" },
  { id: "claude-user-skills", detail: "用户级 Claude Runtime", provider: "claude_code", accepts: ["skill"], scope: "user", status: "ready" },
];
const demoBindings: readonly MountBindingSummary[] = [
  {
    assetId: "skill:review",
    targetId: "project-a",
    status: "mounted",
    targetPath: "~/workspace/project-a/.claude/skills/review",
    provider: "claude_code",
    scope: "project",
  },
];

export function MountManagerPage({ demoMode = false }: { demoMode?: boolean }) {
  const [assets, setAssets] = useState<readonly AssetOption[]>(demoMode ? demoAssets : []);
  const [targets, setTargets] = useState<readonly TargetOption[]>(demoMode ? demoTargets : []);
  const [bindings, setBindings] = useState<readonly MountBindingSummary[]>(demoMode ? demoBindings : []);
  const [view, setView] = useState<"new" | "current">("new");
  const [showAdvancedTargets, setShowAdvancedTargets] = useState(false);
  const [selectedAsset, setSelectedAsset] = useState(demoMode ? demoAssets[0].id : "");
  const [selectedTarget, setSelectedTarget] = useState(demoMode ? demoTargets[0].id : "");
  const [preview, setPreview] = useState<CanonicalMountPreview | null>(null);
  const [selectedMountedEntry, setSelectedMountedEntry] = useState<MountedEntry | null>(null);
  const [unmountPreview, setUnmountPreview] = useState<CanonicalUnmountPreview | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [previewState, setPreviewState] = useState("预览中");
  const [isApplying, setIsApplying] = useState(false);
  const [isUnmounting, setIsUnmounting] = useState(false);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const asset = assets.find((item) => item.id === selectedAsset) ?? assets[0];
  const visibleTargets = useMemo(
    () => targets.filter((item) => showAdvancedTargets || item.scope !== "custom"),
    [showAdvancedTargets, targets],
  );
  const compatibleTargets = useMemo(
    () => asset ? visibleTargets.filter((item) => item.accepts.includes(asset.assetType)) : [],
    [asset, visibleTargets],
  );
  const target = compatibleTargets.find((item) => item.id === selectedTarget) ?? compatibleTargets[0];
  const mountedEntries = useMemo(
    () => deriveMountedEntries(assets, targets, bindings),
    [assets, bindings, targets],
  );
  const previewInput = useMemo(
    () => asset && target ? toPreviewMountInput(asset, target) : null,
    [asset, target],
  );

  useEffect(() => {
    if (demoMode) {
      setAssets(demoAssets);
      setTargets(demoTargets);
      setBindings(demoBindings);
      setSelectedAsset(demoAssets[0].id);
      setSelectedTarget(demoTargets[0].id);
      return undefined;
    }
    let cancelled = false;
    Promise.all([listAssets({ assetType: null }), listMountTargets(), listMountBindings()])
      .then(([loadedAssets, loadedTargets, loadedBindings]) => {
        if (cancelled) return;
        const nextAssets = loadedAssets.map(toAssetOption);
        const nextTargets = loadedTargets.map(toTargetOption);
        setAssets(nextAssets);
        setTargets(nextTargets);
        setBindings(loadedBindings);
        setSelectedAsset(nextAssets[0]?.id ?? "");
        setSelectedTarget(firstCompatibleTargetId(nextAssets[0], nextTargets));
      })
      .catch((error) => {
        if (cancelled) return;
        setAssets([]);
        setTargets([]);
        setOperationError(errorMessage(error));
        setPreviewState("读取真实数据失败");
      });
    return () => {
      cancelled = true;
    };
  }, [demoMode, refreshKey]);

  useEffect(() => {
    if (!asset) return;
    if (!compatibleTargets.some((item) => item.id === selectedTarget)) {
      setSelectedTarget(compatibleTargets[0]?.id ?? "");
    }
  }, [asset, compatibleTargets, selectedTarget]);

  useEffect(() => {
    let cancelled = false;
    if (!previewInput) {
      setPreview(null);
      setPreviewState("等待真实资产和目标");
      return undefined;
    }
    setPreviewState("预览中");
    setOperationError(null);
    canonicalMountPreview(previewInput)
      .then((result) => {
        if (cancelled) return;
        if (result) {
          setPreview(result);
          setPreviewState("预览数据");
        } else {
          setPreview(null);
          setPreviewState("未返回挂载预览");
        }
      })
      .catch((error) => {
        if (cancelled) return;
        setPreview(null);
        setOperationError(errorMessage(error));
        setPreviewState("挂载预览读取失败");
      });
    return () => {
      cancelled = true;
    };
  }, [previewInput, refreshKey]);

  const planLines = preview?.plannedEffects ?? [];
  const warning = preview?.warnings[0] ?? "尚未生成真实挂载预览。";
  const planSummary = warning;
  const canApply = Boolean(preview?.canApply && preview?.previewId);

  const handleApplyMount = async () => {
    if (!canApply || !preview?.previewId || !previewInput) return;

    setIsApplying(true);
    setOperationError(null);
    setPreviewState("执行挂载中");
    try {
      const result = await canonicalMountApply({
        previewId: preview.previewId,
        previewGeneratedAtEpochSeconds: preview.generatedAtEpochSeconds,
        request: previewInput,
      });
      setApplyResult(toApplyResult(result));
      setPreviewState("挂载已执行");
      setRefreshKey((current) => current + 1);
    } catch (error) {
      setApplyResult(null);
      setOperationError(errorMessage(error));
      setPreviewState("挂载失败");
    } finally {
      setIsApplying(false);
    }
  };

  const requestUnmount = async (entry: MountedEntry) => {
    setSelectedMountedEntry(entry);
    setUnmountPreview(null);
    setOperationError(null);
    try {
      const nextPreview = await canonicalUnmountPreview({
        assetId: entry.asset.assetId,
        targetId: entry.target.id,
      });
      setUnmountPreview(nextPreview);
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  };

  const applyUnmount = async () => {
    if (!selectedMountedEntry || !unmountPreview?.canApply) return;
    setIsUnmounting(true);
    setOperationError(null);
    try {
      const result = await canonicalUnmountApply({
        previewId: unmountPreview.previewId,
        previewGeneratedAtEpochSeconds: unmountPreview.generatedAtEpochSeconds,
        request: {
          assetId: selectedMountedEntry.asset.assetId,
          targetId: selectedMountedEntry.target.id,
        },
      });
      setApplyResult({
        mode: "apply",
        ok: result.unmounted,
        previewId: result.previewId,
        backup: null,
        steps: [{
          stepId: `unmount:${selectedMountedEntry.asset.assetId}:${selectedMountedEntry.target.id}`,
          kind: "mount",
          label: "解除挂载",
          status: result.unmounted ? "success" : "failed",
          message: result.unmounted ? "已移除受管理的运行时挂载。" : "解除挂载未完成。",
          affectedPaths: result.affectedPaths,
        }],
        warnings: [],
        errors: result.unmounted ? [] : ["解除挂载未完成。"],
      });
      setRefreshKey((current) => current + 1);
    } catch (error) {
      setOperationError(errorMessage(error));
    } finally {
      setIsUnmounting(false);
    }
  };

  return (
    <div className="operation-workspace">
      <nav className="view-switch" aria-label="挂载视图">
        <button aria-pressed={view === "new"} className={view === "new" ? "active" : ""} data-no-drag="true" onClick={() => setView("new")} style={NO_DRAG_REGION_STYLE} type="button">新建挂载</button>
        <button aria-pressed={view === "current"} className={view === "current" ? "active" : ""} data-no-drag="true" onClick={() => setView("current")} style={NO_DRAG_REGION_STYLE} type="button">当前挂载</button>
      </nav>
      {view === "new" ? <>
        <section className="panel mount-workflow">
          <div className="mount-flow-column"><div className="mount-flow-heading"><span>1</span><div><strong>选择资产</strong><small>{demoMode ? "Visual QA 示例数据" : "资产中心真实数据"}</small></div></div><div className="selectable-stack">{assets.map(({ id, type, detail, icon: Icon }) => <button aria-pressed={selectedAsset === id} className={selectedAsset === id ? "selected" : ""} data-no-drag="true" key={id} onClick={() => { setSelectedAsset(id); setApplyResult(null); }} style={NO_DRAG_REGION_STYLE} type="button"><Icon size={16} /><span><strong>{id}</strong><small>{type} · {detail}</small></span></button>)}{assets.length === 0 && <div className="asset-empty-state"><BookOpen size={20} /><strong>暂无可挂载资产</strong><span>请先扫描并导入资产。</span></div>}</div></div>
          <div className="mount-flow-column"><div className="mount-flow-heading"><span>2</span><div><strong>选择目标</strong><small>仅显示当前资产兼容的 Claude Code / Codex 目标</small></div></div><label className="advanced-target-toggle"><input checked={showAdvancedTargets} data-no-drag="true" onChange={(event) => setShowAdvancedTargets(event.target.checked)} style={NO_DRAG_REGION_STYLE} type="checkbox" />显示高级自定义目标</label><div className="selectable-stack">{compatibleTargets.map(({ id, detail, status, provider }) => <button aria-pressed={selectedTarget === id} className={selectedTarget === id ? "selected" : ""} data-no-drag="true" key={id} onClick={() => { setSelectedTarget(id); setApplyResult(null); }} style={NO_DRAG_REGION_STYLE} type="button"><FolderKanban size={16} /><span><strong>{id}</strong><small>{providerLabel(provider)} · {detail} · {status === "ready" ? "可用" : "阻止"}</small></span></button>)}{compatibleTargets.length === 0 ? <div className="asset-empty-state"><FolderKanban size={20} /><strong>没有兼容目标</strong><span>此资产类型目前没有可用运行目标。</span></div> : null}</div></div>
          <div className="mount-flow-column plan"><div className="mount-flow-heading"><span>3</span><div><strong>预览挂载计划</strong><small>{previewState} · 不会执行文件变更</small></div></div>{asset && target ? <><div className="mount-plan-summary"><div><Link2 size={17} /><span><strong>{asset.id}</strong><small>{asset.type}</small></span></div><i>→</i><div><FolderKanban size={17} /><span><strong>{target.id}</strong><small>{preview?.affectedTargetPath ?? target.detail}</small></span></div></div><div className="plan-lines">{planLines.map((line) => <span key={line}>{line}</span>)}</div></> : <div className="asset-empty-state"><Link2 size={20} /><strong>无法生成挂载计划</strong><span>选择真实资产和兼容目标后再预览。</span></div>}</div>
        </section>
        <section className="panel mount-review-bar"><div className="operation-warning"><AlertTriangle size={17} /><div><strong>{preview?.backupRequired ?? true ? "执行前将创建本地备份" : "无需备份"}</strong><span>{preview?.unsupportedReason ?? planSummary}</span></div></div><div className="operation-actions"><button className="asset-secondary-action" data-no-drag="true" disabled={!previewInput} onClick={() => setRefreshKey((current) => current + 1)} style={NO_DRAG_REGION_STYLE} type="button">刷新挂载计划</button></div><ApplyConfirmationPanel actionLabel="确认挂载" canApply={canApply} description="会创建软链接、目录 junction 或精确编译 MCP runtime 配置；后端只接受已授权 targetId。" isApplying={isApplying} onApply={handleApplyMount} operationError={operationError} result={applyResult} title="执行挂载" /></section>
      </> : <section className="panel current-mounts-panel">
        <div className="section-heading"><div><h3>当前挂载</h3><p>显示由资产中心登记的运行时挂载；解除前会生成预览。</p></div><span className="preview-label">{mountedEntries.length} 项</span></div>
        <div className="reference-list">{mountedEntries.map((entry) => <div key={`${entry.asset.assetId}:${entry.target.id}`}><Link2 size={16} /><span><strong>{entry.asset.id}</strong><small>{entry.asset.type} · {providerLabel(entry.target.provider)} · {entry.runtimePath}</small></span><span className={`asset-status ${bindingStatusTone(entry.status)}`}>{bindingStatusLabel(entry.status)}</span><button className="asset-secondary-action" data-no-drag="true" onClick={() => void requestUnmount(entry)} style={NO_DRAG_REGION_STYLE} type="button">预览解除</button></div>)}{mountedEntries.length === 0 ? <div className="asset-empty-state"><Link2 size={20} /><strong>暂无当前挂载</strong><span>创建挂载后会在这里显示对应的运行时目标。</span></div> : null}</div>
        {selectedMountedEntry && unmountPreview ? <ApplyConfirmationPanel actionLabel="确认解除挂载" canApply={unmountPreview.canApply} description={`仅删除受管理的挂载或 MCP 条目：${unmountPreview.affectedTargetPath}`} isApplying={isUnmounting} onApply={applyUnmount} operationError={operationError} result={applyResult} title={`解除 ${selectedMountedEntry.asset.id} 的挂载`} /> : null}
      </section>}
    </div>
  );
}

function errorMessage(_error: unknown) {
  return "挂载操作未完成。请查看系统状态或导出诊断包后重试。";
}

function toPreviewMountInput(asset: AssetOption, target: TargetOption): CanonicalMountPreviewRequest {
  return {
    assetId: asset.assetId,
    targetId: target.id,
  };
}

function toAssetOption(asset: AssetSummary): AssetOption {
  const type = asset.assetType === "command" ? "Command" : asset.assetType === "mcp" ? "MCP" : "Skill";
  return {
    id: asset.name,
    assetId: asset.id,
    assetType: asset.assetType,
    type,
    detail: asset.description || asset.sourcePath,
    icon: asset.assetType === "command" ? TerminalSquare : asset.assetType === "mcp" ? Blocks : BookOpen,
    mountTargets: asset.mountTargets,
  };
}

function toTargetOption(target: RegisteredMountTarget): TargetOption {
  return {
    id: target.id,
    detail: target.path,
    provider: target.provider,
    accepts: target.accepts,
    scope: target.scope,
    status: target.status,
  };
}

function firstCompatibleTargetId(asset: AssetOption | undefined, targets: readonly TargetOption[]) {
  if (!asset) return "";
  return targets.find((target) => target.scope !== "custom" && target.accepts.includes(asset.assetType))?.id ?? "";
}

function deriveMountedEntries(
  assets: readonly AssetOption[],
  targets: readonly TargetOption[],
  bindings: readonly MountBindingSummary[],
): MountedEntry[] {
  return bindings.flatMap((binding) => {
    const asset = assets.find((candidate) => candidate.assetId === binding.assetId);
    const target = targets.find((candidate) => candidate.id === binding.targetId);
    return asset && target
      ? [{ asset, target, runtimePath: binding.targetPath ?? target.detail, status: binding.status }]
      : [];
  });
}

function bindingStatusLabel(status: MountBindingSummary["status"]) {
  if (status === "mounted") return "已挂载";
  if (status === "out_of_sync") return "待同步";
  return "孤立记录";
}

function bindingStatusTone(status: MountBindingSummary["status"]) {
  if (status === "mounted") return "success";
  if (status === "out_of_sync") return "warning";
  return "danger";
}

function providerLabel(provider: RegisteredMountTarget["provider"]) {
  if (provider === "claude_code") return "Claude Code";
  if (provider === "codex") return "Codex";
  return "高级自定义";
}

function toApplyResult(result: Awaited<ReturnType<typeof canonicalMountApply>>): ApplyResult {
  return {
    mode: "apply",
    ok: result.mounted,
    previewId: result.previewId,
    backup: null,
    steps: [{
      stepId: "canonical-mount",
      kind: "mount",
      label: "挂载资产",
      status: result.mounted ? "success" : "failed",
      message: result.mounted ? "目标已更新并登记本机挂载关系。" : "挂载未完成。",
      affectedPaths: result.affectedPaths,
    }],
    warnings: result.warnings,
    errors: result.mounted ? [] : ["挂载未完成。"],
  };
}
