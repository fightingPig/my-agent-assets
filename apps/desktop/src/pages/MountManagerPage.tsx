import {
  AlertTriangle,
  Blocks,
  BookOpen,
  CheckCircle2,
  Eye,
  Folder,
  Link2,
  RefreshCw,
  TerminalSquare,
  Trash2,
} from "lucide-react";
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
  CanonicalUnmountPreview,
  MountBinding,
  RegisteredMountTarget,
} from "../app/contracts";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";
import {
  locationDetail,
  locationLabel,
  ProviderMark,
  providerLabel,
  type MountDraft,
  useMountDrafts,
} from "../ui-assets";
import { demoTargets } from "../ui-assets/mounts/demo-data";

type PreparedDraft =
  | { draft: MountDraft; kind: "mount"; preview: CanonicalMountPreview }
  | { draft: MountDraft; kind: "unmount"; preview: CanonicalUnmountPreview };

export function MountManagerPage({ demoMode = false }: { demoMode?: boolean }) {
  const { drafts, removeDraft, clearDrafts } = useMountDrafts();
  const [assets, setAssets] = useState<readonly AssetSummary[]>(() => demoMode ? demoAssets : []);
  const [targets, setTargets] = useState<readonly RegisteredMountTarget[]>(() => demoMode ? demoAllTargets() : []);
  const [bindings, setBindings] = useState<readonly MountBinding[]>(() => demoMode ? demoCurrentBindings : []);
  const [prepared, setPrepared] = useState<ReadonlyMap<string, PreparedDraft>>(new Map());
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [loadingPreviews, setLoadingPreviews] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [result, setResult] = useState<ApplyResult | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    if (demoMode) {
      setAssets(demoAssets);
      setTargets(demoAllTargets());
      setBindings(demoCurrentBindings);
      return undefined;
    }
    let cancelled = false;
    Promise.all([listAssets({ assetType: null }), listMountTargets(), listMountBindings()])
      .then(([nextAssets, nextTargets, nextBindings]) => {
        if (cancelled) return;
        setAssets(nextAssets);
        setTargets(nextTargets);
        setBindings(nextBindings);
        setPreviewError(null);
      })
      .catch((error) => {
        if (cancelled) return;
        setPreviewError(safeCommandErrorMessage(error, "挂载概览读取失败，请查看系统状态后重试。"));
      });
    return () => { cancelled = true; };
  }, [demoMode, refreshKey]);

  useEffect(() => {
    let cancelled = false;
    if (drafts.length === 0) {
      setPrepared(new Map());
      setPreviewError(null);
      setLoadingPreviews(false);
      return undefined;
    }

    setResult(null);
    setLoadingPreviews(true);
    setPreviewError(null);
    Promise.all(drafts.map(async (draft): Promise<PreparedDraft> => {
      if (demoMode) return demoPreparedDraft(draft);
      if (draft.operation === "mount") {
        return {
          draft,
          kind: "mount",
          preview: await canonicalMountPreview({ assetId: draft.assetId, targetId: draft.targetId }),
        };
      }
      return {
        draft,
        kind: "unmount",
        preview: await canonicalUnmountPreview({ assetId: draft.assetId, targetId: draft.targetId }),
      };
    }))
      .then((entries) => {
        if (cancelled) return;
        setPrepared(new Map(entries.map((entry) => [entry.draft.id, entry])));
      })
      .catch((error) => {
        if (cancelled) return;
        setPrepared(new Map());
        setPreviewError(safeCommandErrorMessage(error, "挂载预览生成失败，请返回资产页检查目标状态。"));
      })
      .finally(() => {
        if (!cancelled) setLoadingPreviews(false);
      });
    return () => { cancelled = true; };
  }, [demoMode, drafts]);

  const outOfSyncCount = bindings.filter((binding) => binding.status !== "mounted").length;
  const readyTargets = targets.filter((target) => target.status === "ready").length;
  const canApply = drafts.length > 0 && drafts.every((draft) => prepared.get(draft.id)?.preview.canApply);
  const mountedRows = useMemo(() => bindings.map((binding) => ({
    binding,
    asset: assets.find((item) => item.id === binding.assetId),
    target: targets.find((item) => item.id === binding.targetId),
  })), [assets, bindings, targets]);

  const applyAll = async () => {
    if (!canApply) return;
    setIsApplying(true);
    setPreviewError(null);
    const steps: ApplyResult["steps"] = [];
    const warnings: string[] = [];
    try {
      if (demoMode) {
        for (const draft of drafts) {
          steps.push({
            stepId: draft.id,
            kind: "mount",
            label: `${draft.operation === "mount" ? "挂载" : "取消挂载"} ${draft.assetName}`,
            status: "success",
            message: "Visual QA 示例变更已完成。",
            affectedPaths: [draft.targetPath],
          });
        }
      } else {
        for (const draft of drafts) {
          const entry = prepared.get(draft.id);
          if (!entry) throw new Error("挂载预览已失效，请重新生成。");
          if (entry.kind === "mount") {
            const applied = await canonicalMountApply({
              previewId: entry.preview.previewId,
              previewGeneratedAtEpochSeconds: entry.preview.generatedAtEpochSeconds,
              request: { assetId: draft.assetId, targetId: draft.targetId },
            });
            steps.push({
              stepId: draft.id,
              kind: "mount",
              label: `挂载 ${draft.assetName}`,
              status: applied.mounted ? "success" : "failed",
              message: applied.mounted ? "挂载完成。" : "挂载未完成。",
              affectedPaths: applied.affectedPaths,
            });
            warnings.push(...applied.warnings);
            if (!applied.mounted) throw new Error("挂载未完成，后续变更已停止。");
          } else {
            const applied = await canonicalUnmountApply({
              previewId: entry.preview.previewId,
              previewGeneratedAtEpochSeconds: entry.preview.generatedAtEpochSeconds,
              request: { assetId: draft.assetId, targetId: draft.targetId },
            });
            steps.push({
              stepId: draft.id,
              kind: "mount",
              label: `取消挂载 ${draft.assetName}`,
              status: applied.unmounted ? "success" : "failed",
              message: applied.unmounted ? "取消挂载完成。" : "取消挂载未完成。",
              affectedPaths: applied.affectedPaths,
            });
            if (!applied.unmounted) throw new Error("取消挂载未完成，后续变更已停止。");
          }
        }
      }

      setResult({
        mode: "apply",
        ok: true,
        previewId: [...prepared.values()][0]?.preview.previewId ?? "demo-mount-preview",
        backup: null,
        steps,
        warnings,
        errors: [],
      });
      clearDrafts();
      setRefreshKey((value) => value + 1);
    } catch (error) {
      const message = safeCommandErrorMessage(error, "挂载变更执行失败；后续变更已停止。请刷新后重试。");
      setResult({
        mode: "apply",
        ok: false,
        previewId: [...prepared.values()][0]?.preview.previewId ?? "mount-preview-failed",
        backup: null,
        steps,
        warnings,
        errors: [message],
      });
    } finally {
      setIsApplying(false);
    }
  };

  return (
    <div className="operation-workspace mount-preview-workspace">
      <section className="mount-overview-strip" aria-label="挂载状态概览">
        <OverviewMetric icon={Link2} label="当前挂载" value={bindings.length} detail="已登记关系" />
        <OverviewMetric icon={Eye} label="待预览变更" value={drafts.length} detail={drafts.length > 0 ? "等待明确确认" : "没有待处理变更"} tone={drafts.length > 0 ? "warning" : "success"} />
        <OverviewMetric icon={AlertTriangle} label="需要关注" value={outOfSyncCount} detail="待同步或孤立" tone={outOfSyncCount > 0 ? "warning" : "success"} />
        <OverviewMetric icon={CheckCircle2} label="可用目标" value={readyTargets} detail="由核心自动推导" tone="success" />
      </section>

      <section className="panel mount-preview-queue">
        <div className="section-heading">
          <div><h3>待确认变更</h3><p>资产页中的 Provider 开关只创建草稿；这里展示后端预览结果并统一确认。</p></div>
          <span className={drafts.length > 0 ? "status-badge warning" : "status-badge neutral"}>{loadingPreviews ? "生成预览中" : `${drafts.length} 项`}</span>
        </div>
        {drafts.length > 0 ? (
          <div className="mount-preview-list">
            {drafts.map((draft) => {
              const entry = prepared.get(draft.id);
              return (
                <div className="mount-preview-row" key={draft.id}>
                  <span className="mount-preview-asset">{assetTypeIcon(draft.assetType)}<span><strong>{draft.assetName}</strong><small>{assetTypeLabel(draft.assetType)}</small></span></span>
                  <span className="mount-preview-direction"><i>{draft.operation === "mount" ? "挂载" : "取消"}</i></span>
                  <span className="mount-preview-target"><ProviderMark provider={draft.provider} size={19} /><span><strong>{draft.targetLabel}</strong><small>{providerLabel(draft.provider)} · {draft.targetPath}</small></span></span>
                  <span className={`asset-status ${entry?.preview.canApply ? "success" : "warning"}`}>{entry ? entry.preview.canApply ? "可执行" : "已阻止" : "生成中"}</span>
                  <button aria-label={`移除 ${draft.assetName} 的挂载变更`} className="icon-button" data-no-drag="true" onClick={() => removeDraft(draft.id)} style={NO_DRAG_REGION_STYLE} type="button"><Trash2 size={15} /></button>
                  {entry ? <div className="mount-preview-effects">{entry.preview.plannedEffects.map((effect) => <span key={effect}>{effect}</span>)}{entry.preview.warnings.map((warning) => <span className="warning-text" key={warning}>{warning}</span>)}</div> : null}
                </div>
              );
            })}
          </div>
        ) : (
          <div className="asset-empty-state mount-preview-empty"><Eye size={22} /><strong>暂无待预览变更</strong><span>请在 Skills、Commands 或 MCP Servers 中选择资产，再点击 Provider 图标。</span></div>
        )}
        {previewError ? <p className="warning-text mount-preview-error">{previewError}</p> : null}
        <ApplyConfirmationPanel
          actionLabel={`确认应用 ${drafts.length} 项变更`}
          canApply={canApply && !loadingPreviews}
          description="应用时会逐项校验 previewId 和目标最新状态；任一失败都会停止后续变更。"
          isApplying={isApplying}
          onApply={() => void applyAll()}
          operationError={previewError}
          result={result}
          title="写入挂载变更"
        />
      </section>

      <section className="panel current-mounts-panel">
        <div className="section-heading">
          <div><h3>整体挂载情况</h3><p>集中查看 canonical 资产、位置与 Provider 的当前关系。</p></div>
          <button className="asset-secondary-action" data-no-drag="true" onClick={() => setRefreshKey((value) => value + 1)} style={NO_DRAG_REGION_STYLE} type="button"><RefreshCw size={14} />刷新</button>
        </div>
        <div className="current-mount-list">
          {mountedRows.map(({ binding, asset, target }) => (
            <div className="current-mount-row" key={binding.id}>
              <div>{asset ? assetTypeIcon(asset.assetType) : <Link2 size={17} />}<span><strong>{asset?.name ?? binding.assetId}</strong><small>{target ? `${locationLabel(target)} · ${locationDetail(target)}` : "目标已不存在"}</small></span></div>
              {target?.provider === "claude_code" || target?.provider === "codex" ? <ProviderMark provider={target.provider} size={18} withLabel /> : <span>自定义目标</span>}
              <span className={`asset-status ${binding.status === "mounted" ? "success" : "warning"}`}>{binding.status === "mounted" ? "已挂载" : binding.status === "out_of_sync" ? "待同步" : "孤立"}</span>
            </div>
          ))}
          {mountedRows.length === 0 ? <div className="asset-empty-state"><Folder size={20} /><strong>暂无挂载关系</strong><span>在资产页切换 Provider 图标后，从上方预览并确认。</span></div> : null}
        </div>
      </section>
    </div>
  );
}

function OverviewMetric({
  icon: Icon,
  label,
  value,
  detail,
  tone = "neutral",
}: {
  icon: typeof Link2;
  label: string;
  value: number;
  detail: string;
  tone?: "neutral" | "success" | "warning";
}) {
  return <div className={`mount-overview-metric ${tone}`}><Icon size={18} /><span><small>{label}</small><strong>{value}</strong><em>{detail}</em></span></div>;
}

function assetTypeIcon(assetType: AssetSummary["assetType"]) {
  if (assetType === "command") return <TerminalSquare size={17} />;
  if (assetType === "mcp") return <Blocks size={17} />;
  return <BookOpen size={17} />;
}

function assetTypeLabel(assetType: AssetSummary["assetType"]) {
  if (assetType === "command") return "Command";
  if (assetType === "mcp") return "MCP Server";
  return "Skill";
}

function demoPreparedDraft(draft: MountDraft): PreparedDraft {
  const base = {
    previewId: `demo-preview-${draft.id}`,
    assetId: draft.assetId,
    targetId: draft.targetId,
    affectedTargetPath: draft.targetPath,
    plannedEffects: [
      draft.operation === "mount"
        ? `将 ${draft.assetName} 写入 ${providerLabel(draft.provider)} 对应目标。`
        : `仅移除 ${draft.targetLabel} 中的运行时关系，canonical 资产继续保留。`,
    ],
    warnings: [],
    backupRequired: draft.assetType === "mcp",
    canApply: true,
    generatedAtEpochSeconds: 1,
    expiresAtEpochSeconds: 9999999999,
  };
  if (draft.operation === "mount") {
    return {
      draft,
      kind: "mount",
      preview: {
        ...base,
        canonicalPath: `assets/${draft.assetType}/${draft.assetName}`,
        compatible: true,
        adapter: draft.assetType === "mcp" ? draft.provider === "codex" ? "toml_mcp_patch" : "json_mcp_patch" : draft.assetType === "skill" ? "symlink_directory" : "symlink_file",
        disposition: draft.assetType === "mcp" ? "compile_mcp" : "create_link",
      },
    };
  }
  return { draft, kind: "unmount", preview: base };
}

function demoAllTargets() {
  return [...demoTargets("skill"), ...demoTargets("command"), ...demoTargets("mcp")];
}

const demoAssets: readonly AssetSummary[] = [
  { id: "review", name: "review", title: "代码审查工作流", assetType: "skill", status: "mounted", category: "工程质量", description: "统一代码审查流程与输出格式", sourcePath: "assets/skills/review", scope: "user", updatedAt: "今天 10:24", mountTargets: [] },
  { id: "deploy-prod", name: "deploy-prod", title: "生产环境部署", assetType: "command", status: "mounted", category: "交付流程", description: "生成生产部署检查与执行步骤", sourcePath: "assets/commands/deploy-prod.md", scope: "project", updatedAt: "今天 09:40", mountTargets: [] },
  { id: "PostgreSQL", name: "PostgreSQL", title: "PostgreSQL 数据访问", assetType: "mcp", status: "mounted", category: "数据库", description: "本地 MCP 配置", sourcePath: "assets/mcps/postgresql.json", scope: "user", updatedAt: "今天 10:12", mountTargets: [] },
];

const demoCurrentBindings: readonly MountBinding[] = [
  { id: "binding-review-user", assetId: "review", targetId: "demo-skill-claude-user", status: "mounted", lastSyncedAt: "今天 10:24" },
  { id: "binding-review-project", assetId: "review", targetId: "demo-skill-codex-design-system", status: "mounted", lastSyncedAt: "今天 10:20" },
  { id: "binding-command-project", assetId: "deploy-prod", targetId: "demo-command-claude-project-a", status: "out_of_sync", lastSyncedAt: "今天 09:40" },
  { id: "binding-mcp-user", assetId: "PostgreSQL", targetId: "demo-mcp-codex-user", status: "mounted", lastSyncedAt: "今天 10:12" },
];
