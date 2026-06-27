import { Blocks } from "lucide-react";
import { useEffect, useState } from "react";
import { listAssets } from "../app/data-api";
import type { AssetSummary } from "../app/contracts";
import type { AssetDetailContext } from "../app/detail-context";
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

const staticServers: readonly McpItem[] = [
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

type AssetListPageProps = {
  onOpenAssetDetail?: (detail: AssetDetailContext) => void;
};

export function McpServersListPage({ onOpenAssetDetail }: AssetListPageProps = {}) {
  const [items, setItems] = useState<readonly McpItem[]>(staticServers);
  const [stateLabel, setStateLabel] = useState("读取中");

  useEffect(() => {
    let cancelled = false;
    setStateLabel("读取中");
    listAssets({ assetType: "mcp" })
      .then((assets) => {
        if (cancelled) return;
        if (Array.isArray(assets) && assets.length > 0) {
          setItems(assets.map(toMcpItem));
          setStateLabel("只读真实数据");
        } else {
          setItems(staticServers);
          setStateLabel("静态预览");
        }
      })
      .catch(() => {
        if (cancelled) return;
        setItems(staticServers);
        setStateLabel("读取失败，使用静态预览");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <AssetCenterLayout
      actionLabel="挂载 MCP"
      itemLabel="MCP Servers"
      items={items}
      searchPlaceholder="搜索 MCP 名称、能力或配置路径"
      stateLabel={stateLabel}
      onOpenDetail={onOpenAssetDetail ? (server) => onOpenAssetDetail(toAssetDetail(server, "MCP Server", "配置 JSON 预览")) : undefined}
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

function toAssetDetail(server: McpItem, typeLabel: string, previewLabel: string): AssetDetailContext {
  return {
    assetId: `mcp:${server.name}`,
    assetType: "mcp",
    name: server.name,
    title: server.title,
    summary: server.summary,
    status: server.status,
    statusTone: server.statusTone,
    typeLabel,
    category: server.category,
    sourcePath: server.path,
    scope: server.scope,
    updated: server.updated,
    mountTargets: server.mounts,
    previewLabel,
    preview: server.preview,
  };
}

function toMcpItem(asset: AssetSummary): McpItem {
  return {
    id: asset.id,
    name: asset.name,
    title: asset.title,
    category: asset.category || "MCP Server",
    updated: asset.updatedAt ?? "未知",
    mounts: asset.mountTargets,
    summary: asset.description || "本地 MCP 配置",
    status: asset.status === "invalid" ? "配置无效" : "配置正常",
    statusTone: asset.status === "invalid" ? "warning" : "success",
    scope: scopeLabel(asset.scope),
    path: asset.sourcePath,
    icon: Blocks,
    transport: "本地配置",
    source: asset.category || "资产中心",
    capabilities: [asset.assetType, asset.status],
    preview: `{\n  "name": "${asset.name}",\n  "sourcePath": "${asset.sourcePath}"\n}`,
    searchTerms: [asset.assetType, asset.status],
  };
}

function scopeLabel(scope: AssetSummary["scope"]) {
  if (scope === "user") return "用户级";
  if (scope === "project") return "项目级";
  return "资产中心";
}
