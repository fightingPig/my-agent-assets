import { AlertTriangle, Blocks, BookOpen } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { conflictApply, previewConflicts, previewImport } from "../app/data-api";
import type { ApplyResult, ConflictPreview, ConflictResolution, ConflictResolutionChoice, ImportPreview } from "../app/contracts";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { StaticActionButton } from "../components/ui/StaticActionButton";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

const conflicts = [
  {
    id: "postgresql",
    name: "PostgreSQL",
    type: "MCP Server",
    reason: "同名配置内容不同",
    source: "project-a/.mcp.json",
    assetId: "mcp:PostgreSQL",
    allowedResolutions: ["skip", "rename", "overwrite"] as ConflictResolution[],
    icon: Blocks,
    existing: `{\n  "command": "postgres-mcp",\n  "args": ["--read-only"]\n}`,
    incoming: `{\n  "command": "postgres-mcp",\n  "args": ["--schema", "public"]\n}`,
  },
  {
    id: "review",
    name: "review",
    type: "Skill",
    reason: "资产中心已存在同名 Skill",
    source: "my-app/.claude/skills/review",
    assetId: "skill:review",
    allowedResolutions: ["skip", "rename", "overwrite"] as ConflictResolution[],
    icon: BookOpen,
    existing: "# Review\n\n检查正确性、风险和测试覆盖。",
    incoming: "# Review\n\n检查架构、性能和安全边界。",
  },
];

export function ConflictResolverPage({ demoMode = false }: { demoMode?: boolean }) {
  const [items, setItems] = useState(demoMode ? conflicts : []);
  const [selectedId, setSelectedId] = useState(demoMode ? conflicts[0].id : "");
  const [resolutions, setResolutions] = useState<Record<string, ConflictResolution>>({});
  const [previewState, setPreviewState] = useState("预览中");
  const [importPreview, setImportPreview] = useState<ImportPreview | null>(null);
  const [planResult, setPlanResult] = useState<ApplyResult | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [confirmationValue, setConfirmationValue] = useState("");
  const [operationError, setOperationError] = useState<string | null>(null);
  const [isPlanning, setIsPlanning] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    let cancelled = false;
    if (!demoMode) {
      setItems([]);
      setSelectedId("");
      setResolutions({});
      setPreviewState("等待扫描结果");
      return undefined;
    }
    setPreviewState("预览中");
    previewConflicts({ scope: { kind: "user" }, assetIds: ["mcp:PostgreSQL", "skill:review"] })
      .then((result) => {
        if (cancelled) return;
        if (result.length > 0) {
          const mapped = result.map(toConflictItem);
          setItems(mapped);
          setSelectedId(mapped[0].id);
          setResolutions(defaultResolutions(mapped));
          setPreviewState("预览数据");
        } else {
          setItems(conflicts);
          setSelectedId(conflicts[0].id);
          setResolutions(defaultResolutions(conflicts));
          setPreviewState("静态预览");
        }
      })
      .catch(() => {
        if (cancelled) return;
        setItems(conflicts);
        setSelectedId(conflicts[0].id);
        setResolutions(defaultResolutions(conflicts));
        setPreviewState("读取失败，保留 Visual QA 示例数据");
      });
    return () => {
      cancelled = true;
    };
  }, [demoMode, refreshKey]);

  const selected = items.find((conflict) => conflict.id === selectedId) ?? items[0];
  const selectedResolution = selected ? resolutions[selected.id] ?? "skip" : "skip";
  const resolutionChoices = useMemo<ConflictResolutionChoice[]>(
    () => items.map((item) => ({
      conflictId: item.id,
      resolution: resolutions[item.id] ?? "skip",
      renameTo: (resolutions[item.id] ?? "skip") === "rename" ? `${item.name}-imported` : null,
    })),
    [items, resolutions],
  );
  const selectedChoice = selected
    ? resolutionChoices.find((choice) => choice.conflictId === selected.id)
    : undefined;
  const selectedPlan = describeResolution(selectedResolution, selected?.name ?? "当前资产");
  const assetIds = useMemo(() => items.map((item) => item.assetId), [items]);

  useEffect(() => {
    let cancelled = false;
    if (resolutionChoices.length !== items.length || items.length === 0) return undefined;
    setPreviewState("生成决策预览中");
    setPlanResult(null);
    setOperationError(null);
    previewImport({
      scope: { kind: "user" },
      assetIds,
      conflictResolutions: resolutionChoices,
    })
      .then((result) => {
        if (cancelled) return;
        setImportPreview(result.canApply ? result : null);
        setPreviewState(result.canApply ? "决策预览已生成" : "决策预览不可执行");
      })
      .catch((error) => {
        if (cancelled) return;
        setImportPreview(null);
        setOperationError(errorMessage(error));
        setPreviewState("决策预览失败");
      });
    return () => {
      cancelled = true;
    };
  }, [assetIds, items.length, resolutionChoices]);

  const canApply = Boolean(planResult?.ok && importPreview?.previewId);

  const updateResolution = (resolution: ConflictResolution) => {
    if (!selected) return;
    setResolutions((current) => ({ ...current, [selected.id]: resolution }));
    setApplyResult(null);
    setConfirmationValue("");
  };

  const handlePlanConflictApply = async () => {
    if (!importPreview?.previewId) return;
    setIsPlanning(true);
    setOperationError(null);
    try {
      const result = await conflictApply({
        previewId: importPreview.previewId,
        mode: "planOnly",
        scope: { kind: "user" },
        assetIds,
        conflictResolutions: resolutionChoices,
        backupBeforeApply: true,
      });
      setPlanResult(result);
      setPreviewState(result.ok ? "冲突处理计划已生成" : "冲突处理计划失败");
    } catch (error) {
      setPlanResult(null);
      setOperationError(errorMessage(error));
      setPreviewState("冲突处理计划失败");
    } finally {
      setIsPlanning(false);
    }
  };

  const handleApplyConflicts = async () => {
    if (!canApply || !importPreview?.previewId) return;
    setIsApplying(true);
    setOperationError(null);
    try {
      const result = await conflictApply({
        previewId: importPreview.previewId,
        mode: "apply",
        scope: { kind: "user" },
        assetIds,
        conflictResolutions: resolutionChoices,
        backupBeforeApply: true,
      });
      setApplyResult(result);
      setPreviewState(result.ok ? "冲突处理已执行" : "冲突处理失败");
      if (result.ok) {
        setConfirmationValue("");
        setRefreshKey((current) => current + 1);
      }
    } catch (error) {
      setApplyResult(null);
      setOperationError(errorMessage(error));
      setPreviewState("冲突处理失败");
    } finally {
      setIsApplying(false);
    }
  };

  return (
    <div className="master-detail-workspace conflict-workspace">
      <section className="panel master-list-panel">
        <div className="section-heading"><div><h3>待处理冲突</h3><p>需要逐项确认处理方式 · {previewState}</p></div><span className="status-badge warning">{items.length} 项</span></div>
        <div className="master-select-list" role="listbox" aria-label="冲突选择">{items.map(({ id, name, type, reason, icon: Icon }) => <button aria-label={name} aria-selected={selectedId === id} className={selectedId === id ? "selected" : ""} data-no-drag="true" key={id} onClick={() => setSelectedId(id)} role="option" style={NO_DRAG_REGION_STYLE} type="button"><span className="skeleton-icon warning"><Icon size={16} /></span><span><strong>{name}</strong><small>{type} · {reason}</small></span><AlertTriangle size={15} /></button>)}</div>
      </section>
      <section className="panel master-inspector-panel">
        {!selected ? <div className="asset-inspector-empty"><AlertTriangle size={22} /><strong>暂无待处理冲突</strong><span>先运行扫描；发现真实冲突后会在这里逐项处理。</span></div> : <>
        <div className="section-heading"><div><h3>{selected.name}</h3><p>{selected.reason}</p></div><span className="asset-status warning">需要确认</span></div>
        <dl className="entity-field-list compact"><div><dt>资产类型</dt><dd>{selected.type}</dd></div><div><dt>扫描来源</dt><dd>{selected.source}</dd></div><div><dt>决策预览</dt><dd>{selectedPlan.label}</dd></div></dl>
        <div className="side-by-side-diff"><div><strong>资产中心</strong><pre><code>{selected.existing}</code></pre></div><div><strong>扫描结果</strong><pre><code>{selected.incoming}</code></pre></div></div>
        <div className="resolution-options">{(["skip", "rename", "overwrite"] as ConflictResolution[]).map((resolution) => {
          const option = describeResolution(resolution, selected.name);
          const disabled = !selected.allowedResolutions.includes(resolution);
          return <button aria-pressed={selectedResolution === resolution} className={selectedResolution === resolution ? "selected" : ""} data-no-drag="true" disabled={disabled} key={resolution} onClick={() => updateResolution(resolution)} style={NO_DRAG_REGION_STYLE} type="button"><strong>{option.label}</strong><span>{option.description}</span></button>;
        })}</div>
        <div className="operation-warning"><AlertTriangle size={17} /><div><strong>处理计划预览</strong><span>{selectedPlan.planText}{selectedChoice?.renameTo ? `；新名称：${selectedChoice.renameTo}` : ""}。共 {resolutionChoices.length} 个冲突；覆盖或重命名前将创建本地备份。</span></div></div>
        <div className="operation-actions"><StaticActionButton className="asset-secondary-action">导出决策</StaticActionButton><button className="asset-secondary-action" data-no-drag="true" disabled={isPlanning || !importPreview?.previewId} onClick={handlePlanConflictApply} style={NO_DRAG_REGION_STYLE} type="button">{isPlanning ? "生成中" : "生成处理计划"}</button></div>
        <ApplyConfirmationPanel actionLabel="执行冲突处理" canApply={canApply} confirmationValue={confirmationValue} description="会按逐项决策跳过、重命名或覆盖资产；后端会校验 previewId、路径并在破坏性变更前备份。" isApplying={isApplying} onApply={handleApplyConflicts} onConfirmationChange={setConfirmationValue} operationError={operationError} result={applyResult} title="执行冲突处理" />
        </>}
      </section>
    </div>
  );
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "无法调用冲突处理操作。";
}

function toConflictItem(conflict: ConflictPreview): (typeof conflicts)[number] {
  return {
    id: conflict.id,
    name: conflict.name,
    type: conflict.assetType === "mcp" ? "MCP Server" : conflict.assetType === "command" ? "Command" : "Skill",
    reason: conflict.reason,
    source: conflict.assetId,
    assetId: conflict.assetId,
    allowedResolutions: conflict.allowedResolutions,
    icon: conflict.assetType === "mcp" ? Blocks : BookOpen,
    existing: conflict.existingContent,
    incoming: conflict.incomingContent,
  };
}

function defaultResolutions(items: typeof conflicts): Record<string, ConflictResolution> {
  return Object.fromEntries(items.map((item) => [item.id, "skip" satisfies ConflictResolution]));
}

function describeResolution(resolution: ConflictResolution, name: string) {
  if (resolution === "rename") {
    return {
      label: "重命名",
      description: "以新名称导入当前内容",
      planText: `${name} 将以新名称导入，资产中心现有内容保持不变`,
    };
  }
  if (resolution === "overwrite") {
    return {
      label: "覆盖",
      description: "使用扫描结果替换现有内容",
      planText: `${name} 将在未来确认导入时覆盖资产中心内容`,
    };
  }
  return {
    label: "跳过",
    description: "保留资产中心内容",
    planText: `${name} 将被跳过，资产中心现有内容保持不变`,
  };
}
