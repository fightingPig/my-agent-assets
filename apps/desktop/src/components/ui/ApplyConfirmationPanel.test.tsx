import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ApplyConfirmationPanel } from "./ApplyConfirmationPanel";

describe("ApplyConfirmationPanel", () => {
  it("uses preview readiness instead of typed confirmation and keeps the action no-drag", () => {
    const { rerender } = render(
      <ApplyConfirmationPanel
        actionLabel="确认导入"
        canApply
        description="将覆盖已存在的 runtime 内容。"
        isApplying={false}
        onApply={vi.fn()}
        result={null}
        title="高风险操作"
      />,
    );

    expect(screen.queryByPlaceholderText("APPLY")).not.toBeInTheDocument();
    expect(screen.queryByText(/输入 APPLY/)).not.toBeInTheDocument();
    const action = screen.getByRole("button", { name: "确认导入" });
    expect(action).toBeEnabled();
    expect(action).toHaveAttribute("data-no-drag", "true");
    expect(screen.getByText("将覆盖已存在的 runtime 内容。").closest(".operation-warning")).not.toBeNull();

    rerender(
      <ApplyConfirmationPanel
        actionLabel="确认导入"
        canApply={false}
        description="执行说明"
        isApplying={false}
        onApply={vi.fn()}
        result={null}
        title="执行导入"
      />,
    );

    expect(screen.getByRole("button", { name: "确认导入" })).toBeDisabled();
  });

  it("shows step, backup, warning, and failure guidance details", () => {
    const { rerender } = render(
      <ApplyConfirmationPanel
        actionLabel="确认导入"
        canApply
        description="执行说明"
        isApplying={false}
        onApply={vi.fn()}
        result={{
          mode: "apply",
          ok: true,
          previewId: "preview:import:test",
          backup: {
            id: "backup-1",
            label: "导入前备份",
            createdAt: "2026-06-27T00:00:00Z",
            sizeBytes: 42,
            entryCount: 2,
            manifestPath: "/tmp/manifest.json",
            runtimeRoot: "/tmp/home",
            affectedPaths: ["/tmp/home/.my-agent-assets/assets/skills/review"],
          },
          steps: [
            {
              stepId: "import-review",
              kind: "import",
              label: "导入 review",
              status: "success",
              message: "Imported.",
              affectedPaths: [],
            },
          ],
          warnings: ["需要重新扫描资产列表。"],
          errors: [],
        }}
        title="执行导入"
      />,
    );

    expect(screen.getByText("执行完成：成功 1 项，跳过 0 项。")).toBeInTheDocument();
    expect(screen.getByText(/导入前备份（backup-1，2 项）/)).toBeInTheDocument();
    expect(screen.getByText("提示：需要重新扫描资产列表。")).toBeInTheDocument();

    rerender(
      <ApplyConfirmationPanel
        actionLabel="确认导入"
        canApply
        description="执行说明"
        isApplying={false}
        onApply={vi.fn()}
        operationError="后端暂时不可用"
        result={null}
        title="执行导入"
      />,
    );

    expect(screen.getByText("执行失败：后端暂时不可用")).toBeInTheDocument();
    expect(screen.getByText(/刷新预览并重新生成计划/)).toBeInTheDocument();
  });
});
