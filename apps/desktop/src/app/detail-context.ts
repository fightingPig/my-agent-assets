import type { AssetStatusTone } from "../components/assets/AssetCenterLayout";
import type { AssetType } from "./contracts";
import type { StaticProject } from "../pages/project-data";

export type AssetDetailContext = {
  assetId: string;
  assetType: AssetType;
  name: string;
  title: string;
  summary: string;
  status: string;
  statusTone: AssetStatusTone;
  typeLabel: string;
  category: string;
  sourcePath: string;
  scope: string;
  updated: string;
  mountTargets: readonly string[];
  previewLabel: string;
  preview: string;
};

export type ProjectDetailContext = StaticProject;
