import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import styles from "./styles.css?raw";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

afterEach(cleanup);

function setNavigatorPlatform(userAgent: string, platform: string) {
  Object.defineProperty(navigator, "userAgent", { configurable: true, value: userAgent });
  Object.defineProperty(navigator, "platform", { configurable: true, value: platform });
}

describe("macOS preview home", () => {
  beforeEach(() => {
    vi.clearAllMocks();
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

  it("keeps future navigation disabled", () => {
    render(<App />);
    expect(screen.getByRole("button", { name: "Skills" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "首页" })).toBeEnabled();
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
    expect(screen.getByRole("button", { name: /搜索/ })).toHaveAttribute("title", "页面搜索 Ctrl+F");
  });
});
