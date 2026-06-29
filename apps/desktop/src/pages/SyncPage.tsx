import { AlertTriangle, ArrowDown, ArrowUp, CheckCircle2, GitBranch, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { gitStatus, previewSync, syncApply } from "../app/data-api";
import type { ApplyResult, GitStatus, SyncDirection, SyncPreview } from "../app/contracts";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { StaticActionButton } from "../components/ui/StaticActionButton";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

const fallbackGitStatus: GitStatus = {
  repositoryPath: "~/.my-agent-assets",
  isRepository: false,
  statusMessage: "静态预览：尚未读取本地 Git 仓库。",
  branch: "main",
  remoteName: "origin",
  remoteIdentity: "github.com/example/private-assets",
  upstream: "origin/main",
  clean: true,
  ahead: 2,
  behind: 1,
  changedFiles: [],
  conflicts: ["assets.yaml"],
  syncableChanges: ["assets.yaml"],
  blockedChanges: [],
  lastSyncedAt: null,
};

const emptyGitStatus: GitStatus = {
  repositoryPath: "~/.my-agent-assets",
  isRepository: false,
  statusMessage: "尚未读取本地 Git 仓库。",
  branch: "",
  remoteName: "origin",
  clean: true,
  ahead: 0,
  behind: 0,
  changedFiles: [],
  conflicts: [],
  syncableChanges: [],
  blockedChanges: [],
  lastSyncedAt: null,
};

export function SyncPage({ demoMode = false }: { demoMode?: boolean }) {
  const [status, setStatus] = useState<GitStatus>(demoMode ? fallbackGitStatus : emptyGitStatus);
  const [preview, setPreview] = useState<SyncPreview | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [stateLabel, setStateLabel] = useState("读取中");
  const [planningDirection, setPlanningDirection] = useState<SyncDirection | null>(null);
  const [isApplying, setIsApplying] = useState(false);
  const [operationError, setOperationError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    if (demoMode) {
      setStatus(fallbackGitStatus);
      setStateLabel("Visual QA 示例数据");
      return undefined;
    }
    setStatus(emptyGitStatus);
    setStateLabel("读取中");
    gitStatus()
      .then((loaded) => {
        if (cancelled) return;
        if (loaded && typeof loaded === "object" && "repositoryPath" in loaded) {
          setStatus(loaded);
          setPreview(null);
          setApplyResult(null);
          setOperationError(null);
          setStateLabel("只读真实数据");
        } else {
        setStatus(emptyGitStatus);
        setPreview(null);
        setApplyResult(null);
        setOperationError(null);
        setStateLabel("未返回 Git 状态");
        }
      })
      .catch((error) => {
        if (cancelled) return;
        setStatus(emptyGitStatus);
        setPreview(null);
        setApplyResult(null);
        setOperationError(null);
        setStateLabel(`读取失败：${errorMessage(error)}`);
      });
    return () => {
      cancelled = true;
    };
  }, [demoMode]);

  const cleanLabel = status.clean ? "工作区干净" : `${status.changedFiles.length} 项变更`;
  const conflictLabel = status.conflicts.length > 0 ? `${status.conflicts.length} 项预览` : "0 项";
  const previewSummary = preview?.plannedEffects.length
    ? preview.plannedEffects.join(" / ")
    : status.conflicts.length > 0
      ? `检测到冲突文件：${status.conflicts.join(", ")}`
    : "选择 Pull 或 Push 预览后，这里会显示本地 Git 同步计划。";
  const canApply = Boolean(preview?.canApply && preview.previewId);

  const handlePreviewSync = async (direction: SyncDirection) => {
    setPlanningDirection(direction);
    setOperationError(null);
    setStateLabel(direction === "pull" ? "生成 Pull 计划中" : "生成 Push 计划中");
    try {
      const result = await previewSync({ direction });
      setPreview(result);
      setApplyResult(null);
      setStateLabel(direction === "pull" ? "Pull 计划预览" : "Push 计划预览");
    } catch (error) {
      setPreview(null);
      setOperationError(errorMessage(error));
      setStateLabel("同步计划读取失败");
    } finally {
      setPlanningDirection(null);
    }
  };

  const handleApplySync = async () => {
    if (!preview?.previewId || !canApply) return;

    setIsApplying(true);
    setOperationError(null);
    setStateLabel(preview.direction === "pull" ? "执行 Pull 中" : "执行 Push 中");
    try {
      const result = await syncApply({
        previewId: preview.previewId,
        previewGeneratedAtEpochSeconds: preview.generatedAtEpochSeconds,
        request: { direction: preview.direction },
      });
      setApplyResult(toApplyResult(result));
      setStateLabel("同步已执行");
      const loaded = await gitStatus();
      setStatus(loaded);
    } catch (error) {
      setApplyResult(null);
      setOperationError(errorMessage(error));
      setStateLabel("同步失败");
    } finally {
      setIsApplying(false);
    }
  };

  return (
    <div className="operation-workspace sync-workspace">
      <section className="panel sync-repository-card">
        <div className="section-heading"><div><h3>本地 Git 仓库</h3><p>{status.repositoryPath}</p></div><span className="healthy-badge"><CheckCircle2 size={13} />{cleanLabel}</span></div>
        <div className="sync-status-grid"><div><GitBranch size={17} /><span><small>当前分支</small><strong>{status.branch || "未检测到"}</strong></span></div><div><RefreshCw size={17} /><span><small>远程仓库</small><strong>{status.remoteIdentity ?? status.upstream ?? status.remoteName}</strong></span></div><div><ArrowUp size={17} /><span><small>Ahead</small><strong>{status.ahead} commits</strong></span></div><div><ArrowDown size={17} /><span><small>Behind</small><strong>{status.behind} commits</strong></span></div></div>
        <div className="sync-graph"><div className="sync-graph-line"><span className="local-dot" /><strong>本地 {status.branch || "工作区"}</strong><small>{status.statusMessage}</small></div><div className="sync-graph-line"><span /><strong>仓库状态</strong><small>{status.isRepository ? "已识别为本地 Git 仓库" : "未识别为本地 Git 仓库"}</small></div><div className="sync-graph-line"><span className="remote-dot" /><strong>远程仓库</strong><small>{status.remoteIdentity ?? status.upstream ?? `remote: ${status.remoteName}`}</small></div></div>
        <div className="operation-actions"><button className="asset-secondary-action" data-no-drag="true" disabled={planningDirection !== null} onClick={() => handlePreviewSync("pull")} style={NO_DRAG_REGION_STYLE} type="button">{planningDirection === "pull" ? "生成中" : "预览 Pull"}</button><button className="asset-secondary-action" data-no-drag="true" disabled={planningDirection !== null} onClick={() => handlePreviewSync("push")} style={NO_DRAG_REGION_STYLE} type="button">{planningDirection === "push" ? "生成中" : "预览 Push"}</button><StaticActionButton className="asset-secondary-action">导出计划</StaticActionButton></div>
        <ApplyConfirmationPanel actionLabel={preview?.direction === "pull" ? "执行 Pull" : "执行 Push"} canApply={canApply} description="后端会校验 previewId、远端身份与当前仓库状态；Push 仅允许 GitHub Private 仓库并只 stage canonical 白名单。" isApplying={isApplying} onApply={handleApplySync} operationError={operationError} result={applyResult} title="执行同步" />
      </section>

      <div className="detail-two-column sync-lower-grid">
        <section className="panel detail-section"><div className="section-heading"><div><h3>同步历史</h3><p>最近的本地 Git 操作</p></div><span className="preview-label">{stateLabel}</span></div><div className="timeline-list">{status.lastSyncedAt ? <div><CheckCircle2 size={14} /><span>最近一次本地同步</span><time>{status.lastSyncedAt}</time></div> : <div className="asset-empty-state"><RefreshCw size={20} /><strong>暂无同步历史</strong><span>执行真实 Pull 或 Push 后会在本地 operation journal 留下记录。</span></div>}</div></section>
        <section className="panel detail-section"><div className="section-heading"><div><h3>同步检查</h3><p>执行前风险预览</p></div></div><div className="operation-warning"><AlertTriangle size={17} /><div><strong>{preview?.warnings[0] ?? status.statusMessage}</strong><span>{previewSummary}</span></div></div><div className="environment-list"><div><strong>仓库可用</strong><span>{status.isRepository ? "是" : "否"}</span></div><div><strong>白名单变更</strong><span>{status.syncableChanges.length} 项</span></div><div><strong>阻断变更</strong><span>{status.blockedChanges.length} 项</span></div><div><strong>潜在冲突</strong><span>{conflictLabel}</span></div><div><strong>远程可见性</strong><span>{preview?.repositoryVisibility ?? "未验证"}</span></div><div><strong>计划方向</strong><span>{preview?.direction === "pull" ? "Pull" : preview?.direction === "push" ? "Push" : "未选择"}</span></div><div><strong>计划可执行</strong><span>{preview?.canApply ? "是" : "否"}</span></div></div></section>
      </div>
    </div>
  );
}

function toApplyResult(
  result: Awaited<ReturnType<typeof syncApply>>,
): ApplyResult {
  const action = result.direction === "pull" ? "Pull" : "Push";
  return {
    mode: "apply",
    ok: true,
    previewId: result.previewId,
    backup: null,
    steps: [{
      stepId: `git-${result.direction}`,
      kind: "git",
      label: `执行 ${action}`,
      status: "success",
      message: result.pulled
        ? "已完成 fast-forward Pull。"
        : result.pushed
          ? "已完成 Private repository Push。"
          : "同步完成。",
      affectedPaths: result.affectedPaths,
    }],
    warnings: result.warnings,
    errors: [],
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "无法调用同步操作。";
}
