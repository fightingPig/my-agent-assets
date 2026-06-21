import { BookOpen, FolderKanban, Link2 } from "lucide-react";
import { StaticActionButton } from "../components/ui/StaticActionButton";

export function AssetDetailPage() {
  return (
    <div className="detail-workspace">
      <section className="panel entity-hero">
        <div className="entity-hero-title"><span className="entity-hero-icon"><BookOpen size={21} /></span><div><small>代码审查工作流</small><h2>review</h2><p>统一代码审查流程与输出格式，覆盖正确性、回归风险和测试质量。</p></div></div>
        <span className="asset-status success">已挂载</span>
      </section>
      <div className="detail-two-column">
        <div className="detail-column">
          <section className="panel detail-section"><div className="section-heading"><div><h3>资产信息</h3><p>Skill · 工程质量</p></div></div><dl className="entity-field-list"><div><dt>来源路径</dt><dd>assets/skills/review</dd></div><div><dt>作用域</dt><dd>用户级</dd></div><div><dt>最近更新</dt><dd>今天 10:24</dd></div><div><dt>使用引用</dt><dd>2 个运行目标</dd></div></dl></section>
          <section className="panel detail-section"><div className="section-heading"><div><h3>挂载目标</h3><p>当前静态引用关系</p></div><Link2 size={16} /></div><div className="reference-list"><div><FolderKanban size={15} /><span>用户级 Claude Runtime</span><small>~/.claude/skills/review</small></div><div><FolderKanban size={15} /><span>project-a</span><small>project-a/.claude/skills/review</small></div></div></section>
        </div>
        <section className="panel detail-section content-preview-panel"><div className="section-heading"><div><h3>SKILL.md 内容预览</h3><p>只读静态内容</p></div></div><pre><code>{`# Review

检查代码正确性、回归风险、边界条件和测试覆盖。

## 输出

- 按严重级别列出问题
- 提供文件与行号
- 标记剩余测试风险`}</code></pre><div className="detail-actions"><StaticActionButton className="asset-secondary-action">查看引用</StaticActionButton><StaticActionButton className="asset-business-action">管理挂载</StaticActionButton></div></section>
      </div>
    </div>
  );
}
