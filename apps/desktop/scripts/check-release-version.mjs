import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import process from "node:process";

const repositoryRoot = resolve(import.meta.dirname, "../../..");

async function read(relativePath) {
  return readFile(resolve(repositoryRoot, relativePath), "utf8");
}

function packageVersionFromCargo(text, label) {
  const packageSection = text.match(/\[package\]([\s\S]*?)(?:\n\[|$)/)?.[1];
  const version = packageSection?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  if (!version) throw new Error(`Unable to read package version from ${label}.`);
  return version;
}

function packageVersionFromLock(text, packageName) {
  const escaped = packageName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const version = text.match(
    new RegExp(`\\[\\[package\\]\\]\\nname = "${escaped}"\\nversion = "([^"]+)"`),
  )?.[1];
  if (!version) throw new Error(`Unable to read ${packageName} from Cargo.lock.`);
  return version;
}

function expectedWindowsBundleVersion(version) {
  const beta = version.match(/^(\d+\.\d+\.\d+)-beta\.(\d+)$/);
  return beta ? `${beta[1]}-${beta[2]}` : version;
}

const packageJson = JSON.parse(await read("apps/desktop/package.json"));
const packageLock = JSON.parse(await read("apps/desktop/package-lock.json"));
const tauriConfig = JSON.parse(await read("apps/desktop/src-tauri/tauri.conf.json"));
const windowsConfig = JSON.parse(await read("apps/desktop/src-tauri/tauri.windows.conf.json"));
const cargoLock = await read("Cargo.lock");
const expected = packageJson.version;
const versions = new Map([
  ["apps/desktop/package-lock.json", packageLock.version],
  ["apps/desktop/package-lock.json packages['']", packageLock.packages?.[""]?.version],
  ["apps/desktop/src-tauri/Cargo.toml", packageVersionFromCargo(await read("apps/desktop/src-tauri/Cargo.toml"), "desktop Cargo.toml")],
  ["apps/desktop/src-tauri/tauri.conf.json", tauriConfig.version],
  ["crates/core/Cargo.toml", packageVersionFromCargo(await read("crates/core/Cargo.toml"), "core Cargo.toml")],
  ["crates/cli/Cargo.toml", packageVersionFromCargo(await read("crates/cli/Cargo.toml"), "CLI Cargo.toml")],
  ["Cargo.lock my-agent-assets-desktop", packageVersionFromLock(cargoLock, "my-agent-assets-desktop")],
  ["Cargo.lock my-agent-assets-core", packageVersionFromLock(cargoLock, "my-agent-assets-core")],
  ["Cargo.lock my-agent-assets-cli", packageVersionFromLock(cargoLock, "my-agent-assets-cli")],
]);

const mismatches = [...versions].filter(([, value]) => value !== expected);
const expectedWindows = expectedWindowsBundleVersion(expected);
if (windowsConfig.version !== expectedWindows) {
  mismatches.push([
    "apps/desktop/src-tauri/tauri.windows.conf.json",
    `${windowsConfig.version} (expected Windows mapping ${expectedWindows})`,
  ]);
}

const releaseTag = process.env.RELEASE_TAG?.trim();
if (releaseTag && releaseTag !== `v${expected}`) {
  mismatches.push(["RELEASE_TAG", `${releaseTag} (expected v${expected})`]);
}

if (mismatches.length > 0) {
  for (const [source, value] of mismatches) {
    process.stderr.write(`[release-version] ${source}: ${String(value)}; product version: ${expected}\n`);
  }
  process.exitCode = 1;
} else {
  process.stdout.write(
    `[release-version] ${expected} is consistent across frontend, Rust, Tauri, Windows bundle, and lock files${releaseTag ? ` for ${releaseTag}` : ""}.\n`,
  );
}
