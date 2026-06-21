import type { ButtonHTMLAttributes } from "react";
import { NO_DRAG_REGION_STYLE } from "../../lib/platform";

type NativeButtonProps = Omit<
  ButtonHTMLAttributes<HTMLButtonElement>,
  "aria-disabled" | "disabled" | "onClick"
>;

export type StaticActionButtonProps = NativeButtonProps & {
  onClick?: never;
};

export function StaticActionButton({ children, style, type = "button", ...props }: StaticActionButtonProps) {
  return (
    <button
      {...props}
      aria-disabled="true"
      data-no-drag="true"
      disabled
      onClick={undefined}
      style={{ ...style, ...NO_DRAG_REGION_STYLE }}
      type={type}
    >
      {children}
    </button>
  );
}
