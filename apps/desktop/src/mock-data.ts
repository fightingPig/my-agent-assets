import type { LucideIcon } from "lucide-react";
import {
  Blocks,
  BookOpen,
  Bot,
  FolderKanban,
  GitBranch,
  Link2,
  TerminalSquare,
} from "lucide-react";

export type Stat = {
  label: string;
  value: number;
  change: string;
  icon: LucideIcon;
  tone: "green" | "blue" | "violet" | "amber";
};

export const stats: Stat[] = [
  { label: "Skills", value: 28, change: "+3 本月", icon: BookOpen, tone: "green" },
  { label: "Commands", value: 14, change: "+2 本月", icon: TerminalSquare, tone: "blue" },
  { label: "MCP Servers", value: 8, change: "6 个已挂载", icon: Blocks, tone: "violet" },
  { label: "项目", value: 5, change: "4 个状态正常", icon: FolderKanban, tone: "amber" },
];

export const systemChecks = [
  { label: "资产中心", detail: "~/.my-agent-assets", status: "正常" },
  { label: "Git", detail: "main · 工作区干净", status: "正常" },
  { label: "Claude Runtime", detail: "预览模式，未读取", status: "隔离" },
  { label: "符号链接权限", detail: "macOS 可用", status: "正常" },
];

export const recentActivity = [
  { icon: Bot, title: "导入 Skill: review", meta: "来自 project-a", time: "2 分钟前", tone: "violet" },
  { icon: Link2, title: "挂载 MCP: github", meta: "User scope", time: "18 分钟前", tone: "green" },
  { icon: TerminalSquare, title: "更新 Command: commit", meta: "已记录到资产中心", time: "1 小时前", tone: "blue" },
  { icon: GitBranch, title: "Git 同步完成", meta: "origin/main", time: "今天 09:42", tone: "amber" },
];

export const projects = [
  { name: "project-a", path: "~/workspace/project-a", assets: 12, state: "正常" },
  { name: "design-system", path: "~/workspace/design-system", assets: 8, state: "正常" },
  { name: "my-app", path: "~/code/my-app", assets: 5, state: "待扫描" },
];
