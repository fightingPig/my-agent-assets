import { TerminalSquare } from "lucide-react";

const commands = [
  { name: "deploy-prod", detail: "部署工作流入口", status: "可用" },
  { name: "build-project", detail: "项目构建命令", status: "可用" },
  { name: "run-tests", detail: "测试执行命令", status: "可用" },
];

export function CommandsListPage() {
  return (
    <section className="panel skeleton-panel">
      <div className="panel-header">
        <div><h2>命令资产</h2><p>命令仅用于界面预览，不会执行</p></div>
        <span className="healthy-badge">3 项预览</span>
      </div>
      <div className="skeleton-list">
        {commands.map((command) => (
          <div className="skeleton-row" key={command.name}>
            <div className="skeleton-icon"><TerminalSquare size={17} /></div>
            <div className="skeleton-copy"><strong>{command.name}</strong><span>{command.detail}</span></div>
            <span className="status-badge">{command.status}</span>
          </div>
        ))}
      </div>
    </section>
  );
}
