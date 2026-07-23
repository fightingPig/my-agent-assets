import { Activity, ArrowLeft, Blocks, BookOpen, FolderKanban, Link2, TerminalSquare } from "lucide-react";
import type { ProjectDetailContext } from "../app/detail-context";
import type { PageId } from "../app/pages";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";
import { staticProjects } from "./project-data";

const fallbackProject = staticProjects[0];
const projectTone = {
  "正常": "success",
  "未检查": "neutral",
  "需处理": "warning",
  "路径不可用": "danger",
} as const;

type ProjectDetailPageProps = {
  demoMode?: boolean;
  detail?: ProjectDetailContext;
  onPageChange?: (page: PageId) => void;
};

/**
 * A project detail is intentionally informational. Mount changes are managed
 * in the Mount Manager, where every target and provider choice has a preview.
 */
export function ProjectDetailPage({ demoMode = false, detail, onPageChange }: ProjectDetailPageProps) {
  const project = detail ?? (demoMode ? fallbackProject : null);

  if (!project) {
    return (
      <section className="panel detail-section">
        <div className="asset-empty-state">
          <FolderKanban size={22} />
          <strong>未选择维护项目</strong>
          <span>请从项目列表选择一个项目后查看详情。</span>
          {onPageChange ? <button className="asset-secondary-action" data-no-drag="true" onClick={() => onPageChange("projects")} style={NO_DRAG_REGION_STYLE} type="button">返回项目列表</button> : null}
        </div>
      </section>
    );
  }

  const skillMounts = project.mounts.filter((mount) => mount.includes("review") || mount.includes("skill"));
  const commandMounts = project.mounts.filter((mount) => mount.includes("deploy") || mount.includes("build") || mount.includes("test") || mount.includes("format"));
  const mcpMounts = project.mounts.filter((mount) => !skillMounts.includes(mount) && !commandMounts.includes(mount));

  return (
    <div className="detail-workspace project-detail-workspace">
      <div className="detail-navigation">
        {onPageChange ? <button className="text-button" data-no-drag="true" onClick={() => onPageChange("projects")} style={NO_DRAG_REGION_STYLE} type="button"><ArrowLeft size={15} />返回项目列表</button> : null}
      </div>
      <section className="panel entity-hero">
        <div className="entity-hero-title"><span className="entity-hero-icon blue"><FolderKanban size={21} /></span><div><small>维护项目</small><h2>{project.name}</h2><p>{project.description}</p></div></div>
        <span className={`asset-status ${projectTone[project.status]}`}>{project.status}</span>
      </section>

      <div className="detail-two-column">
        <div className="detail-column">
          <section className="panel detail-section">
            <div className="section-heading"><div><h3>项目概览</h3><p>{project.path}</p></div><span>{project.lastCheckedAt ?? "尚未检查"}</span></div>
            <div className="project-metrics"><div><strong>{project.assets}</strong><span>全部资产</span></div><div><strong>{project.skills}</strong><span>Skills</span></div><div><strong>{project.commands}</strong><span>Commands</span></div><div><strong>{project.mcps}</strong><span>MCP</span></div></div>
          </section>
          <section className="panel detail-section">
            <div className="section-heading"><div><h3>本地环境</h3><p>仅显示项目资产维护状态</p></div></div>
            <div className="environment-list"><div><strong>路径可用性</strong><span>{project.status === "路径不可用" ? "项目路径目前不可用" : "项目目录可读取"}</span></div><div><strong>最近检查</strong><span>{project.lastCheckedAt ?? "尚未执行检查"}</span></div><div><strong>扫描提示</strong><span>{project.warningCount ?? 0} 项</span></div></div>
          </section>
          <section className="panel detail-section">
            <div className="section-heading"><div><h3>最近检查</h3><p>项目检查只保存时间、统计和提示摘要</p></div></div>
            <div className="timeline-list"><div><Activity size={14} /><span>{project.lastCheckedAt ? "已完成最近一次项目检查" : "项目尚未检查"}</span><time>{project.lastCheckedAt ?? "-"}</time></div></div>
          </section>
        </div>

        <div className="detail-column">
          <section className="panel detail-section">
            <div className="section-heading"><div><h3>已挂载资产</h3><p>按资产类型分组；变更请前往挂载管理</p></div></div>
            <div className="mounted-groups"><div><h4><BookOpen size={14} />Skills</h4>{renderMounts(skillMounts)}</div><div><h4><TerminalSquare size={14} />Commands</h4>{renderMounts(commandMounts)}</div><div><h4><Blocks size={14} />MCP Servers</h4>{renderMounts(mcpMounts)}</div></div>
          </section>
          <section className="panel detail-section mount-plan-card">
            <div className="section-heading"><div><h3>挂载管理</h3><p>选择资产、目标和 Provider 后生成可审阅计划。</p></div><Link2 size={17} /></div>
            <div className="plan-lines"><span>当前项目有 {project.mounts.length} 项挂载引用</span><span>挂载、卸载都将在执行前展示影响范围</span><span>修改项目路径或移除项目之前，必须先卸载相关绑定</span></div>
            {onPageChange ? <button className="asset-business-action" data-no-drag="true" onClick={() => onPageChange("mounts")} style={NO_DRAG_REGION_STYLE} type="button">前往挂载管理</button> : null}
          </section>
        </div>
      </div>
    </div>
  );
}

function renderMounts(mounts: readonly string[]) {
  return mounts.length > 0
    ? mounts.map((mount) => <span key={mount}>{mount}</span>)
    : <span>暂无</span>;
}
