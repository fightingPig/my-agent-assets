import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { CommandsListPage } from "../../pages/CommandsListPage";
import { McpServersListPage } from "../../pages/McpServersListPage";
import { SkillsListPage } from "../../pages/SkillsListPage";
import { SyncPage } from "../../pages/SyncPage";
import styles from "../../styles.css?raw";

afterEach(cleanup);

describe("Asset Center static UI", () => {
  it("selects the first Skill by default and updates the inspector from a row click", () => {
    render(<SkillsListPage />);
    const inspector = screen.getByRole("complementary", { name: "Skills检查器" });
    const review = screen.getByRole("option", { name: "review" });
    const dbReview = screen.getByRole("option", { name: "db-review" });

    expect(review).toHaveAttribute("aria-selected", "true");
    expect(within(inspector).getByRole("heading", { name: "review" })).toBeInTheDocument();
    fireEvent.click(dbReview);
    expect(dbReview).toHaveAttribute("aria-selected", "true");
    expect(within(inspector).getByRole("heading", { name: "db-review" })).toBeInTheDocument();
  });

  it("filters static Commands with search and status controls", () => {
    render(<CommandsListPage />);
    const search = screen.getByRole("searchbox", { name: "搜索Commands" });
    const status = screen.getByRole("combobox", { name: "Commands状态筛选" });

    fireEvent.change(search, { target: { value: "build" } });
    expect(screen.getByRole("option", { name: "build-project" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "deploy-prod" })).not.toBeInTheDocument();

    fireEvent.change(search, { target: { value: "" } });
    fireEvent.change(status, { target: { value: "待检查" } });
    expect(screen.getByRole("option", { name: "run-tests" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "build-project" })).not.toBeInTheDocument();

    fireEvent.change(search, { target: { value: "not-found" } });
    expect(screen.getByText("没有匹配的Commands")).toBeInTheDocument();
    expect(screen.getByText("暂无可检查资产")).toBeInTheDocument();
  });

  it("uses only local MCP examples and updates JSON details", () => {
    const { container } = render(<McpServersListPage />);
    for (const name of ["PostgreSQL", "Redis", "Filesystem", "SQLite"]) {
      expect(screen.getAllByText(name).length).toBeGreaterThan(0);
    }

    fireEvent.click(screen.getByRole("option", { name: "Filesystem" }));
    expect(screen.getByText(/filesystem-mcp/)).toBeInTheDocument();
    for (const phrase of ["GitHub", "登录", "账号", "OAuth", "云"]) {
      expect(container.textContent).not.toContain(phrase);
    }
  });

  it("keeps disabled business actions explicitly no-drag", () => {
    for (const Page of [SkillsListPage, CommandsListPage, McpServersListPage]) {
      const { unmount } = render(<Page />);
      const actions = screen.getAllByRole("button").filter((button) => button.getAttribute("aria-disabled") === "true");
      expect(actions).toHaveLength(2);
      for (const action of actions) {
        expect(action).toBeDisabled();
        expect(action).toHaveAttribute("data-no-drag", "true");
      }
      expect(styles).toMatch(/\.asset-secondary-action,[\s\S]*?\.asset-business-action\s*\{[^}]*-webkit-app-region:\s*no-drag;/);
      expect(styles).toMatch(/\.asset-business-action:disabled\s*\{[^}]*-webkit-app-region:\s*no-drag;/);
      unmount();
    }
  });

  it("keeps Sync wording repository-local", () => {
    const { container } = render(<SyncPage />);
    expect(screen.getByText("本地 Git 仓库")).toBeInTheDocument();
    expect(screen.getByText("远程仓库")).toBeInTheDocument();
    for (const phrase of ["GitHub", "登录", "账号", "OAuth", "云"]) {
      expect(container.textContent).not.toContain(phrase);
    }
  });
});
