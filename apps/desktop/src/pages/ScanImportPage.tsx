import { AlertTriangle, Check, FolderSearch, House, ScanSearch } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { importApply, previewImport, scanAssets } from "../app/data-api";
import type { ApplyResult, AssetSummary, ImportPreview, ScanResult, ScanScope } from "../app/contracts";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { StaticActionButton } from "../components/ui/StaticActionButton";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

const scopes = [
  { id: "user", title: "用户级", detail: "扫描 ~/.claude 与本地配置", icon: House },
  { id: "project", title: "项目级", detail: "扫描已登记项目的 .claude", icon: FolderSearch },
  { id: "custom", title: "自定义路径", detail: "预览指定目录下的资产", icon: ScanSearch },
] as const;

const staticResults = [
  { name: "api-design", type: "Skill", source: "用户级", result: "新增" },
  { name: "format-code", type: "Command", source: "project-a", result: "新增" },
  { name: "Filesystem", type: "MCP", source: "my-app", result: "更新" },
  { name: "db-review", type: "Skill", source: "project-a", result: "冲突" },
];

export function ScanImportPage({ demoMode = false }: { demoMode?: boolean }) {
  const [selectedScope, setSelectedScope] = useState<(typeof scopes)[number]["id"]>("user");
  const [scanResult, setScanResult] = useState<ScanResult | null>(null);
  const [importPreview, setImportPreview] = useState<ImportPreview | null>(null);
  const [planResult, setPlanResult] = useState<ApplyResult | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [confirmationValue, setConfirmationValue] = useState("");
  const [stateLabel, setStateLabel] = useState("读取中");
  const [isPlanning, setIsPlanning] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  const input = useMemo(() => toScanScope(selectedScope), [selectedScope]);

  useEffect(() => {
    let cancelled = false;
    setStateLabel("读取中");
    setPlanResult(null);
    setOperationError(null);
    scanAssets({ scope: input })
      .then((result) => {
        if (cancelled) return;
        if (result && typeof result === "object" && "counts" in result) {
          setScanResult(result);
          setStateLabel(result.conflictCount > 0 ? "发现冲突" : result.assets.length > 0 ? "只读真实数据" : "未发现本地资产");
          if (result.assets.length > 0 && result.conflictCount === 0) {
            previewImport({
              scope: input,
              assetIds: result.assets.map((asset) => asset.id),
              conflictResolutions: [],
            })
              .then((preview) => {
                if (!cancelled) setImportPreview(preview.steps.length > 0 ? preview : null);
              })
              .catch(() => {
                if (!cancelled) setImportPreview(null);
              });
          } else {
            setImportPreview(null);
          }
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

  const rows = scanResult?.assets.length
    ? scanResult.assets.map(toScanRow)
    : demoMode ? staticResults : [];
  const counts = scanResult?.assets.length
    ? scanResult.counts
    : demoMode
      ? { total: 14, skills: 4, commands: 4, mcps: 4 }
      : { total: 0, skills: 0, commands: 0, mcps: 0 };
  const warning = scanResult?.warnings[0];
  const previewWarning = importPreview?.warnings[0];
  const previewStepText = importPreview?.steps.map((step) => step.label).join(" / ");
  const scannedAssetIds = scanResult?.assets.map((asset) => asset.id) ?? [];
  const hasConflicts = (scanResult?.conflictCount ?? 0) > 0;
  const planSummary = planResult?.steps.length
    ? planResult.steps.map((step) => step.message).join(" / ")
    : previewStepText;
  const canGeneratePlan = Boolean(importPreview?.previewId) && scannedAssetIds.length > 0 && !hasConflicts && !isPlanning;
  const canApply = Boolean(planResult?.ok && importPreview?.previewId && scannedAssetIds.length > 0 && !hasConflicts);

  const handlePlanImport = async () => {
    if (scannedAssetIds.length === 0 || !importPreview?.previewId) return;

    setIsPlanning(true);
    setOperationError(null);
    setStateLabel("生成导入计划中");
    try {
      const result = await importApply({
        previewId: importPreview?.previewId ?? "",
        mode: "planOnly",
        scope: input,
        assetIds: scannedAssetIds,
        conflictResolutions: [],
        backupBeforeApply: true,
      });
      setPlanResult(result);
      setStateLabel(result.ok ? "导入计划已生成" : "导入计划失败");
    } catch (error) {
      setPlanResult(null);
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
      const result = await importApply({
        previewId: importPreview.previewId,
        mode: "apply",
        scope: input,
        assetIds: scannedAssetIds,
        conflictResolutions: [],
        backupBeforeApply: true,
      });
      setApplyResult(result);
      setStateLabel(result.ok ? "导入已执行" : "导入失败");
      if (result.ok) {
        setConfirmationValue("");
        setRefreshKey((current) => current + 1);
      }
    } catch (error) {
      setApplyResult(null);
      setOperationError(errorMessage(error));
      setStateLabel("导入失败");
    } finally {
      setIsApplying(false);
    }
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
        <div className="operation-warning"><AlertTriangle size={17} /><div><strong>{hasConflicts ? `发现 ${scanResult?.conflictCount} 项内容冲突` : previewWarning ?? warning ?? "只读扫描预览"}</strong><span>{hasConflicts ? "请先在冲突处理页面逐项选择跳过、重命名或覆盖；扫描导入不会直接覆盖现有资产。" : planSummary ?? (scanResult?.assets.length ? "当前仅展示发现结果，不执行预览导入或导入。" : "当前扫描没有发现真实资产，确认导入保持禁用。")}</span></div></div>
        <div className="operation-actions"><StaticActionButton className="asset-secondary-action">保存扫描预览</StaticActionButton><button className="asset-secondary-action" data-no-drag="true" disabled={!canGeneratePlan} onClick={handlePlanImport} style={NO_DRAG_REGION_STYLE} type="button">{isPlanning ? "生成中" : "生成导入计划"}</button></div>
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
      </section>
    </div>
  );
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "无法调用导入操作。";
}

function toScanScope(selectedScope: (typeof scopes)[number]["id"]): ScanScope {
  if (selectedScope === "project") return { kind: "project", projectPath: "~/workspace/project-a" };
  if (selectedScope === "custom") return { kind: "custom", path: "~/code/design-system" };
  return { kind: "user" };
}

function toScanRow(asset: AssetSummary) {
  return {
    name: asset.name,
    type: asset.assetType === "skill" ? "Skill" : asset.assetType === "command" ? "Command" : "MCP",
    source: asset.scope === "user" ? "用户级" : asset.scope === "project" ? "项目级" : "资产中心",
    result: asset.status === "invalid" ? "无效" : asset.status === "conflict" ? "冲突" : "发现",
  };
}
