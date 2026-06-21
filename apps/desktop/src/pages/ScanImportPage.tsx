import { Check, ScanSearch } from "lucide-react";

const steps = ["选择扫描范围", "扫描预览", "导入确认"];

export function ScanImportPage() {
  return (
    <section className="panel skeleton-panel">
      <div className="panel-header">
        <div><h2>导入流程</h2><p>本阶段不会读取本机 Claude 数据</p></div>
        <button className="primary-button" type="button"><ScanSearch size={16} />开始预览</button>
      </div>
      <div className="workflow-steps">
        {steps.map((step, index) => (
          <div className="workflow-step" key={step}>
            <span className="step-index">{index + 1}</span>
            <div><strong>{step}</strong><p>{index === 0 ? "选择用户级或项目级范围" : index === 1 ? "查看待导入资产和冲突" : "确认计划后再执行变更"}</p></div>
            {index === 0 && <Check size={16} />}
          </div>
        ))}
      </div>
    </section>
  );
}
