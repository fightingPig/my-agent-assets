import { Activity, Blocks, BookOpen, FolderKanban, Link2, TerminalSquare } from "lucide-react";
import type { ProjectDetailContext } from "../app/detail-context";
import { StaticActionButton } from "../components/ui/StaticActionButton";
import { staticProjects } from "./project-data";

const fallbackProject = staticProjects[0];
const projectTone = { "正常": "success", "有变更": "warning", "待同步": "neutral" } as const;

type ProjectDetailPageProps = {
  detail?: ProjectDetailContext;
};

export function ProjectDetailPage({ detail = fallbackProject }: ProjectDetailPageProps) {
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
          <section className="panel detail-section"><div className="section-heading"><div><h3>本地环境</h3><p>仅展示静态环境预览</p></div></div><div className="environment-list"><div><strong>Claude Runtime</strong><span>项目级 · 预览正常</span></div><div><strong>符号链接</strong><span>3 项已挂载</span></div><div><strong>MCP 配置</strong><span>.mcp.json · 1 项</span></div></div></section>
          <section className="panel detail-section"><div className="section-heading"><div><h3>最近活动</h3><p>项目资产变更预览</p></div></div><div className="timeline-list"><div><Activity size={14} /><span>挂载 db-review</span><time>今天 11:20</time></div><div><Activity size={14} /><span>更新 deploy-prod</span><time>今天 09:40</time></div><div><Activity size={14} /><span>扫描项目资产</span><time>昨天 18:12</time></div></div></section>
        </div>

        <div className="detail-column">
          <section className="panel detail-section"><div className="section-heading"><div><h3>已挂载资产</h3><p>按资产类型分组</p></div></div><div className="mounted-groups"><div><h4><BookOpen size={14} />Skills</h4>{renderMounts(skillMounts)}</div><div><h4><TerminalSquare size={14} />Commands</h4>{renderMounts(commandMounts)}</div><div><h4><Blocks size={14} />MCP Servers</h4>{renderMounts(mcpMounts)}</div></div></section>
          <section className="panel detail-section mount-plan-card"><div className="section-heading"><div><h3>挂载计划预览</h3><p>不会执行任何文件变更</p></div><Link2 size={17} /></div><div className="plan-lines"><span>验证 {detail.assets} 项资产来源</span><span>保持 {Math.max(detail.assets - detail.mcps, 0)} 个现有软链接</span><span>编译 {detail.mcps} 项项目 MCP 配置</span><span>执行前创建本地备份</span></div><StaticActionButton className="asset-business-action">预览挂载计划</StaticActionButton></section>
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
