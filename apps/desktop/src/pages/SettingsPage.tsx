import {
  Activity,
  FileText,
  FolderCog,
  Palette,
  RefreshCw,
  ScanSearch,
  ShieldCheck,
  TerminalSquare,
} from "lucide-react";
import { useEffect, useState } from "react";
import {
  diagnosticExportApply,
  diagnosticExportPreview,
  gitRemoteApply,
  gitRemotePreview,
  initializationPreview,
  safeCommandErrorMessage,
  settingsApply,
  settingsLoad,
  settingsPreview,
} from "../app/data-api";
import type {
  AppInfo,
  ApplyResult,
  DesktopSettings,
  DiagnosticExportPreview,
  GitRemotePreview,
  InitializationPreview,
  SettingsPreview,
} from "../app/contracts";
import { TargetRegistryPanel } from "../components/targets/TargetRegistryPanel";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

const noDragControl = { ...NO_DRAG_REGION_STYLE };

const fallbackSettings: DesktopSettings = {
  assetCenterPath: "~/.my-agent-assets",
  scanRoots: ["~/workspace", "~/code"],
  maxDepth: 5,
  backupBeforeApply: true,
  backupWarningThresholdBytes: 1024 * 1024 * 1024,
  planOnlyByDefault: true,
  gitDefaultBranch: "main",
  gitRemote: "origin",
  allowPublicRemotePush: false,
  appearanceTheme: "system",
  density: "compact",
  logLevel: "info",
  logRetentionDays: 14,
  cliPath: "maa",
};

const demoInitialization: InitializationPreview = {
  previewId: "visual-qa-initialized",
  assetCenterPath: "~/.my-agent-assets",
  plannedPaths: [],
  warnings: [],
  alreadyInitialized: true,
  canApply: true,
  generatedAtEpochSeconds: 0,
  expiresAtEpochSeconds: 0,
};

const uninitializedVisualQaPreview: InitializationPreview = {
  previewId: "visual-qa-uninitialized",
  assetCenterPath: "~/.my-agent-assets",
  plannedPaths: [],
  warnings: ["资产中心尚未初始化。请先前往首页完成初始化。"],
  alreadyInitialized: false,
  canApply: false,
  generatedAtEpochSeconds: 0,
  expiresAtEpochSeconds: 0,
};

type SettingsPageProps = {
  appInfo?: AppInfo;
  demoMode?: boolean;
  visualQaState?: string;
};

export function SettingsPage({ appInfo, demoMode = false, visualQaState }: SettingsPageProps) {
  const visualQaUninitialized = demoMode && visualQaState === "uninitialized";
  const [settings, setSettings] = useState<DesktopSettings | null>(
    demoMode ? fallbackSettings : null,
  );
  const [initialization, setInitialization] = useState<InitializationPreview | null>(
    demoMode ? (visualQaUninitialized ? uninitializedVisualQaPreview : demoInitialization) : null,
  );
  const [initializationChecked, setInitializationChecked] = useState(demoMode);
  const [initializationMessage, setInitializationMessage] = useState("");
  const [stateLabel, setStateLabel] = useState("读取中");
  const [preview, setPreview] = useState<SettingsPreview | null>(null);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [isPlanning, setIsPlanning] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [saveMessage, setSaveMessage] = useState<string | null>(null);
  const [diagnosticExport, setDiagnosticExport] = useState<DiagnosticExportPreview | null>(null);
  const [diagnosticMessage, setDiagnosticMessage] = useState("");
  const [diagnosticBusy, setDiagnosticBusy] = useState(false);
  const [remoteUrl, setRemoteUrl] = useState("");
  const [remotePreview, setRemotePreview] = useState<GitRemotePreview | null>(null);
  const [remoteMessage, setRemoteMessage] = useState("");
  const [remoteBusy, setRemoteBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    if (demoMode) {
      setSettings(fallbackSettings);
      setInitialization(visualQaUninitialized ? uninitializedVisualQaPreview : demoInitialization);
      setInitializationChecked(true);
      setInitializationMessage(
        visualQaUninitialized ? "资产中心尚未初始化。请先前往首页完成初始化。" : "",
      );
      setStateLabel("Visual QA 示例数据");
      return undefined;
    }

    setSettings(null);
    setInitialization(null);
    setInitializationChecked(false);
    setInitializationMessage("");
    setStateLabel("读取中");
    Promise.allSettled([settingsLoad(), initializationPreview()]).then(
      ([loadedSettings, loadedInitialization]) => {
        if (cancelled) return;

        if (
          loadedSettings.status === "fulfilled" &&
          loadedSettings.value &&
          typeof loadedSettings.value === "object" &&
          "assetCenterPath" in loadedSettings.value
        ) {
          setSettings(loadedSettings.value);
        } else {
          setSettings(null);
        }

        if (loadedInitialization.status === "fulfilled") {
          setInitialization(loadedInitialization.value);
          if (!loadedInitialization.value.alreadyInitialized) {
            setInitializationMessage(
              loadedInitialization.value.warnings[0] ??
                "资产中心尚未初始化。请先前往首页完成初始化。",
            );
          }
        } else {
          setInitialization(null);
          setInitializationMessage(
            safeCommandErrorMessage(
              loadedInitialization.reason,
              "无法确认资产中心初始化状态；所有写入操作已禁用。",
            ),
          );
        }
        setInitializationChecked(true);

        if (loadedSettings.status === "rejected") {
          setStateLabel(
            `读取失败：${errorMessage(loadedSettings.reason)}`,
          );
        } else if (loadedInitialization.status === "rejected") {
          setStateLabel("设置已读取，初始化状态无法确认");
        } else {
          setStateLabel("只读真实数据");
        }
      },
    );
    return () => {
      cancelled = true;
    };
  }, [demoMode, visualQaUninitialized]);

  const assetCenterReady =
    initializationChecked && initialization?.alreadyInitialized === true;
  const writesDisabled = !assetCenterReady;

  const updateSetting = <Key extends keyof DesktopSettings>(
    key: Key,
    value: DesktopSettings[Key],
  ) => {
    if (writesDisabled) return;
    setSettings((current) => (current ? { ...current, [key]: value } : current));
    setPreview(null);
    setApplyResult(null);
    setSaveMessage(null);
  };

  const handlePreviewSave = async () => {
    if (!settings || writesDisabled) return;
    setIsPlanning(true);
    setStateLabel("生成保存预览中");
    setSaveMessage(null);
    setApplyResult(null);
    try {
      const result = await settingsPreview({ settings });
      setPreview(result);
      setStateLabel(result.canApply ? "保存预览已生成" : "保存预览被阻断");
    } catch (error) {
      setPreview(null);
      setStateLabel("预览失败");
      setSaveMessage(`预览失败：${errorMessage(error)}`);
    } finally {
      setIsPlanning(false);
    }
  };

  const handleApplySave = async () => {
    if (!settings || writesDisabled || !preview?.canApply) return;
    setIsApplying(true);
    setStateLabel("保存中");
    setSaveMessage(null);
    try {
      const applied = await settingsApply({
        previewId: preview.previewId,
        previewGeneratedAtEpochSeconds: preview.generatedAtEpochSeconds,
        request: { settings },
      });
      const refreshed = await settingsLoad();
      setSettings(refreshed);
      setStateLabel("已保存并重新读取");
      setSaveMessage("设置已写入本地配置，并已从后端重新读取确认。");
      setApplyResult({
        mode: "apply",
        ok: true,
        previewId: applied.previewId,
        backup: null,
        steps: [
          {
            stepId: "settings-save",
            kind: "settings",
            label: "保存本地设置",
            status: "success",
            message: "设置文件已原子写入并重新读取。",
            affectedPaths: applied.affectedPaths,
          },
        ],
        warnings: [],
        errors: [],
      });
      setPreview(null);
    } catch (error) {
      setStateLabel("保存失败");
      setSaveMessage(`保存失败：${errorMessage(error)}`);
      setApplyResult(null);
    } finally {
      setIsApplying(false);
    }
  };

  const handleDiagnosticPreview = async () => {
    if (writesDisabled) return;
    setDiagnosticBusy(true);
    setDiagnosticMessage("");
    try {
      setDiagnosticExport(await diagnosticExportPreview());
    } catch (error) {
      setDiagnosticMessage(`诊断包预览失败：${errorMessage(error)}`);
    } finally {
      setDiagnosticBusy(false);
    }
  };

  const handleDiagnosticApply = async () => {
    if (writesDisabled || !diagnosticExport?.canApply) return;
    setDiagnosticBusy(true);
    try {
      const result = await diagnosticExportApply({
        previewId: diagnosticExport.previewId,
        previewGeneratedAtEpochSeconds: diagnosticExport.generatedAtEpochSeconds,
      });
      setDiagnosticMessage(`已导出脱敏诊断包：${result.packagePath}`);
      setDiagnosticExport(null);
    } catch (error) {
      setDiagnosticMessage(`诊断包导出失败：${errorMessage(error)}`);
    } finally {
      setDiagnosticBusy(false);
    }
  };

  const handleRemotePreview = async () => {
    if (!settings || writesDisabled || !remoteUrl.trim()) return;
    setRemoteBusy(true);
    setRemoteMessage("");
    try {
      const result = await gitRemotePreview({
        remoteName: settings.gitRemote,
        remoteUrl: remoteUrl.trim(),
      });
      setRemotePreview(result);
      if (!result.canApply) setRemoteMessage(result.warnings.join("；"));
    } catch (error) {
      setRemotePreview(null);
      setRemoteMessage(`远程配置预览失败：${errorMessage(error)}`);
    } finally {
      setRemoteBusy(false);
    }
  };

  const handleRemoteApply = async () => {
    if (writesDisabled || !remotePreview?.canApply || !settings) return;
    setRemoteBusy(true);
    try {
      const result = await gitRemoteApply({
        previewId: remotePreview.previewId,
        previewGeneratedAtEpochSeconds: remotePreview.generatedAtEpochSeconds,
        request: {
          remoteName: settings.gitRemote,
          remoteUrl: remotePreview.remoteUrl,
        },
      });
      setRemoteMessage(`已配置 ${result.remoteName}，原 Git 配置备份于 ${result.backupPath}`);
      setRemotePreview(null);
    } catch (error) {
      setRemoteMessage(`远程配置失败：${errorMessage(error)}`);
    } finally {
      setRemoteBusy(false);
    }
  };

  if (!settings) {
    return (
      <section className="panel settings-section">
        <div className="settings-section-title">
          <FolderCog size={17} />
          <div><h3>设置读取状态</h3><p>{stateLabel}</p></div>
        </div>
        <div className="asset-empty-state">
          <ShieldCheck size={22} />
          <strong>暂无可显示设置</strong>
          <span>请检查本地后端连接和配置读取权限。</span>
        </div>
      </section>
    );
  }

  return (
    <div className="settings-workspace">
      {!assetCenterReady ? (
        <p className="settings-initialization-warning warning-text" role="status">
          {initializationMessage || "资产中心尚未初始化。请先前往首页完成初始化，当前设置保持只读。"}
        </p>
      ) : null}

      <section className="panel settings-section">
        <div className="settings-section-title">
          <FolderCog size={17} />
          <div><h3>路径设置</h3><p>本地资产中心与已授权自定义 Target · {stateLabel}</p></div>
        </div>
        <div className="settings-controls">
          <label>
            <span>资产中心（V1 固定路径）</span>
            <input data-no-drag="true" readOnly style={noDragControl} value={settings.assetCenterPath} />
            <small>当前后端固定使用 ~/.my-agent-assets，迁移能力将在后续版本提供。</small>
          </label>
        </div>
        <p className="settings-path-guidance">已维护项目仅在“项目列表”中添加、编辑或移除；这里不再自动扫描 workspace/code 目录。</p>
        <TargetRegistryPanel disabled={writesDisabled} />
      </section>

      <section className="panel settings-section">
        <div className="settings-section-title">
          <ScanSearch size={17} />
          <div><h3>扫描设置</h3><p>发现 Claude Code、Codex 与已授权 Custom 来源的默认参数</p></div>
        </div>
        <div className="settings-controls two">
          <label>
            <span>最大深度</span>
            <input data-no-drag="true" disabled={writesDisabled} min={1} max={20} onChange={(event) => updateSetting("maxDepth", Number(event.target.value))} style={noDragControl} type="number" value={settings.maxDepth} />
          </label>
          <label>
            <span>默认范围</span>
            <select data-no-drag="true" disabled style={noDragControl} value="user"><option value="user">用户级</option></select>
          </label>
        </div>
      </section>

      <section className="panel settings-section">
        <div className="settings-section-title">
          <ShieldCheck size={17} />
          <div><h3>安全设置</h3><p>计划确认与本地备份策略</p></div>
        </div>
        <div className="settings-toggle-list">
          <label>
            <input checked data-no-drag="true" disabled readOnly style={noDragControl} type="checkbox" />
            <span><strong>变更前创建备份</strong><small>安全策略固定开启；所有 apply 操作前生成 manifest</small></span>
          </label>
          <label>
            <input checked data-no-drag="true" disabled readOnly style={noDragControl} type="checkbox" />
            <span><strong>默认仅生成计划</strong><small>安全策略固定开启；必须显式确认后才执行</small></span>
          </label>
        </div>
        <div className="settings-controls two">
          <label>
            <span>备份容量提醒阈值（GiB）</span>
            <input data-no-drag="true" disabled={writesDisabled} min={0.001} onChange={(event) => updateSetting("backupWarningThresholdBytes", Math.round(Number(event.target.value) * 1024 * 1024 * 1024))} step={0.1} style={noDragControl} type="number" value={settings.backupWarningThresholdBytes / 1024 / 1024 / 1024} />
            <small>超过该总容量时提醒清理；不会自动删除任何备份。</small>
          </label>
        </div>
      </section>

      <section className="panel settings-section">
        <div className="settings-section-title">
          <RefreshCw size={17} />
          <div><h3>同步设置</h3><p>本地 Git 仓库同步偏好</p></div>
        </div>
        <div className="settings-controls two">
          <label>
            <span>默认分支</span>
            <input data-no-drag="true" readOnly style={noDragControl} value={settings.gitDefaultBranch} />
            <small>初始化分支当前固定为 main；自定义分支将在后续版本提供。</small>
          </label>
          <label>
            <span>远程名称</span>
            <input data-no-drag="true" disabled={writesDisabled} onChange={(event) => updateSetting("gitRemote", event.target.value)} style={noDragControl} value={settings.gitRemote} />
          </label>
          <label className="settings-control-wide">
            <span>远程仓库 URL</span>
            <div className="path-picker-control">
              <input data-no-drag="true" disabled={writesDisabled} onChange={(event) => { setRemoteUrl(event.target.value); setRemotePreview(null); setRemoteMessage(""); }} placeholder="git@github.com:owner/private-assets.git" style={noDragControl} value={remoteUrl} />
              <button className="asset-secondary-action" data-no-drag="true" disabled={writesDisabled || remoteBusy || !remoteUrl.trim()} onClick={() => void handleRemotePreview()} style={noDragControl} type="button">预览配置</button>
            </div>
            <small>仅在确认后修改资产中心的 Git remote；不会执行 fetch、pull 或 push。</small>
          </label>
        </div>
        {remotePreview ? (
          <ApplyConfirmationPanel
            actionLabel="确认配置远程仓库"
            canApply={!writesDisabled && remotePreview.canApply}
            description={`${remotePreview.previousUrl ?? "尚未配置"} → ${remotePreview.remoteUrl}`}
            isApplying={remoteBusy}
            onApply={() => void handleRemoteApply()}
            operationError={remotePreview.canApply ? null : remotePreview.warnings.join("；")}
            result={null}
            title="配置 Git remote"
          />
        ) : null}
        {remoteMessage ? <p className={remoteMessage.includes("失败") ? "warning-text" : "success-text"} role="status">{remoteMessage}</p> : null}
        <div className="settings-toggle-list">
          <label>
            <input checked={settings.allowPublicRemotePush} data-no-drag="true" disabled={writesDisabled} onChange={(event) => updateSetting("allowPublicRemotePush", event.target.checked)} style={noDragControl} type="checkbox" />
            <span><strong>允许推送到公开远程仓库</strong><small>默认只允许已验证的 GitHub 私有仓库。开启后可推送到任意 Git remote，执行时会高亮公开或未知可见性风险。</small></span>
          </label>
        </div>
      </section>

      <section className="panel settings-section">
        <div className="settings-section-title">
          <Palette size={17} />
          <div><h3>外观设置</h3><p>桌面界面显示偏好 · 后续版本</p></div>
        </div>
        <div className="settings-controls two">
          <label><span>主题</span><select data-no-drag="true" disabled style={noDragControl} value={settings.appearanceTheme}><option value="system">跟随系统</option><option value="light">浅色</option><option value="dark">深色</option></select></label>
          <label><span>界面密度</span><select data-no-drag="true" disabled style={noDragControl} value={settings.density}><option value="compact">紧凑</option><option value="comfortable">舒适</option></select></label>
        </div>
      </section>

      <section className="panel settings-section">
        <div className="settings-section-title">
          <FileText size={17} />
          <div><h3>日志设置</h3><p>本地诊断日志、保留周期与脱敏导出</p></div>
        </div>
        <div className="settings-controls two">
          <label><span>日志级别</span><select data-no-drag="true" disabled style={noDragControl} value={settings.logLevel}><option value="error">Error</option><option value="warn">Warn</option><option value="info">Info</option><option value="debug">Debug</option></select><small>日志级别切换将在后续版本提供。</small></label>
          <label><span>保留周期</span><input data-no-drag="true" disabled={writesDisabled} min={1} max={365} onChange={(event) => updateSetting("logRetentionDays", Number(event.target.value))} style={noDragControl} type="number" value={settings.logRetentionDays} /></label>
        </div>
        <div className="diagnostic-export-settings">
          <Activity size={17} />
          <div><strong>导出脱敏诊断包</strong><span>只包含版本、平台、状态摘要和脱敏审计日志。</span></div>
          <button className="asset-secondary-action" data-no-drag="true" disabled={writesDisabled || diagnosticBusy} onClick={() => void handleDiagnosticPreview()} style={noDragControl} type="button">预览导出</button>
        </div>
        {diagnosticExport ? (
          <div className="initialization-preview">
            <p>{diagnosticExport.includedFiles.map((file) => file.logicalPath).join("、") || "状态摘要"}</p>
            <button className="asset-business-action" data-no-drag="true" disabled={writesDisabled || diagnosticBusy || !diagnosticExport.canApply} onClick={() => void handleDiagnosticApply()} style={noDragControl} type="button">确认导出</button>
          </div>
        ) : null}
        {diagnosticMessage ? <p className="settings-path-guidance" role="status">{diagnosticMessage}</p> : null}
      </section>

      <section className="panel settings-section">
        <div className="settings-section-title">
          <TerminalSquare size={17} />
          <div><h3>CLI 设置</h3><p>maa 命令行工具信息</p></div>
        </div>
        <div className="settings-controls two">
          <label><span>可执行文件</span><input data-no-drag="true" readOnly style={noDragControl} value={settings.cliPath} /><small>自定义 CLI 路径将在后续版本提供。</small></label>
          <label><span>版本</span><input data-no-drag="true" readOnly style={noDragControl} value={appInfo?.version ?? "preview"} /></label>
        </div>
        {saveMessage && stateLabel !== "保存失败" ? <p className={stateLabel === "预览失败" ? "warning-text" : "success-text"} role="status">{saveMessage}</p> : null}
        <div className="settings-actions">
          <button className="asset-business-action" data-no-drag="true" disabled={writesDisabled || isPlanning || isApplying} onClick={handlePreviewSave} style={noDragControl} type="button">{isPlanning ? "生成中" : "生成保存预览"}</button>
        </div>
        {preview || applyResult ? (
          <ApplyConfirmationPanel
            actionLabel="确认保存设置"
            canApply={!writesDisabled && Boolean(preview?.canApply)}
            description={preview?.warnings[0] ?? preview?.plannedEffects.join("；") ?? "保存前必须生成并确认有效预览。"}
            isApplying={isApplying}
            onApply={handleApplySave}
            operationError={stateLabel === "保存失败" ? saveMessage : null}
            result={applyResult}
            title="保存本地设置"
          />
        ) : null}
      </section>
    </div>
  );
}

function errorMessage(error: unknown) {
  return safeCommandErrorMessage(
    error,
    "设置操作未完成。请查看系统状态或导出诊断包后重试。",
  );
}
