import { AlertTriangle } from "lucide-react";

export function ConflictResolverPage() {
  return (
    <div className="conflict-layout">
      <section className="panel skeleton-panel">
        <div className="panel-header"><div><h2>待处理冲突</h2><p>2 项静态预览</p></div><span className="status-badge warning">需要确认</span></div>
        <div className="skeleton-list compact">
          <div className="skeleton-row selected"><div className="skeleton-icon warning"><AlertTriangle size={17} /></div><div className="skeleton-copy"><strong>github</strong><span>MCP · 内容不同</span></div></div>
          <div className="skeleton-row"><div className="skeleton-icon warning"><AlertTriangle size={17} /></div><div className="skeleton-copy"><strong>review</strong><span>Skill · 名称重复</span></div></div>
        </div>
      </section>
      <section className="panel diff-panel">
        <div className="panel-header"><div><h2>差异预览</h2><p>资产中心与扫描结果</p></div></div>
        <div className="diff-placeholder"><div><strong>资产中心</strong><code>{`{ "command": "existing" }`}</code></div><div><strong>扫描结果</strong><code>{`{ "command": "incoming" }`}</code></div></div>
        <div className="skeleton-actions"><button className="text-button" type="button">跳过</button><button className="text-button" type="button">重命名</button><button className="primary-button" type="button">覆盖</button></div>
      </section>
    </div>
  );
}
