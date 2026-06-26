import type { AppInfo } from "./contracts";
import type { AssetDetailContext, ProjectDetailContext } from "./detail-context";
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
  onOpenAssetDetail?: (detail: AssetDetailContext) => void;
  onOpenProjectDetail?: (detail: ProjectDetailContext) => void;
  onPageChange?: (page: PageId) => void;
};

function assertNever(value: never): never {
  throw new Error(`Unhandled page: ${String(value)}`);
}

export function CurrentPage({
  activePage,
  appInfo,
  assetDetail,
  projectDetail,
  onOpenAssetDetail,
  onOpenProjectDetail,
  onPageChange,
}: CurrentPageProps) {
  switch (activePage) {
    case "dashboard": return <DashboardPage appInfo={appInfo} />;
    case "skills": return <SkillsListPage onOpenAssetDetail={onOpenAssetDetail} />;
    case "commands": return <CommandsListPage onOpenAssetDetail={onOpenAssetDetail} />;
    case "mcp": return <McpServersListPage onOpenAssetDetail={onOpenAssetDetail} />;
    case "asset-detail": return <AssetDetailPage detail={assetDetail ?? undefined} />;
    case "projects": return <ProjectsListPage onOpenProjectDetail={onOpenProjectDetail} />;
    case "project-detail": return <ProjectDetailPage detail={projectDetail ?? undefined} />;
    case "scan": return <ScanImportPage />;
    case "mounts": return <MountManagerPage />;
    case "conflicts": return <ConflictResolverPage />;
    case "backups": return <BackupRestorePage />;
    case "sync": return <SyncPage />;
    case "settings": return <SettingsPage />;
  }

  return assertNever(activePage);
}
