import { AlertCircle, Folder, UserRound } from "lucide-react";
import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { listMountBindings, listMountTargets, safeCommandErrorMessage } from "../../app/data-api";
import type { MountBinding, RegisteredMountTarget } from "../../app/contracts";
import { NO_DRAG_REGION_STYLE } from "../../lib/platform";
import { ProviderMark, providerLabel, type SupportedMountProvider } from "../components/ProviderMark";
import type { UiAssetType } from "../contracts";
import { useMountDrafts } from "./MountDraftContext";
import { demoBindings, demoTargets } from "./demo-data";
import { mountCellState, supportsProvider, toMountLocationRows } from "./model";

type MountMatrixProps = {
  assetId: string;
  assetName: string;
  assetType: UiAssetType;
  demoMode?: boolean;
};

const providers: readonly SupportedMountProvider[] = ["claude_code", "codex"];

export function MountMatrix({ assetId, assetName, assetType, demoMode = false }: MountMatrixProps) {
  const [targets, setTargets] = useState<readonly RegisteredMountTarget[]>(() => demoMode ? demoTargets(assetType) : []);
  const [bindings, setBindings] = useState<readonly MountBinding[]>(() => demoMode ? demoBindings(assetId, assetType) : []);
  const [error, setError] = useState<string | null>(null);
  const { drafts, stageDraft } = useMountDrafts();

  useEffect(() => {
    let cancelled = false;
    if (demoMode) {
      setTargets(demoTargets(assetType));
      setBindings(demoBindings(assetId, assetType));
      setError(null);
      return undefined;
    }
    Promise.all([listMountTargets(), listMountBindings()])
      .then(([nextTargets, nextBindings]) => {
        if (cancelled) return;
        setTargets(nextTargets);
        setBindings(nextBindings);
        setError(null);
      })
      .catch((reason) => {
        if (cancelled) return;
        setTargets([]);
        setBindings([]);
        setError(safeCommandErrorMessage(reason, "挂载位置读取失败，请稍后重试。"));
      });
    return () => { cancelled = true; };
  }, [assetId, assetType, demoMode]);

  const rows = useMemo(() => toMountLocationRows(targets, assetType), [assetType, targets]);
  const visibleProviders = providers.filter((provider) => supportsProvider(assetType, provider));
  const tableGridStyle = { "--maa-provider-count": visibleProviders.length } as CSSProperties;
  const cells = rows.flatMap((row) => visibleProviders
    .map((provider) => ({ row, provider, target: row.providers[provider] })));
  const mountedCount = cells.filter(({ target }) => {
    const draft = drafts.find((candidate) => candidate.assetId === assetId && candidate.targetId === target?.id);
    if (draft) return draft.operation === "mount";
    const state = mountCellState(assetId, target, bindings);
    return state === "mounted" || state === "out_of_sync";
  }).length;
  const projectRows = rows.filter((row) => row.scope !== "user");
  const userRows = rows.filter((row) => row.scope === "user");

  return (
    <section className="maa-mount-matrix" aria-label={`${assetName} 挂载位置`}>
      <div className="maa-mount-matrix-heading">
        <div>
          <strong>挂载位置</strong>
          <span>已挂载 {mountedCount} / {cells.length}</span>
        </div>
        <p>点击 Provider 图标切换挂载状态；所有变更先进入预览。</p>
      </div>

      {error ? <div className="maa-inline-error"><AlertCircle size={15} />{error}</div> : null}
      {rows.length > 0 ? (
        <div className="maa-mount-table">
          <div className="maa-mount-table-head" role="row" style={tableGridStyle}>
            <span>位置</span>
            <span>运行状态</span>
            {visibleProviders.map((provider) => (
              <span key={provider}><ProviderMark provider={provider} size={17} withLabel /></span>
            ))}
          </div>
          {userRows.map((row) => <MountRow assetId={assetId} assetName={assetName} assetType={assetType} bindings={bindings} key={row.id} row={row} />)}
          {projectRows.length > 0 ? <div className="maa-mount-table-group">项目级（{projectRows.length}）</div> : null}
          {projectRows.map((row) => <MountRow assetId={assetId} assetName={assetName} assetType={assetType} bindings={bindings} key={row.id} row={row} />)}
        </div>
      ) : (
        <div className="maa-mount-empty">
          <Folder size={20} />
          <strong>暂无兼容挂载位置</strong>
          <span>{assetType === "command" ? "Command 仅支持 Claude Code 目标。" : "请先添加项目或初始化 Provider 运行环境。"}</span>
        </div>
      )}
    </section>
  );

  function MountRow({
    assetId: rowAssetId,
    assetName: rowAssetName,
    assetType: rowAssetType,
    bindings: rowBindings,
    row,
  }: {
    assetId: string;
    assetName: string;
    assetType: UiAssetType;
    bindings: readonly MountBinding[];
    row: ReturnType<typeof toMountLocationRows>[number];
  }) {
    const LocationIcon = row.scope === "user" ? UserRound : Folder;
    return (
      <div className="maa-mount-table-row" role="row" style={tableGridStyle}>
        <span className="maa-mount-location">
          <i><LocationIcon size={16} /></i>
          <span><strong>{row.label}</strong><small>{row.detail}</small></span>
        </span>
        <span className={`maa-runtime-state ${row.status}`}><i />{row.status === "ready" ? "正常" : "不可用"}</span>
        {visibleProviders.map((provider) => {
          const target = row.providers[provider];
          const baseState = mountCellState(rowAssetId, target, rowBindings);
          const draft = drafts.find((candidate) => candidate.assetId === rowAssetId && candidate.targetId === target?.id);
          const visualState = draft ? `pending-${draft.operation}` : baseState;
          const willMount = baseState === "unmounted" || baseState === "blocked";
          const operation = willMount ? "mount" : "unmount";
          const title = target
            ? `${operation === "mount" ? "挂载到" : "取消挂载"} ${row.label} · ${providerLabel(provider)}`
            : `${row.label} 没有 ${providerLabel(provider)} 目标`;
          return (
            <span className="maa-mount-toggle-cell" key={provider}>
              <button
                aria-label={title}
                aria-pressed={visualState === "mounted" || visualState === "out_of_sync" || visualState === "pending-mount"}
                className={`maa-provider-toggle ${visualState}`}
                data-no-drag="true"
                disabled={!target || baseState === "blocked"}
                onClick={() => {
                  if (!target) return;
                  stageDraft({
                    id: `${rowAssetId}:${target.id}`,
                    assetId: rowAssetId,
                    assetName: rowAssetName,
                    assetType: rowAssetType,
                    targetId: target.id,
                    targetLabel: row.label,
                    targetPath: target.path,
                    provider,
                    operation,
                    previousState: baseState,
                  });
                }}
                style={NO_DRAG_REGION_STYLE}
                title={title}
                type="button"
              >
                <ProviderMark provider={provider} size={20} />
              </button>
            </span>
          );
        })}
      </div>
    );
  }
}
