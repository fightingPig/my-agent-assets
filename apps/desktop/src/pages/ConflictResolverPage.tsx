import { AlertTriangle, Blocks, BookOpen } from "lucide-react";
import { useEffect, useState } from "react";
import { previewConflicts } from "../app/data-api";
import type { ConflictPreview } from "../app/contracts";
import { StaticActionButton } from "../components/ui/StaticActionButton";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

const conflicts = [
  {
    id: "postgresql",
    name: "PostgreSQL",
    type: "MCP Server",
    reason: "同名配置内容不同",
    source: "project-a/.mcp.json",
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
    icon: BookOpen,
    existing: "# Review\n\n检查正确性、风险和测试覆盖。",
    incoming: "# Review\n\n检查架构、性能和安全边界。",
  },
];

export function ConflictResolverPage() {
  const [items, setItems] = useState(conflicts);
  const [selectedId, setSelectedId] = useState(conflicts[0].id);
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
          setPreviewState("预览数据");
        } else {
          setItems(conflicts);
          setSelectedId(conflicts[0].id);
          setPreviewState("静态预览");
        }
      })
      .catch(() => {
        if (cancelled) return;
        setItems(conflicts);
        setSelectedId(conflicts[0].id);
        setPreviewState("读取失败，使用静态预览");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const selected = items.find((conflict) => conflict.id === selectedId) ?? items[0];

  return (
    <div className="master-detail-workspace conflict-workspace">
      <section className="panel master-list-panel">
        <div className="section-heading"><div><h3>待处理冲突</h3><p>需要逐项确认处理方式 · {previewState}</p></div><span className="status-badge warning">{items.length} 项</span></div>
        <div className="master-select-list" role="listbox" aria-label="冲突选择">{items.map(({ id, name, type, reason, icon: Icon }) => <button aria-label={name} aria-selected={selectedId === id} className={selectedId === id ? "selected" : ""} data-no-drag="true" key={id} onClick={() => setSelectedId(id)} role="option" style={NO_DRAG_REGION_STYLE} type="button"><span className="skeleton-icon warning"><Icon size={16} /></span><span><strong>{name}</strong><small>{type} · {reason}</small></span><AlertTriangle size={15} /></button>)}</div>
      </section>
      <section className="panel master-inspector-panel">
        <div className="section-heading"><div><h3>{selected.name}</h3><p>{selected.reason}</p></div><span className="asset-status warning">需要确认</span></div>
        <dl className="entity-field-list compact"><div><dt>资产类型</dt><dd>{selected.type}</dd></div><div><dt>扫描来源</dt><dd>{selected.source}</dd></div></dl>
        <div className="side-by-side-diff"><div><strong>资产中心</strong><pre><code>{selected.existing}</code></pre></div><div><strong>扫描结果</strong><pre><code>{selected.incoming}</code></pre></div></div>
        <div className="resolution-options"><div><strong>跳过</strong><span>保留资产中心内容</span></div><div><strong>重命名</strong><span>以新名称导入当前内容</span></div><div><strong>覆盖</strong><span>使用扫描结果替换现有内容</span></div></div>
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
    icon: conflict.assetType === "mcp" ? Blocks : BookOpen,
    existing: conflict.existingContent,
    incoming: conflict.incomingContent,
  };
}
