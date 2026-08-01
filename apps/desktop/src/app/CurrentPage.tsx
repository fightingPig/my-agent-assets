import type { AppInfo } from "./contracts";
import type {
  AssetDetailContext,
  ConflictResolverContext,
  ProjectDetailContext,
} from "./detail-context";
import type { PageId } from "./pages";
import { AssetDetailPage } from "../pages/AssetDetailPage";
import { BackupRestorePage } from "../pages/BackupRestorePage";
import { CommandsListPage } from "../pages/CommandsListPage";
import { ConflictResolverPage } from "../pages/ConflictResolverPage";
import { DashboardPage } from "../pages/DashboardPage";
import { McpServersListPage } from "../pages/McpServersListPage";
import { MountManagerPage } from "../pages/MountManagerPage";
import { ProjectDetailPage } from "../pages/ProjectDetailPage";
import { ProjectsListPage } from "../pages/ProjectsListPage";
import { ScanImportPage } from "../pages/ScanImportPage";
import { SettingsPage } from "../pages/SettingsPage";
import { SkillsListPage } from "../pages/SkillsListPage";
import { SyncPage } from "../pages/SyncPage";

type CurrentPageProps = {
  activePage: PageId;
  appInfo: AppInfo;
  assetDetail?: AssetDetailContext | null;
  projectDetail?: ProjectDetailContext | null;
  conflictContext?: ConflictResolverContext | null;
  onOpenAssetDetail?: (detail: AssetDetailContext) => void;
  onOpenProjectDetail?: (detail: ProjectDetailContext) => void;
  onOpenConflicts?: (context: ConflictResolverContext) => void;
  onPageChange?: (page: PageId) => void;
  demoMode?: boolean;
  visualQaState?: string;
};

function assertNever(value: never): never {
  throw new Error(`Unhandled page: ${String(value)}`);
}

export function CurrentPage({
  activePage,
  appInfo,
  assetDetail,
  projectDetail,
  conflictContext,
  onOpenAssetDetail,
  onOpenProjectDetail,
  onOpenConflicts,
  onPageChange,
  demoMode = false,
  visualQaState,
}: CurrentPageProps) {
  switch (activePage) {
    case "dashboard": return <DashboardPage appInfo={appInfo} demoMode={demoMode} onPageChange={onPageChange} visualQaState={visualQaState} />;
    case "skills": return <SkillsListPage demoMode={demoMode} onOpenAssetDetail={onOpenAssetDetail} onOpenMountPreview={() => onPageChange?.("mounts")} />;
    case "commands": return <CommandsListPage demoMode={demoMode} onOpenAssetDetail={onOpenAssetDetail} onOpenMountPreview={() => onPageChange?.("mounts")} />;
    case "mcp": return <McpServersListPage demoMode={demoMode} onOpenAssetDetail={onOpenAssetDetail} onOpenMountPreview={() => onPageChange?.("mounts")} />;
    case "asset-detail": return <AssetDetailPage demoMode={demoMode} detail={assetDetail ?? undefined} onPageChange={onPageChange} />;
    case "projects": return <ProjectsListPage demoMode={demoMode} onOpenProjectDetail={onOpenProjectDetail} visualQaState={visualQaState} />;
    case "project-detail": return <ProjectDetailPage demoMode={demoMode} detail={projectDetail ?? undefined} onPageChange={onPageChange} />;
    case "scan": return <ScanImportPage demoMode={demoMode} onOpenConflicts={onOpenConflicts} visualQaState={visualQaState} />;
    case "mounts": return <MountManagerPage demoMode={demoMode} />;
    case "conflicts": return <ConflictResolverPage context={conflictContext ?? undefined} demoMode={demoMode} />;
    case "backups": return <BackupRestorePage demoMode={demoMode} />;
    case "sync": return <SyncPage demoMode={demoMode} />;
    case "settings": return <SettingsPage appInfo={appInfo} demoMode={demoMode} visualQaState={visualQaState} />;
  }

  return assertNever(activePage);
}
