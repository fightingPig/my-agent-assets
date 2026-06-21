import { BookOpen } from "lucide-react";

const skills = [
  { name: "review", detail: "用户级 · 2 个挂载", status: "已挂载" },
  { name: "db-review", detail: "project-a · 1 个挂载", status: "已挂载" },
  { name: "react-review", detail: "资产中心 · 暂无挂载", status: "未挂载" },
];

export function SkillsListPage() {
  return (
    <section className="panel skeleton-panel">
      <div className="panel-header">
        <div><h2>技能资产</h2><p>当前展示静态预览数据</p></div>
        <span className="healthy-badge">3 项预览</span>
      </div>
      <div className="skeleton-list">
        {skills.map((skill) => (
          <div className="skeleton-row" key={skill.name}>
            <div className="skeleton-icon"><BookOpen size={17} /></div>
            <div className="skeleton-copy"><strong>{skill.name}</strong><span>{skill.detail}</span></div>
            <span className="status-badge">{skill.status}</span>
          </div>
        ))}
      </div>
    </section>
  );
}
