import { AlertTriangle, Blocks, BookOpen, FolderKanban, Link2, TerminalSquare } from "lucide-react";
import { useState } from "react";
import { StaticActionButton } from "../components/ui/StaticActionButton";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

const assets = [
  { id: "review", type: "Skill", detail: "代码审查工作流", icon: BookOpen },
  { id: "deploy-prod", type: "Command", detail: "生产环境部署", icon: TerminalSquare },
  { id: "PostgreSQL", type: "MCP", detail: "数据库访问", icon: Blocks },
];
const targets = [
  { id: "project-a", detail: "~/workspace/project-a" },
  { id: "my-app", detail: "~/workspace/my-app" },
  { id: "user", detail: "用户级 Claude Runtime" },
];

export function MountManagerPage() {
  const [selectedAsset, setSelectedAsset] = useState(assets[0].id);
  const [selectedTarget, setSelectedTarget] = useState(targets[0].id);
  const asset = assets.find((item) => item.id === selectedAsset)!;
  const target = targets.find((item) => item.id === selectedTarget)!;

  return (
    <div className="operation-workspace">
      <section className="panel mount-workflow">
        <div className="mount-flow-column"><div className="mount-flow-heading"><span>1</span><div><strong>选择资产</strong><small>资产中心静态数据</small></div></div><div className="selectable-stack">{assets.map(({ id, type, detail, icon: Icon }) => <button aria-pressed={selectedAsset === id} className={selectedAsset === id ? "selected" : ""} data-no-drag="true" key={id} onClick={() => setSelectedAsset(id)} style={NO_DRAG_REGION_STYLE} type="button"><Icon size={16} /><span><strong>{id}</strong><small>{type} · {detail}</small></span></button>)}</div></div>
        <div className="mount-flow-column"><div className="mount-flow-heading"><span>2</span><div><strong>选择目标</strong><small>本地运行目标</small></div></div><div className="selectable-stack">{targets.map(({ id, detail }) => <button aria-pressed={selectedTarget === id} className={selectedTarget === id ? "selected" : ""} data-no-drag="true" key={id} onClick={() => setSelectedTarget(id)} style={NO_DRAG_REGION_STYLE} type="button"><FolderKanban size={16} /><span><strong>{id === "user" ? "用户级" : id}</strong><small>{detail}</small></span></button>)}</div></div>
        <div className="mount-flow-column plan"><div className="mount-flow-heading"><span>3</span><div><strong>预览挂载计划</strong><small>不会执行文件变更</small></div></div><div className="mount-plan-summary"><div><Link2 size={17} /><span><strong>{asset.id}</strong><small>{asset.type}</small></span></div><i>→</i><div><FolderKanban size={17} /><span><strong>{target.id === "user" ? "用户级" : target.id}</strong><small>{target.detail}</small></span></div></div><div className="plan-lines"><span>验证资产中心来源</span><span>{asset.type === "MCP" ? "编译目标 MCP 配置" : "创建目标软链接"}</span><span>写入挂载关系记录</span></div></div>
      </section>
      <section className="panel mount-review-bar"><div className="operation-warning"><AlertTriangle size={17} /><div><strong>执行前将创建本地备份</strong><span>若目标已存在同名内容，需要先进入冲突处理。</span></div></div><div className="operation-actions"><StaticActionButton className="asset-secondary-action">导出计划</StaticActionButton><StaticActionButton className="asset-business-action">确认挂载</StaticActionButton></div></section>
    </div>
  );
}
