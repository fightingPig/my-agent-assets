import { Blocks } from "lucide-react";

const servers = [
  { name: "PostgreSQL", detail: "本地配置 · stdio", status: "配置正常" },
  { name: "Redis", detail: "本地配置 · stdio", status: "待检查" },
  { name: "GitHub", detail: "项目配置 · stdio", status: "未启用" },
];

export function McpServersListPage() {
  return (
    <section className="panel skeleton-panel">
      <div className="panel-header">
        <div><h2>MCP 配置</h2><p>状态仅表示本地配置与连接检查结果</p></div>
        <span className="healthy-badge">本地预览</span>
      </div>
      <div className="skeleton-list">
        {servers.map((server) => (
          <div className="skeleton-row" key={server.name}>
            <div className="skeleton-icon"><Blocks size={17} /></div>
            <div className="skeleton-copy"><strong>{server.name}</strong><span>{server.detail}</span></div>
            <span className="status-badge neutral">{server.status}</span>
          </div>
        ))}
      </div>
    </section>
  );
}
