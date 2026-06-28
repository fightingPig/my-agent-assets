import { Activity, Blocks, BookOpen, FolderKanban, Link2, TerminalSquare } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { listAssets, listProjects, mountApply, previewMount } from "../app/data-api";
import type { ApplyResult, AssetSummary, MountPreview, PreviewMountInput, ProjectSummary } from "../app/contracts";
import type { ProjectDetailContext } from "../app/detail-context";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";
import { staticProjects } from "./project-data";

const fallbackProject = staticProjects[0];
const projectTone = { "正常": "success", "有变更": "warning", "待同步": "neutral" } as const;

type ProjectDetailPageProps = {
  demoMode?: boolean;
  detail?: ProjectDetailContext;
};

export function ProjectDetailPage({ demoMode = false, detail: detailProp }: ProjectDetailPageProps) {
  const initialDetail = detailProp ?? (demoMode ? fallbackProject : null);
  const [detail, setDetail] = useState<ProjectDetailContext | null>(initialDetail);
  const [selectedAsset, setSelectedAsset] = useState<AssetSummary | null>(null);
  const [preview, setPreview] = useState<MountPreview | null>(null);
  const [planResult, setPlanResult] = useState<ApplyResult | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [confirmationValue, setConfirmationValue] = useState("");
  const [operationError, setOperationError] = useState<string | null>(null);
  const [isPlanning, setIsPlanning] = useState(false);
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
    listAssets()
      .then((assets) => {
        if (cancelled) return;
        setSelectedAsset(
          assets.find((asset) => detail.mounts.includes(asset.name))
            ?? assets[0]
            ?? null,
        );
      })
      .catch((error) => {
        if (!cancelled) setOperationError(errorMessage(error));
      });
    return () => {
      cancelled = true;
    };
  }, [detail, refreshKey]);

  const previewInput = useMemo(
    () => detail && selectedAsset ? projectMountInput(detail, selectedAsset) : null,
    [detail, selectedAsset],
  );

  useEffect(() => {
    let cancelled = false;
    setPlanResult(null);
    setOperationError(null);
    if (!previewInput) {
      setPreview(null);
      return undefined;
    }
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

  const canApply = Boolean(planResult?.ok && preview?.previewId && previewInput);

  const handlePlanMount = async () => {
    if (!preview?.previewId || !previewInput) return;
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
    if (!canApply || !preview?.previewId || !previewInput || !detail) return;
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
        const projects = await listProjects();
        const refreshed = projects.find(
          (project) => project.id === detail.id || project.path === detail.path,
        );
        if (refreshed) setDetail(toProjectDetail(refreshed));
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
          <section className="panel detail-section mount-plan-card"><div className="section-heading"><div><h3>挂载计划预览</h3><p>{selectedAsset ? `目标资产：${selectedAsset.name}` : "资产中心暂无可挂载资产"}</p></div><Link2 size={17} /></div><div className="plan-lines"><span>验证 {detail.assets} 项项目资产</span><span>保持 {Math.max(detail.assets - detail.mcps, 0)} 个现有软链接</span><span>编译 {detail.mcps} 项项目 MCP 配置</span><span>执行前创建本地备份</span></div><button className="asset-business-action" data-no-drag="true" disabled={isPlanning || !preview?.previewId} onClick={handlePlanMount} style={NO_DRAG_REGION_STYLE} type="button">{isPlanning ? "生成中" : "生成挂载计划"}</button><ApplyConfirmationPanel actionLabel="确认项目挂载" canApply={canApply} confirmationValue={confirmationValue} description={previewInput ? `将 ${previewInput.assetId} 挂载到 ${previewInput.target.runtimePath}。` : "需要先在资产中心准备可挂载资产。"} isApplying={isApplying} onApply={handleApplyMount} onConfirmationChange={setConfirmationValue} operationError={operationError} result={applyResult} title="执行项目挂载" /></section>
        </div>
      </div>
    </div>
  );
}

function projectMountInput(
  project: ProjectDetailContext,
  asset: AssetSummary,
): PreviewMountInput {
  const runtimePath = asset.assetType === "mcp"
    ? `${project.path}/.mcp.json`
    : asset.assetType === "command"
      ? `${project.path}/.claude/commands/${asset.name}.md`
      : `${project.path}/.claude/skills/${asset.name}${asset.sourcePath.endsWith(".md") ? ".md" : ""}`;
  return {
    assetId: asset.id,
    target: {
      scope: "project",
      runtimePath,
      projectPath: project.path,
    },
  };
}

function toProjectDetail(project: ProjectSummary): ProjectDetailContext {
  return {
    id: project.id,
    name: project.name,
    title: project.title,
    path: project.path,
    status: project.status === "changed" ? "有变更" : project.status === "needsSync" ? "待同步" : "正常",
    assets: project.assetCounts.total,
    skills: project.assetCounts.skills,
    commands: project.assetCounts.commands,
    mcps: project.assetCounts.mcps,
    updated: project.updatedAt ?? "未知",
    description: project.description,
    mounts: project.mounts,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "无法调用项目挂载操作。";
}

function renderMounts(mounts: readonly string[]) {
  return mounts.length > 0
    ? mounts.map((mount) => <span key={mount}>{mount}</span>)
    : <span>暂无</span>;
}
