import type { AppInfo } from "./contracts";
import type {
  AssetDetailContext,
  ConflictResolverContext,
  ProjectDetailContext,
} from "./detail-context";
import type { PageId } from "./pages";
import type { AssetProvider } from "./provider";
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
  provider?: AssetProvider;
  demoMode?: boolean;
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
  provider = "claude",
  demoMode = false,
}: CurrentPageProps) {
  switch (activePage) {
    case "dashboard": return <DashboardPage appInfo={appInfo} demoMode={demoMode} />;
    case "skills": return <SkillsListPage demoMode={demoMode} onOpenAssetDetail={onOpenAssetDetail} provider={provider} />;
    case "commands": return <CommandsListPage demoMode={demoMode} onOpenAssetDetail={onOpenAssetDetail} />;
    case "mcp": return <McpServersListPage demoMode={demoMode} onOpenAssetDetail={onOpenAssetDetail} provider={provider} />;
    case "asset-detail": return <AssetDetailPage demoMode={demoMode} detail={assetDetail ?? undefined} />;
    case "projects": return <ProjectsListPage demoMode={demoMode} onOpenProjectDetail={onOpenProjectDetail} />;
    case "project-detail": return <ProjectDetailPage demoMode={demoMode} detail={projectDetail ?? undefined} />;
    case "scan": return <ScanImportPage demoMode={demoMode} onOpenConflicts={onOpenConflicts} />;
    case "mounts": return <MountManagerPage demoMode={demoMode} />;
    case "conflicts": return <ConflictResolverPage context={conflictContext ?? undefined} demoMode={demoMode} />;
    case "backups": return <BackupRestorePage demoMode={demoMode} />;
    case "sync": return <SyncPage demoMode={demoMode} />;
    case "settings": return <SettingsPage demoMode={demoMode} />;
  }

  return assertNever(activePage);
}
