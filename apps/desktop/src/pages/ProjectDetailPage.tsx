import { Activity, Blocks, BookOpen, FolderKanban, Link2, TerminalSquare } from "lucide-react";
import type { ProjectDetailContext } from "../app/detail-context";
import type { PageId } from "../app/pages";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";
import { staticProjects } from "./project-data";

const fallbackProject = staticProjects[0];
const projectTone = {
  "正常": "success",
  "需检查": "warning",
  "未检查": "neutral",
  "无效": "warning",
} as const;

type ProjectDetailPageProps = {
  demoMode?: boolean;
  detail?: ProjectDetailContext;
  onPageChange?: (page: PageId) => void;
};

export function ProjectDetailPage({
  demoMode = false,
  detail: detailProp,
  onPageChange,
}: ProjectDetailPageProps) {
  const detail = detailProp ?? (demoMode ? fallbackProject : null);
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

  const skillMounts = detail.mounts.filter((mount) => mountKind(mount) === "skill");
  const commandMounts = detail.mounts.filter((mount) => mountKind(mount) === "command");
  const mcpMounts = detail.mounts.filter((mount) => mountKind(mount) === "mcp");

  return (
    <div className="detail-workspace">
      <section className="panel entity-hero">
        <div className="entity-hero-title">
          <span className="entity-hero-icon blue"><FolderKanban size={21} /></span>
          <div><small>{detail.title}</small><h2>{detail.name}</h2><p>{detail.description}</p></div>
        </div>
        <span className={`asset-status ${projectTone[detail.status]}`}>{detail.status}</span>
      </section>

      <div className="detail-two-column">
        <div className="detail-column">
          <section className="panel detail-section">
            <div className="section-heading"><div><h3>项目概览</h3><p>{detail.path}</p></div><span>{detail.updated}</span></div>
            <div className="project-metrics"><div><strong>{detail.assets}</strong><span>全部资产</span></div><div><strong>{detail.skills}</strong><span>Skills</span></div><div><strong>{detail.commands}</strong><span>Commands</span></div><div><strong>{detail.mcps}</strong><span>MCP</span></div></div>
          </section>
          <section className="panel detail-section">
            <div className="section-heading"><div><h3>本地环境</h3><p>运行时发现与路径健康</p></div></div>
            <div className="environment-list">
              <div><strong>路径健康</strong><span>{detail.status === "无效" ? "目录不可用" : "目录可读取"}</span></div>
              <div><strong>最近检查</strong><span>{detail.updated}</span></div>
              <div><strong>挂载引用</strong><span>{detail.mounts.length} 项</span></div>
              <div><strong>发现摘要</strong><span>{detail.skills} Skills · {detail.commands} Commands · {detail.mcps} MCP</span></div>
            </div>
          </section>
          <section className="panel detail-section">
            <div className="section-heading"><div><h3>相关活动</h3><p>本机项目维护记录</p></div></div>
            <div className="asset-empty-state"><Activity size={20} /><strong>暂无项目活动记录</strong><span>刷新、挂载和解除挂载后会记录脱敏操作摘要。</span></div>
          </section>
        </div>

        <div className="detail-column">
          <section className="panel detail-section">
            <div className="section-heading"><div><h3>已挂载资产</h3><p>按资产类型分组</p></div></div>
            <div className="mounted-groups">
              <div><h4><BookOpen size={14} />Skills</h4>{renderMounts(skillMounts)}</div>
              <div><h4><TerminalSquare size={14} />Commands</h4>{renderMounts(commandMounts)}</div>
              <div><h4><Blocks size={14} />MCP Servers</h4>{renderMounts(mcpMounts)}</div>
            </div>
          </section>
          <section className="panel detail-section mount-plan-card">
            <div className="section-heading"><div><h3>挂载管理</h3><p>项目详情不直接修改运行时</p></div><Link2 size={17} /></div>
            <p className="asset-inspector-summary">前往挂载管理选择资产、项目位置、Provider 和 MCP 范围，并在执行前查看完整预览。</p>
            <button className="asset-business-action" data-no-drag="true" onClick={() => onPageChange?.("mounts")} style={NO_DRAG_REGION_STYLE} type="button">前往挂载管理</button>
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

function mountKind(mount: string) {
  if (mount.startsWith("skill:")) return "skill";
  if (mount.startsWith("command:")) return "command";
  if (mount.startsWith("mcp:")) return "mcp";
  return "skill";
}
