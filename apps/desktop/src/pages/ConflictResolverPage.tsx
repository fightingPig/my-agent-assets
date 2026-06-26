import { AlertTriangle, Blocks, BookOpen } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { previewConflicts } from "../app/data-api";
import type { ConflictPreview, ConflictResolution, ConflictResolutionChoice } from "../app/contracts";
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

export function ConflictResolverPage() {
  const [items, setItems] = useState(conflicts);
  const [selectedId, setSelectedId] = useState(conflicts[0].id);
  const [resolutions, setResolutions] = useState<Record<string, ConflictResolution>>({});
  const [previewState, setPreviewState] = useState("预览中");

  useEffect(() => {
    let cancelled = false;
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
        setPreviewState("读取失败，使用静态预览");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const selected = items.find((conflict) => conflict.id === selectedId) ?? items[0];
  const selectedResolution = resolutions[selected.id] ?? "skip";
  const resolutionChoices = useMemo<ConflictResolutionChoice[]>(
    () => items.map((item) => ({
      conflictId: item.id,
      resolution: resolutions[item.id] ?? "skip",
      renameTo: (resolutions[item.id] ?? "skip") === "rename" ? `${item.name}-imported` : null,
    })),
    [items, resolutions],
  );
  const selectedChoice = resolutionChoices.find((choice) => choice.conflictId === selected.id);
  const selectedPlan = describeResolution(selectedResolution, selected.name);

  return (
    <div className="master-detail-workspace conflict-workspace">
      <section className="panel master-list-panel">
        <div className="section-heading"><div><h3>待处理冲突</h3><p>需要逐项确认处理方式 · {previewState}</p></div><span className="status-badge warning">{items.length} 项</span></div>
        <div className="master-select-list" role="listbox" aria-label="冲突选择">{items.map(({ id, name, type, reason, icon: Icon }) => <button aria-label={name} aria-selected={selectedId === id} className={selectedId === id ? "selected" : ""} data-no-drag="true" key={id} onClick={() => setSelectedId(id)} role="option" style={NO_DRAG_REGION_STYLE} type="button"><span className="skeleton-icon warning"><Icon size={16} /></span><span><strong>{name}</strong><small>{type} · {reason}</small></span><AlertTriangle size={15} /></button>)}</div>
      </section>
      <section className="panel master-inspector-panel">
        <div className="section-heading"><div><h3>{selected.name}</h3><p>{selected.reason}</p></div><span className="asset-status warning">需要确认</span></div>
        <dl className="entity-field-list compact"><div><dt>资产类型</dt><dd>{selected.type}</dd></div><div><dt>扫描来源</dt><dd>{selected.source}</dd></div><div><dt>决策预览</dt><dd>{selectedPlan.label}</dd></div></dl>
        <div className="side-by-side-diff"><div><strong>资产中心</strong><pre><code>{selected.existing}</code></pre></div><div><strong>扫描结果</strong><pre><code>{selected.incoming}</code></pre></div></div>
        <div className="resolution-options">{(["skip", "rename", "overwrite"] as ConflictResolution[]).map((resolution) => {
          const option = describeResolution(resolution, selected.name);
          const disabled = !selected.allowedResolutions.includes(resolution);
          return <button aria-pressed={selectedResolution === resolution} className={selectedResolution === resolution ? "selected" : ""} data-no-drag="true" disabled={disabled} key={resolution} onClick={() => setResolutions((current) => ({ ...current, [selected.id]: resolution }))} style={NO_DRAG_REGION_STYLE} type="button"><strong>{option.label}</strong><span>{option.description}</span></button>;
        })}</div>
        <div className="operation-warning"><AlertTriangle size={17} /><div><strong>处理计划预览</strong><span>{selectedPlan.planText}{selectedChoice?.renameTo ? `；新名称：${selectedChoice.renameTo}` : ""}。当前 {resolutionChoices.length} 个冲突均只记录本地决策，不执行写入。</span></div></div>
        <div className="operation-actions"><StaticActionButton className="asset-secondary-action">跳过</StaticActionButton><StaticActionButton className="asset-secondary-action">重命名</StaticActionButton><StaticActionButton className="asset-business-action">覆盖</StaticActionButton></div>
      </section>
    </div>
  );
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
