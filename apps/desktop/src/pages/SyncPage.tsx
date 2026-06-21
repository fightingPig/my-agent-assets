import { AlertTriangle, ArrowDown, ArrowUp, CheckCircle2, GitBranch, RefreshCw } from "lucide-react";
import { StaticActionButton } from "../components/ui/StaticActionButton";

export function SyncPage() {
  return (
    <div className="operation-workspace sync-workspace">
      <section className="panel sync-repository-card">
        <div className="section-heading"><div><h3>本地 Git 仓库</h3><p>~/.my-agent-assets</p></div><span className="healthy-badge"><CheckCircle2 size={13} />工作区干净</span></div>
        <div className="sync-status-grid"><div><GitBranch size={17} /><span><small>当前分支</small><strong>main</strong></span></div><div><RefreshCw size={17} /><span><small>远程仓库</small><strong>origin</strong></span></div><div><ArrowUp size={17} /><span><small>Ahead</small><strong>2 commits</strong></span></div><div><ArrowDown size={17} /><span><small>Behind</small><strong>1 commit</strong></span></div></div>
        <div className="sync-graph"><div className="sync-graph-line"><span className="local-dot" /><strong>本地 main</strong><small>新增 api-design 与 format-code</small></div><div className="sync-graph-line"><span /><strong>共同基线</strong><small>更新 Asset Center 页面</small></div><div className="sync-graph-line"><span className="remote-dot" /><strong>远程 origin/main</strong><small>调整扫描设置</small></div></div>
        <div className="operation-actions"><StaticActionButton className="asset-secondary-action">Pull</StaticActionButton><StaticActionButton className="asset-business-action">Push</StaticActionButton></div>
      </section>

      <div className="detail-two-column sync-lower-grid">
        <section className="panel detail-section"><div className="section-heading"><div><h3>同步历史</h3><p>最近的本地 Git 操作</p></div></div><div className="timeline-list"><div><CheckCircle2 size={14} /><span>Push 至 origin/main</span><time>今天 09:42</time></div><div><CheckCircle2 size={14} /><span>Pull origin/main</span><time>昨天 18:20</time></div><div><CheckCircle2 size={14} /><span>提交资产索引更新</span><time>昨天 18:18</time></div></div></section>
        <section className="panel detail-section"><div className="section-heading"><div><h3>同步检查</h3><p>执行前风险预览</p></div></div><div className="operation-warning"><AlertTriangle size={17} /><div><strong>远程仓库包含 1 个新提交</strong><span>Pull 后可能需要处理 assets.yaml 的内容冲突。</span></div></div><div className="environment-list"><div><strong>远程连接</strong><span>可用</span></div><div><strong>未提交变更</strong><span>0 项</span></div><div><strong>潜在冲突</strong><span>1 项预览</span></div></div></section>
      </div>
    </div>
  );
}
