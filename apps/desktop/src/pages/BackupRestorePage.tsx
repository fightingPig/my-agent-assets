import { ArchiveRestore, FileClock, FolderKanban } from "lucide-react";
import { useState } from "react";
import { StaticActionButton } from "../components/ui/StaticActionButton";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

const backups = [
  { id: "backup-20260621-1842", title: "扫描导入前", created: "今天 18:42", size: "24 KB", paths: ["~/.claude/skills/review", "~/workspace/project-a/.mcp.json", "~/.claude/commands/deploy-prod.md"], impact: "恢复 3 项路径，覆盖 2 项当前内容" },
  { id: "backup-20260620-0915", title: "挂载变更前", created: "昨天 09:15", size: "18 KB", paths: ["~/workspace/my-app/.claude/skills/react-review", "~/workspace/my-app/.mcp.json"], impact: "恢复 2 项路径，移除 1 个新软链接" },
  { id: "backup-20260618-1630", title: "资产移除前", created: "3 天前", size: "8 KB", paths: ["~/.claude/commands/format-code.md"], impact: "恢复 1 项 Command 路径" },
];

export function BackupRestorePage() {
  const [selectedId, setSelectedId] = useState(backups[0].id);
  const selected = backups.find((backup) => backup.id === selectedId)!;

  return (
    <div className="master-detail-workspace backup-workspace">
      <section className="panel master-list-panel">
        <div className="section-heading"><div><h3>备份记录</h3><p>本地 backup manifest 预览</p></div><span>{backups.length} 份</span></div>
        <div className="master-select-list" role="listbox" aria-label="备份选择">{backups.map((backup) => <button aria-label={backup.id} aria-selected={selectedId === backup.id} className={selectedId === backup.id ? "selected" : ""} data-no-drag="true" key={backup.id} onClick={() => setSelectedId(backup.id)} role="option" style={NO_DRAG_REGION_STYLE} type="button"><span className="skeleton-icon"><ArchiveRestore size={16} /></span><span><strong>{backup.title}</strong><small>{backup.created} · {backup.size}</small></span></button>)}</div>
      </section>
      <section className="panel master-inspector-panel">
        <div className="section-heading"><div><h3>{selected.title}</h3><p>{selected.id}</p></div><span className="healthy-badge">manifest 完整</span></div>
        <div className="restore-summary"><FileClock size={18} /><div><strong>恢复影响预览</strong><span>{selected.impact}</span></div></div>
        <section className="affected-paths"><h4>受影响路径</h4>{selected.paths.map((path) => <div key={path}><FolderKanban size={14} /><code>{path}</code></div>)}</section>
        <div className="operation-warning neutral"><ArchiveRestore size={17} /><div><strong>恢复前将再次创建备份</strong><span>当前内容不会在没有确认的情况下被覆盖。</span></div></div>
        <div className="operation-actions"><StaticActionButton className="asset-secondary-action">导出清单</StaticActionButton><StaticActionButton className="asset-business-action">恢复此备份</StaticActionButton></div>
      </section>
    </div>
  );
}
