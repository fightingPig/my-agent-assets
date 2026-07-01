export type PageId =
  | "dashboard"
  | "skills"
  | "commands"
  | "mcp"
  | "asset-detail"
  | "projects"
  | "project-detail"
  | "scan"
  | "mounts"
  | "conflicts"
  | "backups"
  | "sync"
  | "settings";

export type PageGroup = "概览" | "资产中心" | "项目" | "运行" | "系统";

export type PageMetadata = {
  id: PageId;
  sidebarLabel: string;
  title: string;
  subtitle: string;
  group: PageGroup;
  enabled: boolean;
  sidebarVisible: boolean;
};

export const PAGE_GROUPS: readonly PageGroup[] = ["概览", "资产中心", "项目", "运行", "系统"];

export const PAGE_REGISTRY: readonly PageMetadata[] = [
  {
    id: "dashboard",
    sidebarLabel: "首页",
    title: "首页",
    subtitle: "集中查看资产、项目和本地运行环境。",
    group: "概览",
    enabled: true,
    sidebarVisible: true,
  },
  {
    id: "skills",
    sidebarLabel: "Skills",
    title: "Skills",
    subtitle: "管理技能资产，查看挂载状态和引用关系。",
    group: "资产中心",
    enabled: true,
    sidebarVisible: true,
  },
  {
    id: "commands",
    sidebarLabel: "Commands",
    title: "Commands",
    subtitle: "管理命令资产，维护可复用的工作流入口。",
    group: "资产中心",
    enabled: true,
    sidebarVisible: true,
  },
  {
    id: "mcp",
    sidebarLabel: "MCP Servers",
    title: "MCP Servers",
    subtitle: "管理 MCP 服务器配置和连接状态。",
    group: "资产中心",
    enabled: true,
    sidebarVisible: true,
  },
  {
    id: "asset-detail",
    sidebarLabel: "资产详情",
    title: "资产详情",
    subtitle: "查看资产内容、挂载状态和引用关系。",
    group: "资产中心",
    enabled: true,
    sidebarVisible: false,
  },
  {
    id: "projects",
    sidebarLabel: "项目列表",
    title: "项目列表",
    subtitle: "管理本机项目和资产挂载目标。",
    group: "项目",
    enabled: true,
    sidebarVisible: true,
  },
  {
    id: "project-detail",
    sidebarLabel: "项目详情",
    title: "项目详情",
    subtitle: "查看项目路径、环境状态和资产挂载情况。",
    group: "项目",
    enabled: true,
    sidebarVisible: false,
  },
  {
    id: "scan",
    sidebarLabel: "扫描导入",
    title: "扫描导入",
    subtitle: "扫描本机 Claude 资产，生成导入预览。",
    group: "运行",
    enabled: true,
    sidebarVisible: true,
  },
  {
    id: "mounts",
    sidebarLabel: "挂载管理",
    title: "挂载管理",
    subtitle: "将资产挂载到项目或用户级目录。",
    group: "运行",
    enabled: true,
    sidebarVisible: true,
  },
  {
    id: "conflicts",
    sidebarLabel: "冲突处理",
    title: "冲突处理",
    subtitle: "检查并解决扫描、挂载或同步产生的冲突。",
    group: "运行",
    enabled: true,
    sidebarVisible: true,
  },
  {
    id: "backups",
    sidebarLabel: "备份历史",
    title: "备份历史",
    subtitle: "查看备份记录、定位文件并阅读手动恢复指南。",
    group: "运行",
    enabled: true,
    sidebarVisible: true,
  },
  {
    id: "sync",
    sidebarLabel: "同步",
    title: "同步",
    subtitle: "使用本地 Git 仓库同步资产数据。",
    group: "运行",
    enabled: true,
    sidebarVisible: true,
  },
  {
    id: "settings",
    sidebarLabel: "设置",
    title: "设置",
    subtitle: "配置路径、扫描、安全、同步和外观偏好。",
    group: "系统",
    enabled: true,
    sidebarVisible: true,
  },
];

const PAGE_BY_ID = new Map(PAGE_REGISTRY.map((page) => [page.id, page]));

export function getPageById(id: PageId): PageMetadata {
  const page = PAGE_BY_ID.get(id);
  if (!page) throw new Error(`Unknown page: ${id}`);
  return page;
}

export function getSidebarPageGroups() {
  return PAGE_GROUPS.map((group) => ({
    group,
    pages: PAGE_REGISTRY.filter((page) => page.group === group && page.sidebarVisible),
  })).filter(({ pages }) => pages.length > 0);
}
