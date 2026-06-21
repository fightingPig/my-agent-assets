import { BookOpen, Link2 } from "lucide-react";

export function AssetDetailPage() {
  return (
    <div className="page-stack">
      <section className="panel skeleton-panel">
        <div className="panel-header"><div><h2>review</h2><p>Skill · 静态详情预览</p></div><span className="healthy-badge">已挂载</span></div>
        <div className="detail-grid">
          <div className="detail-item"><BookOpen size={17} /><div><strong>资产位置</strong><span>assets/skills/review</span></div></div>
          <div className="detail-item"><Link2 size={17} /><div><strong>引用关系</strong><span>用户级与 project-a</span></div></div>
        </div>
      </section>
      <section className="panel preview-box"><strong>内容预览</strong><p>资产正文将在后续数据接入阶段显示。</p></section>
    </div>
  );
}
