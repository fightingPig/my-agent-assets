import {
  ArchiveRestore,
  Blocks,
  BookOpen,
  Command,
  FolderKanban,
  GitBranch,
  Home,
  Link2,
  RefreshCw,
  ScanSearch,
  Settings,
  TerminalSquare,
  type LucideIcon,
} from "lucide-react";
import { NO_DRAG_REGION_STYLE } from "../../lib/platform";

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

export function Sidebar() {
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
