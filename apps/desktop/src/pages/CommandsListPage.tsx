import { TerminalSquare } from "lucide-react";
import {
  AssetCenterLayout,
  InspectorCode,
  InspectorFields,
  InspectorSection,
  InspectorTags,
  type AssetCenterItem,
} from "../components/assets/AssetCenterLayout";

type CommandItem = AssetCenterItem & {
  updated: string;
  tags: readonly string[];
  preview: string;
};

const commands: readonly CommandItem[] = [
  {
    id: "deploy-prod",
    name: "deploy-prod",
    summary: "生成生产部署检查与执行步骤",
    status: "可用",
    statusTone: "success",
    scope: "用户级",
    path: "assets/commands/deploy-prod.md",
    icon: TerminalSquare,
    updated: "今天 09:40",
    tags: ["部署", "检查清单"],
    preview: "# Deploy Production\n\n生成部署计划，检查构建产物并输出确认清单。",
    searchTerms: ["production", "部署"],
  },
  {
    id: "build-project",
    name: "build-project",
    summary: "执行项目构建并汇总构建结果",
    status: "可用",
    statusTone: "success",
    scope: "项目级",
    path: "assets/commands/build-project.md",
    icon: TerminalSquare,
    updated: "昨天 14:22",
    tags: ["构建", "项目"],
    preview: "# Build Project\n\n检测项目工具链，执行构建并整理错误摘要。",
    searchTerms: ["build", "构建"],
  },
  {
    id: "run-tests",
    name: "run-tests",
    summary: "运行测试套件并定位失败用例",
    status: "待检查",
    statusTone: "warning",
    scope: "资产中心",
    path: "assets/commands/run-tests.md",
    icon: TerminalSquare,
    updated: "6 月 19 日",
    tags: ["测试", "质量"],
    preview: "# Run Tests\n\n选择匹配的测试命令，执行后汇总失败原因。",
    searchTerms: ["test", "测试"],
  },
];

export function CommandsListPage() {
  return (
    <AssetCenterLayout
      actionLabel="挂载 Command"
      itemLabel="Commands"
      items={commands}
      searchPlaceholder="搜索 Command 名称、用途或路径"
      renderInspector={(command) => (
        <>
          <InspectorFields fields={[
            { label: "类型", value: "Command" },
            { label: "作用域", value: command.scope },
            { label: "文件路径", value: command.path },
            { label: "最近更新", value: command.updated },
          ]} />
          <InspectorSection title="用途标签"><InspectorTags tags={command.tags} /></InspectorSection>
          <InspectorCode label="Markdown 预览">{command.preview}</InspectorCode>
        </>
      )}
    />
  );
}
