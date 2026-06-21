import {
  Activity,
  ChevronRight,
  CircleCheck,
  FolderKanban,
  ListChecks,
} from "lucide-react";
import { projects, recentActivity, stats, systemChecks } from "../mock-data";

export type AppInfo = {
  name: string;
  version: string;
  platform: string;
  arch: string;
  backendReady: boolean;
};

type DashboardPageProps = {
  appInfo: AppInfo;
};

export function DashboardPage({ appInfo }: DashboardPageProps) {
  return (
    <>
      <section className="stats-grid" aria-label="资产统计">
        {stats.map((stat) => {
          const Icon = stat.icon;
          return (
            <article className="stat-card" key={stat.label}>
              <div className={`icon-box ${stat.tone}`}><Icon size={20} /></div>
              <div className="stat-copy"><span>{stat.label}</span><strong>{stat.value}</strong><small>{stat.change}</small></div>
            </article>
          );
        })}
      </section>

      <div className="dashboard-grid">
        <section className="panel activity-panel">
          <div className="panel-header"><div><h2>最近活动</h2><p>资产中心的最新变更</p></div><button className="text-button">查看全部<ChevronRight size={14} /></button></div>
          <div className="activity-list">
            {recentActivity.map((item) => {
              const Icon = item.icon;
              return (
                <div className="activity-item" key={item.title}>
                  <div className={`activity-icon ${item.tone}`}><Icon size={16} /></div>
                  <div className="activity-copy"><strong>{item.title}</strong><span>{item.meta}</span></div>
                  <time>{item.time}</time>
                </div>
              );
            })}
          </div>
        </section>

        <section className="panel projects-panel">
          <div className="panel-header"><div><h2>常用项目</h2><p>最近访问的运行目标</p></div><button className="icon-button" aria-label="项目列表"><ChevronRight size={17} /></button></div>
          <div className="project-list">
            {projects.map((project) => (
              <div className="project-item" key={project.name}>
                <div className="project-folder"><FolderKanban size={18} /></div>
                <div className="project-copy"><strong>{project.name}</strong><span>{project.path}</span></div>
                <div className="project-meta"><span>{project.assets} 项资产</span><small className={project.state === "正常" ? "ok" : "pending"}>{project.state}</small></div>
              </div>
            ))}
          </div>
        </section>

        <section className="panel health-panel">
          <div className="panel-header"><div><h2>系统状态</h2><p>当前为安全隔离的预览环境</p></div><span className="healthy-badge"><CircleCheck size={14} />界面正常</span></div>
          <div className="check-grid">
            {systemChecks.map((check) => (
              <div className="check-item" key={check.label}>
                <div className="check-icon"><ListChecks size={17} /></div>
                <div><strong>{check.label}</strong><span>{check.detail}</span></div>
                <small>{check.status}</small>
              </div>
            ))}
          </div>
          <div className="backend-strip">
            <Activity size={15} />
            <span>Tauri 后端</span>
            <strong>{appInfo.backendReady ? "已连接" : "浏览器预览"}</strong>
            <code>{appInfo.platform} · {appInfo.arch} · v{appInfo.version}</code>
          </div>
        </section>
      </div>
    </>
  );
}
