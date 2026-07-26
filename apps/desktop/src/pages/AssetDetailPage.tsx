import { AlertTriangle, BookOpen, ExternalLink, FolderKanban, FolderOpen, Link2, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import {
  canonicalAssetOpen,
  canonicalDeleteApply,
  canonicalDeletePreview,
} from "../app/data-api";
import type { ApplyResult, CanonicalDeletePreview } from "../app/contracts";
import type { AssetDetailContext } from "../app/detail-context";
import type { PageId } from "../app/pages";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

const fallbackDetail: AssetDetailContext = {
  assetId: "skill:review",
  assetType: "skill",
  name: "review",
  title: "代码审查工作流",
  summary: "统一代码审查流程与输出格式，覆盖正确性、回归风险和测试质量。",
  status: "已挂载",
  statusTone: "success",
  typeLabel: "Skill",
  category: "工程质量",
  sourcePath: "assets/skills/review",
  scope: "用户级",
  updated: "今天 10:24",
  mountTargets: ["~/.claude/skills/review", "project-a/.claude/skills/review"],
  previewLabel: "SKILL.md 内容预览",
  preview: "# Review\n\n检查代码正确性、回归风险、边界条件和测试覆盖。",
};

type AssetDetailPageProps = {
  demoMode?: boolean;
  detail?: AssetDetailContext;
  onPageChange?: (page: PageId) => void;
};

export function AssetDetailPage({ demoMode = false, detail: detailProp, onPageChange }: AssetDetailPageProps) {
  const [detail, setDetail] = useState<AssetDetailContext | null>(detailProp ?? (demoMode ? fallbackDetail : null));
  const [deletePreview, setDeletePreview] = useState<CanonicalDeletePreview | null>(null);
  const [isApplying, setIsApplying] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [result, setResult] = useState<ApplyResult | null>(null);

  useEffect(() => setDetail(detailProp ?? (demoMode ? fallbackDetail : null)), [demoMode, detailProp]);

  if (!detail) {
    return <section className="panel detail-section"><div className="asset-empty-state"><BookOpen size={22} /><strong>未选择真实资产</strong><span>请从 Skills、Commands 或 MCP Servers 检查器打开资产详情。</span></div></section>;
  }

  const handleOpen = async () => {
    if (detail.assetType === "mcp") {
      onPageChange?.("mcp");
      return;
    }
    try {
      const opened = await canonicalAssetOpen({
        assetId: detail.assetId,
        action: detail.assetType === "skill" ? "reveal" : "open_external",
      });
      setMessage(`已打开：${opened.path}`);
    } catch {
      setMessage("无法打开资产，请检查文件是否仍存在。");
    }
  };

  const previewDelete = async () => {
    try {
      setDeletePreview(await canonicalDeletePreview({
        assetId: detail.assetId,
        mode: "unmount_all",
        removeMcpTargetEntries: false,
      }));
      setMessage(null);
    } catch {
      setDeletePreview(null);
      setMessage("删除影响预览生成失败。");
    }
  };

  const applyDelete = async () => {
    if (!deletePreview?.canApply) return;
    setIsApplying(true);
    try {
      const applied = await canonicalDeleteApply({
        previewId: deletePreview.previewId,
        previewGeneratedAtEpochSeconds: deletePreview.generatedAtEpochSeconds,
        request: {
          assetId: detail.assetId,
          mode: "unmount_all",
          removeMcpTargetEntries: false,
        },
      });
      setResult({
        mode: "apply",
        ok: applied.deleted,
        previewId: applied.previewId,
        backup: null,
        steps: [{ stepId: "delete", kind: "backup", label: "删除资产", status: applied.deleted ? "success" : "failed", message: applied.deleted ? "资产及关联挂载已删除。" : "资产未删除。", affectedPaths: applied.affectedPaths }],
        warnings: [],
        errors: applied.deleted ? [] : ["资产未删除。"],
      });
      if (applied.deleted) setDetail(null);
    } catch {
      setMessage("删除未完成；事务会自动回滚。");
    } finally {
      setIsApplying(false);
    }
  };

  return (
    <div className="detail-workspace">
      <section className="panel entity-hero">
        <div className="entity-hero-title"><span className="entity-hero-icon"><BookOpen size={21} /></span><div><small>{detail.title}</small><h2>{detail.name}</h2><p>{detail.summary}</p></div></div>
        <div className="entity-hero-actions">
          <span className={`asset-status ${detail.statusTone}`}>{detail.status}</span>
          {!demoMode ? <button className="asset-secondary-action" data-no-drag="true" onClick={() => void handleOpen()} style={NO_DRAG_REGION_STYLE} type="button">{detail.assetType === "skill" ? <FolderOpen size={14} /> : <ExternalLink size={14} />}{detail.assetType === "skill" ? "在文件管理器中显示" : detail.assetType === "command" ? "使用外部编辑器打开" : "返回 MCP 编辑"}</button> : null}
        </div>
      </section>
      {message ? <p className="asset-open-message" role="status">{message}</p> : null}
      <div className="detail-two-column">
        <div className="detail-column">
          <section className="panel detail-section"><div className="section-heading"><div><h3>资产信息</h3><p>{detail.typeLabel} · {detail.category}</p></div></div><dl className="entity-field-list"><div><dt>来源路径</dt><dd>{detail.sourcePath}</dd></div><div><dt>作用域</dt><dd>{detail.scope}</dd></div><div><dt>最近更新</dt><dd>{detail.updated}</dd></div><div><dt>使用引用</dt><dd>{detail.mountTargets.length} 个运行目标</dd></div></dl></section>
          <section className="panel detail-section"><div className="section-heading"><div><h3>挂载引用</h3><p>只读关系；请在挂载管理解除</p></div><Link2 size={16} /></div><div className="reference-list">{detail.mountTargets.length > 0 ? detail.mountTargets.map((target) => <div key={target}><FolderKanban size={15} /><span>运行目标</span><small>{target}</small></div>) : <div><FolderKanban size={15} /><span>暂无挂载目标</span><small>资产中心</small></div>}</div><button className="asset-secondary-action" data-no-drag="true" onClick={() => onPageChange?.("mounts")} style={NO_DRAG_REGION_STYLE} type="button">前往挂载管理</button></section>
        </div>
        <div className="detail-column">
          <section className="panel detail-section content-preview-panel"><div className="section-heading"><div><h3>{detail.previewLabel}</h3><p>只读 canonical 内容</p></div></div><pre><code>{detail.preview}</code></pre></section>
          {!demoMode && detail.assetType !== "mcp" ? <section className="panel detail-section asset-danger-zone"><div className="section-heading"><div><h3>高风险操作</h3><p>删除 canonical 资产并精确清理全部关联挂载</p></div><Trash2 size={17} /></div><div className="operation-warning"><AlertTriangle size={17} /><div><strong>删除后无法由应用自动恢复</strong><span>执行前会列出影响并创建 portable/local 备份；历史恢复需按备份教程手动完成。</span></div></div><button className="asset-danger-action" data-no-drag="true" onClick={() => void previewDelete()} style={NO_DRAG_REGION_STYLE} type="button">预览删除影响</button>{deletePreview ? <><div className="plan-lines">{deletePreview.plannedEffects.map((effect) => <span key={effect}>{effect}</span>)}</div><ApplyConfirmationPanel actionLabel="确认删除资产" canApply={deletePreview.canApply} description="将先解除所有关联挂载，再删除 canonical 内容和 registry 记录。" isApplying={isApplying} onApply={() => void applyDelete()} operationError={message} result={result} title="执行高风险删除" /></> : null}</section> : null}
        </div>
      </div>
    </div>
  );
}
