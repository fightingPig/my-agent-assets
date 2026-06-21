import { AlertTriangle, Check, FolderSearch, House, ScanSearch } from "lucide-react";
import { useState } from "react";
import { StaticActionButton } from "../components/ui/StaticActionButton";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

const scopes = [
  { id: "user", title: "用户级", detail: "扫描 ~/.claude 与本地配置", icon: House },
  { id: "project", title: "项目级", detail: "扫描已登记项目的 .claude", icon: FolderSearch },
  { id: "custom", title: "自定义路径", detail: "预览指定目录下的资产", icon: ScanSearch },
] as const;

const results = [
  { name: "api-design", type: "Skill", source: "用户级", result: "新增" },
  { name: "format-code", type: "Command", source: "project-a", result: "新增" },
  { name: "Filesystem", type: "MCP", source: "my-app", result: "更新" },
  { name: "db-review", type: "Skill", source: "project-a", result: "冲突" },
];

export function ScanImportPage() {
  const [selectedScope, setSelectedScope] = useState<(typeof scopes)[number]["id"]>("user");

  return (
    <div className="operation-workspace">
      <section className="panel operation-stepper" aria-label="扫描步骤">
        {["选择扫描范围", "扫描预览", "导入确认"].map((step, index) => <div className={index === 0 ? "active" : ""} key={step}><span>{index === 0 ? <Check size={13} /> : index + 1}</span><strong>{step}</strong></div>)}
      </section>

      <section className="panel operation-section">
        <div className="section-heading"><div><h3>选择扫描范围</h3><p>选择仅更新本地预览，不读取文件系统</p></div><span className="preview-label">静态数据</span></div>
        <div className="scope-card-grid">
          {scopes.map(({ id, title, detail, icon: Icon }) => <button aria-pressed={selectedScope === id} className={`scope-card ${selectedScope === id ? "selected" : ""}`} data-no-drag="true" key={id} onClick={() => setSelectedScope(id)} style={NO_DRAG_REGION_STYLE} type="button"><span><Icon size={18} /></span><strong>{title}</strong><small>{detail}</small></button>)}
        </div>
      </section>

      <div className="scan-summary-grid">
        <div className="panel"><span>发现资产</span><strong>14</strong><small>静态扫描结果</small></div><div className="panel"><span>Skills</span><strong>4</strong><small>1 项新增</small></div><div className="panel"><span>Commands</span><strong>4</strong><small>1 项新增</small></div><div className="panel"><span>MCP Servers</span><strong>4</strong><small>1 项更新</small></div>
      </div>

      <section className="panel operation-section">
        <div className="section-heading"><div><h3>导入预览</h3><p>当前范围：{scopes.find((scope) => scope.id === selectedScope)?.title}</p></div><span>4 项待确认</span></div>
        <div className="preview-table" role="table" aria-label="导入预览表"><div className="preview-table-head" role="row"><span>资产</span><span>类型</span><span>来源</span><span>结果</span></div>{results.map((result) => <div className="preview-table-row" role="row" key={result.name}><strong>{result.name}</strong><span>{result.type}</span><span>{result.source}</span><span className={result.result === "冲突" ? "warning-text" : "success-text"}>{result.result}</span></div>)}</div>
        <div className="operation-warning"><AlertTriangle size={17} /><div><strong>发现 1 项命名冲突</strong><span>db-review 需要在导入前选择跳过、重命名或覆盖。</span></div></div>
        <div className="operation-actions"><StaticActionButton className="asset-secondary-action">保存扫描预览</StaticActionButton><StaticActionButton className="asset-business-action">确认导入</StaticActionButton></div>
      </section>
    </div>
  );
}
