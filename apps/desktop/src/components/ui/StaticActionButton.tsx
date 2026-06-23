import type { AriaAttributes, CSSProperties, ReactNode } from "react";
import { NO_DRAG_REGION_STYLE } from "../../lib/platform";

export type StaticActionButtonProps = {
  children: ReactNode;
  className?: string;
  title?: string;
  "aria-label"?: AriaAttributes["aria-label"];
  style?: CSSProperties;
};

export function StaticActionButton({
  children,
  className,
  title,
  "aria-label": ariaLabel,
  style,
}: StaticActionButtonProps) {
  return (
    <button
      aria-label={ariaLabel}
      aria-disabled="true"
      className={className}
      data-no-drag="true"
      disabled
      style={{ ...style, ...NO_DRAG_REGION_STYLE }}
      title={title}
      type="button"
    >
      {children}
    </button>
  );
}
