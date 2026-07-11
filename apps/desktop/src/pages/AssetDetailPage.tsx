import { BookOpen, ExternalLink, FolderKanban, FolderOpen, Link2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  canonicalMountApply,
  canonicalMountPreview,
  canonicalAssetOpen,
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
import type { AssetDetailContext } from "../app/detail-context";
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
  preview: `# Review

检查代码正确性、回归风险、边界条件和测试覆盖。

## 输出

- 按严重级别列出问题
- 提供文件与行号
- 标记剩余测试风险`,
};

type AssetDetailPageProps = {
  demoMode?: boolean;
  detail?: AssetDetailContext;
};

export function AssetDetailPage({ demoMode = false, detail: detailProp }: AssetDetailPageProps) {
  const initialDetail = detailProp ?? (demoMode ? fallbackDetail : null);
  const [detail, setDetail] = useState<AssetDetailContext | null>(initialDetail);
  const [target, setTarget] = useState<RegisteredMountTarget | null>(null);
  const [preview, setPreview] = useState<CanonicalMountPreview | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [isApplying, setIsApplying] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const [openMessage, setOpenMessage] = useState<string | null>(null);
  const previewInput = useMemo<CanonicalMountPreviewRequest | null>(
    () => detail && target ? { assetId: detail.assetId, targetId: target.id } : null,
    [detail, target],
  );

  useEffect(() => {
    setDetail(detailProp ?? (demoMode ? fallbackDetail : null));
  }, [demoMode, detailProp]);

  useEffect(() => {
    let cancelled = false;
    if (!detail) {
      setTarget(null);
      return undefined;
    }
    listMountTargets()
      .then((targets) => {
        if (cancelled) return;
        const compatible = targets.filter((candidate) =>
          candidate.accepts.includes(detail.assetType) && candidate.status === "ready"
        );
        setTarget(
          compatible.find((candidate) => candidate.scope === "user")
            ?? compatible[0]
            ?? null,
        );
      })
      .catch((error) => {
        if (!cancelled) setOperationError(errorMessage(error));
      });
    return () => {
      cancelled = true;
    };
  }, [detail]);

  useEffect(() => {
    let cancelled = false;
    setOperationError(null);
    if (!previewInput) {
      setPreview(null);
      return undefined;
    }
    canonicalMountPreview(previewInput)
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

  const canApply = Boolean(preview?.canApply && preview?.previewId);

  const handleApplyMount = async () => {
    if (!canApply || !preview?.previewId || !previewInput || !detail) return;
    setIsApplying(true);
    setOperationError(null);
    try {
      const result = await canonicalMountApply({
        previewId: preview.previewId,
        previewGeneratedAtEpochSeconds: preview.generatedAtEpochSeconds,
        request: previewInput,
      });
      setApplyResult(toApplyResult(result));
      if (result.mounted) {
        const refreshed = await listAssets({ assetType: detail.assetType });
        const asset = refreshed.find((item) => item.id === detail.assetId);
        if (asset) setDetail((current) => current ? mergeAssetSummary(current, asset) : current);
        setRefreshKey((current) => current + 1);
      }
    } catch (error) {
      setApplyResult(null);
      setOperationError(errorMessage(error));
    } finally {
      setIsApplying(false);
    }
  };

  const handleOpenAsset = async () => {
    if (!detail || detail.assetType === "mcp") return;
    setOpenMessage(null);
    try {
      const result = await canonicalAssetOpen({
        assetId: detail.assetId,
        action: detail.assetType === "skill" ? "reveal" : "open_external",
      });
      setOpenMessage(
        detail.assetType === "skill"
          ? `已在文件管理器中显示：${result.path}`
          : `已交给系统默认应用打开：${result.path}`,
      );
    } catch (error) {
      setOpenMessage(errorMessage(error));
    }
  };

  if (!detail) {
    return (
      <section className="panel detail-section">
        <div className="asset-empty-state">
          <BookOpen size={22} />
          <strong>未选择真实资产</strong>
          <span>请从 Skills、Commands 或 MCP Servers 检查器打开资产详情。</span>
        </div>
      </section>
    );
  }

  return (
    <div className="detail-workspace">
        <section className="panel entity-hero">
        <div className="entity-hero-title"><span className="entity-hero-icon"><BookOpen size={21} /></span><div><small>{detail.title}</small><h2>{detail.name}</h2><p>{detail.summary}</p></div></div>
        <div className="entity-hero-actions">
          <span className={`asset-status ${detail.statusTone}`}>{detail.status}</span>
          {!demoMode && detail.assetType !== "mcp" ? <button className="asset-secondary-action" data-no-drag="true" onClick={() => void handleOpenAsset()} style={NO_DRAG_REGION_STYLE} type="button">{detail.assetType === "skill" ? <FolderOpen size={14} /> : <ExternalLink size={14} />}{detail.assetType === "skill" ? "在文件管理器中显示" : "使用外部编辑器打开"}</button> : null}
        </div>
      </section>
      {openMessage ? <p className="asset-open-message">{openMessage}</p> : null}
      <div className="detail-two-column">
        <div className="detail-column">
          <section className="panel detail-section"><div className="section-heading"><div><h3>资产信息</h3><p>{detail.typeLabel} · {detail.category}</p></div></div><dl className="entity-field-list"><div><dt>来源路径</dt><dd>{detail.sourcePath}</dd></div><div><dt>作用域</dt><dd>{detail.scope}</dd></div><div><dt>最近更新</dt><dd>{detail.updated}</dd></div><div><dt>使用引用</dt><dd>{detail.mountTargets.length} 个运行目标</dd></div></dl></section>
          <section className="panel detail-section"><div className="section-heading"><div><h3>挂载目标</h3><p>当前引用关系</p></div><Link2 size={16} /></div><div className="reference-list">{detail.mountTargets.length > 0 ? detail.mountTargets.map((target) => <div key={target}><FolderKanban size={15} /><span>{target.includes("project") ? "项目级 Claude Runtime" : "用户级 Claude Runtime"}</span><small>{target}</small></div>) : <div><FolderKanban size={15} /><span>暂无挂载目标</span><small>资产中心</small></div>}</div></section>
        </div>
        <section className="panel detail-section content-preview-panel"><div className="section-heading"><div><h3>{detail.previewLabel}</h3><p>只读内容</p></div></div><pre><code>{detail.preview}</code></pre><div className="detail-actions"><button className="asset-business-action" data-no-drag="true" disabled={!previewInput} onClick={() => setRefreshKey((current) => current + 1)} style={NO_DRAG_REGION_STYLE} type="button">刷新挂载预览</button></div><ApplyConfirmationPanel actionLabel="确认挂载" canApply={canApply} description={target ? `将挂载到已授权目标 ${target.id}（${preview?.affectedTargetPath ?? target.path}）；执行前校验 previewId，并在替换现有目标前创建备份。` : "没有兼容的已授权目标，请先在设置中注册目标。"} isApplying={isApplying} onApply={handleApplyMount} operationError={operationError} result={applyResult} title="执行资产挂载" /></section>
      </div>
    </div>
  );
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

function toApplyResult(
  result: Awaited<ReturnType<typeof canonicalMountApply>>,
): ApplyResult {
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

function errorMessage(_error: unknown) {
  return "资产操作未完成。请查看系统状态或导出诊断包后重试。";
}
