import { BookOpen, FolderKanban, Link2 } from "lucide-react";

const columns = [
  { title: "1. 选择资产", icon: BookOpen, items: ["review", "db-review", "deploy-prod"] },
  { title: "2. 选择目标", icon: FolderKanban, items: ["用户级目录", "project-a", "my-app"] },
  { title: "3. 预览挂载计划", icon: Link2, items: ["创建 2 个软链接", "编译 1 项 MCP", "保留原始资产"] },
];

export function MountManagerPage() {
  return (
    <section className="panel skeleton-panel">
      <div className="panel-header"><div><h2>挂载流程</h2><p>选择内容仅用于静态界面预览</p></div><button className="primary-button" type="button">确认计划</button></div>
      <div className="mount-columns">
        {columns.map(({ title, icon: Icon, items }) => (
          <section className="mount-column" key={title}>
            <div className="mount-column-title"><Icon size={17} /><strong>{title}</strong></div>
            {items.map((item) => <div className="mount-option" key={item}>{item}</div>)}
          </section>
        ))}
      </div>
    </section>
  );
}
