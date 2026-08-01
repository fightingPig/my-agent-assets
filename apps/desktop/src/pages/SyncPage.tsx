import { AlertTriangle, ArrowDown, ArrowUp, CheckCircle2, GitBranch, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { gitStatus, listAuditLog, previewSync, safeCommandErrorMessage, settingsApply, settingsLoad, settingsPreview, syncApply } from "../app/data-api";
import type { ApplyResult, AuditLogEntry, DesktopSettings, GitStatus, SettingsPreview as SettingsSavePreview, SyncDirection, SyncPreview } from "../app/contracts";
import { DEFAULT_ASSET_CENTER_PATH, DEFAULT_GIT_REMOTE_NAME } from "../app/defaults";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

const fallbackGitStatus: GitStatus = {
  repositoryPath: DEFAULT_ASSET_CENTER_PATH,
  isRepository: false,
  statusMessage: "静态预览：尚未读取本地 Git 仓库。",
  branch: "main",
  remoteName: DEFAULT_GIT_REMOTE_NAME,
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
  repositoryPath: DEFAULT_ASSET_CENTER_PATH,
  isRepository: false,
  statusMessage: "尚未读取本地 Git 仓库。",
  branch: "",
  remoteName: DEFAULT_GIT_REMOTE_NAME,
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
  const [settings, setSettings] = useState<DesktopSettings | null>(null);
  const [preview, setPreview] = useState<SyncPreview | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [stateLabel, setStateLabel] = useState("读取中");
  const [planningDirection, setPlanningDirection] = useState<SyncDirection | null>(null);
  const [isApplying, setIsApplying] = useState(false);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [pendingPushPolicy, setPendingPushPolicy] = useState<DesktopSettings | null>(null);
  const [pushPolicyPreview, setPushPolicyPreview] = useState<SettingsSavePreview | null>(null);
  const [pushPolicyResult, setPushPolicyResult] = useState<ApplyResult | null>(null);
  const [pushPolicyError, setPushPolicyError] = useState<string | null>(null);
  const [isPlanningPushPolicy, setIsPlanningPushPolicy] = useState(false);
  const [isApplyingPushPolicy, setIsApplyingPushPolicy] = useState(false);
  const [syncHistory, setSyncHistory] = useState<readonly AuditLogEntry[]>([]);

  useEffect(() => {
    let cancelled = false;
    if (demoMode) {
      setStatus(fallbackGitStatus);
      setSettings({
        assetCenterPath: DEFAULT_ASSET_CENTER_PATH,
        scanRoots: [],
        maxDepth: 5,
        backupBeforeApply: true,
        backupWarningThresholdBytes: 1024 * 1024 * 1024,
        planOnlyByDefault: true,
        gitDefaultBranch: "main",
        gitRemote: DEFAULT_GIT_REMOTE_NAME,
        allowPublicRemotePush: false,
        appearanceTheme: "system",
        density: "compact",
        logLevel: "info",
        logRetentionDays: 14,
        cliPath: "maa",
      });
      setSyncHistory([]);
      setStateLabel("Visual QA 示例数据");
      return undefined;
    }
    setStatus(emptyGitStatus);
    setSettings(null);
    setStateLabel("读取中");
    Promise.allSettled([gitStatus(), settingsLoad(), listAuditLog()])
      .then(([loadedStatus, loadedSettings, loadedAuditEntries]) => {
        if (cancelled) return;
        setSettings(loadedSettings.status === "fulfilled" ? loadedSettings.value : null);
        setSyncHistory(
          loadedAuditEntries.status === "fulfilled"
            ? syncAuditEntries(loadedAuditEntries.value)
            : [],
        );
        if (
          loadedStatus.status === "fulfilled" &&
          loadedStatus.value &&
          typeof loadedStatus.value === "object" &&
          "repositoryPath" in loadedStatus.value
        ) {
          setStatus(loadedStatus.value);
          setPreview(null);
          setApplyResult(null);
          setOperationError(null);
          setStateLabel(
            loadedAuditEntries.status === "rejected"
              ? "只读真实数据 · 同步历史暂不可用"
              : loadedSettings.status === "rejected"
                ? "只读真实数据 · 同步设置暂不可用"
                : "只读真实数据",
          );
        } else {
          setStatus(emptyGitStatus);
          setPreview(null);
          setApplyResult(null);
          setOperationError(null);
          setStateLabel(
            loadedStatus.status === "rejected"
              ? `读取失败：${errorMessage(loadedStatus.reason)}`
              : "未返回 Git 状态",
          );
        }
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

  const handlePublicRemotePushChange = async (allowPublicRemotePush: boolean) => {
    if (!settings) return;
    const candidate = { ...settings, allowPublicRemotePush };
    setPendingPushPolicy(candidate);
    setPushPolicyPreview(null);
    setPushPolicyResult(null);
    setIsPlanningPushPolicy(true);
    setPushPolicyError(null);
    try {
      const result = await settingsPreview({ settings: candidate });
      setPushPolicyPreview(result);
      setPreview(null);
      setApplyResult(null);
      setStateLabel(allowPublicRemotePush ? "公开远程策略预览" : "私有仓库保护策略预览");
    } catch (error) {
      setPendingPushPolicy(null);
      setPushPolicyError(errorMessage(error));
    } finally {
      setIsPlanningPushPolicy(false);
    }
  };

  const handleApplyPushPolicy = async () => {
    if (!pendingPushPolicy || !pushPolicyPreview?.canApply) return;
    setIsApplyingPushPolicy(true);
    setPushPolicyError(null);
    try {
      const result = await settingsApply({
        previewId: pushPolicyPreview.previewId,
        previewGeneratedAtEpochSeconds: pushPolicyPreview.generatedAtEpochSeconds,
        request: { settings: pendingPushPolicy },
      });
      const refreshed = await settingsLoad();
      setSettings(refreshed);
      setPendingPushPolicy(null);
      setPushPolicyPreview(null);
      setPushPolicyResult({
        mode: "apply",
        ok: true,
        previewId: result.previewId,
        backup: null,
        steps: [{
          stepId: "push-policy-save",
          kind: "settings",
          label: "保存 Push 安全策略",
          status: "success",
          message: "同步策略已写入并重新读取。",
          affectedPaths: result.affectedPaths,
        }],
        warnings: [],
        errors: [],
      });
      setStateLabel(refreshed.allowPublicRemotePush ? "已允许公开远程 Push" : "已恢复私有仓库保护");
    } catch (error) {
      setPushPolicyResult(null);
      setPushPolicyError(errorMessage(error));
      setStateLabel("同步策略保存失败");
    } finally {
      setIsApplyingPushPolicy(false);
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
      setStateLabel(result.outcomeUnknown ? "同步结果待确认" : "同步已执行");
      const [loadedStatus, loadedAuditEntries] = await Promise.allSettled([
        gitStatus(),
        listAuditLog(),
      ]);
      if (loadedStatus.status === "fulfilled") setStatus(loadedStatus.value);
      if (loadedAuditEntries.status === "fulfilled") {
        setSyncHistory(syncAuditEntries(loadedAuditEntries.value));
      } else {
        setStateLabel(
          result.outcomeUnknown
            ? "同步结果待确认 · 同步历史暂不可用"
            : "同步已执行 · 同步历史暂不可用",
        );
      }
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
        <div className="settings-toggle-list sync-policy-toggle"><label><input checked={pendingPushPolicy?.allowPublicRemotePush ?? settings?.allowPublicRemotePush ?? false} data-no-drag="true" disabled={!settings || isPlanningPushPolicy || isApplyingPushPolicy} onChange={(event) => void handlePublicRemotePushChange(event.target.checked)} style={NO_DRAG_REGION_STYLE} type="checkbox" /><span><strong>允许推送到公开远程仓库</strong><small>默认只允许已验证的 GitHub 私有仓库。开启后可推送到任意 Git remote，并会在执行前高亮公开或未知可见性风险。</small></span></label></div>
        {pushPolicyPreview || pushPolicyResult || pushPolicyError ? <ApplyConfirmationPanel actionLabel="确认保存策略" canApply={Boolean(pushPolicyPreview?.canApply)} description={pushPolicyPreview?.warnings[0] ?? pushPolicyPreview?.plannedEffects.join("；") ?? "策略保存前必须生成并确认有效预览。"} isApplying={isApplyingPushPolicy} onApply={handleApplyPushPolicy} operationError={pushPolicyError} result={pushPolicyResult} title="保存 Push 安全策略" /> : null}
        <div className="operation-actions"><button className="asset-secondary-action" data-no-drag="true" disabled={planningDirection !== null} onClick={() => handlePreviewSync("pull")} style={NO_DRAG_REGION_STYLE} type="button">{planningDirection === "pull" ? "生成中" : "预览 Pull"}</button><button className="asset-secondary-action" data-no-drag="true" disabled={planningDirection !== null} onClick={() => handlePreviewSync("push")} style={NO_DRAG_REGION_STYLE} type="button">{planningDirection === "push" ? "生成中" : "预览 Push"}</button></div>
        <ApplyConfirmationPanel actionLabel={preview?.direction === "pull" ? "执行 Pull" : "执行 Push"} canApply={canApply} description={preview?.allowPublicRemotePush ?? settings?.allowPublicRemotePush ? "后端会校验 previewId、远端身份与当前仓库状态；已允许公开远程 Push，执行前请确认资产、portable backup 与配置可能对远程仓库访问者可见。" : "后端会校验 previewId、远端身份与当前仓库状态；Push 仅允许已验证的 GitHub 私有仓库并只 stage canonical 白名单。"} isApplying={isApplying} onApply={handleApplySync} operationError={operationError} result={applyResult} title="执行同步" />
      </section>

      <div className="detail-two-column sync-lower-grid">
        <section className="panel detail-section"><div className="section-heading"><div><h3>同步历史</h3><p>最近的本地 Git 操作</p></div><span className="preview-label">{stateLabel}</span></div><div className="timeline-list">{syncHistory.length > 0 ? syncHistory.map((entry) => <div key={`${entry.occurredAtEpochSeconds}:${entry.operationType}`}><CheckCircle2 size={14} /><span>本地 Git 同步 · {entry.outcome === "completed" ? "已完成" : "需要检查"}</span><time>{formatAuditTime(entry.occurredAtEpochSeconds)}</time></div>) : status.lastSyncedAt ? <div><CheckCircle2 size={14} /><span>最近一次本地同步</span><time>{status.lastSyncedAt}</time></div> : <div className="asset-empty-state"><RefreshCw size={20} /><strong>暂无同步历史</strong><span>执行真实 Pull 或 Push 后会在本地 operation journal 留下记录。</span></div>}</div></section>
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
    ok: !result.outcomeUnknown,
    previewId: result.previewId,
    backup: null,
    steps: [{
      stepId: `git-${result.direction}`,
      kind: "git",
      label: `执行 ${action}`,
      status: result.outcomeUnknown ? "failed" : "success",
      message: result.pulled
        ? "已完成 fast-forward Pull。"
        : result.pushed
          ? "已完成 Push。"
          : result.outcomeUnknown
            ? "远程结果暂时无法确认；本地提交已保留。"
            : "同步完成。",
      affectedPaths: result.affectedPaths,
    }],
    warnings: result.warnings,
    errors: result.outcomeUnknown
      ? ["远程结果暂时无法确认；本地提交已保留。请先刷新同步状态，确认远程分支后再决定是否重试。"]
      : [],
  };
}

function errorMessage(error: unknown) {
  return safeCommandErrorMessage(error, "同步操作未完成。请查看系统状态或导出诊断包后重试。");
}

function syncAuditEntries(entries: readonly AuditLogEntry[]) {
  return entries
    .filter((entry) => entry.operationType === "git-sync")
    .sort((left, right) => right.occurredAtEpochSeconds - left.occurredAtEpochSeconds)
    .slice(0, 5);
}

function formatAuditTime(epochSeconds: number) {
  return new Date(epochSeconds * 1000).toLocaleString("zh-CN");
}
