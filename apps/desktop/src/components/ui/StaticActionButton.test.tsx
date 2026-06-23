import { render, screen } from "@testing-library/react";
import type { CSSProperties } from "react";
import { describe, expect, expectTypeOf, it } from "vitest";
import { StaticActionButton, type StaticActionButtonProps } from "./StaticActionButton";

type ForbiddenEventProp =
  | "onClick"
  | "onMouseDown"
  | "onPointerDown"
  | "onKeyDown"
  | "onSubmit";

describe("StaticActionButton", () => {
  it("always renders as a disabled no-drag control without a click handler", () => {
    render(<StaticActionButton className="asset-business-action">执行预览</StaticActionButton>);
    const button = screen.getByRole("button", { name: "执行预览" });
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute("aria-disabled", "true");
    expect(button).toHaveAttribute("data-no-drag", "true");
    expect(button).toHaveAttribute("type", "button");
    expect(button.onclick).toBeNull();

    const element = StaticActionButton({
      children: "执行预览",
      style: { WebkitAppRegion: "drag" } as CSSProperties,
    });
    expect(element.props.style).toMatchObject({ WebkitAppRegion: "no-drag" });
  });

  it("exposes no event handler props at the type boundary", () => {
    expectTypeOf<Extract<keyof StaticActionButtonProps, ForbiddenEventProp>>().toEqualTypeOf<never>();
  });
});
