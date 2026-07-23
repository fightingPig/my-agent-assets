export type ProjectStatus = "正常" | "未检查" | "需处理" | "路径不可用";

export type StaticProject = {
  id: string;
  name: string;
  title: string;
  path: string;
  status: ProjectStatus;
  assets: number;
  skills: number;
  commands: number;
  mcps: number;
  updated: string;
  description: string;
  mounts: readonly string[];
  lastCheckedAt?: string;
  warningCount?: number;
};

export const staticProjects: readonly StaticProject[] = [
  {
    id: "project-a",
    name: "project-a",
    title: "Agent 工作流实验项目",
    path: "~/workspace/project-a",
    status: "正常",
    assets: 4,
    skills: 2,
    commands: 1,
    mcps: 1,
    updated: "今天 11:20",
    description: "用于验证 Skills、Commands 和 MCP 挂载流程的本地项目。",
    mounts: ["review", "db-review", "deploy-prod", "PostgreSQL"],
  },
  {
    id: "my-app",
    name: "my-app",
    title: "产品主应用",
    path: "~/workspace/my-app",
    status: "需处理",
    assets: 7,
    skills: 3,
    commands: 2,
    mcps: 2,
    updated: "昨天 19:05",
    description: "当前 GUI 设计与桌面端集成测试项目。",
    mounts: ["review", "react-review", "build-project", "run-tests", "Filesystem"],
  },
  {
    id: "design-system",
    name: "design-system",
    title: "UI 组件库",
    path: "~/code/design-system",
    status: "未检查",
    assets: 3,
    skills: 1,
    commands: 2,
    mcps: 0,
    updated: "3 天前",
    description: "维护共享 UI 组件、tokens 和页面布局规范。",
    mounts: ["react-review", "format-code", "build-project"],
  },
];
