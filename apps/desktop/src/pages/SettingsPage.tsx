import { FolderCog, ScanSearch, ShieldCheck, RefreshCw, Palette, FileText, TerminalSquare } from "lucide-react";

const sections = [
  { title: "路径设置", detail: "资产中心与项目扫描路径", icon: FolderCog },
  { title: "扫描设置", detail: "扫描根目录与最大深度", icon: ScanSearch },
  { title: "安全设置", detail: "计划确认与备份策略", icon: ShieldCheck },
  { title: "同步设置", detail: "本地 Git 仓库同步偏好", icon: RefreshCw },
  { title: "外观设置", detail: "主题与界面显示偏好", icon: Palette },
  { title: "日志设置", detail: "本地日志级别与保留周期", icon: FileText },
  { title: "CLI 设置", detail: "maa 命令行工具路径", icon: TerminalSquare },
];

export function SettingsPage() {
  return (
    <section className="panel skeleton-panel">
      <div className="panel-header"><div><h2>应用设置</h2><p>所有设置均保存在本机</p></div><span className="healthy-badge">静态预览</span></div>
      <div className="settings-grid">
        {sections.map(({ title, detail, icon: Icon }) => <div className="settings-item" key={title}><span className="skeleton-icon"><Icon size={17} /></span><span className="skeleton-copy"><strong>{title}</strong><span>{detail}</span></span></div>)}
      </div>
    </section>
  );
}
