import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { AssetDetailPage } from "./AssetDetailPage";
import { BackupRestorePage } from "./BackupRestorePage";
import { ConflictResolverPage } from "./ConflictResolverPage";
import { MountManagerPage } from "./MountManagerPage";
import { ProjectDetailPage } from "./ProjectDetailPage";
import { ProjectsListPage } from "./ProjectsListPage";
import { ScanImportPage } from "./ScanImportPage";
import { SettingsPage } from "./SettingsPage";
import { SyncPage } from "./SyncPage";

afterEach(cleanup);

describe("remaining V1 static pages", () => {
  it("filters projects and updates the selected inspector", () => {
    render(<ProjectsListPage demoMode />);
    const inspector = screen.getByRole("complementary", { name: "项目检查器" });
    const projectA = screen.getByRole("option", { name: "project-a" });
    expect(projectA).toHaveAttribute("aria-selected", "true");
    expect(within(inspector).getByRole("heading", { name: "project-a" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("option", { name: "my-app" }));
    expect(within(inspector).getByRole("heading", { name: "my-app" })).toBeInTheDocument();
    expect(within(inspector).getByText("产品主应用")).toBeInTheDocument();

    fireEvent.change(screen.getByRole("searchbox", { name: "搜索项目" }), { target: { value: "design" } });
    expect(screen.getByRole("option", { name: "design-system" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "project-a" })).not.toBeInTheDocument();

    fireEvent.change(screen.getByRole("searchbox", { name: "搜索项目" }), { target: { value: "" } });
    fireEvent.change(screen.getByRole("combobox", { name: "项目状态筛选" }), { target: { value: "正常" } });
    expect(screen.getByRole("option", { name: "project-a" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "my-app" })).not.toBeInTheDocument();
  });

  it("renders project and asset detail workspaces", () => {
    const { rerender } = render(<ProjectDetailPage demoMode />);
    for (const heading of ["项目概览", "本地环境", "已挂载资产", "最近活动", "挂载计划预览"]) {
      expect(screen.getByRole("heading", { name: heading })).toBeInTheDocument();
    }
    rerender(<AssetDetailPage demoMode />);
    expect(screen.getByRole("heading", { name: "资产信息" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "挂载目标" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "SKILL.md 内容预览" })).toBeInTheDocument();
  });

  it("updates only the local Scan scope selection", () => {
    render(<ScanImportPage demoMode />);
    const userScope = screen.getByRole("button", { name: /用户级/ });
    const projectScope = screen.getByRole("button", { name: /项目级/ });
    expect(userScope).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(projectScope);
    expect(projectScope).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText("当前范围：项目级")).toBeInTheDocument();
    expect(screen.getByRole("table", { name: "导入预览表" })).toBeInTheDocument();
    expect(screen.getByText("只读扫描预览")).toBeInTheDocument();
  });

  it("updates the Mount asset and target preview", () => {
    render(<MountManagerPage demoMode />);
    const deploy = screen.getByRole("button", { name: /deploy-prod/ });
    const myApp = screen.getByRole("button", { name: /my-app/ });
    fireEvent.click(deploy);
    fireEvent.click(myApp);
    expect(deploy).toHaveAttribute("aria-pressed", "true");
    expect(myApp).toHaveAttribute("aria-pressed", "true");
    expect(screen.getAllByText("deploy-prod").length).toBeGreaterThan(1);
    expect(screen.getAllByText("my-app").length).toBeGreaterThan(1);
    expect(screen.getByText("执行前将创建本地备份")).toBeInTheDocument();
  });

  it("switches Conflict and Backup master-detail selections", () => {
    const { rerender } = render(<ConflictResolverPage demoMode />);
    fireEvent.click(screen.getByRole("option", { name: "review" }));
    expect(screen.getByText("资产中心已存在同名 Skill")).toBeInTheDocument();
    expect(screen.getByText(/检查架构、性能和安全边界/)).toBeInTheDocument();

    rerender(<BackupRestorePage demoMode />);
    fireEvent.click(screen.getByRole("option", { name: "backup-20260620-0915" }));
    expect(screen.getAllByText("挂载变更前").length).toBeGreaterThan(1);
    expect(screen.getByText("~/.my-agent-assets/backups/local/backup-20260620-0915/manifest.json")).toBeInTheDocument();
    expect(screen.getByText("手动恢复说明")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /恢复/ })).not.toBeInTheDocument();
  });

  it("renders local Git sync status and history", () => {
    render(<SyncPage demoMode />);
    expect(screen.getByRole("heading", { name: "本地 Git 仓库" })).toBeInTheDocument();
    expect(screen.getByText("Ahead")).toBeInTheDocument();
    expect(screen.getByText("Behind")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "同步历史" })).toBeInTheDocument();
    expect(screen.getAllByText("静态预览：尚未读取本地 Git 仓库。").length).toBeGreaterThan(0);
  });

  it("renders only the seven allowed Settings sections with a local save action", () => {
    const { container } = render(<SettingsPage demoMode />);
    for (const section of ["路径设置", "扫描设置", "安全设置", "同步设置", "外观设置", "日志设置", "CLI 设置"]) {
      expect(screen.getByRole("heading", { name: section })).toBeInTheDocument();
    }
    const controls = Array.from(container.querySelectorAll<HTMLInputElement | HTMLSelectElement>("input,select"));
    expect(controls.length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "保存设置" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "检查 CLI" })).toBeDisabled();
  });

  it("keeps forbidden product concepts out of rendered UI", () => {
    const pages = [ProjectDetailPage, AssetDetailPage, ScanImportPage];
    const forbidden = ["登录", "账号", "OAuth", "云账号", "团队空间", "订阅", "Billing", "GitHub 绑定"];
    for (const Page of pages) {
      const { container, unmount } = render(<Page />);
      for (const phrase of forbidden) expect(container.textContent).not.toContain(phrase);
      unmount();
    }
    for (const Page of [ProjectsListPage, MountManagerPage, ConflictResolverPage, BackupRestorePage, SyncPage, SettingsPage]) {
      const { container, unmount } = render(<Page demoMode />);
      for (const phrase of forbidden) expect(container.textContent).not.toContain(phrase);
      unmount();
    }
  });
});
