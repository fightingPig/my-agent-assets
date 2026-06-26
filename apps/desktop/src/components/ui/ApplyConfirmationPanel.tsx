import type { ApplyResult } from "../../app/contracts";
import { NO_DRAG_REGION_STYLE } from "../../lib/platform";

export type ApplyConfirmationPanelProps = {
  title: string;
  description: string;
  confirmationValue: string;
  onConfirmationChange: (value: string) => void;
  onApply: () => void;
  canApply: boolean;
  isApplying: boolean;
  actionLabel: string;
  result: ApplyResult | null;
};

const CONFIRMATION_TOKEN = "APPLY";

export function ApplyConfirmationPanel({
  title,
  description,
  confirmationValue,
  onConfirmationChange,
  onApply,
  canApply,
  isApplying,
  actionLabel,
  result,
}: ApplyConfirmationPanelProps) {
  const confirmed = confirmationValue.trim() === CONFIRMATION_TOKEN;
  const disabled = !canApply || !confirmed || isApplying;
  const resultMessage = result
    ? result.ok
      ? `执行完成：${result.steps.length} 个步骤，${result.backup ? "已创建备份" : "未创建备份"}。`
      : `执行失败：${result.errors[0] ?? "请查看步骤结果。"}`
    : "执行前会再次调用后端 apply，并校验 previewId。";

  return (
    <div className="apply-confirmation-panel">
      <div>
        <strong>{title}</strong>
        <span>{description}</span>
      </div>
      <label>
        <span>输入 {CONFIRMATION_TOKEN} 以启用执行</span>
        <input
          data-no-drag="true"
          onChange={(event) => onConfirmationChange(event.target.value)}
          placeholder={CONFIRMATION_TOKEN}
          style={NO_DRAG_REGION_STYLE}
          value={confirmationValue}
        />
      </label>
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
      <p className={result?.ok === false ? "warning-text" : "success-text"}>{resultMessage}</p>
    </div>
  );
}
