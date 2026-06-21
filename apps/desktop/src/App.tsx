import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Activity,
  ArchiveRestore,
  Blocks,
  BookOpen,
  Bot,
  ChevronRight,
  CircleCheck,
  Command,
  FolderKanban,
  GitBranch,
  Home,
  Link2,
  ListChecks,
  Plus,
  RefreshCw,
  ScanSearch,
  Search,
  Settings,
  ShieldCheck,
  TerminalSquare,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useState, type PointerEvent as ReactPointerEvent } from "react";
import {
  DRAG_REGION_ATTR,
  DRAG_REGION_STYLE,
  getDesktopPlatform,
  getPlatformShortcuts,
  isTauriRuntime,
  NO_DRAG_REGION_STYLE,
} from "./lib/platform";
import { projects, recentActivity, stats, systemChecks } from "./mock-data";

type AppInfo = {
  name: string;
  version: string;
  platform: string;
  arch: string;
  backendReady: boolean;
};

type NavItem = { label: string; icon: LucideIcon; active?: boolean };

const navGroups: { label: string; items: NavItem[] }[] = [
  { label: "概览", items: [{ label: "首页", icon: Home, active: true }] },
  {
    label: "资产中心",
    items: [
      { label: "Skills", icon: BookOpen },
      { label: "Commands", icon: TerminalSquare },
      { label: "MCP Servers", icon: Blocks },
    ],
  },
  { label: "项目", items: [{ label: "项目列表", icon: FolderKanban }] },
  {
    label: "运行",
    items: [
      { label: "扫描导入", icon: ScanSearch },
      { label: "挂载管理", icon: Link2 },
      { label: "备份恢复", icon: ArchiveRestore },
      { label: "Git 同步", icon: RefreshCw },
    ],
  },
  { label: "系统", items: [{ label: "设置", icon: Settings }] },
];

const fallbackInfo: AppInfo = {
  name: "My Agent Assets",
  version: "0.1.0",
  platform: "macOS",
  arch: "arm64",
  backendReady: false,
};

function Sidebar() {
  return (
    <aside className="sidebar">
      <div className="brand-row">
        <div className="brand-mark"><Command size={17} /></div>
        <span>My Agent Assets</span>
      </div>
      <nav aria-label="主导航">
        {navGroups.map((group) => (
          <section className="nav-group" key={group.label}>
            <div className="nav-label">{group.label}</div>
            {group.items.map((item) => {
              const Icon = item.icon;
              return (
                <button
                  className={`nav-item ${item.active ? "active" : ""}`}
                  disabled={!item.active}
                  key={item.label}
                  style={NO_DRAG_REGION_STYLE}
                  title={item.active ? item.label : `${item.label}将在确认首页后实现`}
                >
                  <Icon size={17} />
                  <span>{item.label}</span>
                </button>
              );
            })}
          </section>
        ))}
      </nav>
      <div className="sidebar-footer">
        <div className="connection-row"><span className="status-dot" />预览环境</div>
        <div className="branch-row"><GitBranch size={14} />main</div>
      </div>
    </aside>
  );
}

function MacOverlayDragArea() {
  const handlePointerDown = async (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;

    const target = event.target as HTMLElement;
    if (target.closest("button,input,textarea,select,a,[data-no-drag='true']")) return;

    try {
      await getCurrentWindow().startDragging();
    } catch (error) {
      console.error("[MacOverlayDragArea] startDragging failed", error);
    }
  };

  return (
    <div
      className="mac-overlay-drag-area"
      {...DRAG_REGION_ATTR}
      style={DRAG_REGION_STYLE}
      onPointerDown={handlePointerDown}
    >
      <div className="mac-overlay-sidebar" {...DRAG_REGION_ATTR} style={DRAG_REGION_STYLE} />
      <div className="mac-overlay-main" {...DRAG_REGION_ATTR} style={DRAG_REGION_STYLE} />
    </div>
  );
}

function PageHeader({ shortcuts }: { shortcuts: { globalSearch: string; pageSearch: string } }) {
  return (
    <div className="page-header">
      <div className="page-heading">
        <h1>首页</h1>
        <p className="page-subtitle">集中查看资产、项目和本地运行环境。</p>
      </div>
      <div className="page-header-actions">
        <button
          className="search-button"
          style={NO_DRAG_REGION_STYLE}
          title={`页面搜索 ${shortcuts.pageSearch}`}
        >
          <Search size={16} />
          <span>搜索</span>
          <kbd>{shortcuts.globalSearch}</kbd>
        </button>
        <button className="preview-button" style={NO_DRAG_REGION_STYLE} title="当前使用预览数据">
          <ShieldCheck size={14} />
          预览数据
        </button>
        <button className="primary-button" style={NO_DRAG_REGION_STYLE}>
          <Plus size={16} />
          快速操作
        </button>
      </div>
    </div>
  );
}

function App() {
  const [appInfo, setAppInfo] = useState<AppInfo>(fallbackInfo);
  const platform = getDesktopPlatform();
  const shortcuts = getPlatformShortcuts(platform);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    invoke<AppInfo>("app_info").then(setAppInfo).catch(() => setAppInfo(fallbackInfo));
  }, []);

  return (
    <div className={`app-frame platform-${platform}`}>
      {platform === "macos" && <MacOverlayDragArea />}
      <div className="app-body">
        <Sidebar />
        <main className="app-main">
          <PageHeader shortcuts={shortcuts} />

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
        </main>
      </div>
    </div>
  );
}

export default App;
