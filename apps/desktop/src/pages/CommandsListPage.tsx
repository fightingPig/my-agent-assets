import { TerminalSquare } from "lucide-react";
import { useEffect, useState } from "react";
import { listAssets } from "../app/data-api";
import type { AssetSummary } from "../app/contracts";
import type { AssetDetailContext } from "../app/detail-context";
import {
  AssetCenterLayout,
  InspectorCode,
  InspectorSection,
  InspectorTags,
  type AssetCenterItem,
} from "../components/assets/AssetCenterLayout";

type CommandItem = AssetCenterItem & {
  tags: readonly string[];
  preview: string;
};

const staticCommands: readonly CommandItem[] = [
  {
    id: "deploy-prod",
    name: "deploy-prod",
    title: "生产环境部署",
    category: "交付流程",
    summary: "生成生产部署检查与执行步骤",
    status: "可用",
    statusTone: "success",
    scope: "用户级",
    path: "assets/commands/deploy-prod.md",
    icon: TerminalSquare,
    updated: "今天 09:40",
    mounts: ["~/.claude/commands/deploy-prod.md", "project-a/.claude/commands/deploy-prod.md"],
    tags: ["部署", "检查清单"],
    preview: "# Deploy Production\n\n生成部署计划，检查构建产物并输出确认清单。",
    searchTerms: ["production", "部署"],
  },
  {
    id: "build-project",
    name: "build-project",
    title: "项目构建",
    category: "构建工具",
    summary: "执行项目构建并汇总构建结果",
    status: "可用",
    statusTone: "success",
    scope: "项目级",
    path: "assets/commands/build-project.md",
    icon: TerminalSquare,
    updated: "昨天 14:22",
    mounts: ["project-a/.claude/commands/build-project.md", "my-app/.claude/commands/build-project.md"],
    tags: ["构建", "项目"],
    preview: "# Build Project\n\n检测项目工具链，执行构建并整理错误摘要。",
    searchTerms: ["build", "构建"],
  },
  {
    id: "run-tests",
    name: "run-tests",
    title: "运行测试套件",
    category: "质量保障",
    summary: "运行测试套件并定位失败用例",
    status: "待检查",
    statusTone: "warning",
    scope: "资产中心",
    path: "assets/commands/run-tests.md",
    icon: TerminalSquare,
    updated: "6 月 19 日",
    mounts: ["my-app/.claude/commands/run-tests.md"],
    tags: ["测试", "质量"],
    preview: "# Run Tests\n\n选择匹配的测试命令，执行后汇总失败原因。",
    searchTerms: ["test", "测试"],
  },
  {
    id: "format-code",
    name: "format-code",
    title: "格式化项目代码",
    category: "代码质量",
    summary: "识别项目格式化工具并生成安全执行计划",
    status: "可用",
    statusTone: "success",
    scope: "项目级",
    path: "assets/commands/format-code.md",
    icon: TerminalSquare,
    updated: "6 月 17 日",
    mounts: ["design-system/.claude/commands/format-code.md"],
    tags: ["格式化", "代码质量"],
    preview: "# Format Code\n\n识别 formatter 配置，预览影响范围后执行格式化。",
    searchTerms: ["format", "formatter", "格式化"],
  },
];

type AssetListPageProps = {
  demoMode?: boolean;
  onOpenAssetDetail?: (detail: AssetDetailContext) => void;
};

export function CommandsListPage({ demoMode = false, onOpenAssetDetail }: AssetListPageProps = {}) {
  const [items, setItems] = useState<readonly CommandItem[]>(demoMode ? staticCommands : []);
  const [stateLabel, setStateLabel] = useState("读取中");

  useEffect(() => {
    let cancelled = false;
    if (demoMode) {
      setItems(staticCommands);
      setStateLabel("Visual QA 示例数据");
      return undefined;
    }
    setItems([]);
    setStateLabel("读取中");
    listAssets({ assetType: "command" })
      .then((assets) => {
        if (cancelled) return;
        const mapped = assets.map(toCommandItem);
        setItems(mapped);
        setStateLabel(mapped.length > 0 ? "只读真实数据" : "未发现本地数据");
      })
      .catch((error) => {
        if (cancelled) return;
        setItems([]);
        setStateLabel(`读取失败：${errorMessage(error)}`);
      });
    return () => {
      cancelled = true;
    };
  }, [demoMode]);

  return (
    <AssetCenterLayout
      actionLabel="挂载 Command"
      emptyDescription="请先扫描或导入 Claude Command。"
      emptyTitle="未发现 Commands"
      itemLabel="Commands"
      items={items}
      searchPlaceholder="搜索 Command 名称、用途或路径"
      stateLabel={stateLabel}
      onOpenDetail={onOpenAssetDetail ? (command) => onOpenAssetDetail(toAssetDetail(command, "Command", "Markdown 内容预览")) : undefined}
      renderInspector={(command) => (
        <>
          <InspectorSection title="用途标签"><InspectorTags tags={command.tags} /></InspectorSection>
          <InspectorCode label="Markdown 预览">{command.preview}</InspectorCode>
        </>
      )}
    />
  );
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "无法读取本地 Command。";
}

function toAssetDetail(command: CommandItem, typeLabel: string, previewLabel: string): AssetDetailContext {
  return {
    assetId: `command:${command.name}`,
    assetType: "command",
    name: command.name,
    title: command.title,
    summary: command.summary,
    status: command.status,
    statusTone: command.statusTone,
    typeLabel,
    category: command.category,
    sourcePath: command.path,
    scope: command.scope,
    updated: command.updated,
    mountTargets: command.mounts,
    previewLabel,
    preview: command.preview,
  };
}

function toCommandItem(asset: AssetSummary): CommandItem {
  return {
    id: asset.id,
    name: asset.name,
    title: asset.title,
    category: asset.category || "本地 Command",
    summary: asset.description || "本地命令资产",
    status: asset.status === "invalid" ? "无效" : "可用",
    statusTone: asset.status === "invalid" ? "warning" : "success",
    scope: scopeLabel(asset.scope),
    path: asset.sourcePath,
    icon: TerminalSquare,
    updated: asset.updatedAt ?? "未知",
    mounts: asset.mountTargets,
    tags: [asset.assetType, asset.status],
    preview: asset.description ? `# ${asset.name}\n\n${asset.description}` : `# ${asset.name}`,
    searchTerms: [asset.assetType, asset.status],
  };
}

function scopeLabel(scope: AssetSummary["scope"]) {
  if (scope === "user") return "用户级";
  if (scope === "project") return "项目级";
  return "资产中心";
}
