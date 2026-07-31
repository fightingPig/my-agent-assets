import type {
  SupportedMountProvider,
  UiAssetType,
  UiMountBinding,
  UiMountTarget,
} from "../contracts";

export type MountLocationRow = {
  id: string;
  label: string;
  detail: string;
  scope: UiMountTarget["scope"];
  status: "ready" | "warning" | "blocked";
  providers: Partial<Record<SupportedMountProvider, UiMountTarget>>;
};

export type MountCellState = "mounted" | "out_of_sync" | "unmounted" | "blocked";

export function supportsProvider(assetType: UiAssetType, provider: SupportedMountProvider) {
  return assetType !== "command" || provider === "claude_code";
}

export function toMountLocationRows(
  targets: readonly UiMountTarget[],
  assetType: UiAssetType,
): MountLocationRow[] {
  const compatible = targets.filter((target) =>
    target.accepts.includes(assetType) &&
    (target.provider === "claude_code" || target.provider === "codex")
  );
  const byLocation = new Map<string, MountLocationRow>();

  for (const target of compatible) {
    const provider = target.provider as SupportedMountProvider;
    if (!supportsProvider(assetType, provider)) continue;
    const key = locationKey(target);
    const current = byLocation.get(key) ?? {
      id: key,
      label: locationLabel(target),
      detail: locationDetail(target),
      scope: target.scope,
      status: target.status === "ready" ? "ready" : "blocked",
      providers: {},
    } satisfies MountLocationRow;
    current.providers[provider] = target;
    if (target.status !== "ready") current.status = "blocked";
    byLocation.set(key, current);
  }

  const rows = [...byLocation.values()];
  return [
    ...rows.filter((row) => row.scope === "user"),
    ...rows.filter((row) => row.scope !== "user"),
  ];
}

export function mountCellState(
  assetId: string,
  target: UiMountTarget | undefined,
  bindings: readonly UiMountBinding[],
): MountCellState {
  if (!target || target.status !== "ready") return "blocked";
  const binding = bindings.find((candidate) =>
    candidate.assetId === assetId && candidate.targetId === target.id
  );
  if (!binding) return "unmounted";
  return binding.status === "mounted" ? "mounted" : "out_of_sync";
}

export function locationLabel(target: UiMountTarget) {
  if (target.scope === "user") return "用户级";
  if (target.scope === "local") return "Claude Local";
  if (target.scope === "custom") return customLocationName(target.path);
  return projectName(target.projectPath ?? target.path);
}

export function locationDetail(target: UiMountTarget) {
  if (target.scope === "user") return "用户级全局作用域";
  return target.projectPath ?? target.path;
}

function locationKey(target: UiMountTarget) {
  if (target.scope === "user") return "user";
  if (target.scope === "local") return `local:${target.projectPath ?? target.path}`;
  if (target.scope === "project") return `project:${target.projectPath ?? target.path}`;
  return `custom:${target.projectPath ?? target.path}`;
}

function projectName(path: string) {
  const normalized = path.replace(/[\\/]+$/, "");
  return normalized.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
}

function customLocationName(path: string) {
  const name = projectName(path);
  return name || "自定义位置";
}
