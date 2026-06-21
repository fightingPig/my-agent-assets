import { GitBranch, RefreshCw } from "lucide-react";

export function SyncPage() {
  return (
    <div className="page-stack">
      <section className="panel skeleton-panel">
        <div className="panel-header"><div><h2>本地 Git 仓库</h2><p>资产中心版本状态</p></div><span className="healthy-badge">工作区干净</span></div>
        <div className="detail-grid">
          <div className="detail-item"><GitBranch size={17} /><div><strong>当前分支</strong><span>main</span></div></div>
          <div className="detail-item"><RefreshCw size={17} /><div><strong>远程仓库</strong><span>origin · 已配置</span></div></div>
        </div>
      </section>
      <section className="panel preview-box sync-preview"><div><strong>同步操作</strong><p>操作使用本机已有的 Git 配置。</p></div><div className="skeleton-actions"><button className="text-button" type="button">Pull</button><button className="primary-button" type="button">Push</button></div></section>
    </div>
  );
}
