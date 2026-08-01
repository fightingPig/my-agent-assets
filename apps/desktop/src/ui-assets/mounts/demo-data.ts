import type {
  AssetType,
  MountBinding,
  RegisteredMountTarget,
  RuntimeProvider,
} from "../../app/contracts";

const locations = [
  { key: "user", scope: "user" as const, path: "~", projectPath: undefined },
  { key: "project-a", scope: "project" as const, path: "~/.claude", projectPath: "~/project-a" },
  { key: "design-system", scope: "project" as const, path: "~/.claude", projectPath: "~/design-system" },
  { key: "my-app", scope: "project" as const, path: "~/.claude", projectPath: "~/my-app" },
];

export function demoTargets(assetType: AssetType): RegisteredMountTarget[] {
  const providers: RuntimeProvider[] = assetType === "command"
    ? ["claude_code"]
    : ["claude_code", "codex"];
  return locations.flatMap((location) => providers.map((provider) => ({
    id: `demo-${assetType}-${provider === "claude_code" ? "claude" : "codex"}-${location.key}`,
    kind: targetKind(assetType, provider, location.scope),
    provider,
    accepts: [assetType],
    adapter: assetType === "mcp"
      ? provider === "codex" ? "toml_mcp_patch" : "json_mcp_patch"
      : assetType === "skill" ? "symlink_directory" : "symlink_file",
    scope: location.scope,
    path: targetPath(assetType, provider, location),
    ...(location.projectPath ? { projectPath: location.projectPath } : {}),
    providerState: "initialized",
    status: location.key === "my-app" ? "ready" : "ready",
  } satisfies RegisteredMountTarget)));
}

export function demoBindings(assetId: string, assetType: AssetType): MountBinding[] {
  const targets = demoTargets(assetType);
  const mounted = targets.filter((target) => {
    if (target.scope === "user") return true;
    if (target.projectPath?.endsWith("project-a")) return target.provider === "claude_code";
    if (target.projectPath?.endsWith("design-system")) return target.provider === "codex";
    return false;
  });
  return mounted.map((target, index) => ({
    id: `demo-binding-${assetId}-${target.id}`,
    assetId,
    targetId: target.id,
    status: index === mounted.length - 1 && target.projectPath?.endsWith("design-system")
      ? "out_of_sync"
      : "mounted",
    lastSyncedAt: index === 0 ? "今天 10:24" : "今天 10:22",
  }));
}

function targetKind(
  assetType: AssetType,
  provider: RuntimeProvider,
  scope: "user" | "project",
): RegisteredMountTarget["kind"] {
  if (assetType === "command") return scope === "user" ? "claude_user_commands" : "claude_project_commands";
  if (assetType === "mcp") {
    if (provider === "codex") return scope === "user" ? "codex_user_mcp_toml" : "codex_project_mcp_toml";
    return scope === "user" ? "claude_user_mcp_json" : "claude_project_mcp_json";
  }
  if (provider === "codex") return scope === "user" ? "codex_user_skills" : "codex_project_skills";
  return scope === "user" ? "claude_user_skills" : "claude_project_skills";
}

function targetPath(
  assetType: AssetType,
  provider: RuntimeProvider,
  location: (typeof locations)[number],
) {
  const root = location.projectPath ?? "~";
  if (assetType === "mcp") return provider === "codex" ? `${root}/.codex/config.toml` : `${root}/.claude.json`;
  if (assetType === "command") return `${root}/.claude/commands`;
  return provider === "codex" ? `${root}/.agents/skills` : `${root}/.claude/skills`;
}
