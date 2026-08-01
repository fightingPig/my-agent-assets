import { describe, expect, it } from "vitest";
import { statusToneForLabel } from "./status";

describe("shared status tones", () => {
  it.each([
    ["正常", "success"],
    ["待同步", "warning"],
    ["未连接", "neutral"],
    ["执行失败", "danger"],
    ["pending review", "warning"],
    ["loading", "neutral"],
    ["conflict", "danger"],
  ] as const)("maps %s to %s", (label, tone) => {
    expect(statusToneForLabel(label)).toBe(tone);
  });
});
