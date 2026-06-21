import { Blocks } from "lucide-react";
import {
  AssetCenterLayout,
  InspectorCode,
  InspectorFields,
  InspectorSection,
  InspectorTags,
  type AssetCenterItem,
} from "../components/assets/AssetCenterLayout";

type McpItem = AssetCenterItem & {
  transport: string;
  source: string;
  capabilities: readonly string[];
  preview: string;
};

const servers: readonly McpItem[] = [
  {
    id: "postgresql",
    name: "PostgreSQL",
    title: "PostgreSQL 数据访问",
    category: "数据库",
    updated: "今天 10:12",
    mounts: ["project-a/.mcp.json"],
    summary: "本地数据库查询与结构检查",
    status: "配置正常",
    statusTone: "success",
    scope: "用户级",
    path: "assets/mcps/postgresql.json",
    icon: Blocks,
    transport: "stdio",
    source: "本地配置",
    capabilities: ["查询", "Schema", "只读"],
    preview: "{\n  \"command\": \"postgres-mcp\",\n  \"args\": [\"--read-only\"]\n}",
    searchTerms: ["database", "数据库"],
  },
  {
    id: "redis",
    name: "Redis",
    title: "Redis 缓存检查",
    category: "数据库",
    updated: "昨天 18:30",
    mounts: ["my-app/.mcp.json"],
    summary: "本地缓存键值与状态检查",
    status: "待检查",
    statusTone: "warning",
    scope: "用户级",
    path: "assets/mcps/redis.json",
    icon: Blocks,
    transport: "stdio",
    source: "本地配置",
    capabilities: ["键值", "缓存", "只读"],
    preview: "{\n  \"command\": \"redis-mcp\",\n  \"args\": [\"--inspect\"]\n}",
    searchTerms: ["cache", "缓存"],
  },
  {
    id: "filesystem",
    name: "Filesystem",
    title: "本地文件访问",
    category: "文件系统",
    updated: "今天 09:05",
    mounts: ["my-app/.mcp.json"],
    summary: "项目目录与文件内容访问",
    status: "配置正常",
    statusTone: "success",
    scope: "项目级",
    path: "assets/mcps/filesystem.json",
    icon: Blocks,
    transport: "stdio",
    source: "项目配置",
    capabilities: ["目录", "文件", "受限路径"],
    preview: "{\n  \"command\": \"filesystem-mcp\",\n  \"args\": [\"./workspace\"]\n}",
    searchTerms: ["files", "文件"],
  },
  {
    id: "sqlite",
    name: "SQLite",
    title: "SQLite 数据访问",
    category: "数据库",
    updated: "3 天前",
    mounts: [],
    summary: "本地 SQLite 文件查询",
    status: "未启用",
    statusTone: "neutral",
    scope: "资产中心",
    path: "assets/mcps/sqlite.json",
    icon: Blocks,
    transport: "stdio",
    source: "本地配置",
    capabilities: ["查询", "表结构", "本地文件"],
    preview: "{\n  \"command\": \"sqlite-mcp\",\n  \"args\": [\"./data/app.db\"]\n}",
    searchTerms: ["database", "本地文件"],
  },
];

export function McpServersListPage() {
  return (
    <AssetCenterLayout
      actionLabel="挂载 MCP"
      itemLabel="MCP Servers"
      items={servers}
      searchPlaceholder="搜索 MCP 名称、能力或配置路径"
      renderInspector={(server) => (
        <>
          <InspectorFields fields={[
            { label: "Transport", value: server.transport },
            { label: "配置来源", value: server.source },
          ]} />
          <InspectorSection title="能力范围"><InspectorTags tags={server.capabilities} /></InspectorSection>
          <InspectorCode label="配置 JSON 预览">{server.preview}</InspectorCode>
        </>
      )}
    />
  );
}
