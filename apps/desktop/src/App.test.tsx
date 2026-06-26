import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { CurrentPage } from "./app/CurrentPage";
import { PAGE_REGISTRY } from "./app/pages";
import styles from "./styles.css?raw";

const { invoke, startDragging } = vi.hoisted(() => ({
  invoke: vi.fn(),
  startDragging: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ startDragging }),
}));

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

function setNavigatorPlatform(userAgent: string, platform: string) {
  Object.defineProperty(navigator, "userAgent", { configurable: true, value: userAgent });
  Object.defineProperty(navigator, "platform", { configurable: true, value: platform });
}

describe("macOS preview home", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    startDragging.mockResolvedValue(undefined);
    setNavigatorPlatform("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)", "MacIntel");
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    invoke.mockResolvedValue({
      name: "My Agent Assets",
      version: "0.1.0",
      platform: "macOS",
      arch: "arm64",
      backendReady: true,
    });
  });

  it("labels all business content as preview data", async () => {
    render(<App />);
    expect(screen.getByText("预览数据")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "首页" })).toBeInTheDocument();
    expect(await screen.findByText("已连接")).toBeInTheDocument();
  });

  it("switches every visible primary page and updates PageHeader", () => {
    const { container } = render(<App />);
    const pageHeader = container.querySelector<HTMLElement>(".page-header")!;
    const visiblePages = PAGE_REGISTRY.filter((page) => page.sidebarVisible);

    expect(visiblePages).toHaveLength(11);
    for (const page of visiblePages) {
      const navButton = screen.getByRole("button", { name: page.sidebarLabel });
      expect(navButton).toBeEnabled();
      fireEvent.click(navButton);
      expect(navButton).toHaveAttribute("aria-current", "page");
      expect(within(pageHeader).getByRole("heading", { name: page.title })).toBeInTheDocument();
      expect(within(pageHeader).getByText(page.subtitle)).toBeInTheDocument();
    }
  });

  it("keeps detail pages out of primary sidebar navigation", () => {
    render(<App />);
    expect(screen.queryByRole("button", { name: "资产详情" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "项目详情" })).not.toBeInTheDocument();
  });

  it("opens hidden detail pages from list inspectors without adding sidebar routes", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Skills" }));
    fireEvent.click(await screen.findByRole("button", { name: "查看详情" }));
    expect(screen.getByRole("heading", { name: "资产详情" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "SKILL.md 内容预览" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "资产详情" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "项目列表" }));
    fireEvent.click(await screen.findByRole("button", { name: "查看详情" }));
    expect(screen.getByRole("heading", { name: "项目详情" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "已挂载资产" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "项目详情" })).not.toBeInTheDocument();
  });

  it("renders the required static skeleton content", () => {
    render(<App />);
    const expectedContent = [
      ["Skills", "review"],
      ["Commands", "deploy-prod"],
      ["MCP Servers", "PostgreSQL"],
      ["项目列表", "project-a"],
      ["扫描导入", "导入预览"],
      ["挂载管理", "预览挂载计划"],
      ["冲突处理", "待处理冲突"],
      ["备份恢复", "恢复影响预览"],
      ["同步", "本地 Git 仓库"],
      ["设置", "CLI 设置"],
    ];

    for (const [navigation, content] of expectedContent) {
      fireEvent.click(screen.getByRole("button", { name: navigation }));
      expect(screen.getAllByText(content).length).toBeGreaterThan(0);
    }
  });

  it("keeps identity and authentication concepts out of rendered pages", () => {
    const { container } = render(<App />);
    const forbidden = ["账号", "用户中心", "OAuth", "绑定 GitHub", "云账号", "团队空间", "订阅", "Billing"];

    for (const page of PAGE_REGISTRY.filter((item) => item.sidebarVisible)) {
      fireEvent.click(screen.getByRole("button", { name: page.sidebarLabel }));
      const text = container.textContent ?? "";
      for (const phrase of forbidden) expect(text).not.toContain(phrase);
    }
  });

  it("provides static detail page skeletons outside sidebar navigation", () => {
    const appInfo = { name: "My Agent Assets", version: "0.1.0", platform: "macOS", arch: "arm64", backendReady: true };
    const { rerender } = render(<CurrentPage activePage="asset-detail" appInfo={appInfo} />);
    expect(screen.getByRole("heading", { name: "SKILL.md 内容预览" })).toBeInTheDocument();
    rerender(<CurrentPage activePage="project-detail" appInfo={appInfo} />);
    expect(screen.getByRole("heading", { name: "已挂载资产" })).toBeInTheDocument();
  });

  it("uses a two-column native overlay area without business controls", () => {
    const { container } = render(<App />);
    const overlay = container.querySelector(".mac-overlay-drag-area");
    expect(overlay).toBeInTheDocument();
    expect(overlay).toHaveTextContent("");
    expect(container.querySelector(".mac-overlay-sidebar")).toBeInTheDocument();
    expect(container.querySelector(".mac-overlay-main")).toBeInTheDocument();
    expect(container.querySelector(".traffic")).not.toBeInTheDocument();
    expect(container.querySelector(".traffic-lights")).not.toBeInTheDocument();
    expect(container.querySelector(".mac-window-controls")).not.toBeInTheDocument();
    expect(container.querySelector(".windows-controls")).not.toBeInTheDocument();
    expect(overlay).not.toHaveAttribute("aria-hidden");
  });

  it("starts dragging on every primary pointer press", async () => {
    const { container } = render(<App />);
    const overlay = container.querySelector(".mac-overlay-drag-area")!;
    fireEvent.pointerDown(overlay, { button: 0 });
    fireEvent.pointerDown(overlay, { button: 0 });

    await waitFor(() => expect(startDragging).toHaveBeenCalledTimes(2));
  });

  it("does not drag for secondary or interactive pointer targets", () => {
    const { container } = render(<App />);
    const overlay = container.querySelector(".mac-overlay-drag-area")!;
    fireEvent.pointerDown(overlay, { button: 2 });

    const button = document.createElement("button");
    overlay.appendChild(button);
    fireEvent.pointerDown(button, { button: 0 });

    const noDrag = document.createElement("div");
    noDrag.dataset.noDrag = "true";
    overlay.appendChild(noDrag);
    fireEvent.pointerDown(noDrag, { button: 0 });

    expect(startDragging).not.toHaveBeenCalled();
  });

  it("reports startDragging failures without throwing", async () => {
    const error = new Error("drag unavailable");
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => undefined);
    startDragging.mockRejectedValueOnce(error);
    const { container } = render(<App />);

    fireEvent.pointerDown(container.querySelector(".mac-overlay-drag-area")!, { button: 0 });

    await waitFor(() => {
      expect(consoleError).toHaveBeenCalledWith("[MacOverlayDragArea] startDragging failed", error);
    });
    consoleError.mockRestore();
  });

  it("fills the webview and keeps all business actions in PageHeader", () => {
    const { container } = render(<App />);
    const frame = container.querySelector(".app-frame");
    const body = container.querySelector(".app-body");
    const main = container.querySelector(".app-main");
    const pageHeader = container.querySelector(".page-header");
    const actions = container.querySelector(".page-header-actions");

    expect(frame).toBeInTheDocument();
    expect(body).toBeInTheDocument();
    expect(main).toBeInTheDocument();
    expect(pageHeader).toBeInTheDocument();
    expect(styles).toMatch(/\.app-frame\s*\{[^}]*width:\s*100vw;[^}]*height:\s*100vh;[^}]*overflow:\s*hidden;/s);
    expect(styles).toMatch(/\.app-body\s*\{[^}]*height:\s*calc\(100vh - var\(--overlay-height\)\);/s);
    expect(styles).toMatch(/\.app-main\s*\{[^}]*padding:\s*34px 36px 36px;/s);
    expect(styles).toMatch(/\.page-header\s*\{[^}]*margin-top:\s*0;/s);
    expect(actions).toContainElement(screen.getByRole("button", { name: /搜索/ }));
    expect(actions).toContainElement(screen.getByRole("button", { name: "预览数据" }));
    expect(actions).toContainElement(screen.getByRole("button", { name: "快速操作" }));
    expect(screen.getAllByText("预览数据")).toHaveLength(1);
  });

  it("marks every interactive shell control as no-drag", () => {
    render(<App />);
    expect(screen.getByRole("button", { name: /搜索/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "预览数据" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "快速操作" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "首页" })).toBeInTheDocument();
    expect(styles).toMatch(/\.search-button, \.preview-button, \.primary-button\s*\{[^}]*-webkit-app-region:\s*no-drag;/s);
    expect(styles).toMatch(/\.nav-item\s*\{[^}]*-webkit-app-region:\s*no-drag;/s);
    expect(styles).toMatch(/\.dropdown-menu,[^}]*-webkit-app-region:\s*no-drag;/s);
  });
});

describe("Windows app shell", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setNavigatorPlatform("Mozilla/5.0 (Windows NT 10.0; Win64; x64)", "Win32");
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    invoke.mockResolvedValue({
      name: "My Agent Assets",
      version: "0.1.0",
      platform: "windows",
      arch: "x86_64",
      backendReady: true,
    });
  });

  it("uses native Windows decorations and shortcuts without custom controls", () => {
    const { container } = render(<App />);
    expect(container.querySelector(".mac-overlay-drag-area")).not.toBeInTheDocument();
    expect(container.querySelector(".windows-controls")).not.toBeInTheDocument();
    expect(container.querySelector(".macos-controls")).not.toBeInTheDocument();
    expect(container.querySelector(".app-body")).toBeInTheDocument();
    expect(styles).toMatch(/\.platform-windows \.app-body,[^}]*\{\s*height:\s*100vh;/s);
    expect(screen.getByText("Ctrl+K")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /搜索/ })).toHaveAttribute("title", "全局搜索 Ctrl+K");
  });
});
