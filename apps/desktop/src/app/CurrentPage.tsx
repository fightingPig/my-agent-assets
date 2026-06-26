import type { AppInfo } from "./contracts";
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
  onPageChange?: (page: PageId) => void;
};

function assertNever(value: never): never {
  throw new Error(`Unhandled page: ${String(value)}`);
}

export function CurrentPage({ activePage, appInfo, onPageChange }: CurrentPageProps) {
  switch (activePage) {
    case "dashboard": return <DashboardPage appInfo={appInfo} />;
    case "skills": return <SkillsListPage onPageChange={onPageChange} />;
    case "commands": return <CommandsListPage onPageChange={onPageChange} />;
    case "mcp": return <McpServersListPage onPageChange={onPageChange} />;
    case "asset-detail": return <AssetDetailPage />;
    case "projects": return <ProjectsListPage onPageChange={onPageChange} />;
    case "project-detail": return <ProjectDetailPage />;
    case "scan": return <ScanImportPage />;
    case "mounts": return <MountManagerPage />;
    case "conflicts": return <ConflictResolverPage />;
    case "backups": return <BackupRestorePage />;
    case "sync": return <SyncPage />;
    case "settings": return <SettingsPage />;
  }

  return assertNever(activePage);
}
