import { Activity, Blocks, BookOpen, FolderKanban, Link2, TerminalSquare } from "lucide-react";
import { StaticActionButton } from "../components/ui/StaticActionButton";
import { staticProjects } from "./project-data";

const project = staticProjects[0];

export function ProjectDetailPage() {
  return (
    <div className="detail-workspace">
      <section className="panel entity-hero">
        <div className="entity-hero-title"><span className="entity-hero-icon blue"><FolderKanban size={21} /></span><div><small>{project.title}</small><h2>{project.name}</h2><p>{project.description}</p></div></div>
        <span className="asset-status success">{project.status}</span>
      </section>

      <div className="detail-two-column">
        <div className="detail-column">
          <section className="panel detail-section"><div className="section-heading"><div><h3>项目概览</h3><p>{project.path}</p></div><span>{project.updated}</span></div><div className="project-metrics"><div><strong>4</strong><span>全部资产</span></div><div><strong>2</strong><span>Skills</span></div><div><strong>1</strong><span>Command</span></div><div><strong>1</strong><span>MCP</span></div></div></section>
          <section className="panel detail-section"><div className="section-heading"><div><h3>本地环境</h3><p>仅展示静态环境预览</p></div></div><div className="environment-list"><div><strong>Claude Runtime</strong><span>项目级 · 预览正常</span></div><div><strong>符号链接</strong><span>3 项已挂载</span></div><div><strong>MCP 配置</strong><span>.mcp.json · 1 项</span></div></div></section>
          <section className="panel detail-section"><div className="section-heading"><div><h3>最近活动</h3><p>项目资产变更预览</p></div></div><div className="timeline-list"><div><Activity size={14} /><span>挂载 db-review</span><time>今天 11:20</time></div><div><Activity size={14} /><span>更新 deploy-prod</span><time>今天 09:40</time></div><div><Activity size={14} /><span>扫描项目资产</span><time>昨天 18:12</time></div></div></section>
        </div>

        <div className="detail-column">
          <section className="panel detail-section"><div className="section-heading"><div><h3>已挂载资产</h3><p>按资产类型分组</p></div></div><div className="mounted-groups"><div><h4><BookOpen size={14} />Skills</h4><span>review</span><span>db-review</span></div><div><h4><TerminalSquare size={14} />Commands</h4><span>deploy-prod</span></div><div><h4><Blocks size={14} />MCP Servers</h4><span>PostgreSQL</span></div></div></section>
          <section className="panel detail-section mount-plan-card"><div className="section-heading"><div><h3>挂载计划预览</h3><p>不会执行任何文件变更</p></div><Link2 size={17} /></div><div className="plan-lines"><span>验证 4 项资产来源</span><span>保持 3 个现有软链接</span><span>编译 1 项项目 MCP 配置</span><span>执行前创建本地备份</span></div><StaticActionButton className="asset-business-action">预览挂载计划</StaticActionButton></section>
        </div>
      </div>
    </div>
  );
}
