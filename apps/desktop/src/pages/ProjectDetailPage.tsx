import { FolderKanban, Link2 } from "lucide-react";

export function ProjectDetailPage() {
  return (
    <div className="page-stack">
      <section className="panel skeleton-panel">
        <div className="panel-header"><div><h2>project-a</h2><p>项目静态详情预览</p></div><span className="healthy-badge">环境正常</span></div>
        <div className="detail-grid">
          <div className="detail-item"><FolderKanban size={17} /><div><strong>项目路径</strong><span>~/workspace/project-a</span></div></div>
          <div className="detail-item"><Link2 size={17} /><div><strong>挂载概览</strong><span>2 Skills · 1 Command · 1 MCP</span></div></div>
        </div>
      </section>
      <section className="panel preview-box"><strong>项目资产</strong><p>完整挂载关系将在后续页面阶段展示。</p></section>
    </div>
  );
}
