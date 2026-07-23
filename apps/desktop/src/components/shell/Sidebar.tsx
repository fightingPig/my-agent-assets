import {
  Archive,
  AlertTriangle,
  Blocks,
  BookOpen,
  FolderKanban,
  HardDrive,
  Home,
  Link2,
  RefreshCw,
  ScanSearch,
  Settings,
  TerminalSquare,
  type LucideIcon,
} from "lucide-react";
import brandMark from "../../assets/my-agent-assets-mark.svg";
import { getSidebarPageGroups, type PageId } from "../../app/pages";
import { NO_DRAG_REGION_STYLE } from "../../lib/platform";

// Keep detail-page entries so adding a new PageId also requires an explicit icon decision.
const pageIcons: Record<PageId, LucideIcon> = {
  dashboard: Home,
  skills: BookOpen,
  commands: TerminalSquare,
  mcp: Blocks,
  "asset-detail": BookOpen,
  projects: FolderKanban,
  "project-detail": FolderKanban,
  scan: ScanSearch,
  mounts: Link2,
  conflicts: AlertTriangle,
  backups: Archive,
  sync: RefreshCw,
  settings: Settings,
};

type SidebarProps = {
  activePage: PageId;
  onPageChange: (page: PageId) => void;
};

export function Sidebar({ activePage, onPageChange }: SidebarProps) {
  return (
    <aside className="sidebar">
      <div className="brand-row">
        <div className="brand-mark"><img alt="" aria-hidden="true" src={brandMark} /></div>
        <span>My Agent Assets</span>
      </div>
      <nav aria-label="主导航">
        {getSidebarPageGroups().map(({ group, pages }) => (
          <section className="nav-group" key={group}>
            <div className="nav-label">{group}</div>
            {pages.map((page) => {
              const Icon = pageIcons[page.id];
              return (
                <button
                  className={`nav-item ${activePage === page.id ? "active" : ""}`}
                  data-no-drag="true"
                  disabled={!page.enabled}
                  key={page.id}
                  onClick={() => onPageChange(page.id)}
                  style={NO_DRAG_REGION_STYLE}
                  title={page.sidebarLabel}
                  aria-current={activePage === page.id ? "page" : undefined}
                >
                  <Icon size={17} />
                  <span>{page.sidebarLabel}</span>
                </button>
              );
            })}
          </section>
        ))}
      </nav>
      <div className="sidebar-footer">
        <div className="connection-row"><span className="status-dot" />本地运行</div>
        <div className="branch-row"><HardDrive size={14} />本地数据</div>
      </div>
    </aside>
  );
}
