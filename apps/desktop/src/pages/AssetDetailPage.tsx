import { BookOpen, FolderKanban, Link2 } from "lucide-react";
import type { AssetDetailContext } from "../app/detail-context";
import { StaticActionButton } from "../components/ui/StaticActionButton";

const fallbackDetail: AssetDetailContext = {
  name: "review",
  title: "代码审查工作流",
  summary: "统一代码审查流程与输出格式，覆盖正确性、回归风险和测试质量。",
  status: "已挂载",
  statusTone: "success",
  typeLabel: "Skill",
  category: "工程质量",
  sourcePath: "assets/skills/review",
  scope: "用户级",
  updated: "今天 10:24",
  mountTargets: ["~/.claude/skills/review", "project-a/.claude/skills/review"],
  previewLabel: "SKILL.md 内容预览",
  preview: `# Review

检查代码正确性、回归风险、边界条件和测试覆盖。

## 输出

- 按严重级别列出问题
- 提供文件与行号
- 标记剩余测试风险`,
};

type AssetDetailPageProps = {
  detail?: AssetDetailContext;
};

export function AssetDetailPage({ detail = fallbackDetail }: AssetDetailPageProps) {
  return (
    <div className="detail-workspace">
      <section className="panel entity-hero">
        <div className="entity-hero-title"><span className="entity-hero-icon"><BookOpen size={21} /></span><div><small>{detail.title}</small><h2>{detail.name}</h2><p>{detail.summary}</p></div></div>
        <span className={`asset-status ${detail.statusTone}`}>{detail.status}</span>
      </section>
      <div className="detail-two-column">
        <div className="detail-column">
          <section className="panel detail-section"><div className="section-heading"><div><h3>资产信息</h3><p>{detail.typeLabel} · {detail.category}</p></div></div><dl className="entity-field-list"><div><dt>来源路径</dt><dd>{detail.sourcePath}</dd></div><div><dt>作用域</dt><dd>{detail.scope}</dd></div><div><dt>最近更新</dt><dd>{detail.updated}</dd></div><div><dt>使用引用</dt><dd>{detail.mountTargets.length} 个运行目标</dd></div></dl></section>
          <section className="panel detail-section"><div className="section-heading"><div><h3>挂载目标</h3><p>当前引用关系</p></div><Link2 size={16} /></div><div className="reference-list">{detail.mountTargets.length > 0 ? detail.mountTargets.map((target) => <div key={target}><FolderKanban size={15} /><span>{target.includes("project") ? "项目级 Claude Runtime" : "用户级 Claude Runtime"}</span><small>{target}</small></div>) : <div><FolderKanban size={15} /><span>暂无挂载目标</span><small>资产中心</small></div>}</div></section>
        </div>
        <section className="panel detail-section content-preview-panel"><div className="section-heading"><div><h3>{detail.previewLabel}</h3><p>只读内容</p></div></div><pre><code>{detail.preview}</code></pre><div className="detail-actions"><StaticActionButton className="asset-secondary-action">查看引用</StaticActionButton><StaticActionButton className="asset-business-action">管理挂载</StaticActionButton></div></section>
      </div>
    </div>
  );
}
