import { AlertTriangle, Check, FolderSearch, House, ScanSearch } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  canonicalBatchImportApply,
  canonicalBatchImportPreview,
  discoverRuntimeSources,
  adoptApply,
  previewAdopt,
} from "../app/data-api";
import type {
  ApplyResult,
  AdoptPreview,
  BatchImportPreview,
  DiscoveredRuntimeSource,
  RuntimeDiscoveryResult,
  RuntimeDiscoveryScope,
} from "../app/contracts";
import type { ConflictResolverContext } from "../app/detail-context";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { StaticActionButton } from "../components/ui/StaticActionButton";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

const scopes = [
  { id: "user", title: "用户级", detail: "扫描 Claude Code 与 Codex 用户级来源", icon: House },
  { id: "project", title: "项目级", detail: "扫描显式项目的 Claude/Codex 来源", icon: FolderSearch },
  { id: "custom", title: "自定义路径", detail: "预览指定目录下的资产", icon: ScanSearch },
] as const;

const staticResults = [
  { name: "api-design", type: "Skill", source: "用户级", result: "新增" },
  { name: "format-code", type: "Command", source: "project-a", result: "新增" },
  { name: "Filesystem", type: "MCP", source: "my-app", result: "更新" },
  { name: "db-review", type: "Skill", source: "project-a", result: "冲突" },
];

export function ScanImportPage({
  demoMode = false,
  onOpenConflicts,
}: {
  demoMode?: boolean;
  onOpenConflicts?: (context: ConflictResolverContext) => void;
}) {
  const [selectedScope, setSelectedScope] = useState<(typeof scopes)[number]["id"]>("user");
  const [scanResult, setScanResult] = useState<RuntimeDiscoveryResult | null>(null);
  const [importPreview, setImportPreview] = useState<BatchImportPreview | null>(null);
  const [adoptPreview, setAdoptPreview] = useState<AdoptPreview | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [adoptResult, setAdoptResult] = useState<ApplyResult | null>(null);
  const [confirmationValue, setConfirmationValue] = useState("");
  const [stateLabel, setStateLabel] = useState("读取中");
  const [isPlanning, setIsPlanning] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [isAdopting, setIsAdopting] = useState(false);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  const input = useMemo(() => toScanScope(selectedScope), [selectedScope]);

  useEffect(() => {
    let cancelled = false;
    setStateLabel("读取中");
    setImportPreview(null);
    setAdoptPreview(null);
    setOperationError(null);
    discoverRuntimeSources(input)
      .then((result) => {
        if (cancelled) return;
        if (result && typeof result === "object" && "sources" in result) {
          setScanResult(result);
          setStateLabel(result.sources.length > 0 ? "只读真实数据" : "未发现本地资产");
        } else {
          setScanResult(null);
          setImportPreview(null);
          setStateLabel("未返回扫描结果");
        }
      })
      .catch((error) => {
        if (cancelled) return;
        setScanResult(null);
        setImportPreview(null);
        setOperationError(errorMessage(error));
        setStateLabel(`读取失败：${errorMessage(error)}`);
      });
    return () => {
      cancelled = true;
    };
  }, [input, refreshKey]);

  const rows = scanResult?.sources.length
    ? scanResult.sources.map(toScanRow)
    : demoMode ? staticResults : [];
  const counts = scanResult?.sources.length
    ? countSources(scanResult.sources)
    : demoMode
      ? { total: 14, skills: 4, commands: 4, mcps: 4 }
      : { total: 0, skills: 0, commands: 0, mcps: 0 };
  const warning = scanResult?.warnings[0];
  const previewWarning = importPreview?.warnings[0];
  const sourceIds = scanResult?.sources
    .filter((source) => source.eligibleImport)
    .map((source) => source.sourceId) ?? [];
  const conflictCount = importPreview?.items.filter((item) => item.disposition === "conflict").length ?? 0;
  const hasConflicts = conflictCount > 0;
  const planSummary = importPreview?.items
    .map((item) => `${item.assetId}：${dispositionLabel(item.disposition)}`)
    .join(" / ");
  const canGeneratePlan = sourceIds.length > 0 && !isPlanning;
  const canApply = Boolean(importPreview?.canApply && importPreview.previewId && !hasConflicts);
  const canAdopt = Boolean(adoptPreview?.canApply && adoptPreview.previewId);

  const handlePlanImport = async () => {
    if (sourceIds.length === 0) return;

    setIsPlanning(true);
    setOperationError(null);
    setStateLabel("生成导入计划中");
    try {
      const result = await canonicalBatchImportPreview({
        scope: input,
        selections: sourceIds.map((sourceId) => ({
          sourceId,
          resolution: { kind: "unresolved" },
        })),
      });
      setImportPreview(result);
      setStateLabel(result.canApply ? "导入计划已生成" : "导入计划需要处理");
    } catch (error) {
      setImportPreview(null);
      setOperationError(errorMessage(error));
      setStateLabel("导入计划失败");
    } finally {
      setIsPlanning(false);
    }
  };

  const handleApplyImport = async () => {
    if (!canApply || !importPreview?.previewId) return;

    setIsApplying(true);
    setOperationError(null);
    setStateLabel("执行导入中");
    try {
      const result = await canonicalBatchImportApply({
        previewId: importPreview.previewId,
        previewGeneratedAtEpochSeconds: importPreview.generatedAtEpochSeconds,
        request: {
          scope: input,
          selections: sourceIds.map((sourceId) => ({
            sourceId,
            resolution: { kind: "unresolved" },
          })),
        },
      });
      setApplyResult(toApplyResult(result));
      setStateLabel("导入已执行");
      setConfirmationValue("");
      setRefreshKey((current) => current + 1);
    } catch (error) {
      setApplyResult(null);
      setOperationError(errorMessage(error));
      setStateLabel("导入失败");
    } finally {
      setIsApplying(false);
    }
  };

  const handlePlanAdopt = async () => {
    if (sourceIds.length === 0) return;
    setIsPlanning(true);
    setOperationError(null);
    setStateLabel("生成接管计划中");
    try {
      const result = await previewAdopt({
        scope: input,
        selections: sourceIds.map((sourceId) => ({
          sourceId,
          resolution: { kind: "unresolved" },
        })),
      });
      setAdoptPreview(result);
      setStateLabel(result.canApply ? "接管计划已生成" : "接管计划需要处理");
    } catch (error) {
      setAdoptPreview(null);
      setOperationError(errorMessage(error));
      setStateLabel("接管计划失败");
    } finally {
      setIsPlanning(false);
    }
  };

  const handleApplyAdopt = async () => {
    if (!adoptPreview?.canApply) return;
    setIsAdopting(true);
    setOperationError(null);
    setStateLabel("执行导入并接管中");
    try {
      const result = await adoptApply({
        previewId: adoptPreview.previewId,
        previewGeneratedAtEpochSeconds: adoptPreview.generatedAtEpochSeconds,
        request: {
          scope: input,
          selections: sourceIds.map((sourceId) => ({
            sourceId,
            resolution: { kind: "unresolved" },
          })),
        },
      });
      setAdoptResult(toAdoptApplyResult(result));
      setStateLabel("导入并接管已执行");
      setRefreshKey((current) => current + 1);
    } catch (error) {
      setAdoptResult(null);
      setOperationError(errorMessage(error));
      setStateLabel("导入并接管失败");
    } finally {
      setIsAdopting(false);
    }
  };

  const handleOpenConflicts = () => {
    if (!importPreview || !hasConflicts) return;
    onOpenConflicts?.({ scope: input, preview: importPreview });
  };

  return (
    <div className="operation-workspace">
      <section className="panel operation-stepper" aria-label="扫描步骤">
        {["选择扫描范围", "扫描预览", "导入确认"].map((step, index) => <div className={index === 0 ? "active" : ""} key={step}><span>{index === 0 ? <Check size={13} /> : index + 1}</span><strong>{step}</strong></div>)}
      </section>

      <section className="panel operation-section">
        <div className="section-heading"><div><h3>选择扫描范围</h3><p>选择仅更新本地预览，不执行导入</p></div><span className="preview-label">{stateLabel}</span></div>
        <div className="scope-card-grid">
          {scopes.map(({ id, title, detail, icon: Icon }) => <button aria-pressed={selectedScope === id} className={`scope-card ${selectedScope === id ? "selected" : ""}`} data-no-drag="true" key={id} onClick={() => { setSelectedScope(id); setApplyResult(null); setConfirmationValue(""); }} style={NO_DRAG_REGION_STYLE} type="button"><span><Icon size={18} /></span><strong>{title}</strong><small>{detail}</small></button>)}
        </div>
      </section>

      <div className="scan-summary-grid">
        <div className="panel"><span>发现资产</span><strong>{counts.total}</strong><small>{stateLabel}</small></div><div className="panel"><span>Skills</span><strong>{counts.skills}</strong><small>只读扫描</small></div><div className="panel"><span>Commands</span><strong>{counts.commands}</strong><small>只读扫描</small></div><div className="panel"><span>MCP Servers</span><strong>{counts.mcps}</strong><small>只读扫描</small></div>
      </div>

      <section className="panel operation-section">
        <div className="section-heading"><div><h3>导入预览</h3><p>当前范围：{scopes.find((scope) => scope.id === selectedScope)?.title}</p></div><span>{rows.length} 项待确认</span></div>
        <div className="preview-table" role="table" aria-label="导入预览表"><div className="preview-table-head" role="row"><span>资产</span><span>类型</span><span>来源</span><span>结果</span></div>{rows.map((result) => <div className="preview-table-row" role="row" key={`${result.type}:${result.name}`}><strong>{result.name}</strong><span>{result.type}</span><span>{result.source}</span><span className={result.result === "冲突" || result.result === "无效" ? "warning-text" : "success-text"}>{result.result}</span></div>)}{rows.length === 0 && <div className="asset-empty-state"><ScanSearch size={20} /><strong>未发现可导入资产</strong><span>调整扫描范围或检查本地 Claude 目录。</span></div>}</div>
        <div className="operation-warning"><AlertTriangle size={17} /><div><strong>{hasConflicts ? `发现 ${conflictCount} 项内容冲突` : previewWarning ?? warning ?? "只读扫描预览"}</strong><span>{hasConflicts ? "请逐项选择跳过、重命名或覆盖；扫描导入不会直接覆盖现有资产。" : planSummary ?? (scanResult?.sources.length ? "当前仅展示发现结果，生成计划后才能确认导入。" : "当前扫描没有发现真实资产，确认导入保持禁用。")}</span></div></div>
        <div className="operation-actions"><StaticActionButton className="asset-secondary-action">保存扫描预览</StaticActionButton>{hasConflicts ? <button className="asset-secondary-action" data-no-drag="true" onClick={handleOpenConflicts} style={NO_DRAG_REGION_STYLE} type="button">处理冲突</button> : null}<button className="asset-secondary-action" data-no-drag="true" disabled={!canGeneratePlan} onClick={handlePlanImport} style={NO_DRAG_REGION_STYLE} type="button">{isPlanning ? "生成中" : "生成导入计划"}</button><button className="asset-secondary-action" data-no-drag="true" disabled={!canGeneratePlan} onClick={handlePlanAdopt} style={NO_DRAG_REGION_STYLE} type="button">生成接管计划</button></div>
        <ApplyConfirmationPanel
          actionLabel="确认导入"
          canApply={canApply}
          confirmationValue={confirmationValue}
          description="会把当前扫描资产写入资产中心；后端会校验 previewId 并在替换前创建备份。"
          isApplying={isApplying}
          onApply={handleApplyImport}
          onConfirmationChange={setConfirmationValue}
          operationError={operationError}
          result={applyResult}
          title="执行导入"
        />
        <ApplyConfirmationPanel
          actionLabel="导入并接管"
          canApply={canAdopt}
          description="先导入到资产中心，再备份原生效位置并将 canonical 版本挂载回原 target；整个流程由后端单事务执行。"
          isApplying={isAdopting}
          onApply={handleApplyAdopt}
          operationError={operationError}
          result={adoptResult}
          title="执行导入并接管"
        />
      </section>
    </div>
  );
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "无法调用导入操作。";
}

function toScanScope(selectedScope: (typeof scopes)[number]["id"]): RuntimeDiscoveryScope {
  if (selectedScope === "project") return { kind: "project", projectPath: "~/workspace/project-a" };
  if (selectedScope === "custom") {
    return {
      kind: "custom",
      path: "~/code/design-system/.agents/skills",
      assetKind: "skill",
      sourceFormat: "skill_directory",
    };
  }
  return { kind: "user" };
}

function toScanRow(asset: DiscoveredRuntimeSource) {
  return {
    name: asset.assetName,
    type: asset.assetKind === "skill" ? "Skill" : asset.assetKind === "command" ? "Command" : "MCP",
    source: `${providerLabel(asset.provider)} · ${asset.scope === "user" ? "用户级" : asset.scope === "project" ? "项目级" : "自定义"}`,
    result: asset.eligibleImport ? "发现" : asset.isManaged ? "已管理" : "无效",
  };
}

function countSources(sources: DiscoveredRuntimeSource[]) {
  return {
    total: sources.length,
    skills: sources.filter((source) => source.assetKind === "skill").length,
    commands: sources.filter((source) => source.assetKind === "command").length,
    mcps: sources.filter((source) => source.assetKind === "mcp").length,
  };
}

function providerLabel(provider: DiscoveredRuntimeSource["provider"]) {
  if (provider === "claude_code") return "Claude Code";
  if (provider === "codex") return "Codex";
  return "Custom";
}

function dispositionLabel(disposition: BatchImportPreview["items"][number]["disposition"]) {
  if (disposition === "conflict") return "冲突";
  if (disposition === "overwrite") return "覆盖";
  if (disposition === "rename") return "重命名";
  if (disposition === "skip") return "跳过";
  if (disposition === "unchanged") return "无需变更";
  return "新增";
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
      label: `导入 ${item.assetId}`,
      status: "success",
      message: item.status === "skipped" ? "已跳过。" : item.status === "unchanged" ? "无需变更。" : "已写入资产中心。",
      affectedPaths: item.affectedPaths,
    })),
    warnings: [],
    errors: [],
  };
}

function toAdoptApplyResult(
  result: Awaited<ReturnType<typeof adoptApply>>,
): ApplyResult {
  return {
    mode: "apply",
    ok: result.items.every((item) => item.mounted || !item.targetId),
    previewId: result.previewId,
    backup: null,
    steps: result.items.map((item) => ({
      stepId: item.sourceId,
      kind: "mount",
      label: `接管 ${item.assetId}`,
      status: item.mounted || !item.targetId ? "success" : "failed",
      message: item.mounted ? "已导入并挂载回原运行目标。" : "该来源已跳过。",
      affectedPaths: result.affectedPaths,
    })),
    warnings: [],
    errors: [],
  };
}
