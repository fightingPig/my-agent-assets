import { FolderKanban } from "lucide-react";

const projects = [
  { name: "project-a", detail: "~/workspace/project-a", status: "4 项资产" },
  { name: "my-app", detail: "~/workspace/my-app", status: "7 项资产" },
  { name: "design-system", detail: "~/code/design-system", status: "3 项资产" },
];

export function ProjectsListPage() {
  return (
    <section className="panel skeleton-panel">
      <div className="panel-header">
        <div><h2>本机项目</h2><p>项目路径为静态预览，不访问文件系统</p></div>
        <span className="healthy-badge">3 个项目</span>
      </div>
      <div className="skeleton-list">
        {projects.map((project) => (
          <div className="skeleton-row" key={project.name}>
            <div className="skeleton-icon blue"><FolderKanban size={17} /></div>
            <div className="skeleton-copy"><strong>{project.name}</strong><span>{project.detail}</span></div>
            <span className="status-badge neutral">{project.status}</span>
          </div>
        ))}
      </div>
    </section>
  );
}
