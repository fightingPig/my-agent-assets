import { AlertTriangle, Blocks, BookOpen, FolderKanban, Link2, TerminalSquare, Unlink } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  canonicalMountApply,
  canonicalMountPreview,
  canonicalUnmountApply,
  canonicalUnmountPreview,
  listAssets,
  listMountBindings,
  listMountTargets,
  safeCommandErrorMessage,
} from "../app/data-api";
import type {
  ApplyResult,
  AssetSummary,
  CanonicalMountPreview,
  CanonicalMountPreviewRequest,
  CanonicalUnmountPreview,
  MountBinding,
  RegisteredMountTarget,
} from "../app/contracts";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

type View = "create" | "current";

export function MountManagerPage({ demoMode = false }: { demoMode?: boolean }) {
  const [view, setView] = useState<View>("create");
  const [assets, setAssets] = useState<AssetSummary[]>([]);
  const [targets, setTargets] = useState<RegisteredMountTarget[]>([]);
  const [bindings, setBindings] = useState<MountBinding[]>([]);
  const [selectedAssetId, setSelectedAssetId] = useState("");
  const [selectedTargetId, setSelectedTargetId] = useState("");
  const [preview, setPreview] = useState<CanonicalMountPreview | null>(null);
  const [unmountPreview, setUnmountPreview] = useState<CanonicalUnmountPreview | null>(null);
  const [selectedBinding, setSelectedBinding] = useState<MountBinding | null>(null);
  const [result, setResult] = useState<ApplyResult | null>(null);
  const [isApplying, setIsApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    if (demoMode) return;
    let cancelled = false;
    Promise.all([listAssets({ assetType: null }), listMountTargets(), listMountBindings()])
      .then(([loadedAssets, loadedTargets, loadedBindings]) => {
        if (cancelled) return;
        setAssets(loadedAssets);
        setTargets(loadedTargets);
        setBindings(loadedBindings);
        setSelectedAssetId((current) => current || loadedAssets[0]?.id || "");
      })
      .catch((error) => !cancelled && setError(safeCommandErrorMessage(error, "挂载数据读取失败，请查看系统状态后重试。")));
    return () => { cancelled = true; };
  }, [demoMode, refreshKey]);

  const asset = assets.find((item) => item.id === selectedAssetId) ?? assets[0];
  const compatibleTargets = useMemo(
    () => targets.filter((target) =>
      asset && target.accepts.includes(asset.assetType) && target.status === "ready"
    ),
    [asset, targets],
  );
  const target = compatibleTargets.find((item) => item.id === selectedTargetId) ?? compatibleTargets[0];
  const previewInput = asset && target
    ? { assetId: asset.id, targetId: target.id } satisfies CanonicalMountPreviewRequest
    : null;

  useEffect(() => {
    setSelectedTargetId(compatibleTargets[0]?.id ?? "");
  }, [selectedAssetId, compatibleTargets]);

  const generateMountPreview = async () => {
    if (!previewInput) return;
    setError(null);
    setResult(null);
    try {
      setPreview(await canonicalMountPreview(previewInput));
    } catch (error) {
      setPreview(null);
      setError(safeCommandErrorMessage(error, "挂载预览生成失败，请检查资产和目标状态。"));
    }
  };

  const applyMount = async () => {
    if (!preview?.canApply || !previewInput) return;
    setIsApplying(true);
    try {
      const applied = await canonicalMountApply({
        previewId: preview.previewId,
        previewGeneratedAtEpochSeconds: preview.generatedAtEpochSeconds,
        request: previewInput,
      });
      setResult(toApplyResult(applied.mounted, applied.previewId, applied.affectedPaths, "挂载"));
      setPreview(null);
      setRefreshKey((value) => value + 1);
    } catch (error) {
      setError(safeCommandErrorMessage(error, "挂载执行失败，未完成的事务会在下次启动时自动回滚。"));
    } finally {
      setIsApplying(false);
    }
  };

  const previewUnmount = async (binding: MountBinding) => {
    setSelectedBinding(binding);
    setResult(null);
    setError(null);
    try {
      setUnmountPreview(await canonicalUnmountPreview({
        assetId: binding.assetId,
        targetId: binding.targetId,
      }));
    } catch (error) {
      setUnmountPreview(null);
      setError(safeCommandErrorMessage(error, "解除挂载预览生成失败。"));
    }
  };

  const applyUnmount = async () => {
    if (!selectedBinding || !unmountPreview?.canApply) return;
    setIsApplying(true);
    try {
      const applied = await canonicalUnmountApply({
        previewId: unmountPreview.previewId,
        previewGeneratedAtEpochSeconds: unmountPreview.generatedAtEpochSeconds,
        request: { assetId: selectedBinding.assetId, targetId: selectedBinding.targetId },
      });
      setResult(toApplyResult(applied.unmounted, applied.previewId, applied.affectedPaths, "解除挂载"));
      setUnmountPreview(null);
      setSelectedBinding(null);
      setRefreshKey((value) => value + 1);
    } catch (error) {
      setError(safeCommandErrorMessage(error, "解除挂载失败；运行时内容未被安全移除。"));
    } finally {
      setIsApplying(false);
    }
  };

  return (
    <div className="operation-workspace">
      <div className="segmented-view-switch" role="tablist" aria-label="挂载工作视图">
        <button aria-selected={view === "create"} data-no-drag="true" onClick={() => setView("create")} role="tab" style={NO_DRAG_REGION_STYLE} type="button">新建挂载</button>
        <button aria-selected={view === "current"} data-no-drag="true" onClick={() => setView("current")} role="tab" style={NO_DRAG_REGION_STYLE} type="button">当前挂载 <span>{bindings.length}</span></button>
      </div>

      {view === "create" ? (
        <>
          <section className="panel mount-workflow">
            <div className="mount-flow-column">
              <FlowHeading number="1" title="选择资产" subtitle="Canonical Asset Center" />
              <div className="selectable-stack">{assets.map((item) => <button aria-pressed={asset?.id === item.id} className={asset?.id === item.id ? "selected" : ""} data-no-drag="true" key={item.id} onClick={() => { setSelectedAssetId(item.id); setPreview(null); }} style={NO_DRAG_REGION_STYLE} type="button">{assetIcon(item)}<span><strong>{item.name}</strong><small>{assetTypeLabel(item)} · {item.description}</small></span></button>)}{assets.length === 0 ? <Empty title="暂无可挂载资产" detail="请先在扫描导入页添加资产。" /> : null}</div>
            </div>
            <div className="mount-flow-column">
              <FlowHeading number="2" title="选择位置与 Provider" subtitle="用户级 / 已维护项目 / 高级自定义" />
              <div className="selectable-stack">{compatibleTargets.map((item) => <button aria-pressed={target?.id === item.id} className={target?.id === item.id ? "selected" : ""} data-no-drag="true" key={item.id} onClick={() => { setSelectedTargetId(item.id); setPreview(null); }} style={NO_DRAG_REGION_STYLE} type="button"><FolderKanban size={17} /><span><strong>{targetLabel(item)}</strong><small>{scopeLabel(item)} · {item.path}</small></span></button>)}{asset && compatibleTargets.length === 0 ? <Empty title="没有兼容目标" detail={asset.assetType === "command" ? "Command 仅支持 Claude-compatible 目标。" : "请先维护项目或注册高级自定义目标。"} /> : null}</div>
            </div>
            <div className="mount-flow-column plan">
              <FlowHeading number="3" title="预览挂载计划" subtitle="执行前不修改任何文件" />
              {asset && target ? <><div className="mount-plan-summary"><div>{assetIcon(asset)}<span><strong>{asset.name}</strong><small>{assetTypeLabel(asset)}</small></span></div><i>→</i><div><FolderKanban size={17} /><span><strong>{targetLabel(target)}</strong><small>{preview?.affectedTargetPath ?? target.path}</small></span></div></div><div className="plan-lines">{preview?.plannedEffects.map((line) => <span key={line}>{line}</span>)}</div></> : <Empty title="等待资产和目标" detail="完成前两步后生成挂载预览。" />}
            </div>
          </section>
          <section className="panel mount-review-bar">
            <div className="operation-warning"><AlertTriangle size={17} /><div><strong>{preview?.backupRequired ? "执行前将创建本地备份" : "挂载尚未执行"}</strong><span>{preview?.unsupportedReason ?? preview?.warnings[0] ?? "先生成计划，再明确确认写入。"}</span></div></div>
            <button className="asset-secondary-action" data-no-drag="true" disabled={!previewInput} onClick={() => void generateMountPreview()} style={NO_DRAG_REGION_STYLE} type="button">生成挂载计划</button>
            <ApplyConfirmationPanel actionLabel="确认挂载" canApply={Boolean(preview?.canApply)} description="后端根据资产、位置、Provider 和范围推导目标；执行前校验预览并创建所需备份。" isApplying={isApplying} onApply={() => void applyMount()} operationError={error} result={result} title="执行挂载" />
          </section>
        </>
      ) : (
        <section className="panel current-mounts-panel">
          <div className="section-heading"><div><h3>当前挂载</h3><p>逐条查看 canonical 资产与运行时位置，并在预览后解除。</p></div><span>{bindings.length} 条</span></div>
          <div className="current-mount-list">{bindings.map((binding) => {
            const boundAsset = assets.find((item) => item.id === binding.assetId);
            const boundTarget = targets.find((item) => item.id === binding.targetId);
            return <div className="current-mount-row" key={binding.id}><div>{boundAsset ? assetIcon(boundAsset) : <Link2 size={17} />}<span><strong>{boundAsset?.name ?? binding.assetId}</strong><small>{boundTarget ? `${targetLabel(boundTarget)} · ${boundTarget.path}` : "目标已不存在"}</small></span></div><span className={`asset-status ${binding.status === "mounted" ? "success" : "warning"}`}>{binding.status === "mounted" ? "已挂载" : binding.status === "out_of_sync" ? "待同步" : "孤立"}</span><button className="asset-secondary-action" data-no-drag="true" onClick={() => void previewUnmount(binding)} style={NO_DRAG_REGION_STYLE} type="button"><Unlink size={14} />预览解除</button></div>;
          })}{bindings.length === 0 ? <Empty title="暂无挂载关系" detail="新建挂载后会在这里显示。" /> : null}</div>
          {unmountPreview ? <div className="unmount-preview-panel"><div className="operation-warning"><AlertTriangle size={17} /><div><strong>解除挂载影响</strong><span>{unmountPreview.plannedEffects.join("；") || unmountPreview.affectedTargetPath}</span></div></div><ApplyConfirmationPanel actionLabel="确认解除挂载" canApply={unmountPreview.canApply} description="只移除所选运行目标的链接或 MCP entry，canonical 资产继续保留。" isApplying={isApplying} onApply={() => void applyUnmount()} operationError={error} result={result} title="执行解除挂载" /></div> : null}
        </section>
      )}
    </div>
  );
}

function FlowHeading({ number, title, subtitle }: { number: string; title: string; subtitle: string }) {
  return <div className="mount-flow-heading"><span>{number}</span><div><strong>{title}</strong><small>{subtitle}</small></div></div>;
}

function Empty({ title, detail }: { title: string; detail: string }) {
  return <div className="asset-empty-state"><Link2 size={20} /><strong>{title}</strong><span>{detail}</span></div>;
}

function assetIcon(asset: Pick<AssetSummary, "assetType">) {
  if (asset.assetType === "command") return <TerminalSquare size={17} />;
  if (asset.assetType === "mcp") return <Blocks size={17} />;
  return <BookOpen size={17} />;
}

function assetTypeLabel(asset: Pick<AssetSummary, "assetType">) {
  return asset.assetType === "command" ? "Command" : asset.assetType === "mcp" ? "MCP" : "Skill";
}

function targetLabel(target: RegisteredMountTarget) {
  const provider = target.provider === "claude_code" ? "Claude Code" : target.provider === "codex" ? "Codex" : "自定义";
  const kind = target.accepts[0] === "mcp" ? "MCP" : target.accepts[0] === "command" ? "Commands" : "Skills";
  return `${provider} ${kind}`;
}

function scopeLabel(target: RegisteredMountTarget) {
  if (target.scope === "user") return "用户级";
  if (target.scope === "project") return "已维护项目";
  if (target.scope === "local") return "Claude Local";
  return "高级自定义";
}

function toApplyResult(ok: boolean, previewId: string, affectedPaths: string[], label: string): ApplyResult {
  return {
    mode: "apply",
    ok,
    previewId,
    backup: null,
    steps: [{ stepId: label, kind: "mount", label, status: ok ? "success" : "failed", message: ok ? `${label}完成。` : `${label}未完成。`, affectedPaths }],
    warnings: [],
    errors: ok ? [] : [`${label}未完成。`],
  };
}
