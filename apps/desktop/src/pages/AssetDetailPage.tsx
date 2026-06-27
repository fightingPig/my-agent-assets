import { BookOpen, FolderKanban, Link2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { listAssets, mountApply, previewMount } from "../app/data-api";
import type { ApplyResult, AssetSummary, MountPreview, PreviewMountInput } from "../app/contracts";
import type { AssetDetailContext } from "../app/detail-context";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { StaticActionButton } from "../components/ui/StaticActionButton";
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
  preview: `# Review

检查代码正确性、回归风险、边界条件和测试覆盖。

## 输出

- 按严重级别列出问题
- 提供文件与行号
- 标记剩余测试风险`,
};

type AssetDetailPageProps = {
  detail?: AssetDetailContext;
};

export function AssetDetailPage({ detail: detailProp = fallbackDetail }: AssetDetailPageProps) {
  const [detail, setDetail] = useState(detailProp);
  const [preview, setPreview] = useState<MountPreview | null>(null);
  const [planResult, setPlanResult] = useState<ApplyResult | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [confirmationValue, setConfirmationValue] = useState("");
  const [operationError, setOperationError] = useState<string | null>(null);
  const [isPlanning, setIsPlanning] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const previewInput = useMemo(() => assetMountInput(detail), [detail]);

  useEffect(() => {
    setDetail(detailProp);
  }, [detailProp]);

  useEffect(() => {
    let cancelled = false;
    setPlanResult(null);
    setOperationError(null);
    previewMount(previewInput)
      .then((result) => {
        if (!cancelled) setPreview(result);
      })
      .catch((error) => {
        if (cancelled) return;
        setPreview(null);
        setOperationError(errorMessage(error));
      });
    return () => {
      cancelled = true;
    };
  }, [previewInput, refreshKey]);

  const canApply = Boolean(planResult?.ok && preview?.previewId);

  const handlePlanMount = async () => {
    if (!preview?.previewId) return;
    setIsPlanning(true);
    setOperationError(null);
    try {
      setPlanResult(await mountApply({
        previewId: preview.previewId,
        mode: "planOnly",
        assetId: previewInput.assetId,
        target: previewInput.target,
        backupBeforeApply: preview.backupRequired,
      }));
    } catch (error) {
      setPlanResult(null);
      setOperationError(errorMessage(error));
    } finally {
      setIsPlanning(false);
    }
  };

  const handleApplyMount = async () => {
    if (!canApply || !preview?.previewId) return;
    setIsApplying(true);
    setOperationError(null);
    try {
      const result = await mountApply({
        previewId: preview.previewId,
        mode: "apply",
        assetId: previewInput.assetId,
        target: previewInput.target,
        backupBeforeApply: preview.backupRequired,
      });
      setApplyResult(result);
      if (result.ok) {
        const refreshed = await listAssets({ assetType: detail.assetType });
        const asset = refreshed.find((item) => item.id === detail.assetId);
        if (asset) setDetail((current) => mergeAssetSummary(current, asset));
        setConfirmationValue("");
        setRefreshKey((current) => current + 1);
      }
    } catch (error) {
      setApplyResult(null);
      setOperationError(errorMessage(error));
    } finally {
      setIsApplying(false);
    }
  };

  return (
    <div className="detail-workspace">
      <section className="panel entity-hero">
        <div className="entity-hero-title"><span className="entity-hero-icon"><BookOpen size={21} /></span><div><small>{detail.title}</small><h2>{detail.name}</h2><p>{detail.summary}</p></div></div>
        <span className={`asset-status ${detail.statusTone}`}>{detail.status}</span>
      </section>
      <div className="detail-two-column">
        <div className="detail-column">
          <section className="panel detail-section"><div className="section-heading"><div><h3>资产信息</h3><p>{detail.typeLabel} · {detail.category}</p></div></div><dl className="entity-field-list"><div><dt>来源路径</dt><dd>{detail.sourcePath}</dd></div><div><dt>作用域</dt><dd>{detail.scope}</dd></div><div><dt>最近更新</dt><dd>{detail.updated}</dd></div><div><dt>使用引用</dt><dd>{detail.mountTargets.length} 个运行目标</dd></div></dl></section>
          <section className="panel detail-section"><div className="section-heading"><div><h3>挂载目标</h3><p>当前引用关系</p></div><Link2 size={16} /></div><div className="reference-list">{detail.mountTargets.length > 0 ? detail.mountTargets.map((target) => <div key={target}><FolderKanban size={15} /><span>{target.includes("project") ? "项目级 Claude Runtime" : "用户级 Claude Runtime"}</span><small>{target}</small></div>) : <div><FolderKanban size={15} /><span>暂无挂载目标</span><small>资产中心</small></div>}</div></section>
        </div>
        <section className="panel detail-section content-preview-panel"><div className="section-heading"><div><h3>{detail.previewLabel}</h3><p>只读内容</p></div></div><pre><code>{detail.preview}</code></pre><div className="detail-actions"><StaticActionButton className="asset-secondary-action">查看引用</StaticActionButton><button className="asset-business-action" data-no-drag="true" disabled={isPlanning || !preview?.previewId} onClick={handlePlanMount} style={NO_DRAG_REGION_STYLE} type="button">{isPlanning ? "生成中" : "生成挂载计划"}</button></div><ApplyConfirmationPanel actionLabel="确认挂载" canApply={canApply} confirmationValue={confirmationValue} description={`将挂载到 ${previewInput.target.runtimePath}；执行前校验 previewId，并在替换现有目标前创建备份。`} isApplying={isApplying} onApply={handleApplyMount} onConfirmationChange={setConfirmationValue} operationError={operationError} result={applyResult} title="执行资产挂载" /></section>
      </div>
    </div>
  );
}

function assetMountInput(detail: AssetDetailContext): PreviewMountInput {
  const runtimePath = detail.assetType === "mcp"
    ? "~/.claude.json"
    : detail.assetType === "command"
      ? `~/.claude/commands/${detail.name}.md`
      : `~/.claude/skills/${detail.name}${detail.sourcePath.endsWith(".md") ? ".md" : ""}`;
  return {
    assetId: detail.assetId,
    target: {
      scope: "user",
      runtimePath,
      projectPath: null,
    },
  };
}

function mergeAssetSummary(detail: AssetDetailContext, asset: AssetSummary): AssetDetailContext {
  return {
    ...detail,
    status: asset.status === "invalid" ? "无效" : asset.mountTargets.length > 0 ? "已挂载" : "可用",
    statusTone: asset.status === "invalid" ? "warning" : "success",
    sourcePath: asset.sourcePath,
    scope: asset.scope === "user" ? "用户级" : asset.scope === "project" ? "项目级" : "资产中心",
    updated: asset.updatedAt ?? detail.updated,
    mountTargets: asset.mountTargets,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "无法调用资产挂载操作。";
}
