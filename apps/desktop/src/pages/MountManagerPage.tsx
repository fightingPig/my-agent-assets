import { AlertTriangle, Blocks, BookOpen, FolderKanban, Link2, TerminalSquare } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { listAssets, listProjects, mountApply, previewMount } from "../app/data-api";
import type { ApplyResult, AssetSummary, MountPreview, PreviewMountInput, ProjectSummary } from "../app/contracts";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { StaticActionButton } from "../components/ui/StaticActionButton";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

type AssetOption = {
  id: string;
  type: "Skill" | "Command" | "MCP";
  detail: string;
  icon: typeof BookOpen;
};

type TargetOption = {
  id: string;
  detail: string;
};

const demoAssets: readonly AssetOption[] = [
  { id: "review", type: "Skill", detail: "代码审查工作流", icon: BookOpen },
  { id: "deploy-prod", type: "Command", detail: "生产环境部署", icon: TerminalSquare },
  { id: "PostgreSQL", type: "MCP", detail: "数据库访问", icon: Blocks },
];
const demoTargets: readonly TargetOption[] = [
  { id: "project-a", detail: "~/workspace/project-a" },
  { id: "my-app", detail: "~/workspace/my-app" },
  { id: "user", detail: "用户级 Claude Runtime" },
];

export function MountManagerPage({ demoMode = false }: { demoMode?: boolean }) {
  const [assets, setAssets] = useState<readonly AssetOption[]>(demoMode ? demoAssets : []);
  const [targets, setTargets] = useState<readonly TargetOption[]>(demoMode ? demoTargets : []);
  const [selectedAsset, setSelectedAsset] = useState(demoMode ? demoAssets[0].id : "");
  const [selectedTarget, setSelectedTarget] = useState(demoMode ? demoTargets[0].id : "");
  const [preview, setPreview] = useState<MountPreview | null>(null);
  const [planResult, setPlanResult] = useState<ApplyResult | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [confirmationValue, setConfirmationValue] = useState("");
  const [previewState, setPreviewState] = useState("预览中");
  const [isPlanning, setIsPlanning] = useState(false);
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
    Promise.all([listAssets({ assetType: null }), listProjects()])
      .then(([loadedAssets, loadedProjects]) => {
        if (cancelled) return;
        const nextAssets = loadedAssets.map(toAssetOption);
        const nextTargets = [
          { id: "user", detail: "用户级 Claude Runtime" },
          ...loadedProjects.map(toTargetOption),
        ];
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
    setPlanResult(null);
    setOperationError(null);
    previewMount(previewInput)
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

  const planLines = preview?.steps.map((step) => step.label) ?? [];
  const warning = preview?.warnings[0] ?? "尚未生成真实挂载预览。";
  const planSummary = planResult?.steps.length
    ? planResult.steps.map((step) => step.message).join(" / ")
    : warning;
  const canApply = Boolean(planResult?.ok && preview?.previewId);

  const handlePlanMount = async () => {
    if (!preview?.previewId || !previewInput) return;

    setIsPlanning(true);
    setOperationError(null);
    setPreviewState("生成挂载计划中");
    try {
      const result = await mountApply({
        previewId: preview?.previewId ?? "",
        mode: "planOnly",
        assetId: previewInput.assetId,
        target: previewInput.target,
        backupBeforeApply: preview?.backupRequired ?? true,
      });
      setPlanResult(result);
      setPreviewState(result.ok ? "挂载计划已生成" : "挂载计划失败");
    } catch (error) {
      setPlanResult(null);
      setOperationError(errorMessage(error));
      setPreviewState("挂载计划失败");
    } finally {
      setIsPlanning(false);
    }
  };

  const handleApplyMount = async () => {
    if (!canApply || !preview?.previewId || !previewInput) return;

    setIsApplying(true);
    setOperationError(null);
    setPreviewState("执行挂载中");
    try {
      const result = await mountApply({
        previewId: preview.previewId,
        mode: "apply",
        assetId: previewInput.assetId,
        target: previewInput.target,
        backupBeforeApply: preview.backupRequired,
      });
      setApplyResult(result);
      setPreviewState(result.ok ? "挂载已执行" : "挂载失败");
      if (result.ok) {
        setConfirmationValue("");
        setRefreshKey((current) => current + 1);
      }
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
        <div className="mount-flow-column"><div className="mount-flow-heading"><span>1</span><div><strong>选择资产</strong><small>{demoMode ? "Visual QA 示例数据" : "Claude 资产中心真实数据"}</small></div></div><div className="selectable-stack">{assets.map(({ id, type, detail, icon: Icon }) => <button aria-pressed={selectedAsset === id} className={selectedAsset === id ? "selected" : ""} data-no-drag="true" key={id} onClick={() => { setSelectedAsset(id); setApplyResult(null); setConfirmationValue(""); }} style={NO_DRAG_REGION_STYLE} type="button"><Icon size={16} /><span><strong>{id}</strong><small>{type} · {detail}</small></span></button>)}{assets.length === 0 && <div className="asset-empty-state"><BookOpen size={20} /><strong>暂无可挂载资产</strong><span>请先扫描并导入 Claude 资产。</span></div>}</div></div>
        <div className="mount-flow-column"><div className="mount-flow-heading"><span>2</span><div><strong>选择目标</strong><small>本地运行目标</small></div></div><div className="selectable-stack">{targets.map(({ id, detail }) => <button aria-pressed={selectedTarget === id} className={selectedTarget === id ? "selected" : ""} data-no-drag="true" key={id} onClick={() => { setSelectedTarget(id); setApplyResult(null); setConfirmationValue(""); }} style={NO_DRAG_REGION_STYLE} type="button"><FolderKanban size={16} /><span><strong>{id === "user" ? "用户级" : id}</strong><small>{detail}</small></span></button>)}</div></div>
        <div className="mount-flow-column plan"><div className="mount-flow-heading"><span>3</span><div><strong>预览挂载计划</strong><small>{previewState} · 不会执行文件变更</small></div></div>{asset && target ? <><div className="mount-plan-summary"><div><Link2 size={17} /><span><strong>{asset.id}</strong><small>{asset.type}</small></span></div><i>→</i><div><FolderKanban size={17} /><span><strong>{target.id === "user" ? "用户级" : target.id}</strong><small>{preview?.target.runtimePath ?? target.detail}</small></span></div></div><div className="plan-lines">{planLines.map((line) => <span key={line}>{line}</span>)}</div></> : <div className="asset-empty-state"><Link2 size={20} /><strong>无法生成挂载计划</strong><span>选择真实资产和运行目标后再预览。</span></div>}</div>
      </section>
      <section className="panel mount-review-bar"><div className="operation-warning"><AlertTriangle size={17} /><div><strong>{preview?.backupRequired ?? true ? "执行前将创建本地备份" : "无需备份"}</strong><span>{planSummary}</span></div></div><div className="operation-actions"><StaticActionButton className="asset-secondary-action">导出计划</StaticActionButton><button className="asset-secondary-action" data-no-drag="true" disabled={isPlanning || !preview?.previewId} onClick={handlePlanMount} style={NO_DRAG_REGION_STYLE} type="button">{isPlanning ? "生成中" : "生成挂载计划"}</button></div><ApplyConfirmationPanel actionLabel="确认挂载" canApply={canApply} confirmationValue={confirmationValue} description="会创建软链接或编译 MCP runtime 配置；后端会校验 previewId 并在替换前创建备份。" isApplying={isApplying} onApply={handleApplyMount} onConfirmationChange={setConfirmationValue} operationError={operationError} result={applyResult} title="执行挂载" /></section>
    </div>
  );
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "无法调用挂载操作。";
}

function toPreviewMountInput(asset: AssetOption, target: TargetOption): PreviewMountInput {
  const assetType = asset.type === "Command" ? "command" : asset.type === "MCP" ? "mcp" : "skill";
  const runtimePath = target.id === "user"
    ? `~/.claude/${assetType === "command" ? "commands" : assetType === "mcp" ? "mcp" : "skills"}/${asset.id}${assetType === "command" ? ".md" : ""}`
    : `${target.detail}/.claude/${assetType === "command" ? "commands" : assetType === "mcp" ? "mcp" : "skills"}/${asset.id}${assetType === "command" ? ".md" : ""}`;

  return {
    assetId: `${assetType}:${asset.id}`,
    target: {
      scope: target.id === "user" ? "user" : "project",
      runtimePath,
      projectPath: target.id === "user" ? null : target.detail,
    },
  };
}

function toAssetOption(asset: AssetSummary): AssetOption {
  const type = asset.assetType === "command" ? "Command" : asset.assetType === "mcp" ? "MCP" : "Skill";
  return {
    id: asset.name,
    type,
    detail: asset.description || asset.sourcePath,
    icon: asset.assetType === "command" ? TerminalSquare : asset.assetType === "mcp" ? Blocks : BookOpen,
  };
}

function toTargetOption(project: ProjectSummary): TargetOption {
  return { id: project.name, detail: project.path };
}
