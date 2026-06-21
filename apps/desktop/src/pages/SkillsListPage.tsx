import { BookOpen } from "lucide-react";
import {
  AssetCenterLayout,
  InspectorCode,
  type AssetCenterItem,
} from "../components/assets/AssetCenterLayout";

type SkillItem = AssetCenterItem & {
  preview: string;
};

const skills: readonly SkillItem[] = [
  {
    id: "review",
    name: "review",
    title: "代码审查工作流",
    category: "工程质量",
    summary: "统一代码审查流程与输出格式",
    status: "已挂载",
    statusTone: "success",
    scope: "用户级",
    path: "assets/skills/review",
    icon: BookOpen,
    updated: "今天 10:24",
    mounts: ["~/.claude/skills/review", "project-a/.claude/skills/review"],
    preview: "# Review\n\n检查正确性、回归风险、边界条件和测试覆盖。",
    searchTerms: ["代码审查", "review"],
  },
  {
    id: "db-review",
    name: "db-review",
    title: "数据库变更审查",
    category: "数据工程",
    summary: "数据库变更与查询质量检查",
    status: "已挂载",
    statusTone: "success",
    scope: "项目级",
    path: "assets/skills/db-review",
    icon: BookOpen,
    updated: "昨天 16:08",
    mounts: ["project-a/.claude/skills/db-review"],
    preview: "# Database Review\n\n检查迁移安全、索引使用和事务边界。",
    searchTerms: ["数据库", "SQL"],
  },
  {
    id: "react-review",
    name: "react-review",
    title: "React 组件审查",
    category: "前端工程",
    summary: "React 组件质量与交互检查",
    status: "未挂载",
    statusTone: "neutral",
    scope: "资产中心",
    path: "assets/skills/react-review",
    icon: BookOpen,
    updated: "6 月 18 日",
    mounts: [],
    preview: "# React Review\n\n检查状态边界、可访问性和渲染性能。",
    searchTerms: ["React", "组件"],
  },
  {
    id: "api-design",
    name: "api-design",
    title: "API 设计评审",
    category: "架构设计",
    summary: "检查 API 契约、一致性与演进兼容性",
    status: "已挂载",
    statusTone: "success",
    scope: "项目级",
    path: "assets/skills/api-design",
    icon: BookOpen,
    updated: "今天 08:45",
    mounts: ["my-app/.claude/skills/api-design"],
    preview: "# API Design\n\n检查资源建模、错误语义、版本策略和兼容边界。",
    searchTerms: ["API", "契约", "架构"],
  },
];

export function SkillsListPage() {
  return (
    <AssetCenterLayout
      actionLabel="挂载 Skill"
      itemLabel="Skills"
      items={skills}
      searchPlaceholder="搜索 Skill 名称、路径或作用域"
      renderInspector={(skill) => (
        <InspectorCode label="SKILL.md 预览">{skill.preview}</InspectorCode>
      )}
    />
  );
}
