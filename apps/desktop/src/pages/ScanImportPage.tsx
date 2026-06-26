import { AlertTriangle, Check, FolderSearch, House, ScanSearch } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { importApply, previewImport, scanAssets } from "../app/data-api";
import type { ApplyResult, AssetSummary, ImportPreview, ScanResult, ScanScope } from "../app/contracts";
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

export function ScanImportPage() {
  const [selectedScope, setSelectedScope] = useState<(typeof scopes)[number]["id"]>("user");
  const [scanResult, setScanResult] = useState<ScanResult | null>(null);
  const [importPreview, setImportPreview] = useState<ImportPreview | null>(null);
  const [planResult, setPlanResult] = useState<ApplyResult | null>(null);
  const [stateLabel, setStateLabel] = useState("读取中");
  const [isPlanning, setIsPlanning] = useState(false);

  const input = useMemo(() => toScanScope(selectedScope), [selectedScope]);

  useEffect(() => {
    let cancelled = false;
    setStateLabel("读取中");
    setPlanResult(null);
    scanAssets({ scope: input })
      .then((result) => {
        if (cancelled) return;
        if (result && typeof result === "object" && "counts" in result) {
          setScanResult(result);
          setStateLabel(result.assets.length > 0 ? "只读真实数据" : "静态预览");
          if (result.assets.length > 0) {
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
          setStateLabel("静态预览");
        }
      })
      .catch(() => {
        if (cancelled) return;
        setScanResult(null);
        setImportPreview(null);
        setStateLabel("读取失败，使用静态预览");
      });
    return () => {
      cancelled = true;
    };
  }, [input]);

  const rows = scanResult?.assets.length ? scanResult.assets.map(toScanRow) : staticResults;
  const counts = scanResult?.assets.length
    ? scanResult.counts
    : { total: 14, skills: 4, commands: 4, mcps: 4 };
  const warning = scanResult?.warnings[0];
  const previewWarning = importPreview?.warnings[0];
  const previewStepText = importPreview?.steps.map((step) => step.label).join(" / ");
  const scannedAssetIds = scanResult?.assets.map((asset) => asset.id) ?? [];
  const planSummary = planResult?.steps.length
    ? planResult.steps.map((step) => step.message).join(" / ")
    : previewStepText;
  const canGeneratePlan = Boolean(importPreview?.previewId) && scannedAssetIds.length > 0 && !isPlanning;

  const handlePlanImport = async () => {
    if (scannedAssetIds.length === 0 || !importPreview?.previewId) return;

    setIsPlanning(true);
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
    } catch {
      setPlanResult(null);
      setStateLabel("导入计划失败");
    } finally {
      setIsPlanning(false);
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
          {scopes.map(({ id, title, detail, icon: Icon }) => <button aria-pressed={selectedScope === id} className={`scope-card ${selectedScope === id ? "selected" : ""}`} data-no-drag="true" key={id} onClick={() => setSelectedScope(id)} style={NO_DRAG_REGION_STYLE} type="button"><span><Icon size={18} /></span><strong>{title}</strong><small>{detail}</small></button>)}
        </div>
      </section>

      <div className="scan-summary-grid">
        <div className="panel"><span>发现资产</span><strong>{counts.total}</strong><small>{stateLabel}</small></div><div className="panel"><span>Skills</span><strong>{counts.skills}</strong><small>只读扫描</small></div><div className="panel"><span>Commands</span><strong>{counts.commands}</strong><small>只读扫描</small></div><div className="panel"><span>MCP Servers</span><strong>{counts.mcps}</strong><small>只读扫描</small></div>
      </div>

      <section className="panel operation-section">
        <div className="section-heading"><div><h3>导入预览</h3><p>当前范围：{scopes.find((scope) => scope.id === selectedScope)?.title}</p></div><span>{rows.length} 项待确认</span></div>
        <div className="preview-table" role="table" aria-label="导入预览表"><div className="preview-table-head" role="row"><span>资产</span><span>类型</span><span>来源</span><span>结果</span></div>{rows.map((result) => <div className="preview-table-row" role="row" key={`${result.type}:${result.name}`}><strong>{result.name}</strong><span>{result.type}</span><span>{result.source}</span><span className={result.result === "冲突" || result.result === "无效" ? "warning-text" : "success-text"}>{result.result}</span></div>)}</div>
        <div className="operation-warning"><AlertTriangle size={17} /><div><strong>{previewWarning ?? warning ?? "只读扫描预览"}</strong><span>{planSummary ?? (scanResult?.assets.length ? "当前仅展示发现结果，不执行预览导入或导入。" : "未读取到真实资产时保留静态预览，确认导入仍然禁用。")}</span></div></div>
        <div className="operation-actions"><StaticActionButton className="asset-secondary-action">保存扫描预览</StaticActionButton><button className="asset-secondary-action" data-no-drag="true" disabled={!canGeneratePlan} onClick={handlePlanImport} style={NO_DRAG_REGION_STYLE} type="button">{isPlanning ? "生成中" : "生成导入计划"}</button><StaticActionButton className="asset-business-action">确认导入</StaticActionButton></div>
      </section>
    </div>
  );
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
    result: asset.status === "invalid" ? "无效" : "发现",
  };
}
