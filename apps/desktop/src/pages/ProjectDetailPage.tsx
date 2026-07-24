import { Activity, Blocks, BookOpen, FolderKanban, Link2, TerminalSquare } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  canonicalMountApply,
  canonicalMountPreview,
  listAssets,
  listMountTargets,
  listProjects,
} from "../app/data-api";
import type {
  ApplyResult,
  AssetSummary,
  CanonicalMountPreview,
  CanonicalMountPreviewRequest,
  ProjectSummary,
  RegisteredMountTarget,
} from "../app/contracts";
import type { ProjectDetailContext } from "../app/detail-context";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";
import { staticProjects } from "./project-data";

const fallbackProject = staticProjects[0];
const projectTone = { "正常": "success", "有变更": "warning", "待同步": "neutral", "无效": "warning" } as const;

type ProjectDetailPageProps = {
  demoMode?: boolean;
  detail?: ProjectDetailContext;
};

export function ProjectDetailPage({ demoMode = false, detail: detailProp }: ProjectDetailPageProps) {
  const initialDetail = detailProp ?? (demoMode ? fallbackProject : null);
  const [detail, setDetail] = useState<ProjectDetailContext | null>(initialDetail);
  const [selectedAsset, setSelectedAsset] = useState<AssetSummary | null>(null);
  const [target, setTarget] = useState<RegisteredMountTarget | null>(null);
  const [preview, setPreview] = useState<CanonicalMountPreview | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [isApplying, setIsApplying] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    setDetail(detailProp ?? (demoMode ? fallbackProject : null));
  }, [demoMode, detailProp]);

  useEffect(() => {
    let cancelled = false;
    if (!detail) {
      setSelectedAsset(null);
      return undefined;
    }
    Promise.all([listAssets(), listMountTargets()])
      .then(([assets, targets]) => {
        if (cancelled) return;
        const asset = (
          assets.find((asset) => detail.mounts.includes(asset.name))
            ?? assets[0]
            ?? null
        );
        setSelectedAsset(asset);
        setTarget(asset
          ? targets.find((candidate) =>
            candidate.projectPath === detail.path &&
            candidate.accepts.includes(asset.assetType) &&
            candidate.status === "ready"
          ) ?? null
          : null);
      })
      .catch((error) => {
        if (!cancelled) setOperationError(errorMessage(error));
      });
    return () => {
      cancelled = true;
    };
  }, [detail, refreshKey]);

  const previewInput = useMemo(
    () => selectedAsset && target
      ? { assetId: selectedAsset.id, targetId: target.id } satisfies CanonicalMountPreviewRequest
      : null,
    [selectedAsset, target],
  );

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

  const canApply = Boolean(preview?.canApply && preview?.previewId && previewInput);

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
        const projects = await listProjects();
        const refreshed = projects.find(
          (project) => project.id === detail.id || project.path === detail.path,
        );
        if (refreshed) setDetail(toProjectDetail(refreshed));
        setRefreshKey((current) => current + 1);
      }
    } catch (error) {
      setApplyResult(null);
      setOperationError(errorMessage(error));
    } finally {
      setIsApplying(false);
    }
  };

  if (!detail) {
    return (
      <section className="panel detail-section">
        <div className="asset-empty-state">
          <FolderKanban size={22} />
          <strong>未选择真实项目</strong>
          <span>请从项目列表检查器打开项目详情。</span>
        </div>
      </section>
    );
  }

  const skillMounts = detail.mounts.filter((mount) => mount.includes("review") || mount.includes("skill"));
  const commandMounts = detail.mounts.filter((mount) => mount.includes("deploy") || mount.includes("build") || mount.includes("test") || mount.includes("format"));
  const mcpMounts = detail.mounts.filter((mount) => mount.includes("PostgreSQL") || mount.includes("Filesystem") || mount.includes("Redis") || mount.includes("SQLite"));

  return (
    <div className="detail-workspace">
      <section className="panel entity-hero">
        <div className="entity-hero-title"><span className="entity-hero-icon blue"><FolderKanban size={21} /></span><div><small>{detail.title}</small><h2>{detail.name}</h2><p>{detail.description}</p></div></div>
        <span className={`asset-status ${projectTone[detail.status]}`}>{detail.status}</span>
      </section>

      <div className="detail-two-column">
        <div className="detail-column">
          <section className="panel detail-section"><div className="section-heading"><div><h3>项目概览</h3><p>{detail.path}</p></div><span>{detail.updated}</span></div><div className="project-metrics"><div><strong>{detail.assets}</strong><span>全部资产</span></div><div><strong>{detail.skills}</strong><span>Skills</span></div><div><strong>{detail.commands}</strong><span>Commands</span></div><div><strong>{detail.mcps}</strong><span>MCP</span></div></div></section>
          <section className="panel detail-section"><div className="section-heading"><div><h3>本地环境</h3><p>{demoMode ? "Visual QA 示例环境" : "项目只读汇总"}</p></div></div><div className="environment-list"><div><strong>Claude Runtime</strong><span>项目级 · {detail.assets} 项资产</span></div><div><strong>挂载引用</strong><span>{detail.mounts.length} 项</span></div><div><strong>MCP 配置</strong><span>{detail.mcps} 项</span></div></div></section>
          <section className="panel detail-section"><div className="section-heading"><div><h3>最近活动</h3><p>项目资产变更</p></div></div>{demoMode ? <div className="timeline-list"><div><Activity size={14} /><span>挂载 db-review</span><time>今天 11:20</time></div><div><Activity size={14} /><span>更新 deploy-prod</span><time>今天 09:40</time></div><div><Activity size={14} /><span>扫描项目资产</span><time>昨天 18:12</time></div></div> : <div className="asset-empty-state"><Activity size={20} /><strong>暂无真实活动记录</strong><span>项目活动数据源尚未接入。</span></div>}</section>
        </div>

        <div className="detail-column">
          <section className="panel detail-section"><div className="section-heading"><div><h3>已挂载资产</h3><p>按资产类型分组</p></div></div><div className="mounted-groups"><div><h4><BookOpen size={14} />Skills</h4>{renderMounts(skillMounts)}</div><div><h4><TerminalSquare size={14} />Commands</h4>{renderMounts(commandMounts)}</div><div><h4><Blocks size={14} />MCP Servers</h4>{renderMounts(mcpMounts)}</div></div></section>
          <section className="panel detail-section mount-plan-card"><div className="section-heading"><div><h3>挂载计划预览</h3><p>{selectedAsset ? `目标资产：${selectedAsset.name}` : "资产中心暂无可挂载资产"}</p></div><Link2 size={17} /></div><div className="plan-lines"><span>验证 {detail.assets} 项项目资产</span><span>保持 {Math.max(detail.assets - detail.mcps, 0)} 个现有软链接</span><span>编译 {detail.mcps} 项项目 MCP 配置</span><span>执行前创建本地备份</span></div><button className="asset-business-action" data-no-drag="true" disabled={!previewInput} onClick={() => setRefreshKey((current) => current + 1)} style={NO_DRAG_REGION_STYLE} type="button">刷新挂载预览</button><ApplyConfirmationPanel actionLabel="确认项目挂载" canApply={canApply} description={target ? `将 ${previewInput?.assetId ?? "资产"} 挂载到已授权项目目标 ${target.id}（${preview?.affectedTargetPath ?? target.path}）。` : "当前项目没有与所选资产兼容的已授权目标，请先在设置中注册。"} isApplying={isApplying} onApply={handleApplyMount} operationError={operationError} result={applyResult} title="执行项目挂载" /></section>
        </div>
      </div>
    </div>
  );
}

function toProjectDetail(project: ProjectSummary): ProjectDetailContext {
  return {
    id: project.id,
    name: project.name,
    title: project.title,
    path: project.path,
    status: project.status === "changed" ? "有变更" : project.status === "needsSync" ? "待同步" : project.status === "invalid" ? "无效" : "正常",
    assets: project.assetCounts.total,
    skills: project.assetCounts.skills,
    commands: project.assetCounts.commands,
    mcps: project.assetCounts.mcps,
    updated: project.updatedAt ?? "未知",
    description: project.description,
    mounts: project.mounts,
  };
}

function errorMessage(_error: unknown) {
  return "项目挂载未完成。请查看系统状态或导出诊断包后重试。";
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
      stepId: "canonical-project-mount",
      kind: "mount",
      label: "挂载项目资产",
      status: result.mounted ? "success" : "failed",
      message: result.mounted ? "项目目标已更新并登记挂载关系。" : "项目挂载未完成。",
      affectedPaths: result.affectedPaths,
    }],
    warnings: result.warnings,
    errors: result.mounted ? [] : ["项目挂载未完成。"],
  };
}

function renderMounts(mounts: readonly string[]) {
  return mounts.length > 0
    ? mounts.map((mount) => <span key={mount}>{mount}</span>)
    : <span>暂无</span>;
}
