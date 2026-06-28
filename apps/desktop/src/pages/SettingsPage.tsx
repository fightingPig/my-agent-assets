import { FileText, FolderCog, Palette, RefreshCw, ScanSearch, ShieldCheck, TerminalSquare } from "lucide-react";
import { useEffect, useState } from "react";
import { settingsLoad, settingsSave } from "../app/data-api";
import type { DesktopSettings } from "../app/contracts";
import { StaticActionButton } from "../components/ui/StaticActionButton";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

const noDragControl = { ...NO_DRAG_REGION_STYLE };

const fallbackSettings: DesktopSettings = {
  assetCenterPath: "~/.my-agent-assets",
  scanRoots: ["~/workspace", "~/code"],
  maxDepth: 5,
  backupBeforeApply: true,
  planOnlyByDefault: true,
  gitDefaultBranch: "main",
  gitRemote: "origin",
  appearanceTheme: "system",
  density: "compact",
  logLevel: "info",
  logRetentionDays: 14,
  cliPath: "maa",
};

export function SettingsPage({ demoMode = false }: { demoMode?: boolean }) {
  const [settings, setSettings] = useState<DesktopSettings | null>(demoMode ? fallbackSettings : null);
  const [stateLabel, setStateLabel] = useState("读取中");
  const [isSaving, setIsSaving] = useState(false);
  const [saveMessage, setSaveMessage] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    if (demoMode) {
      setSettings(fallbackSettings);
      setStateLabel("Visual QA 示例数据");
      return undefined;
    }
    setSettings(null);
    setStateLabel("读取中");
    settingsLoad()
      .then((loaded) => {
        if (cancelled) return;
        if (loaded && typeof loaded === "object" && "assetCenterPath" in loaded) {
          setSettings(loaded);
          setStateLabel("只读真实数据");
        } else {
          setSettings(null);
          setStateLabel("未返回设置数据");
        }
      })
      .catch((error) => {
        if (cancelled) return;
        setSettings(null);
        setStateLabel(`读取失败：${errorMessage(error)}`);
      });
    return () => {
      cancelled = true;
    };
  }, [demoMode]);

  const updateSetting = <Key extends keyof DesktopSettings>(key: Key, value: DesktopSettings[Key]) => {
    setSettings((current) => current ? { ...current, [key]: value } : current);
  };

  const handleScanRootsChange = (value: string) => {
    updateSetting("scanRoots", value.split(",").map((item) => item.trim()).filter(Boolean));
  };

  const handleSave = async () => {
    if (!settings) return;
    setIsSaving(true);
    setStateLabel("保存中");
    setSaveMessage(null);
    try {
      await settingsSave({ settings });
      const refreshed = await settingsLoad();
      setSettings(refreshed);
      setStateLabel("已保存并重新读取");
      setSaveMessage("设置已写入本地配置，并已从后端重新读取确认。");
    } catch (error) {
      setStateLabel("保存失败");
      setSaveMessage(`保存失败：${errorMessage(error)} 请检查资产中心路径权限后重试。`);
    } finally {
      setIsSaving(false);
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
      <section className="panel settings-section"><div className="settings-section-title"><FolderCog size={17} /><div><h3>路径设置</h3><p>本地资产中心与项目扫描位置 · {stateLabel}</p></div></div><div className="settings-controls"><label><span>资产中心（V1 固定路径）</span><input data-no-drag="true" readOnly style={noDragControl} value={settings.assetCenterPath} /><small>当前后端固定使用 ~/.my-agent-assets，迁移能力将在后续版本提供。</small></label><label><span>扫描根目录</span><input data-no-drag="true" onChange={(event) => handleScanRootsChange(event.target.value)} style={noDragControl} value={settings.scanRoots.join(", ")} /></label></div></section>
      <section className="panel settings-section"><div className="settings-section-title"><ScanSearch size={17} /><div><h3>扫描设置</h3><p>发现本地 Claude 资产的默认参数</p></div></div><div className="settings-controls two"><label><span>最大深度</span><input data-no-drag="true" min={1} max={20} onChange={(event) => updateSetting("maxDepth", Number(event.target.value))} style={noDragControl} type="number" value={settings.maxDepth} /></label><label><span>默认范围</span><select data-no-drag="true" disabled style={noDragControl} value="user"><option value="user">用户级</option></select></label></div></section>
      <section className="panel settings-section"><div className="settings-section-title"><ShieldCheck size={17} /><div><h3>安全设置</h3><p>计划确认与本地备份策略</p></div></div><div className="settings-toggle-list"><label><input checked={settings.backupBeforeApply} data-no-drag="true" onChange={(event) => updateSetting("backupBeforeApply", event.target.checked)} style={noDragControl} type="checkbox" /><span><strong>变更前创建备份</strong><small>所有 apply 操作前生成 manifest</small></span></label><label><input checked={settings.planOnlyByDefault} data-no-drag="true" onChange={(event) => updateSetting("planOnlyByDefault", event.target.checked)} style={noDragControl} type="checkbox" /><span><strong>默认仅生成计划</strong><small>必须显式确认后才执行</small></span></label></div></section>
      <section className="panel settings-section"><div className="settings-section-title"><RefreshCw size={17} /><div><h3>同步设置</h3><p>本地 Git 仓库同步偏好</p></div></div><div className="settings-controls two"><label><span>默认分支</span><input data-no-drag="true" onChange={(event) => updateSetting("gitDefaultBranch", event.target.value)} style={noDragControl} value={settings.gitDefaultBranch} /></label><label><span>远程仓库</span><input data-no-drag="true" onChange={(event) => updateSetting("gitRemote", event.target.value)} style={noDragControl} value={settings.gitRemote} /></label></div></section>
      <section className="panel settings-section"><div className="settings-section-title"><Palette size={17} /><div><h3>外观设置</h3><p>桌面界面显示偏好</p></div></div><div className="settings-controls two"><label><span>主题</span><select data-no-drag="true" onChange={(event) => updateSetting("appearanceTheme", event.target.value as DesktopSettings["appearanceTheme"])} style={noDragControl} value={settings.appearanceTheme}><option value="system">跟随系统</option><option value="light">浅色</option><option value="dark">深色</option></select></label><label><span>界面密度</span><select data-no-drag="true" onChange={(event) => updateSetting("density", event.target.value as DesktopSettings["density"])} style={noDragControl} value={settings.density}><option value="compact">紧凑</option><option value="comfortable">舒适</option></select></label></div></section>
      <section className="panel settings-section"><div className="settings-section-title"><FileText size={17} /><div><h3>日志设置</h3><p>本地诊断日志与保留周期</p></div></div><div className="settings-controls two"><label><span>日志级别</span><select data-no-drag="true" onChange={(event) => updateSetting("logLevel", event.target.value as DesktopSettings["logLevel"])} style={noDragControl} value={settings.logLevel}><option value="error">Error</option><option value="warn">Warn</option><option value="info">Info</option><option value="debug">Debug</option></select></label><label><span>保留周期</span><input data-no-drag="true" min={1} max={365} onChange={(event) => updateSetting("logRetentionDays", Number(event.target.value))} style={noDragControl} type="number" value={settings.logRetentionDays} /></label></div></section>
      <section className="panel settings-section"><div className="settings-section-title"><TerminalSquare size={17} /><div><h3>CLI 设置</h3><p>maa 命令行工具信息</p></div></div><div className="settings-controls two"><label><span>可执行文件</span><input data-no-drag="true" onChange={(event) => updateSetting("cliPath", event.target.value)} style={noDragControl} value={settings.cliPath} /></label><label><span>版本</span><input data-no-drag="true" readOnly style={noDragControl} value="0.1.0" /></label></div>{saveMessage ? <p className={stateLabel === "保存失败" ? "warning-text" : "success-text"} role="status">{saveMessage}</p> : null}<div className="settings-actions"><StaticActionButton className="asset-secondary-action">检查 CLI</StaticActionButton><button className="asset-business-action" data-no-drag="true" disabled={isSaving} onClick={handleSave} style={noDragControl} type="button">{isSaving ? "保存中" : "保存设置"}</button></div></section>
    </div>
  );
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "无法调用设置保存操作。";
}
