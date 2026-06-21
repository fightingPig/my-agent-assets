import { BookOpen } from "lucide-react";
import {
  AssetCenterLayout,
  InspectorCode,
  InspectorFields,
  InspectorSection,
  InspectorTags,
  type AssetCenterItem,
} from "../components/assets/AssetCenterLayout";

type SkillItem = AssetCenterItem & {
  updated: string;
  mounts: readonly string[];
  preview: string;
};

const skills: readonly SkillItem[] = [
  {
    id: "review",
    name: "review",
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
];

export function SkillsListPage() {
  return (
    <AssetCenterLayout
      actionLabel="挂载 Skill"
      itemLabel="Skills"
      items={skills}
      searchPlaceholder="搜索 Skill 名称、路径或作用域"
      renderInspector={(skill) => (
        <>
          <InspectorFields fields={[
            { label: "类型", value: "Skill" },
            { label: "作用域", value: skill.scope },
            { label: "资产路径", value: skill.path },
            { label: "最近更新", value: skill.updated },
          ]} />
          <InspectorSection title="挂载目标">
            {skill.mounts.length > 0 ? <InspectorTags tags={skill.mounts} /> : <p className="asset-muted-copy">当前没有挂载目标。</p>}
          </InspectorSection>
          <InspectorCode label="SKILL.md 预览">{skill.preview}</InspectorCode>
        </>
      )}
    />
  );
}
