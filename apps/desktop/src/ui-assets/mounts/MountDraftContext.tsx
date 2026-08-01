import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import type { MountCellState } from "./model";
import type { SupportedMountProvider, UiAssetType } from "../contracts";

export type MountDraftOperation = "mount" | "unmount";

export type MountDraft = {
  id: string;
  assetId: string;
  assetName: string;
  assetType: UiAssetType;
  targetId: string;
  targetLabel: string;
  targetPath: string;
  provider: SupportedMountProvider;
  operation: MountDraftOperation;
  previousState: MountCellState;
};

type MountDraftContextValue = {
  drafts: readonly MountDraft[];
  stageDraft: (draft: MountDraft) => void;
  removeDraft: (id: string) => void;
  clearDrafts: () => void;
};

const defaultMountDraftContext: MountDraftContextValue = {
  drafts: [],
  stageDraft: () => undefined,
  removeDraft: () => undefined,
  clearDrafts: () => undefined,
};

const MountDraftContext = createContext<MountDraftContextValue>(defaultMountDraftContext);

export function MountDraftProvider({
  children,
  initialDrafts = [],
}: {
  children: ReactNode;
  initialDrafts?: readonly MountDraft[];
}) {
  const [drafts, setDrafts] = useState<readonly MountDraft[]>(initialDrafts);

  const stageDraft = useCallback((draft: MountDraft) => {
    setDrafts((current) => {
      const existing = current.find((candidate) => candidate.id === draft.id);
      if (existing?.operation === draft.operation) {
        return current.filter((candidate) => candidate.id !== draft.id);
      }
      return [...current.filter((candidate) => candidate.id !== draft.id), draft];
    });
  }, []);

  const removeDraft = useCallback((id: string) => {
    setDrafts((current) => current.filter((candidate) => candidate.id !== id));
  }, []);

  const clearDrafts = useCallback(() => setDrafts([]), []);

  const value = useMemo(() => ({ drafts, stageDraft, removeDraft, clearDrafts }), [
    clearDrafts,
    drafts,
    removeDraft,
    stageDraft,
  ]);

  return <MountDraftContext.Provider value={value}>{children}</MountDraftContext.Provider>;
}

export function useMountDrafts() {
  return useContext(MountDraftContext);
}

export function demoMountDrafts(pageId: string): readonly MountDraft[] {
  const assetType: UiAssetType = pageId === "mcp" ? "mcp" : pageId === "commands" ? "command" : "skill";
  const assetName = assetType === "mcp" ? "PostgreSQL" : assetType === "command" ? "deploy-prod" : "review";
  const base: MountDraft = {
    id: `${assetName}:claude:project-a`,
    assetId: assetName,
    assetName,
    assetType,
    targetId: `demo-${assetType}-claude-project-a`,
    targetLabel: "project-a",
    targetPath: "~/project-a",
    provider: "claude_code",
    operation: "unmount",
    previousState: "mounted",
  };
  if (pageId === "mounts") {
    return [
      base,
      {
        ...base,
        id: `${assetName}:codex:design-system`,
        targetId: `demo-${assetType}-codex-design-system`,
        targetLabel: "design-system",
        targetPath: "~/design-system",
        provider: assetType === "command" ? "claude_code" : "codex",
        operation: "mount",
        previousState: "unmounted",
      },
    ];
  }
  return pageId === "skills" || pageId === "commands" || pageId === "mcp" ? [base] : [];
}
