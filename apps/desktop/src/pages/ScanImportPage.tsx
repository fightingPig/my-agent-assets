import { open } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, Check, FolderOpen, FolderSearch, House, ScanSearch } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  canonicalBatchImportApply,
  canonicalBatchImportPreview,
  discoverRuntimeSources,
  initializationPreview,
  listProjects,
  adoptApply,
  previewAdopt,
  safeCommandErrorMessage,
} from "../app/data-api";
import { DEFAULT_ASSET_CENTER_PATH } from "../app/defaults";
import type {
  ApplyResult,
  AdoptPreview,
  BatchImportPreview,
  InitializationPreview,
  DiscoveredRuntimeSource,
  RuntimeDiscoveryResult,
  RuntimeDiscoveryScope,
  ProjectSummary,
} from "../app/contracts";
import type { ConflictResolverContext } from "../app/detail-context";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";
import { statusToneForLabel } from "../ui-assets";

const scopes = [
  { id: "user", title: "用户级", detail: "扫描 Claude Code 与 Codex 用户级来源", icon: House },
  { id: "project", title: "项目级", detail: "扫描显式项目的 Claude/Codex 来源", icon: FolderSearch },
  { id: "custom", title: "自定义路径", detail: "预览指定目录下的资产", icon: ScanSearch },
] as const;

const userScanScope: RuntimeDiscoveryScope = { kind: "user" };
const customSourceOptions = [
  { value: "skill_directory", label: "Skill 目录（SKILL.md）", assetKind: "skill", sourceFormat: "skill_directory" },
  { value: "command_directory", label: "Claude Command 目录（.md）", assetKind: "command", sourceFormat: "markdown" },
  { value: "claude_mcp_json", label: "Claude MCP JSON", assetKind: "mcp", sourceFormat: "claude_mcp_json" },
  { value: "codex_mcp_toml", label: "Codex MCP TOML", assetKind: "mcp", sourceFormat: "codex_mcp_toml" },
] as const;

type CustomSourceOption = (typeof customSourceOptions)[number];
type ScanRow = {
  sourceId: string | null;
  name: string;
  type: string;
  source: string;
  result: string;
  eligibleImport: boolean;
};

const staticResults: ScanRow[] = [
  { sourceId: "demo:api-design", name: "api-design", type: "Skill", source: "用户级", result: "新增", eligibleImport: true },
  { sourceId: "demo:format-code", name: "format-code", type: "Command", source: "project-a", result: "新增", eligibleImport: true },
  { sourceId: "demo:filesystem", name: "Filesystem", type: "MCP", source: "my-app", result: "更新", eligibleImport: true },
  { sourceId: "demo:db-review", name: "db-review", type: "Skill", source: "project-a", result: "冲突", eligibleImport: true },
];

export function ScanImportPage({
  demoMode = false,
  onOpenConflicts,
  visualQaState,
}: {
  demoMode?: boolean;
  onOpenConflicts?: (context: ConflictResolverContext) => void;
  visualQaState?: string;
}) {
  const [selectedScope, setSelectedScope] = useState<(typeof scopes)[number]["id"]>("user");
  const [scanResult, setScanResult] = useState<RuntimeDiscoveryResult | null>(null);
  const [importPreview, setImportPreview] = useState<BatchImportPreview | null>(null);
  const [adoptPreview, setAdoptPreview] = useState<AdoptPreview | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [adoptResult, setAdoptResult] = useState<ApplyResult | null>(null);
  const [stateLabel, setStateLabel] = useState("读取中");
  const [isPlanning, setIsPlanning] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [isAdopting, setIsAdopting] = useState(false);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [initialization, setInitialization] = useState<InitializationPreview | null>(
    demoMode && visualQaState === "uninitialized"
      ? uninitializedVisualQaPreview()
      : null,
  );
  const [refreshKey, setRefreshKey] = useState(0);
  const [managedProjects, setManagedProjects] = useState<ProjectSummary[]>([]);
  const [selectedProjectPath, setSelectedProjectPath] = useState("");
  const [customPath, setCustomPath] = useState("");
  const [customSource, setCustomSource] = useState<CustomSourceOption>(customSourceOptions[0]);
  const [selectedSourceIds, setSelectedSourceIds] = useState<string[]>(
    demoMode ? staticResults.flatMap((row) => row.sourceId ? [row.sourceId] : []) : [],
  );

  const input = useMemo(
    () => toScanScope(selectedScope, selectedProjectPath, customPath, customSource, managedProjects),
    [customPath, customSource, managedProjects, selectedProjectPath, selectedScope],
  );

  useEffect(() => {
    let cancelled = false;
    if (demoMode) return undefined;
    listProjects()
      .then((projects) => {
        if (cancelled) return;
        setManagedProjects(projects);
        setSelectedProjectPath((current) => current || (projects.length > 0 ? "__all__" : ""));
      })
      .catch(() => {
        if (!cancelled) setManagedProjects([]);
      });
    return () => {
      cancelled = true;
    };
  }, [demoMode]);

  useEffect(() => {
    if (demoMode) {
      setInitialization(
        visualQaState === "uninitialized" ? uninitializedVisualQaPreview() : null,
      );
      return undefined;
    }
    let cancelled = false;
    initializationPreview()
      .then((preview) => {
        if (!cancelled) setInitialization(preview);
      })
      .catch((error) => {
        if (!cancelled) {
          setInitialization({
            previewId: "initialization-error",
            assetCenterPath: DEFAULT_ASSET_CENTER_PATH,
            plannedPaths: [],
            warnings: [safeCommandErrorMessage(error, "无法确认资产中心状态，请先在首页检查初始化。")],
            alreadyInitialized: false,
            canApply: false,
            generatedAtEpochSeconds: 0,
            expiresAtEpochSeconds: 0,
          });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [demoMode, refreshKey, visualQaState]);

  const chooseCustomPath = async () => {
    const isDirectory = customSource.assetKind !== "mcp";
    const selected = await open({
      directory: isDirectory,
      multiple: false,
      title: isDirectory ? "选择自定义资产目录" : "选择 MCP 配置文件",
      filters: isDirectory
        ? undefined
        : [{ name: "MCP 配置", extensions: customSource.sourceFormat === "codex_mcp_toml" ? ["toml"] : ["json"] }],
    });
    if (typeof selected === "string") setCustomPath(selected);
  };

  useEffect(() => {
    let cancelled = false;
    if (!input) {
      setScanResult(null);
      setSelectedSourceIds([]);
      setImportPreview(null);
      setAdoptPreview(null);
      setStateLabel(selectedScope === "project" ? "请选择已维护项目" : "请选择自定义来源类型并输入路径");
      return undefined;
    }
    setStateLabel("读取中");
    setImportPreview(null);
    setAdoptPreview(null);
    setOperationError(null);
    discoverRuntimeSources(input)
      .then((result) => {
        if (cancelled) return;
        if (result && typeof result === "object" && "sources" in result) {
          setScanResult(result);
          setSelectedSourceIds(result.sources
            .filter((source) => source.eligibleImport)
            .map((source) => source.sourceId));
          setStateLabel(result.sources.length > 0 ? "只读真实数据" : "未发现本地资产");
        } else {
          setScanResult(null);
          setSelectedSourceIds([]);
          setImportPreview(null);
          setStateLabel("未返回扫描结果");
        }
      })
      .catch((error) => {
        if (cancelled) return;
        setScanResult(null);
        setSelectedSourceIds([]);
        setImportPreview(null);
        setOperationError(errorMessage(error));
        setStateLabel(`读取失败：${errorMessage(error)}`);
      });
    return () => {
      cancelled = true;
    };
  }, [input, refreshKey, selectedScope]);

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
  const adoptWarning = adoptPreview?.warnings[0];
  const eligibleSourceIds = scanResult?.sources
    .filter((source) => source.eligibleImport)
    .map((source) => source.sourceId)
    ?? (demoMode ? staticResults.flatMap((row) => row.eligibleImport && row.sourceId ? [row.sourceId] : []) : []);
  const sourceIds = selectedSourceIds.filter((sourceId) => eligibleSourceIds.includes(sourceId));
  const conflictCount = importPreview?.items.filter((item) => item.disposition === "conflict").length ?? 0;
  const hasConflicts = conflictCount > 0;
  const planSummary = importPreview?.items
    .map((item) => `${item.assetId}：${dispositionLabel(item.disposition)}`)
    .join(" / ");
  const adoptPlanSummary = adoptPreview
    ? [...adoptPreview.importPlan, ...adoptPreview.mountPlan, ...adoptPreview.backupPlan].join(" / ")
    : "";
  const assetCenterReady = (demoMode && visualQaState !== "uninitialized") || initialization?.alreadyInitialized === true;
  const initializationNotice = !assetCenterReady
    ? initialization?.warnings[0] ?? "资产中心状态检查中，写入计划暂不可用。"
    : null;
  const canGeneratePlan = assetCenterReady && Boolean(input) && sourceIds.length > 0 && !isPlanning;
  const canApply = assetCenterReady && Boolean(importPreview?.canApply && importPreview.previewId && !hasConflicts);
  const canAdopt = assetCenterReady && Boolean(adoptPreview?.canApply && adoptPreview.previewId);

  const handleSourceSelection = (sourceId: string, selected: boolean) => {
    setSelectedSourceIds((current) => selected
      ? [...new Set([...current, sourceId])]
      : current.filter((candidate) => candidate !== sourceId));
    setImportPreview(null);
    setAdoptPreview(null);
    setApplyResult(null);
    setAdoptResult(null);
    setOperationError(null);
    setStateLabel("已更新资产选择");
  };

  const handlePlanImport = async () => {
    if (!input || sourceIds.length === 0) return;

    setIsPlanning(true);
    setOperationError(null);
    setAdoptPreview(null);
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
    if (!input || !canApply || !importPreview?.previewId) return;

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
    if (!input || sourceIds.length === 0) return;
    setIsPlanning(true);
    setOperationError(null);
    setImportPreview(null);
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
    if (!input || !adoptPreview?.canApply) return;
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
    if (!input || !importPreview || !hasConflicts) return;
    onOpenConflicts?.({ scope: input, preview: importPreview });
  };

  return (
    <div className="operation-workspace">
      <section className="panel operation-stepper" aria-label="扫描步骤">
        {["选择扫描范围", "扫描预览", "导入确认"].map((step, index) => <div className={index === 0 ? "active" : ""} key={step}><span>{index === 0 ? <Check size={13} /> : index + 1}</span><strong>{step}</strong></div>)}
      </section>

      <section className="panel operation-section">
        <div className="section-heading"><div><h3>选择扫描范围</h3><p>选择仅更新本地预览，不执行导入</p></div><span className={`preview-label ${statusToneForLabel(stateLabel)}`}>{stateLabel}</span></div>
        <div className="scope-card-grid">
          {scopes.map(({ id, title, detail, icon: Icon }) => <button aria-pressed={selectedScope === id} className={`scope-card ${selectedScope === id ? "selected" : ""}`} data-no-drag="true" key={id} onClick={() => { setSelectedScope(id); setApplyResult(null); }} style={NO_DRAG_REGION_STYLE} type="button"><span><Icon size={18} /></span><strong>{title}</strong><small>{detail}</small></button>)}
        </div>
        {selectedScope === "project" && !demoMode ? <label className="scan-project-picker"><span>已维护项目</span><select aria-label="选择已维护项目" data-no-drag="true" disabled={managedProjects.length === 0} onChange={(event) => setSelectedProjectPath(event.target.value)} style={NO_DRAG_REGION_STYLE} value={selectedProjectPath}><option value="">{managedProjects.length === 0 ? "请先在项目列表添加项目" : "选择项目范围"}</option><option value="__all__">全部已维护项目</option>{managedProjects.map((project) => <option key={project.id} value={project.path}>{project.name} · {project.path}</option>)}</select></label> : null}
        {selectedScope === "custom" && !demoMode ? <div className="scan-custom-source"><label><span>来源类型</span><select aria-label="自定义来源类型" data-no-drag="true" onChange={(event) => { setCustomSource(customSourceOptions.find((option) => option.value === event.target.value) ?? customSourceOptions[0]); setCustomPath(""); }} style={NO_DRAG_REGION_STYLE} value={customSource.value}>{customSourceOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</select></label><label><span>已授权路径</span><div className="path-picker-control"><input aria-label="自定义来源路径" data-no-drag="true" readOnly style={NO_DRAG_REGION_STYLE} value={customPath} /><button className="asset-secondary-action" data-no-drag="true" onClick={() => void chooseCustomPath()} style={NO_DRAG_REGION_STYLE} type="button"><FolderOpen size={14} />选择</button></div></label></div> : null}
      </section>

      <div className="scan-summary-grid">
        <div className="panel"><span>发现资产</span><strong>{counts.total}</strong><small>{stateLabel}</small></div><div className="panel"><span>Skills</span><strong>{counts.skills}</strong><small>只读扫描</small></div><div className="panel"><span>Commands</span><strong>{counts.commands}</strong><small>只读扫描</small></div><div className="panel"><span>MCP Servers</span><strong>{counts.mcps}</strong><small>只读扫描</small></div>
      </div>

      <section className="panel operation-section">
        <div className="section-heading"><div><h3>导入预览</h3><p>当前范围：{scopes.find((scope) => scope.id === selectedScope)?.title}{selectedScope === "project" && selectedProjectPath ? ` · ${selectedProjectPath === "__all__" ? "全部已维护项目" : selectedProjectPath}` : ""}</p></div><span>{sourceIds.length} / {eligibleSourceIds.length} 项已选择</span></div>
        <div className="preview-table" role="table" aria-label="导入预览表"><div className="preview-table-head" role="row"><span>资产</span><span>类型</span><span>来源</span><span>结果</span></div>{rows.map((result) => <div className="preview-table-row" role="row" key={result.sourceId ?? `${result.type}:${result.name}`}><label className="scan-source-select"><input aria-label={`选择 ${result.name}`} checked={result.sourceId ? sourceIds.includes(result.sourceId) : true} data-no-drag="true" disabled={!result.sourceId || !result.eligibleImport} onChange={(event) => result.sourceId && handleSourceSelection(result.sourceId, event.target.checked)} style={NO_DRAG_REGION_STYLE} type="checkbox" /><strong>{result.name}</strong></label><span>{result.type}</span><span>{result.source}</span><span className={result.result === "冲突" || result.result === "无效" ? "warning-text" : "success-text"}>{result.result}</span></div>)}{rows.length === 0 && <div className="asset-empty-state"><ScanSearch size={20} /><strong>未发现可导入资产</strong><span>调整扫描范围或检查本地 Claude 目录。</span></div>}</div>
        <div className="operation-warning"><AlertTriangle size={17} /><div><strong>{initializationNotice ?? (hasConflicts ? `发现 ${conflictCount} 项内容冲突` : previewWarning ?? adoptWarning ?? warning ?? "只读扫描预览")}</strong><span>{initializationNotice ? "请先在首页完成资产中心初始化；当前仍可查看只读扫描结果。" : hasConflicts ? "请逐项选择跳过、重命名或覆盖；扫描导入不会直接覆盖现有资产。" : planSummary || adoptPlanSummary || (scanResult?.sources.length ? "当前仅展示发现结果，生成计划后才能确认导入。" : "当前扫描没有发现真实资产，确认导入保持禁用。")}</span></div></div>
        <div className="operation-actions">{hasConflicts ? <button className="asset-secondary-action" data-no-drag="true" onClick={handleOpenConflicts} style={NO_DRAG_REGION_STYLE} type="button">处理冲突</button> : null}<button className="asset-secondary-action" data-no-drag="true" disabled={!canGeneratePlan} onClick={handlePlanImport} style={NO_DRAG_REGION_STYLE} type="button">{isPlanning ? "生成中" : "生成导入计划"}</button><button className="asset-secondary-action" data-no-drag="true" disabled={!canGeneratePlan} onClick={handlePlanAdopt} style={NO_DRAG_REGION_STYLE} type="button">生成接管计划</button></div>
        <ApplyConfirmationPanel
          actionLabel="确认导入"
          canApply={canApply}
          description="会把当前扫描资产写入资产中心；后端会校验 previewId 并在替换前创建备份。"
          isApplying={isApplying}
          onApply={handleApplyImport}
          operationError={operationError}
          result={applyResult}
          title="执行导入"
        />
        <div className="operation-warning high-risk-mode">
          <AlertTriangle size={17} />
          <div>
            <strong>高风险接管模式</strong>
            <span>接管会在导入后替换当前生效位置。仅在明确需要让资产中心成为该位置的唯一真实来源时使用。</span>
          </div>
        </div>
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

function uninitializedVisualQaPreview(): InitializationPreview {
  return {
    previewId: "visual-qa-uninitialized",
    assetCenterPath: DEFAULT_ASSET_CENTER_PATH,
    plannedPaths: [],
    warnings: ["资产中心尚未初始化。请先前往首页完成初始化。"],
    alreadyInitialized: false,
    canApply: false,
    generatedAtEpochSeconds: 0,
    expiresAtEpochSeconds: 0,
  };
}

function errorMessage(error: unknown) {
  return safeCommandErrorMessage(error, "导入操作未完成。请查看首页系统状态后重试。");
}

function toScanScope(
  selectedScope: (typeof scopes)[number]["id"],
  selectedProjectPath: string,
  customPath: string,
  customSource: CustomSourceOption,
  managedProjects: ProjectSummary[],
): RuntimeDiscoveryScope | null {
  if (selectedScope === "project") {
    if (selectedProjectPath === "__all__") {
      return { kind: "managed_projects", projectIds: managedProjects.map((project) => project.id) };
    }
    return selectedProjectPath ? { kind: "project", projectPath: selectedProjectPath } : null;
  }
  if (selectedScope === "custom") {
    return customPath.trim() ? {
      kind: "custom",
      path: customPath.trim(),
      assetKind: customSource.assetKind,
      sourceFormat: customSource.sourceFormat,
    } : null;
  }
  return userScanScope;
}

function toScanRow(asset: DiscoveredRuntimeSource) {
  return {
    sourceId: asset.sourceId,
    name: asset.assetName,
    type: asset.assetKind === "skill" ? "Skill" : asset.assetKind === "command" ? "Command" : "MCP",
    source: `${providerLabel(asset.provider)} · ${asset.scope === "user" ? "用户级" : asset.scope === "project" ? "项目级" : "自定义"}`,
    result: asset.eligibleImport ? "发现" : asset.isManaged ? "已管理" : "无效",
    eligibleImport: asset.eligibleImport,
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
