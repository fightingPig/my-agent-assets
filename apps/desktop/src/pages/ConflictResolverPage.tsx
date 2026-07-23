import { AlertTriangle, Blocks, BookOpen } from "lucide-react";
import { useMemo, useState } from "react";
import {
  canonicalBatchImportApply,
  canonicalBatchImportPreview,
} from "../app/data-api";
import type {
  ApplyResult,
  BatchImportPreview,
  CanonicalImportPreview,
  CanonicalImportResolution,
} from "../app/contracts";
import type { ConflictResolverContext } from "../app/detail-context";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

type Resolution = "skip" | "rename" | "overwrite";

type ConflictItem = {
  id: string;
  sourceId: string;
  name: string;
  type: string;
  reason: string;
  source: string;
  assetId: string;
  icon: typeof Blocks;
  existing: string;
  incoming: string;
  rawSource: string;
};

const demoItems: readonly ConflictItem[] = [
  {
    id: "demo:mcp:postgresql",
    sourceId: "demo:mcp:postgresql",
    name: "PostgreSQL",
    type: "MCP Server",
    reason: "同名配置内容不同",
    source: "project-a/.mcp.json",
    assetId: "mcp:PostgreSQL",
    icon: Blocks,
    existing: `{\n  "command": "postgres-mcp",\n  "args": ["--read-only"]\n}`,
    incoming: `{\n  "command": "postgres-mcp",\n  "args": ["--schema", "public"]\n}`,
    rawSource: `{\n  "command": "postgres-mcp",\n  "args": ["--schema", "public"]\n}`,
  },
  {
    id: "demo:skill:review",
    sourceId: "demo:skill:review",
    name: "review",
    type: "Skill",
    reason: "资产中心已存在同名 Skill",
    source: "my-app/.claude/skills/review",
    assetId: "skill:review",
    icon: BookOpen,
    existing: "# Review\n\n检查正确性、风险和测试覆盖。",
    incoming: "# Review\n\n检查架构、性能和安全边界。",
    rawSource: "# Review\n\n检查架构、性能和安全边界。",
  },
];

export function ConflictResolverPage({
  context,
  demoMode = false,
}: {
  context?: ConflictResolverContext;
  demoMode?: boolean;
}) {
  const items = useMemo(
    () => context
      ? context.preview.items.filter((item) => item.conflict).map(toConflictItem)
      : demoMode
        ? [...demoItems]
        : [],
    [context, demoMode],
  );
  const contextKey = context?.preview.previewId ?? (demoMode ? "demo" : "empty");
  return <ConflictResolverWorkspace context={context} items={items} key={contextKey} />;
}

function ConflictResolverWorkspace({
  context,
  items,
}: {
  context?: ConflictResolverContext;
  items: ConflictItem[];
}) {
  const [selectedId, setSelectedId] = useState(items[0]?.id ?? "");
  const [resolutions, setResolutions] = useState<Record<string, Resolution>>(
    () => defaultResolutions(items),
  );
  const [renameValues, setRenameValues] = useState<Record<string, string>>({});
  const [resolvedPreview, setResolvedPreview] = useState<BatchImportPreview | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [previewState, setPreviewState] = useState(
    items.length > 0 ? "等待逐项决策" : "等待扫描结果",
  );
  const [operationError, setOperationError] = useState<string | null>(null);
  const [isPlanning, setIsPlanning] = useState(false);
  const [isApplying, setIsApplying] = useState(false);

  const selected = items.find((item) => item.id === selectedId) ?? items[0];
  const selectedResolution = selected ? resolutions[selected.id] ?? "skip" : "skip";
  const selectedPlan = describeResolution(
    selectedResolution,
    selected?.name ?? "当前资产",
    selected ? renameValues[selected.id] ?? "" : "",
  );
  const selections = context
    ? context.preview.items.map((item) => ({
        sourceId: item.sourceId,
        resolution: item.conflict
          ? toCanonicalResolution(
            resolutions[item.sourceId] ?? "skip",
            renameValues[item.sourceId] ?? "",
          )
          : { kind: "unresolved" as const },
      }))
    : [];
  const unresolvedRename = items.some((item) =>
    resolutions[item.id] === "rename" && !(renameValues[item.id] ?? "").trim(),
  );
  const canPlan = Boolean(context && items.length > 0 && !isPlanning && !unresolvedRename);
  const canApply = Boolean(resolvedPreview?.canApply && resolvedPreview.previewId);

  const updateResolution = (resolution: Resolution) => {
    if (!selected) return;
    setResolutions((current) => ({ ...current, [selected.id]: resolution }));
    setResolvedPreview(null);
    setApplyResult(null);
    setPreviewState("决策已变更，请重新生成处理计划");
  };

  const handlePlan = async () => {
    if (!context || selections.length === 0) return;
    setIsPlanning(true);
    setOperationError(null);
    try {
      const preview = await canonicalBatchImportPreview({
        scope: context.scope,
        selections,
      });
      setResolvedPreview(preview);
      setApplyResult(null);
      setPreviewState(preview.canApply ? "冲突处理计划已生成" : "冲突处理计划不可执行");
    } catch (error) {
      setResolvedPreview(null);
      setOperationError(errorMessage(error));
      setPreviewState("冲突处理计划失败");
    } finally {
      setIsPlanning(false);
    }
  };

  const handleApply = async () => {
    if (!context || !resolvedPreview?.canApply) return;
    setIsApplying(true);
    setOperationError(null);
    try {
      const result = await canonicalBatchImportApply({
        previewId: resolvedPreview.previewId,
        previewGeneratedAtEpochSeconds: resolvedPreview.generatedAtEpochSeconds,
        request: {
          scope: context.scope,
          selections,
        },
      });
      setApplyResult(toApplyResult(result));
      setPreviewState("冲突处理已执行");
    } catch (error) {
      setApplyResult(null);
      setOperationError(errorMessage(error));
      setPreviewState("冲突处理失败");
    } finally {
      setIsApplying(false);
    }
  };

  return (
    <div className="master-detail-workspace conflict-workspace">
      <section className="panel master-list-panel">
        <div className="section-heading">
          <div><h3>待处理冲突</h3><p>需要逐项确认处理方式 · {previewState}</p></div>
          <span className="status-badge warning">{items.length} 项</span>
        </div>
        <div className="master-select-list" role="listbox" aria-label="冲突选择">
          {items.map(({ id, name, type, reason, icon: Icon }) => (
            <button
              aria-label={name}
              aria-selected={selectedId === id}
              className={selectedId === id ? "selected" : ""}
              data-no-drag="true"
              key={id}
              onClick={() => setSelectedId(id)}
              role="option"
              style={NO_DRAG_REGION_STYLE}
              type="button"
            >
              <span className="skeleton-icon warning"><Icon size={16} /></span>
              <span><strong>{name}</strong><small>{type} · {reason}</small></span>
              <AlertTriangle size={15} />
            </button>
          ))}
        </div>
      </section>
      <section className="panel master-inspector-panel">
        {!selected ? (
          <div className="asset-inspector-empty">
            <AlertTriangle size={22} />
            <strong>暂无待处理冲突</strong>
            <span>请从扫描导入页生成真实冲突预览后进入此页面。</span>
          </div>
        ) : (
          <>
            <div className="section-heading">
              <div><h3>{selected.name}</h3><p>{selected.reason}</p></div>
              <span className="asset-status warning">需要确认</span>
            </div>
            <dl className="entity-field-list compact">
              <div><dt>资产类型</dt><dd>{selected.type}</dd></div>
              <div><dt>扫描来源</dt><dd>{selected.source}</dd></div>
              <div><dt>决策预览</dt><dd>{selectedPlan.label}</dd></div>
            </dl>
            <div className="side-by-side-diff">
              <div><strong>资产中心</strong><pre><code>{selected.existing}</code></pre></div>
              <div><strong>扫描结果</strong><pre><code>{selected.incoming}</code></pre></div>
            </div>
            {selected.type === "MCP Server" ? (
              <details className="conflict-raw-source">
                <summary>查看原始 MCP 来源</summary>
                <pre><code>{selected.rawSource}</code></pre>
              </details>
            ) : null}
            <div className="resolution-options">
              {(["skip", "rename", "overwrite"] as Resolution[]).map((resolution) => {
                const option = describeResolution(resolution, selected.name, renameValues[selected.id] ?? "");
                return (
                  <button
                    aria-pressed={selectedResolution === resolution}
                    className={selectedResolution === resolution ? "selected" : ""}
                    data-no-drag="true"
                    key={resolution}
                    onClick={() => updateResolution(resolution)}
                    style={NO_DRAG_REGION_STYLE}
                    type="button"
                  >
                    <strong>{option.label}</strong><span>{option.description}</span>
                  </button>
                );
              })}
            </div>
            {selectedResolution === "rename" ? (
              <label className="rename-conflict-field">
                <span>新资产名称</span>
                <input
                  aria-label="新资产名称"
                  data-no-drag="true"
                  onChange={(event) => {
                    setRenameValues((current) => ({ ...current, [selected.id]: event.target.value }));
                    setResolvedPreview(null);
                    setApplyResult(null);
                    setPreviewState("请输入合法的新名称后重新生成处理计划");
                  }}
                  placeholder="输入新的唯一名称"
                  style={NO_DRAG_REGION_STYLE}
                  value={renameValues[selected.id] ?? ""}
                />
                <small>不会自动改名；名称必须由你明确指定。</small>
              </label>
            ) : null}
            <div className="operation-warning">
              <AlertTriangle size={17} />
              <div>
                <strong>处理计划预览</strong>
                <span>{selectedPlan.planText}。共 {items.length} 个冲突；覆盖或重命名前将创建 portable backup。</span>
              </div>
            </div>
            <div className="operation-actions">
              <button
                className="asset-secondary-action"
                data-no-drag="true"
                disabled={!canPlan}
                onClick={handlePlan}
                style={NO_DRAG_REGION_STYLE}
                type="button"
              >
                {isPlanning ? "生成中" : unresolvedRename ? "先填写新名称" : "生成处理计划"}
              </button>
            </div>
            <ApplyConfirmationPanel
              actionLabel="执行冲突处理"
              canApply={canApply}
              description="按逐项决策跳过、重命名或覆盖 canonical assets；后端会重新校验 previewId 和文件指纹。"
              isApplying={isApplying}
              onApply={handleApply}
              operationError={operationError}
              result={applyResult}
              title="执行冲突处理"
            />
          </>
        )}
      </section>
    </div>
  );
}

function toConflictItem(item: CanonicalImportPreview): ConflictItem {
  const conflict = item.conflict;
  if (!conflict) throw new Error("Cannot render a non-conflicting import item");
  return {
    id: item.sourceId,
    sourceId: item.sourceId,
    name: item.sourceName,
    type: item.assetType === "mcp"
      ? "MCP Server"
      : item.assetType === "command"
        ? "Command"
        : "Skill",
    reason: conflict.reason,
    source: item.sourcePath,
    assetId: item.assetId,
    icon: item.assetType === "mcp" ? Blocks : BookOpen,
    existing: conflict.existingContent,
    incoming: conflict.incomingContent,
    rawSource: conflict.rawSource,
  };
}

function defaultResolutions(items: readonly ConflictItem[]): Record<string, Resolution> {
  return Object.fromEntries(items.map((item) => [item.id, "skip" satisfies Resolution]));
}

function toCanonicalResolution(
  resolution: Resolution,
  newName: string,
): CanonicalImportResolution {
  if (resolution === "rename") {
    return { kind: "rename", newName: newName.trim() };
  }
  return { kind: resolution };
}

function describeResolution(resolution: Resolution, name: string, newName: string) {
  if (resolution === "rename") {
    return {
      label: "重命名",
      description: "以新名称导入当前内容",
      planText: newName.trim()
        ? `${name} 将以 ${newName.trim()} 导入，资产中心现有内容保持不变`
        : "需要先输入新的唯一资产名称",
    };
  }
  if (resolution === "overwrite") {
    return {
      label: "覆盖",
      description: "使用扫描结果替换现有内容",
      planText: `${name} 将覆盖资产中心内容`,
    };
  }
  return {
    label: "跳过",
    description: "保留资产中心内容",
    planText: `${name} 将被跳过，资产中心现有内容保持不变`,
  };
}

function toApplyResult(
  result: Awaited<ReturnType<typeof canonicalBatchImportApply>>,
): ApplyResult {
  return {
    mode: "apply",
    ok: true,
    previewId: result.previewId,
    backup: null,
    steps: result.items.map((item) => ({
      stepId: item.assetId,
      kind: "import",
      label: `处理 ${item.assetId}`,
      status: item.status === "skipped" ? "skipped" : "success",
      message: item.status === "skipped" ? "已跳过。" : "已写入 canonical asset。",
      affectedPaths: item.affectedPaths,
    })),
    warnings: [],
    errors: [],
  };
}

function errorMessage(_error: unknown) {
  return "冲突处理未完成。请查看系统状态或导出诊断包后重试。";
}
