import { FolderKanban, Pencil, Plus, RefreshCw, Search, SlidersHorizontal, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  inspectProjects,
  listProjects,
  projectAddApply,
  projectAddPreview,
  projectEditApply,
  projectEditPreview,
  projectRemoveApply,
  projectRemovePreview,
} from "../app/data-api";
import type {
  ProjectAddPreviewRequest,
  ProjectChangePreview,
  ProjectChangeResult,
  ProjectEditPreviewRequest,
  ProjectRemovePreviewRequest,
  ProjectSummary,
} from "../app/contracts";
import type { ProjectDetailContext } from "../app/detail-context";
import { InspectorFields, InspectorSection, InspectorTags } from "../components/assets/AssetCenterLayout";
import { isTauriRuntime, NO_DRAG_REGION_STYLE } from "../lib/platform";
import { staticProjects, type StaticProject } from "./project-data";

const projectTone = {
  "正常": "success",
  "未检查": "neutral",
  "需处理": "warning",
  "路径不可用": "danger",
} as const;

type ProjectOperation = "add" | "edit" | "remove";
type ProjectRequest = ProjectAddPreviewRequest | ProjectEditPreviewRequest | ProjectRemovePreviewRequest;

type ProjectsListPageProps = {
  demoMode?: boolean;
  onOpenProjectDetail?: (detail: ProjectDetailContext) => void;
};

export function ProjectsListPage({ demoMode = false, onOpenProjectDetail }: ProjectsListPageProps = {}) {
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState("all");
  const [projects, setProjects] = useState<readonly StaticProject[]>(demoMode ? staticProjects : []);
  const [stateLabel, setStateLabel] = useState("读取中");
  const [selectedId, setSelectedId] = useState(demoMode ? staticProjects[0].id : "");
  const [projectPreview, setProjectPreview] = useState<ProjectChangePreview | null>(null);
  const [projectOperation, setProjectOperation] = useState<ProjectOperation | null>(null);
  const [projectRequest, setProjectRequest] = useState<ProjectRequest | null>(null);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [operationResult, setOperationResult] = useState<ProjectChangeResult | null>(null);
  const [isApplying, setIsApplying] = useState(false);

  useEffect(() => {
    let cancelled = false;
    if (demoMode) {
      setProjects(staticProjects);
      setSelectedId(staticProjects[0].id);
      setStateLabel("Visual QA 示例数据");
      return undefined;
    }
    setProjects([]);
    setSelectedId("");
    setStateLabel("读取中");
    listProjects()
      .then((loaded) => {
        if (cancelled) return;
        const mapped = loaded.map(toStaticProject);
        setProjects(mapped);
        setSelectedId(mapped[0]?.id ?? "");
        setStateLabel(mapped.length > 0 ? "已维护项目" : "尚未添加项目");
      })
      .catch((error) => {
        if (cancelled) return;
        setProjects([]);
        setSelectedId("");
        setStateLabel(`读取失败：${errorMessage(error)}`);
      });
    return () => {
      cancelled = true;
    };
  }, [demoMode]);

  const visibleProjects = useMemo(() => projects.filter((project) => {
    const matchesStatus = status === "all" || project.status === status;
    const searchable = `${project.name} ${project.title} ${project.path} ${project.description}`.toLocaleLowerCase();
    return matchesStatus && searchable.includes(query.trim().toLocaleLowerCase());
  }), [projects, query, status]);
  const selected = visibleProjects.find((project) => project.id === selectedId) ?? visibleProjects[0];

  const reloadProjects = async (preferredId = selected?.id) => {
    const loaded = await listProjects();
    const mapped = loaded.map(toStaticProject);
    setProjects(mapped);
    setSelectedId(mapped.find((project) => project.id === preferredId)?.id ?? mapped[0]?.id ?? "");
    setStateLabel(mapped.length > 0 ? "已维护项目" : "尚未添加项目");
  };

  const chooseProjectDirectory = async (): Promise<string | null> => {
    if (!isTauriRuntime()) {
      setOperationError("项目目录只能在桌面应用中通过系统文件夹选择器添加。");
      return null;
    }
    const selectedPath = await open({ directory: true, multiple: false, title: "选择要维护的项目目录" });
    return typeof selectedPath === "string" ? selectedPath : null;
  };

  const requestAdd = async () => {
    setOperationError(null);
    setOperationResult(null);
    try {
      const path = await chooseProjectDirectory();
      if (!path) return;
      const request = { path } satisfies ProjectAddPreviewRequest;
      setProjectPreview(await projectAddPreview(request));
      setProjectRequest(request);
      setProjectOperation("add");
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  };

  const requestEdit = async () => {
    if (!selected) return;
    setOperationError(null);
    setOperationResult(null);
    try {
      const name = window.prompt("项目显示名称", selected.name)?.trim();
      if (!name) return;
      const request = { projectId: selected.id, name } satisfies ProjectEditPreviewRequest;
      setProjectPreview(await projectEditPreview(request));
      setProjectRequest(request);
      setProjectOperation("edit");
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  };

  const requestPathEdit = async () => {
    if (!selected) return;
    setOperationError(null);
    setOperationResult(null);
    try {
      const path = await chooseProjectDirectory();
      if (!path) return;
      const request = { projectId: selected.id, path } satisfies ProjectEditPreviewRequest;
      setProjectPreview(await projectEditPreview(request));
      setProjectRequest(request);
      setProjectOperation("edit");
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  };

  const requestRemove = async () => {
    if (!selected) return;
    setOperationError(null);
    setOperationResult(null);
    try {
      const request = { projectId: selected.id } satisfies ProjectRemovePreviewRequest;
      setProjectPreview(await projectRemovePreview(request));
      setProjectRequest(request);
      setProjectOperation("remove");
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  };

  const applyPreview = async () => {
    if (!projectPreview || !projectOperation || !projectRequest) return;
    setIsApplying(true);
    setOperationError(null);
    try {
      const envelope = {
        previewId: projectPreview.previewId,
        previewGeneratedAtEpochSeconds: projectPreview.generatedAtEpochSeconds,
      };
      const result = projectOperation === "add"
        ? await projectAddApply({ ...envelope, request: projectRequest as ProjectAddPreviewRequest })
        : projectOperation === "edit"
          ? await projectEditApply({ ...envelope, request: projectRequest as ProjectEditPreviewRequest })
          : await projectRemoveApply({ ...envelope, request: projectRequest as ProjectRemovePreviewRequest });
      setOperationResult(result);
      setProjectPreview(null);
      setProjectOperation(null);
      setProjectRequest(null);
      await reloadProjects(result.project.id);
    } catch (error) {
      setOperationError(errorMessage(error));
    } finally {
      setIsApplying(false);
    }
  };

  const refreshProjects = async (projectIds: string[]) => {
    setOperationError(null);
    try {
      const refreshed = await inspectProjects({ projectIds });
      await reloadProjects(projectIds[0] ?? refreshed[0]?.project.id);
      setStateLabel(projectIds.length === 0 ? "已刷新全部维护项目" : "已刷新当前项目");
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  };

  return (
    <div className="project-page-stack">
      <div className="project-page-actions">
        <button className="primary-button" data-no-drag="true" onClick={requestAdd} style={NO_DRAG_REGION_STYLE} type="button"><Plus size={16} />添加项目</button>
        <button className="asset-secondary-action" data-no-drag="true" disabled={projects.length === 0} onClick={() => void refreshProjects([])} style={NO_DRAG_REGION_STYLE} type="button"><RefreshCw size={15} />刷新全部</button>
      </div>
      {operationError ? <div className="operation-inline-error" role="status">{operationError}</div> : null}
      {operationResult ? <div className="operation-inline-success" role="status">项目{operationResult.operation === "add" ? "已添加" : operationResult.operation === "edit" ? "已更新" : "已移除"}。项目目录未被删除。</div> : null}
      <div className="project-center-layout">
      <section className="panel project-browser" aria-label="项目列表">
        <div className="asset-toolbar">
          <label className="asset-search-field"><Search size={15} /><input aria-label="搜索项目" data-no-drag="true" onChange={(event) => setQuery(event.target.value)} placeholder="搜索项目名称、标题或路径" style={NO_DRAG_REGION_STYLE} type="search" value={query} /></label>
          <label className="asset-filter-field"><SlidersHorizontal size={14} /><select aria-label="项目状态筛选" data-no-drag="true" onChange={(event) => setStatus(event.target.value)} style={NO_DRAG_REGION_STYLE} value={status}><option value="all">全部状态</option><option value="正常">正常</option><option value="未检查">未检查</option><option value="需处理">需处理</option><option value="路径不可用">路径不可用</option></select></label>
        </div>
        <div className="asset-list-heading"><span>本机项目</span><small>{visibleProjects.length} / {projects.length} · {stateLabel}</small></div>
        <div className="project-list-dense" role="listbox" aria-label="项目选择">
          {visibleProjects.map((project) => (
            <button aria-label={project.name} aria-selected={selected?.id === project.id} className={`project-list-row ${selected?.id === project.id ? "selected" : ""}`} data-no-drag="true" key={project.id} onClick={() => setSelectedId(project.id)} role="option" style={NO_DRAG_REGION_STYLE} type="button">
              <span className="project-row-icon"><FolderKanban size={18} /></span>
              <span className="project-row-copy"><strong>{project.name}</strong><small>{project.title}</small><span>{project.path} · {project.updated}</span></span>
              <span className="project-asset-count">{project.assets} 项资产</span>
              <span className={`asset-status ${projectTone[project.status]}`}>{project.status}</span>
            </button>
          ))}
          {visibleProjects.length === 0 && <div className="asset-empty-state"><Search size={22} /><strong>{projects.length === 0 ? "尚未添加维护项目" : "没有匹配的项目"}</strong><span>{projects.length === 0 ? "通过“添加项目”选择一个本机目录。应用不会自动收录工作区中的其他项目。" : "调整搜索关键词或状态筛选。"}</span></div>}
        </div>
      </section>

      <aside className="panel project-inspector" aria-label="项目检查器">
        {selected ? <>
          <div className="project-inspector-header"><div><small>{selected.title}</small><h2>{selected.name}</h2></div><span className={`asset-status ${projectTone[selected.status]}`}>{selected.status}</span></div>
          <div className="project-inspector-content">
            <p className="asset-inspector-summary">{selected.description}</p>
            <div className="project-metrics"><div><strong>{selected.assets}</strong><span>全部资产</span></div><div><strong>{selected.skills}</strong><span>Skills</span></div><div><strong>{selected.commands}</strong><span>Commands</span></div><div><strong>{selected.mcps}</strong><span>MCP</span></div></div>
            <InspectorFields fields={[{ label: "项目路径", value: selected.path }, { label: "最近检查", value: selected.lastCheckedAt ?? "尚未检查" }, { label: "扫描提示", value: `${selected.warningCount ?? 0} 项` }]} />
            <InspectorSection title="当前挂载"><InspectorTags tags={selected.mounts} /></InspectorSection>
          </div>
          <div className="asset-inspector-actions">
            <button className="asset-secondary-action" data-no-drag="true" onClick={() => void refreshProjects([selected.id])} style={NO_DRAG_REGION_STYLE} type="button"><RefreshCw size={14} />刷新当前</button>
            <button className="asset-secondary-action" data-no-drag="true" onClick={() => void requestEdit()} style={NO_DRAG_REGION_STYLE} type="button"><Pencil size={14} />改名称</button>
            <button className="asset-secondary-action" data-no-drag="true" onClick={() => void requestPathEdit()} style={NO_DRAG_REGION_STYLE} type="button">改路径</button>
            {onOpenProjectDetail ? <button className="asset-secondary-action" data-no-drag="true" onClick={() => onOpenProjectDetail(selected)} style={NO_DRAG_REGION_STYLE} type="button">查看详情</button> : null}
            <button className="asset-danger-action" data-no-drag="true" onClick={() => void requestRemove()} style={NO_DRAG_REGION_STYLE} type="button"><Trash2 size={14} />移除管理</button>
          </div>
        </> : <div className="asset-inspector-empty"><strong>暂无可检查项目</strong><span>调整筛选后选择一个项目。</span></div>}
      </aside>
      </div>
      {projectPreview ? <section className="panel project-change-preview"><div className="section-heading"><div><h3>{projectOperation === "add" ? "确认添加项目" : projectOperation === "edit" ? "确认更新项目" : "确认移除项目"}</h3><p>{projectOperation === "remove" ? "只会移除本地管理记录，不会删除项目目录或项目资产。" : "系统将在确认后更新本地项目管理记录。"}</p></div></div><div className="plan-lines"><span>项目：{projectPreview.project.name}</span><span>路径：{projectPreview.project.path}</span>{projectPreview.blockingBindings.map((binding) => <span className="warning-text" key={binding}>必须先卸载绑定：{binding}</span>)}{projectPreview.warnings.map((warning) => <span className="warning-text" key={warning}>{warning}</span>)}</div><div className="asset-inspector-actions"><button className="asset-secondary-action" data-no-drag="true" disabled={isApplying} onClick={() => { setProjectPreview(null); setProjectOperation(null); setProjectRequest(null); }} style={NO_DRAG_REGION_STYLE} type="button">取消</button><button className="asset-business-action" data-no-drag="true" disabled={!projectPreview.canApply || isApplying} onClick={() => void applyPreview()} style={NO_DRAG_REGION_STYLE} type="button">{isApplying ? "执行中" : projectOperation === "remove" ? "确认移除管理" : "确认执行"}</button></div></section> : null}
    </div>
  );
}

function errorMessage(_error: unknown) {
  return "本地项目读取未完成。请查看系统状态或导出诊断包后重试。";
}

function toStaticProject(project: ProjectSummary): StaticProject {
  return {
    id: project.id,
    name: project.name,
    title: project.title,
    path: project.path,
    status: project.status === "ready" ? "正常" : project.status === "unchecked" ? "未检查" : project.status === "missing_path" ? "路径不可用" : "需处理",
    assets: project.assetCounts.total,
    skills: project.assetCounts.skills,
    commands: project.assetCounts.commands,
    mcps: project.assetCounts.mcps,
    updated: project.updatedAt ?? "未知",
    description: project.description,
    mounts: project.mounts,
    lastCheckedAt: project.lastCheckedAt ?? undefined,
    warningCount: project.warningCount,
  };
}
