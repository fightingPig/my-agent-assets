import { ArchiveRestore } from "lucide-react";

const backups = [
  { id: "backup-20260621-1842", detail: "扫描导入前 · 6 项路径" },
  { id: "backup-20260620-0915", detail: "挂载变更前 · 3 项路径" },
  { id: "backup-20260618-1630", detail: "资产移除前 · 1 项路径" },
];

export function BackupRestorePage() {
  return (
    <div className="backup-layout">
      <section className="panel skeleton-panel">
        <div className="panel-header"><div><h2>备份记录</h2><p>当前展示静态预览数据</p></div></div>
        <div className="skeleton-list compact">
          {backups.map((backup) => <div className="skeleton-row" key={backup.id}><div className="skeleton-icon"><ArchiveRestore size={17} /></div><div className="skeleton-copy"><strong>{backup.id}</strong><span>{backup.detail}</span></div></div>)}
        </div>
      </section>
      <section className="panel preview-box"><strong>恢复影响预览</strong><p>选择备份后，这里将显示待恢复路径和覆盖范围。</p><button className="primary-button" type="button">预览恢复</button></section>
    </div>
  );
}
