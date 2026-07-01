import { AlertTriangle, Blocks, BookOpen, FolderKanban, Link2, TerminalSquare } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  canonicalMountApply,
  canonicalMountPreview,
  listAssets,
  listMountTargets,
} from "../app/data-api";
import type {
  ApplyResult,
  AssetSummary,
  CanonicalMountPreview,
  CanonicalMountPreviewRequest,
  RegisteredMountTarget,
} from "../app/contracts";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

type AssetOption = {
  id: string;
  assetId: string;
  type: "Skill" | "Command" | "MCP";
  detail: string;
  icon: typeof BookOpen;
};

type TargetOption = {
  id: string;
  detail: string;
  status: "ready" | "blocked" | "invalid";
};

const demoAssets: readonly AssetOption[] = [
  { id: "review", assetId: "skill:review", type: "Skill", detail: "代码审查工作流", icon: BookOpen },
  { id: "deploy-prod", assetId: "command:deploy-prod", type: "Command", detail: "生产环境部署", icon: TerminalSquare },
  { id: "PostgreSQL", assetId: "mcp:PostgreSQL", type: "MCP", detail: "数据库访问", icon: Blocks },
];
const demoTargets: readonly TargetOption[] = [
  { id: "project-a", detail: "~/workspace/project-a", status: "ready" },
  { id: "my-app", detail: "~/workspace/my-app", status: "ready" },
  { id: "claude-user-skills", detail: "用户级 Claude Runtime", status: "ready" },
];

export function MountManagerPage({ demoMode = false }: { demoMode?: boolean }) {
  const [assets, setAssets] = useState<readonly AssetOption[]>(demoMode ? demoAssets : []);
  const [targets, setTargets] = useState<readonly TargetOption[]>(demoMode ? demoTargets : []);
  const [selectedAsset, setSelectedAsset] = useState(demoMode ? demoAssets[0].id : "");
  const [selectedTarget, setSelectedTarget] = useState(demoMode ? demoTargets[0].id : "");
  const [preview, setPreview] = useState<CanonicalMountPreview | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [previewState, setPreviewState] = useState("预览中");
  const [isApplying, setIsApplying] = useState(false);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const asset = assets.find((item) => item.id === selectedAsset) ?? assets[0];
  const target = targets.find((item) => item.id === selectedTarget) ?? targets[0];
  const previewInput = useMemo(
    () => asset && target ? toPreviewMountInput(asset, target) : null,
    [asset, target],
  );

  useEffect(() => {
    if (demoMode) {
      setAssets(demoAssets);
      setTargets(demoTargets);
      setSelectedAsset(demoAssets[0].id);
      setSelectedTarget(demoTargets[0].id);
      return undefined;
    }
    let cancelled = false;
    Promise.all([listAssets({ assetType: null }), listMountTargets()])
      .then(([loadedAssets, loadedTargets]) => {
        if (cancelled) return;
        const nextAssets = loadedAssets.map(toAssetOption);
        const nextTargets = loadedTargets.map(toTargetOption);
        setAssets(nextAssets);
        setTargets(nextTargets);
        setSelectedAsset(nextAssets[0]?.id ?? "");
        setSelectedTarget(nextTargets[0]?.id ?? "");
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
  }, [demoMode]);

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

  return (
    <div className="operation-workspace">
      <section className="panel mount-workflow">
        <div className="mount-flow-column"><div className="mount-flow-heading"><span>1</span><div><strong>选择资产</strong><small>{demoMode ? "Visual QA 示例数据" : "Claude 资产中心真实数据"}</small></div></div><div className="selectable-stack">{assets.map(({ id, type, detail, icon: Icon }) => <button aria-pressed={selectedAsset === id} className={selectedAsset === id ? "selected" : ""} data-no-drag="true" key={id} onClick={() => { setSelectedAsset(id); setApplyResult(null); }} style={NO_DRAG_REGION_STYLE} type="button"><Icon size={16} /><span><strong>{id}</strong><small>{type} · {detail}</small></span></button>)}{assets.length === 0 && <div className="asset-empty-state"><BookOpen size={20} /><strong>暂无可挂载资产</strong><span>请先扫描并导入 Claude 资产。</span></div>}</div></div>
        <div className="mount-flow-column"><div className="mount-flow-heading"><span>2</span><div><strong>选择目标</strong><small>本地运行目标</small></div></div><div className="selectable-stack">{targets.map(({ id, detail, status }) => <button aria-pressed={selectedTarget === id} className={selectedTarget === id ? "selected" : ""} data-no-drag="true" key={id} onClick={() => { setSelectedTarget(id); setApplyResult(null); }} style={NO_DRAG_REGION_STYLE} type="button"><FolderKanban size={16} /><span><strong>{id}</strong><small>{detail} · {status === "ready" ? "可用" : "阻止"}</small></span></button>)}</div></div>
        <div className="mount-flow-column plan"><div className="mount-flow-heading"><span>3</span><div><strong>预览挂载计划</strong><small>{previewState} · 不会执行文件变更</small></div></div>{asset && target ? <><div className="mount-plan-summary"><div><Link2 size={17} /><span><strong>{asset.id}</strong><small>{asset.type}</small></span></div><i>→</i><div><FolderKanban size={17} /><span><strong>{target.id}</strong><small>{preview?.affectedTargetPath ?? target.detail}</small></span></div></div><div className="plan-lines">{planLines.map((line) => <span key={line}>{line}</span>)}</div></> : <div className="asset-empty-state"><Link2 size={20} /><strong>无法生成挂载计划</strong><span>选择真实资产和运行目标后再预览。</span></div>}</div>
      </section>
      <section className="panel mount-review-bar"><div className="operation-warning"><AlertTriangle size={17} /><div><strong>{preview?.backupRequired ?? true ? "执行前将创建本地备份" : "无需备份"}</strong><span>{preview?.unsupportedReason ?? planSummary}</span></div></div><div className="operation-actions"><button className="asset-secondary-action" data-no-drag="true" disabled={!previewInput} onClick={() => setRefreshKey((current) => current + 1)} style={NO_DRAG_REGION_STYLE} type="button">刷新挂载计划</button></div><ApplyConfirmationPanel actionLabel="确认挂载" canApply={canApply} description="会创建软链接、目录 junction 或精确编译 MCP runtime 配置；后端只接受已授权 targetId。" isApplying={isApplying} onApply={handleApplyMount} operationError={operationError} result={applyResult} title="执行挂载" /></section>
    </div>
  );
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "无法调用挂载操作。";
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
    type,
    detail: asset.description || asset.sourcePath,
    icon: asset.assetType === "command" ? TerminalSquare : asset.assetType === "mcp" ? Blocks : BookOpen,
  };
}

function toTargetOption(target: RegisteredMountTarget): TargetOption {
  return { id: target.id, detail: target.path, status: target.status };
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
