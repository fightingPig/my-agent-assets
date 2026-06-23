import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { constants as fsConstants } from "node:fs";
import { access } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import process from "node:process";

const desktopDir = resolve(import.meta.dirname, "..");
const artifactDir = join(desktopDir, "artifacts", "visual-qa");
const viteBinary = join(desktopDir, "node_modules", ".bin", "vite");
const viewports = [
  { width: 1440, height: 900 },
  { width: 1180, height: 760 },
];
const defaultChromePath = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

function delay(milliseconds) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, milliseconds));
}

async function freePort() {
  return await new Promise((resolvePort, reject) => {
    const server = createServer();
    server.unref();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close(() => reject(new Error("Unable to allocate a local TCP port.")));
        return;
      }
      server.close(() => resolvePort(address.port));
    });
  });
}

async function resolveChromePath() {
  const chromePath = process.env.CHROME_BIN || defaultChromePath;
  try {
    await access(chromePath, fsConstants.X_OK);
    return chromePath;
  } catch {
    const source = process.env.CHROME_BIN ? "CHROME_BIN" : "the default macOS Chrome path";
    throw new Error(`Chrome executable from ${source} was not found or is not executable: ${chromePath}`);
  }
}

async function waitForHttp(url, child, label) {
  const deadline = Date.now() + 20_000;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`${label} exited before becoming ready.`);
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      // The server is still starting.
    }
    await delay(100);
  }
  throw new Error(`${label} did not become ready within 20 seconds.`);
}

async function waitForChrome(debugPort, child) {
  const endpoint = `http://127.0.0.1:${debugPort}/json/list`;
  const deadline = Date.now() + 20_000;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error("Chrome exited before DevTools became ready.");
    try {
      const targets = await (await fetch(endpoint)).json();
      const page = targets.find((target) => target.type === "page");
      if (page?.webSocketDebuggerUrl) return page.webSocketDebuggerUrl;
    } catch {
      // Chrome is still starting.
    }
    await delay(100);
  }
  throw new Error("Chrome DevTools did not become ready within 20 seconds.");
}

class CdpClient {
  constructor(url) {
    this.nextId = 1;
    this.pending = new Map();
    this.socket = new WebSocket(url);
  }

  async connect() {
    await new Promise((resolveConnect, reject) => {
      this.socket.addEventListener("open", resolveConnect, { once: true });
      this.socket.addEventListener("error", () => reject(new Error("Chrome DevTools WebSocket failed to open.")), { once: true });
    });
    this.socket.addEventListener("message", (event) => {
      const message = JSON.parse(String(event.data));
      if (!message.id) return;
      const pending = this.pending.get(message.id);
      if (!pending) return;
      this.pending.delete(message.id);
      if (message.error) pending.reject(new Error(message.error.message));
      else pending.resolve(message.result);
    });
  }

  send(method, params = {}) {
    const id = this.nextId++;
    return new Promise((resolveCommand, reject) => {
      this.pending.set(id, { resolve: resolveCommand, reject });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }

  close() {
    this.socket.close();
  }
}

async function evaluate(client, expression) {
  const result = await client.send("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (result.exceptionDetails) throw new Error(result.exceptionDetails.text || "Chrome evaluation failed.");
  return result.result.value;
}

async function waitForQaReport(client) {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    if (await evaluate(client, "window.__VISUAL_QA_READY__ === true")) {
      return await evaluate(client, "window.__VISUAL_QA_REPORT__");
    }
    await delay(75);
  }
  throw new Error("Visual QA page did not publish a report within 15 seconds.");
}

async function stopChild(child) {
  if (!child || child.exitCode !== null) return;
  child.kill("SIGTERM");
  await Promise.race([
    new Promise((resolveExit) => child.once("exit", resolveExit)),
    delay(3_000).then(() => child.exitCode === null && child.kill("SIGKILL")),
  ]);
}

async function main() {
  const chromePath = await resolveChromePath();
  const vitePort = await freePort();
  const debugPort = await freePort();
  const viteUrl = `http://127.0.0.1:${vitePort}`;
  const chromeProfile = await mkdtemp(join(tmpdir(), "maa-visual-qa-chrome-"));
  let vite;
  let chrome;
  let client;
  const childLogs = [];

  try {
    await mkdir(artifactDir, { recursive: true });
    vite = spawn(viteBinary, ["--host", "127.0.0.1", "--port", String(vitePort), "--strictPort"], {
      cwd: desktopDir,
      env: { ...process.env, NO_COLOR: "1" },
      stdio: ["ignore", "pipe", "pipe"],
    });
    vite.stdout.on("data", (chunk) => childLogs.push(`[vite] ${chunk}`));
    vite.stderr.on("data", (chunk) => childLogs.push(`[vite] ${chunk}`));
    await waitForHttp(`${viteUrl}/visual-qa.html`, vite, "Vite");

    chrome = spawn(chromePath, [
      "--headless=new",
      "--disable-background-networking",
      "--disable-component-update",
      "--disable-default-apps",
      "--disable-extensions",
      "--disable-features=Translate",
      "--disable-sync",
      "--metrics-recording-only",
      "--no-first-run",
      `--remote-debugging-port=${debugPort}`,
      `--user-data-dir=${chromeProfile}`,
      "about:blank",
    ], { stdio: ["ignore", "ignore", "pipe"] });
    chrome.stderr.on("data", (chunk) => childLogs.push(`[chrome] ${chunk}`));

    client = new CdpClient(await waitForChrome(debugPort, chrome));
    await client.connect();
    await client.send("Page.enable");
    await client.send("Runtime.enable");

    await client.send("Page.navigate", { url: `${viteUrl}/visual-qa.html?platform=macos&page=dashboard` });
    await waitForQaReport(client);
    const manifest = await evaluate(client, "window.__VISUAL_QA_MANIFEST__");
    if (!Array.isArray(manifest) || manifest.length !== 13) {
      throw new Error(`Expected 13 Visual QA pages, received ${manifest?.length ?? "none"}.`);
    }

    const results = [];
    for (const page of manifest) {
      for (const viewport of viewports) {
        await client.send("Emulation.setDeviceMetricsOverride", {
          width: viewport.width,
          height: viewport.height,
          deviceScaleFactor: 1,
          mobile: false,
        });
        const url = `${viteUrl}/visual-qa.html?platform=macos&page=${encodeURIComponent(page.id)}`;
        await client.send("Page.navigate", { url });
        const report = await waitForQaReport(client);
        await client.send("Runtime.evaluate", { expression: "window.scrollTo(0, 0)" });
        const screenshot = await client.send("Page.captureScreenshot", {
          format: "png",
          fromSurface: true,
          captureBeyondViewport: false,
        });
        const screenshotPath = join(artifactDir, `${page.id}-${viewport.width}x${viewport.height}-macos.png`);
        await writeFile(screenshotPath, Buffer.from(screenshot.data, "base64"));
        results.push({ ...report, screenshotPath });
        process.stdout.write(`captured ${page.id} ${viewport.width}x${viewport.height}\n`);
      }
    }

    const severeCount = results.reduce((sum, result) => sum + result.severeIssues.length, 0);
    const warningCount = results.reduce((sum, result) => sum + result.warningIssues.length, 0);
    const summary = {
      generatedAt: new Date().toISOString(),
      chromePath,
      viteUrl,
      totalPages: manifest.length,
      totalScreenshots: results.length,
      severeCount,
      warningCount,
      results,
    };
    const summaryPath = join(artifactDir, "summary.json");
    await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
    process.stdout.write(`summary ${summaryPath}\nsevere ${severeCount}, warnings ${warningCount}\n`);
    if (severeCount > 0) process.exitCode = 2;
  } catch (error) {
    const details = childLogs.slice(-20).join("");
    throw new Error(`${error instanceof Error ? error.message : String(error)}${details ? `\n${details}` : ""}`);
  } finally {
    client?.close();
    await stopChild(chrome);
    await stopChild(vite);
    await rm(chromeProfile, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(`[visual-qa] ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
});
