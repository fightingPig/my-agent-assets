import type { ApplyResult } from "../../app/contracts";
import { NO_DRAG_REGION_STYLE } from "../../lib/platform";

export type ApplyConfirmationPanelProps = {
  title: string;
  description: string;
  onApply: () => void;
  canApply: boolean;
  isApplying: boolean;
  actionLabel: string;
  result: ApplyResult | null;
  operationError?: string | null;
};

export function ApplyConfirmationPanel({
  title,
  description,
  onApply,
  canApply,
  isApplying,
  actionLabel,
  result,
  operationError = null,
}: ApplyConfirmationPanelProps) {
  const disabled = !canApply || isApplying;
  const successfulSteps = result?.steps.filter((step) => step.status === "success").length ?? 0;
  const skippedSteps = result?.steps.filter((step) => step.status === "skipped").length ?? 0;
  const failedSteps = result?.steps.filter((step) => step.status === "failed").length ?? 0;
  const resultMessage = result
    ? result.ok
      ? `执行完成：成功 ${successfulSteps} 项，跳过 ${skippedSteps} 项。`
      : `执行失败：${result.errors[0] ?? "请查看步骤结果。"}`
    : operationError
      ? `执行失败：${operationError}`
    : "执行前会再次调用后端 apply，并校验 previewId。";
  const failureGuidance = result?.ok === false || operationError
    ? "未完成的变更不会自动重试。请检查错误信息，刷新预览并重新生成计划后再执行。"
    : null;

  return (
    <div className="apply-confirmation-panel" style={{ gridTemplateColumns: "minmax(0, 1fr) auto" }}>
      <div className="operation-warning">
        <strong>{title}</strong>
        <span>{description}</span>
      </div>
      <button
        className="asset-business-action"
        data-no-drag="true"
        disabled={disabled}
        onClick={onApply}
        style={NO_DRAG_REGION_STYLE}
        type="button"
      >
        {isApplying ? "执行中" : actionLabel}
      </button>
      <div
        className={`apply-result-summary ${result?.ok === false || operationError ? "failed" : result?.ok ? "succeeded" : ""}`}
        role="status"
      >
        <p className={result?.ok === false || operationError ? "warning-text" : "success-text"}>{resultMessage}</p>
        {result?.backup ? (
          <p>
            备份：{result.backup.label}（{result.backup.id}，{result.backup.entryCount} 项）
          </p>
        ) : result?.ok ? <p>本次执行未创建备份。</p> : null}
        {failedSteps > 0 ? <p>失败步骤：{failedSteps} 项。</p> : null}
        {result?.warnings.map((warning) => <p className="warning-text" key={warning}>提示：{warning}</p>)}
        {result?.errors.slice(1).map((error) => <p className="warning-text" key={error}>错误：{error}</p>)}
        {failureGuidance ? <p>{failureGuidance}</p> : null}
      </div>
    </div>
  );
}
