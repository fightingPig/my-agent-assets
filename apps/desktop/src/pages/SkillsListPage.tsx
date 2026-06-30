import { BookOpen } from "lucide-react";
import { useEffect, useState } from "react";
import { discoverRuntimeSources, listAssets } from "../app/data-api";
import type { AssetSummary, DiscoveredRuntimeSource } from "../app/contracts";
import type { AssetDetailContext } from "../app/detail-context";
import type { AssetProvider } from "../app/provider";
import {
  AssetCenterLayout,
  InspectorCode,
  type AssetCenterItem,
} from "../components/assets/AssetCenterLayout";

type SkillItem = AssetCenterItem & {
  preview: string;
};

const staticSkills: readonly SkillItem[] = [
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

type AssetListPageProps = {
  demoMode?: boolean;
  onOpenAssetDetail?: (detail: AssetDetailContext) => void;
  provider?: AssetProvider;
};

export function SkillsListPage({
  demoMode = false,
  onOpenAssetDetail,
  provider = "claude",
}: AssetListPageProps = {}) {
  const [items, setItems] = useState<readonly SkillItem[]>(demoMode ? staticSkills : []);
  const [stateLabel, setStateLabel] = useState("读取中");

  useEffect(() => {
    let cancelled = false;
    if (demoMode) {
      setItems(staticSkills);
      setStateLabel("Visual QA 示例数据");
      return undefined;
    }

    setItems([]);
    setStateLabel("读取中");
    const request = provider === "codex"
      ? discoverRuntimeSources({ kind: "user" }).then((result) => result.sources
        .filter((source) => source.provider === "codex" && source.assetKind === "skill")
        .map(toCodexSkillItem))
      : listAssets({ assetType: "skill" }).then((assets) => assets.map(toSkillItem));
    request
      .then((assets) => {
        if (cancelled) return;
        setItems(assets);
        setStateLabel(assets.length > 0 ? "只读真实数据" : "未发现本地数据");
      })
      .catch((error) => {
        if (cancelled) return;
        setItems([]);
        setStateLabel(`读取失败：${errorMessage(error)}`);
      });
    return () => {
      cancelled = true;
    };
  }, [demoMode, provider]);

  return (
    <AssetCenterLayout
      actionLabel={provider === "codex" ? "Codex Skill 只读" : "挂载 Skill"}
      emptyDescription={provider === "codex"
        ? "请在 ~/.agents/skills、项目 .agents/skills 或 /etc/codex/skills 中添加包含 SKILL.md 的目录。"
        : "请先扫描或导入 Claude Skill。"}
      emptyTitle={provider === "codex" ? "未发现 Codex Skills" : "未发现 Skills"}
      itemLabel="Skills"
      items={items}
      searchPlaceholder="搜索 Skill 名称、路径或作用域"
      stateLabel={stateLabel}
      usageLabel={provider === "codex" ? "内容特征" : "挂载与使用"}
      usageCountLabel={provider === "codex" ? "项特征" : "个挂载"}
      onOpenDetail={provider === "claude" && onOpenAssetDetail
        ? (skill) => onOpenAssetDetail(toAssetDetail(skill, "Skill", "SKILL.md 内容预览"))
        : undefined}
      renderInspector={(skill) => (
        <InspectorCode label="SKILL.md 预览">{skill.preview}</InspectorCode>
      )}
    />
  );
}

function toCodexSkillItem(skill: DiscoveredRuntimeSource): SkillItem {
  const features = [
    skill.sourceFormat,
    skill.isSymlink ? "symlink" : null,
    skill.eligibleImport ? "可导入" : null,
  ].filter((value): value is string => Boolean(value));
  const warningText = skill.warnings.length > 0
    ? `\n\nWarnings:\n${skill.warnings.map((warning) => `- ${warning}`).join("\n")}`
    : "";
  const symlinkText = skill.symlinkTarget ? `\n\nSymlink target: ${skill.symlinkTarget}` : "";

  return {
    id: skill.sourceId,
    name: skill.assetName,
    title: skill.assetName,
    category: "Codex Skill",
    summary: "Shared core 发现的本地 Codex Skill",
    status: skill.warnings.length > 0 ? "需要检查" : "已发现",
    statusTone: skill.warnings.length > 0 ? "warning" : "success",
    scope: codexScopeLabel(skill.scope),
    path: skill.sourcePath,
    icon: BookOpen,
    updated: "本地来源",
    mounts: features,
    preview: `# ${skill.assetName}\n\n来源格式：${skill.sourceFormat}${symlinkText}${warningText}`,
    searchTerms: [skill.scope, ...features, ...skill.warnings],
  };
}

function toAssetDetail(skill: SkillItem, typeLabel: string, previewLabel: string): AssetDetailContext {
  return {
    assetId: `skill:${skill.name}`,
    assetType: "skill",
    name: skill.name,
    title: skill.title,
    summary: skill.summary,
    status: skill.status,
    statusTone: skill.statusTone,
    typeLabel,
    category: skill.category,
    sourcePath: skill.path,
    scope: skill.scope,
    updated: skill.updated,
    mountTargets: skill.mounts,
    previewLabel,
    preview: skill.preview,
  };
}

function toSkillItem(asset: AssetSummary): SkillItem {
  return {
    id: asset.id,
    name: asset.name,
    title: asset.title,
    category: asset.category || "本地 Skill",
    summary: asset.description || "本地 Skill 资产",
    status: asset.status === "invalid" ? "无效" : asset.mountTargets.length > 0 ? "已挂载" : "可用",
    statusTone: asset.status === "invalid" ? "warning" : "success",
    scope: scopeLabel(asset.scope),
    path: asset.sourcePath,
    icon: BookOpen,
    updated: asset.updatedAt ?? "未知",
    mounts: asset.mountTargets,
    preview: asset.description ? `# ${asset.name}\n\n${asset.description}` : `# ${asset.name}`,
    searchTerms: [asset.assetType, asset.status],
  };
}

function scopeLabel(scope: AssetSummary["scope"]) {
  if (scope === "user") return "用户级";
  if (scope === "project") return "项目级";
  return "资产中心";
}

function codexScopeLabel(scope: DiscoveredRuntimeSource["scope"]) {
  if (scope === "user") return "用户级";
  if (scope === "project") return "项目级";
  return "自定义";
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "无法读取本地 Skill。";
}
