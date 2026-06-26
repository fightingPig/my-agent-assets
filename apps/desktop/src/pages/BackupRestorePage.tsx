import { ArchiveRestore, FileClock, FolderKanban } from "lucide-react";
import { useEffect, useState } from "react";
import { previewRestore, restoreApply } from "../app/data-api";
import type { ApplyResult, RestorePreview } from "../app/contracts";
import { StaticActionButton } from "../components/ui/StaticActionButton";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

const backups = [
  { id: "backup-20260621-1842", title: "扫描导入前", created: "今天 18:42", size: "24 KB", paths: ["~/.claude/skills/review", "~/workspace/project-a/.mcp.json", "~/.claude/commands/deploy-prod.md"], impact: "恢复 3 项路径，覆盖 2 项当前内容" },
  { id: "backup-20260620-0915", title: "挂载变更前", created: "昨天 09:15", size: "18 KB", paths: ["~/workspace/my-app/.claude/skills/react-review", "~/workspace/my-app/.mcp.json"], impact: "恢复 2 项路径，移除 1 个新软链接" },
  { id: "backup-20260618-1630", title: "资产移除前", created: "3 天前", size: "8 KB", paths: ["~/.claude/commands/format-code.md"], impact: "恢复 1 项 Command 路径" },
];

export function BackupRestorePage() {
  const [selectedId, setSelectedId] = useState(backups[0].id);
  const [preview, setPreview] = useState<RestorePreview | null>(null);
  const [planResult, setPlanResult] = useState<ApplyResult | null>(null);
  const [previewState, setPreviewState] = useState("预览中");
  const [isPlanning, setIsPlanning] = useState(false);
  const selected = backups.find((backup) => backup.id === selectedId)!;

  useEffect(() => {
    let cancelled = false;
    setPreviewState("预览中");
    setPlanResult(null);
    previewRestore({ backupId: selectedId })
      .then((result) => {
        if (cancelled) return;
        if (result.affectedPaths.length > 0 || result.steps.length > 0) {
          setPreview(result);
          setPreviewState("预览数据");
        } else {
          setPreview(null);
          setPreviewState("静态预览");
        }
      })
      .catch(() => {
        if (cancelled) return;
        setPreview(null);
        setPreviewState("读取失败，使用静态预览");
      });
    return () => {
      cancelled = true;
    };
  }, [selectedId]);

  const affectedPaths = preview?.affectedPaths.length ? preview.affectedPaths : selected.paths;
  const impact = preview?.steps.length
    ? preview.steps.map((step) => step.label).join(" / ")
    : selected.impact;
  const planSummary = planResult?.steps.length
    ? planResult.steps.map((step) => step.message).join(" / ")
    : null;

  const handlePlanRestore = async () => {
    setIsPlanning(true);
    setPreviewState("生成恢复计划中");
    try {
      const result = await restoreApply({
        previewId: `restore-plan:${selectedId}`,
        mode: "planOnly",
        backupId: selectedId,
        backupBeforeRestore: preview?.backupBeforeRestore ?? true,
      });
      setPlanResult(result);
      setPreviewState(result.ok ? "恢复计划已生成" : "恢复计划失败");
    } catch {
      setPlanResult(null);
      setPreviewState("恢复计划失败");
    } finally {
      setIsPlanning(false);
    }
  };

  return (
    <div className="master-detail-workspace backup-workspace">
      <section className="panel master-list-panel">
        <div className="section-heading"><div><h3>备份记录</h3><p>本地 backup manifest 预览</p></div><span>{backups.length} 份</span></div>
        <div className="master-select-list" role="listbox" aria-label="备份选择">{backups.map((backup) => <button aria-label={backup.id} aria-selected={selectedId === backup.id} className={selectedId === backup.id ? "selected" : ""} data-no-drag="true" key={backup.id} onClick={() => setSelectedId(backup.id)} role="option" style={NO_DRAG_REGION_STYLE} type="button"><span className="skeleton-icon"><ArchiveRestore size={16} /></span><span><strong>{backup.title}</strong><small>{backup.created} · {backup.size}</small></span></button>)}</div>
      </section>
      <section className="panel master-inspector-panel">
        <div className="section-heading"><div><h3>{selected.title}</h3><p>{selected.id}</p></div><span className="healthy-badge">{previewState}</span></div>
        <div className="restore-summary"><FileClock size={18} /><div><strong>恢复影响预览</strong><span>{impact}</span></div></div>
        <section className="affected-paths"><h4>受影响路径</h4>{affectedPaths.map((path) => <div key={path}><FolderKanban size={14} /><code>{path}</code></div>)}</section>
        <div className="operation-warning neutral"><ArchiveRestore size={17} /><div><strong>{preview?.backupBeforeRestore ?? true ? "恢复前将再次创建备份" : "不需要额外备份"}</strong><span>{planSummary ?? preview?.warnings[0] ?? "当前内容不会在没有确认的情况下被覆盖。"}</span></div></div>
        <div className="operation-actions"><StaticActionButton className="asset-secondary-action">导出清单</StaticActionButton><button className="asset-secondary-action" data-no-drag="true" disabled={isPlanning} onClick={handlePlanRestore} style={NO_DRAG_REGION_STYLE} type="button">{isPlanning ? "生成中" : "生成恢复计划"}</button><StaticActionButton className="asset-business-action">恢复此备份</StaticActionButton></div>
      </section>
    </div>
  );
}
