import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StaticActionButton } from "./StaticActionButton";

describe("StaticActionButton", () => {
  it("always renders as a disabled no-drag control without a click handler", () => {
    render(<StaticActionButton className="asset-business-action">执行预览</StaticActionButton>);
    const button = screen.getByRole("button", { name: "执行预览" });
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute("aria-disabled", "true");
    expect(button).toHaveAttribute("data-no-drag", "true");
    expect(button.onclick).toBeNull();

    const element = StaticActionButton({ children: "执行预览" });
    expect(element.props.style).toMatchObject({ WebkitAppRegion: "no-drag" });
  });

  it("rejects onClick at the type boundary", () => {
    // @ts-expect-error Static actions cannot receive behavior.
    const invalid = <StaticActionButton onClick={() => undefined}>Invalid</StaticActionButton>;
    expect(invalid.props.onClick).toBeTypeOf("function");
  });
});
