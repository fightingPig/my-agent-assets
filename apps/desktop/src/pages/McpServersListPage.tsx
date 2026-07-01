import { AlertTriangle, Blocks, Plus, RefreshCw, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  canonicalMcpGet,
  canonicalMcpSaveApply,
  canonicalMcpSavePreview,
  canonicalMountApply,
  canonicalMountPreview,
  listAssets,
} from "../app/data-api";
import type {
  AssetSummary,
  CanonicalMcp,
  McpAssetDefinition,
  McpSavePreview,
  McpSavePreviewRequest,
  McpTransport,
  CanonicalMountPreview,
} from "../app/contracts";
import type { AssetDetailContext } from "../app/detail-context";
import type { AssetProvider } from "../app/provider";
import {
  AssetCenterLayout,
  InspectorCode,
  InspectorFields,
  InspectorSection,
  InspectorTags,
  type AssetCenterItem,
} from "../components/assets/AssetCenterLayout";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

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
  demoMode?: boolean;
  onOpenAssetDetail?: (detail: AssetDetailContext) => void;
  provider?: AssetProvider;
};

type McpEditorState = {
  assetId?: string;
  name: string;
  title: string;
  description: string;
  transport: McpTransport;
  command: string;
  argsText: string;
  cwd: string;
  url: string;
  envJson: string;
  headersJson: string;
  extraJson: string;
  providerExtensionsJson: string;
  bindings: McpAssetDefinition["bindings"];
};

type McpEditorProps = {
  editor: McpEditorState;
  savePreview: McpSavePreview | null;
  syncPreviews: Record<string, CanonicalMountPreview>;
  busy: boolean;
  message: string;
  onChange: (editor: McpEditorState) => void;
  onClose: () => void;
  onPreviewSave: () => void;
  onApplySave: () => void;
  onPreviewTargetSync: (targetId: string) => void;
  onApplyTargetSync: (targetId: string) => void;
};

export function McpServersListPage({
  demoMode = false,
  onOpenAssetDetail,
  provider = "claude",
}: AssetListPageProps = {}) {
  const [items, setItems] = useState<readonly McpItem[]>(demoMode ? staticServers : []);
  const [stateLabel, setStateLabel] = useState("读取中");
  const [editor, setEditor] = useState<McpEditorState | null>(null);
  const [savePreview, setSavePreview] = useState<McpSavePreview | null>(null);
  const [editorMessage, setEditorMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const [syncPreviews, setSyncPreviews] = useState<Record<string, CanonicalMountPreview>>({});

  useEffect(() => {
    let cancelled = false;
    if (demoMode) {
      setItems(staticServers);
      setStateLabel("Visual QA 示例数据");
      return undefined;
    }
    setItems([]);
    setStateLabel("读取中");
    const request = listAssets({ assetType: "mcp" }).then((assets) => assets.map(toMcpItem));
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
  }, [demoMode, provider, refreshKey]);

  const openCreate = () => {
    setEditor(emptyEditor());
    setSavePreview(null);
    setSyncPreviews({});
    setEditorMessage("");
  };

  const openEdit = async (server: McpItem) => {
    setBusy(true);
    setEditorMessage("");
    try {
      const definition = await canonicalMcpGet(server.id.startsWith("mcp:") ? server.id : `mcp:${server.name}`);
      setEditor(editorFromDefinition(definition));
      setSavePreview(null);
      setSyncPreviews({});
    } catch (error) {
      setEditorMessage(errorMessage(error));
    } finally {
      setBusy(false);
    }
  };

  const requestSavePreview = async () => {
    if (!editor) return;
    setBusy(true);
    setEditorMessage("");
    setSavePreview(null);
    try {
      const request = editorRequest(editor);
      const preview = await canonicalMcpSavePreview(request);
      setSavePreview(preview);
      setEditorMessage(preview.canApply ? "保存预览已生成，请确认影响后保存。" : "保存被兼容性校验阻止。");
    } catch (error) {
      setEditorMessage(errorMessage(error));
    } finally {
      setBusy(false);
    }
  };

  const applySave = async () => {
    if (!editor || !savePreview?.canApply) return;
    setBusy(true);
    setEditorMessage("");
    try {
      await canonicalMcpSaveApply({
        previewId: savePreview.previewId,
        previewGeneratedAtEpochSeconds: savePreview.generatedAtEpochSeconds,
        request: editorRequest(editor),
      });
      setEditorMessage(
        savePreview.outOfSyncTargetIds.length > 0
          ? "Canonical 配置已保存；现有目标已标记 outOfSync，请逐目标显式同步。"
          : "Canonical 配置已保存。",
      );
      setSavePreview(null);
      setRefreshKey((value) => value + 1);
      if (editor.assetId) {
        const definition = await canonicalMcpGet(editor.assetId);
        setEditor(editorFromDefinition(definition));
      }
    } catch (error) {
      setEditorMessage(errorMessage(error));
    } finally {
      setBusy(false);
    }
  };

  const previewTargetSync = async (targetId: string) => {
    if (!editor?.assetId) return;
    setBusy(true);
    setEditorMessage("");
    try {
      const preview = await canonicalMountPreview({ assetId: editor.assetId, targetId });
      setSyncPreviews((current) => ({ ...current, [targetId]: preview }));
    } catch (error) {
      setEditorMessage(errorMessage(error));
    } finally {
      setBusy(false);
    }
  };

  const applyTargetSync = async (targetId: string) => {
    if (!editor?.assetId) return;
    const preview = syncPreviews[targetId];
    if (!preview?.canApply) return;
    setBusy(true);
    setEditorMessage("");
    try {
      await canonicalMountApply({
        previewId: preview.previewId,
        previewGeneratedAtEpochSeconds: preview.generatedAtEpochSeconds,
        request: { assetId: editor.assetId, targetId },
      });
      const definition = await canonicalMcpGet(editor.assetId);
      setEditor(editorFromDefinition(definition));
      setSyncPreviews((current) => {
        const next = { ...current };
        delete next[targetId];
        return next;
      });
      setEditorMessage(`目标 ${targetId} 已显式同步。`);
      setRefreshKey((value) => value + 1);
    } catch (error) {
      setEditorMessage(errorMessage(error));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="mcp-page-stack">
      <div className="mcp-page-actions">
        <div>
          <strong>Canonical MCP</strong>
          <span>统一模型是唯一真实配置；保存不会自动写入 Claude Code 或 Codex。</span>
        </div>
        <button className="primary-button" data-no-drag="true" onClick={openCreate} style={NO_DRAG_REGION_STYLE} type="button"><Plus size={15} />新增 MCP</button>
      </div>
      {editor ? (
        <McpEditor
          busy={busy}
          editor={editor}
          message={editorMessage}
          onApplySave={applySave}
          onChange={(next) => {
            setEditor(next);
            setSavePreview(null);
            setSyncPreviews({});
          }}
          onClose={() => {
            setEditor(null);
            setSavePreview(null);
            setSyncPreviews({});
            setEditorMessage("");
          }}
          onPreviewSave={requestSavePreview}
          onPreviewTargetSync={previewTargetSync}
          onApplyTargetSync={applyTargetSync}
          savePreview={savePreview}
          syncPreviews={syncPreviews}
        />
      ) : editorMessage ? <p className="mcp-editor-message error">{editorMessage}</p> : null}
      <AssetCenterLayout
      emptyDescription="请新增 canonical MCP，或从 Claude Code / Codex 扫描并导入。"
      emptyTitle="未发现 MCP Servers"
      itemLabel="MCP Servers"
      items={items}
      searchPlaceholder="搜索 MCP 名称、能力或配置路径"
      stateLabel={stateLabel}
      usageLabel="挂载与使用"
      usageCountLabel="个挂载"
      onOpenDetail={onOpenAssetDetail
        ? (server) => onOpenAssetDetail(toAssetDetail(server, "MCP Server", "配置 JSON 预览"))
        : undefined}
      renderActions={(server) => !demoMode ? (
        <button className="asset-business-action" data-no-drag="true" disabled={busy} onClick={() => void openEdit(server)} style={NO_DRAG_REGION_STYLE} type="button">编辑配置</button>
      ) : null}
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
    </div>
  );
}

function McpEditor({
  editor,
  savePreview,
  syncPreviews,
  busy,
  message,
  onChange,
  onClose,
  onPreviewSave,
  onApplySave,
  onPreviewTargetSync,
  onApplyTargetSync,
}: McpEditorProps) {
  const update = <K extends keyof McpEditorState>(key: K, value: McpEditorState[K]) => {
    onChange({ ...editor, [key]: value });
  };
  const canonicalPreview = useMemo(() => {
    try {
      return JSON.stringify(editorRequest(editor).canonical, null, 2);
    } catch (error) {
      return `配置尚未通过 JSON 校验：${errorMessage(error)}`;
    }
  }, [editor]);

  return (
    <section className="panel mcp-editor-panel" aria-label={editor.assetId ? "编辑 MCP" : "新增 MCP"}>
      <div className="mcp-editor-heading">
        <div>
          <small>{editor.assetId ? "编辑 canonical definition" : "创建 canonical definition"}</small>
          <h2>{editor.assetId ?? "新 MCP Server"}</h2>
        </div>
        <button aria-label="关闭 MCP 编辑器" className="icon-button" data-no-drag="true" onClick={onClose} style={NO_DRAG_REGION_STYLE} type="button"><X size={16} /></button>
      </div>

      <div className="mcp-editor-grid">
        <div className="mcp-editor-fields">
          <label><span>名称 / Asset ID</span><input data-no-drag="true" disabled={Boolean(editor.assetId)} onChange={(event) => update("name", event.target.value)} style={NO_DRAG_REGION_STYLE} value={editor.name} /></label>
          <label><span>标题</span><input data-no-drag="true" onChange={(event) => update("title", event.target.value)} style={NO_DRAG_REGION_STYLE} value={editor.title} /></label>
          <label className="mcp-field-wide"><span>描述</span><input data-no-drag="true" onChange={(event) => update("description", event.target.value)} style={NO_DRAG_REGION_STYLE} value={editor.description} /></label>
          <label><span>Transport</span><select data-no-drag="true" onChange={(event) => update("transport", event.target.value as McpTransport)} style={NO_DRAG_REGION_STYLE} value={editor.transport}><option value="stdio">stdio</option><option value="http">http</option><option value="sse">sse</option></select></label>
          {editor.transport === "stdio" ? (
            <>
              <label><span>Command</span><input data-no-drag="true" onChange={(event) => update("command", event.target.value)} style={NO_DRAG_REGION_STYLE} value={editor.command} /></label>
              <label className="mcp-field-wide"><span>Args（每行一个）</span><textarea data-no-drag="true" onChange={(event) => update("argsText", event.target.value)} style={NO_DRAG_REGION_STYLE} value={editor.argsText} /></label>
              <label className="mcp-field-wide"><span>Working directory</span><input data-no-drag="true" onChange={(event) => update("cwd", event.target.value)} style={NO_DRAG_REGION_STYLE} value={editor.cwd} /></label>
              <JsonField label="Environment JSON" value={editor.envJson} onChange={(value) => update("envJson", value)} />
            </>
          ) : (
            <>
              <label className="mcp-field-wide"><span>URL</span><input data-no-drag="true" onChange={(event) => update("url", event.target.value)} style={NO_DRAG_REGION_STYLE} value={editor.url} /></label>
              <JsonField label="Headers JSON" value={editor.headersJson} onChange={(value) => update("headersJson", value)} />
            </>
          )}
          <JsonField label="高级 spec 字段 JSON" value={editor.extraJson} onChange={(value) => update("extraJson", value)} />
          <JsonField label="Provider extensions JSON" value={editor.providerExtensionsJson} onChange={(value) => update("providerExtensionsJson", value)} />
        </div>
        <div className="mcp-editor-preview">
          <strong>Canonical JSON 预览</strong>
          <pre><code>{canonicalPreview}</code></pre>
        </div>
      </div>

      {savePreview ? (
        <div className={`mcp-save-preview ${savePreview.canApply ? "" : "blocked"}`}>
          <div><strong>{savePreview.operation === "create" ? "新增计划" : "编辑计划"}</strong><span>{savePreview.canonicalPath}</span></div>
          {savePreview.plannedEffects.map((effect) => <p key={effect}>{effect}</p>)}
          {savePreview.targetCompatibility.map((target) => (
            <p key={target.targetId}>{target.targetId}: {target.compatible ? "兼容" : `阻止 · ${target.blockedReason}`}</p>
          ))}
          {savePreview.warnings.map((warning) => <p className="warning" key={warning}>{warning}</p>)}
        </div>
      ) : null}

      {editor.bindings.length > 0 ? (
        <div className="mcp-binding-list">
          <strong>目标同步状态</strong>
          {editor.bindings.map((binding) => {
            const preview = syncPreviews[binding.targetId];
            return (
              <div key={binding.targetId}>
                <span><b>{binding.targetId}</b><small>{binding.status}</small></span>
                {preview ? (
                  <button className="asset-business-action" data-no-drag="true" disabled={busy || !preview.canApply} onClick={() => onApplyTargetSync(binding.targetId)} style={NO_DRAG_REGION_STYLE} type="button">确认同步</button>
                ) : (
                  <button className="asset-secondary-action" data-no-drag="true" disabled={busy || binding.status === "mounted"} onClick={() => onPreviewTargetSync(binding.targetId)} style={NO_DRAG_REGION_STYLE} type="button"><RefreshCw size={13} />生成同步预览</button>
                )}
              </div>
            );
          })}
          <p><AlertTriangle size={13} />只有“确认同步”会精确 patch 对应 Claude/Codex live config；保存 canonical 不会自动同步。</p>
        </div>
      ) : null}

      {message ? <p className={`mcp-editor-message ${savePreview && !savePreview.canApply ? "error" : ""}`}>{message}</p> : null}
      <div className="mcp-editor-actions">
        <button className="asset-secondary-action" data-no-drag="true" disabled={busy} onClick={onPreviewSave} style={NO_DRAG_REGION_STYLE} type="button">生成保存预览</button>
        <button className="asset-business-action" data-no-drag="true" disabled={busy || !savePreview?.canApply} onClick={onApplySave} style={NO_DRAG_REGION_STYLE} type="button">确认保存</button>
      </div>
    </section>
  );
}

function JsonField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return <label className="mcp-field-wide"><span>{label}</span><textarea data-no-drag="true" onChange={(event) => onChange(event.target.value)} spellCheck={false} style={NO_DRAG_REGION_STYLE} value={value} /></label>;
}

function emptyEditor(): McpEditorState {
  return {
    name: "",
    title: "",
    description: "",
    transport: "stdio",
    command: "",
    argsText: "",
    cwd: "",
    url: "",
    envJson: "{}",
    headersJson: "{}",
    extraJson: "{}",
    providerExtensionsJson: "{}",
    bindings: [],
  };
}

function editorFromDefinition(definition: McpAssetDefinition): McpEditorState {
  const { canonical } = definition;
  const knownKeys = new Set(["type", "command", "args", "env", "cwd", "url", "headers"]);
  const extra = Object.fromEntries(
    Object.entries(canonical.spec).filter(([key]) => !knownKeys.has(key)),
  );
  return {
    assetId: definition.assetId,
    name: canonical.name,
    title: definition.title ?? "",
    description: definition.description ?? "",
    transport: canonical.spec.type ?? "stdio",
    command: canonical.spec.command ?? "",
    argsText: canonical.spec.args?.join("\n") ?? "",
    cwd: canonical.spec.cwd ?? "",
    url: canonical.spec.url ?? "",
    envJson: JSON.stringify(canonical.spec.env ?? {}, null, 2),
    headersJson: JSON.stringify(canonical.spec.headers ?? {}, null, 2),
    extraJson: JSON.stringify(extra, null, 2),
    providerExtensionsJson: JSON.stringify(canonical.providerExtensions ?? {}, null, 2),
    bindings: definition.bindings,
  };
}

function editorRequest(editor: McpEditorState): McpSavePreviewRequest {
  const extra = parseExtraFields(editor.extraJson);
  const spec: CanonicalMcp["spec"] = {
    ...extra,
    type: editor.transport,
  };
  if (editor.transport === "stdio") {
    spec.command = editor.command.trim();
    spec.args = editor.argsText.split("\n").map((value) => value.trim()).filter(Boolean);
    const env = parseStringMap(editor.envJson, "Environment");
    if (Object.keys(env).length > 0) spec.env = env;
    if (editor.cwd.trim()) spec.cwd = editor.cwd.trim();
  } else {
    spec.url = editor.url.trim();
    const headers = parseStringMap(editor.headersJson, "Headers");
    if (Object.keys(headers).length > 0) spec.headers = headers;
  }
  return {
    ...(editor.assetId ? { assetId: editor.assetId } : {}),
    canonical: {
      schemaVersion: 1,
      name: editor.name.trim(),
      spec,
      providerExtensions: parseObject(editor.providerExtensionsJson, "Provider extensions"),
    },
    ...(editor.title.trim() ? { title: editor.title.trim() } : {}),
    ...(editor.description.trim() ? { description: editor.description.trim() } : {}),
  };
}

function parseExtraFields(value: string): Record<string, unknown> {
  const parsed = parseObject(value, "高级 spec 字段");
  const reserved = ["type", "command", "args", "env", "cwd", "url", "headers"];
  const duplicate = reserved.find((key) => Object.hasOwn(parsed, key));
  if (duplicate) {
    throw new Error(`高级 spec 字段不能重复结构化字段 ${duplicate}。`);
  }
  return parsed;
}

function parseObject(value: string, label: string): Record<string, unknown> {
  const parsed: unknown = JSON.parse(value || "{}");
  if (!parsed || Array.isArray(parsed) || typeof parsed !== "object") {
    throw new Error(`${label}必须是 JSON object。`);
  }
  return parsed as Record<string, unknown>;
}

function parseStringMap(value: string, label: string): Record<string, string> {
  const parsed = parseObject(value, label);
  if (Object.values(parsed).some((entry) => typeof entry !== "string")) {
    throw new Error(`${label} 的所有值必须是字符串。`);
  }
  return parsed as Record<string, string>;
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

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "无法读取本地 MCP 配置。";
}
