import { FolderKanban, Plus, Search, SlidersHorizontal, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  listProjects,
  projectRemoveApply,
  projectRemovePreview,
  projectSaveApply,
  projectSavePreview,
} from "../app/data-api";
import type {
  ApplyResult,
  ProjectChangePreview,
  ProjectRemoveRequest,
  ProjectSaveRequest,
  ProjectSummary,
} from "../app/contracts";
import type { ProjectDetailContext } from "../app/detail-context";
import { InspectorFields, InspectorSection, InspectorTags } from "../components/assets/AssetCenterLayout";
import { ApplyConfirmationPanel } from "../components/ui/ApplyConfirmationPanel";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";
import { staticProjects, type StaticProject } from "./project-data";

const projectTone = { "正常": "success", "有变更": "warning", "待同步": "neutral", "无效": "warning" } as const;

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
  const [refreshKey, setRefreshKey] = useState(0);
  const [editor, setEditor] = useState<ProjectSaveRequest | null>(null);
  const [savePreview, setSavePreview] = useState<ProjectChangePreview | null>(null);
  const [removePreview, setRemovePreview] = useState<ProjectChangePreview | null>(null);
  const [isApplying, setIsApplying] = useState(false);
  const [operationMessage, setOperationMessage] = useState<string | null>(null);

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
        setStateLabel(mapped.length > 0 ? "只读真实数据" : "未发现本地项目");
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
  }, [demoMode, refreshKey]);

  const visibleProjects = useMemo(() => projects.filter((project) => {
    const matchesStatus = status === "all" || project.status === status;
    const searchable = `${project.name} ${project.title} ${project.path} ${project.description}`.toLocaleLowerCase();
    return matchesStatus && searchable.includes(query.trim().toLocaleLowerCase());
  }), [projects, query, status]);
  const selected = visibleProjects.find((project) => project.id === selectedId) ?? visibleProjects[0];

  const openCreate = () => {
    setEditor({ name: "", title: "", path: "", description: "" });
    setSavePreview(null);
    setRemovePreview(null);
    setOperationMessage(null);
  };

  const openEdit = (project: StaticProject) => {
    setEditor({ id: project.id, name: project.name, title: project.title, path: project.path, description: project.description });
    setSavePreview(null);
    setRemovePreview(null);
    setOperationMessage(null);
  };

  const previewSave = async () => {
    if (!editor) return;
    setOperationMessage(null);
    try {
      const preview = await projectSavePreview(editor);
      setSavePreview(preview);
    } catch (error) {
      setSavePreview(null);
      setOperationMessage(errorMessage(error));
    }
  };

  const applySave = async () => {
    if (!editor || !savePreview?.canApply) return;
    setIsApplying(true);
    try {
      await projectSaveApply({
        previewId: savePreview.previewId,
        previewGeneratedAtEpochSeconds: savePreview.generatedAtEpochSeconds,
        request: editor,
      });
      setEditor(null);
      setSavePreview(null);
      setOperationMessage("项目管理记录已更新；本地项目目录未被修改。");
      setRefreshKey((value) => value + 1);
    } catch (error) {
      setOperationMessage(errorMessage(error));
    } finally {
      setIsApplying(false);
    }
  };

  const previewRemove = async (project: StaticProject) => {
    const request: ProjectRemoveRequest = { id: project.id };
    setEditor(null);
    setOperationMessage(null);
    try {
      setRemovePreview(await projectRemovePreview(request));
    } catch (error) {
      setRemovePreview(null);
      setOperationMessage(errorMessage(error));
    }
  };

  const applyRemove = async () => {
    if (!removePreview?.canApply || !removePreview.project) return;
    setIsApplying(true);
    try {
      await projectRemoveApply({
        previewId: removePreview.previewId,
        previewGeneratedAtEpochSeconds: removePreview.generatedAtEpochSeconds,
        request: { id: removePreview.project.id },
      });
      setRemovePreview(null);
      setOperationMessage("项目管理记录已移除；本地项目目录未被删除。");
      setRefreshKey((value) => value + 1);
    } catch (error) {
      setOperationMessage(errorMessage(error));
    } finally {
      setIsApplying(false);
    }
  };

  return (
    <div className="project-center-layout">
      <section className="panel project-browser" aria-label="项目列表">
        <div className="asset-toolbar">
          <label className="asset-search-field"><Search size={15} /><input aria-label="搜索项目" data-no-drag="true" onChange={(event) => setQuery(event.target.value)} placeholder="搜索项目名称、标题或路径" style={NO_DRAG_REGION_STYLE} type="search" value={query} /></label>
          <label className="asset-filter-field"><SlidersHorizontal size={14} /><select aria-label="项目状态筛选" data-no-drag="true" onChange={(event) => setStatus(event.target.value)} style={NO_DRAG_REGION_STYLE} value={status}><option value="all">全部状态</option><option value="正常">正常</option><option value="有变更">有变更</option><option value="待同步">待同步</option><option value="无效">无效</option></select></label>
          {!demoMode ? <button className="asset-business-action" data-no-drag="true" onClick={openCreate} style={NO_DRAG_REGION_STYLE} type="button"><Plus size={14} />添加项目</button> : null}
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
          {visibleProjects.length === 0 && <div className="asset-empty-state"><Search size={22} /><strong>{projects.length === 0 ? "尚未维护项目" : "没有匹配的项目"}</strong><span>{projects.length === 0 ? "添加一个已有本地目录后，才会在这里显示和参与项目扫描。" : "调整搜索关键词或状态筛选。"}</span></div>}
        </div>
      </section>

      <aside className="panel project-inspector" aria-label="项目检查器">
        {selected ? <>
          <div className="project-inspector-header"><div><small>{selected.title}</small><h2>{selected.name}</h2></div><span className={`asset-status ${projectTone[selected.status]}`}>{selected.status}</span></div>
          <div className="project-inspector-content">
            <p className="asset-inspector-summary">{selected.description}</p>
            <div className="project-metrics"><div><strong>{selected.assets}</strong><span>全部资产</span></div><div><strong>{selected.skills}</strong><span>Skills</span></div><div><strong>{selected.commands}</strong><span>Commands</span></div><div><strong>{selected.mcps}</strong><span>MCP</span></div></div>
            <InspectorFields fields={[{ label: "项目路径", value: selected.path }, { label: "最近更新", value: selected.updated }]} />
            <InspectorSection title="当前挂载"><InspectorTags tags={selected.mounts} /></InspectorSection>
          </div>
          <div className="asset-inspector-actions">
            {onOpenProjectDetail ? <button className="asset-secondary-action" data-no-drag="true" onClick={() => onOpenProjectDetail(selected)} style={NO_DRAG_REGION_STYLE} type="button">查看详情</button> : null}
            {!demoMode ? <button className="asset-secondary-action" data-no-drag="true" onClick={() => openEdit(selected)} style={NO_DRAG_REGION_STYLE} type="button">编辑管理信息</button> : null}
            {!demoMode ? <button className="asset-danger-action" data-no-drag="true" onClick={() => void previewRemove(selected)} style={NO_DRAG_REGION_STYLE} type="button"><Trash2 size={14} />移除管理</button> : null}
          </div>
        </> : <div className="asset-inspector-empty"><strong>暂无可检查项目</strong><span>调整筛选后选择一个项目。</span></div>}
      </aside>
      {editor ? <section className="panel project-management-panel" aria-label="项目管理"><div className="section-heading"><div><h3>{editor.id ? "编辑项目管理信息" : "添加已有项目"}</h3><p>仅保存本机管理记录；不会创建、移动或删除项目目录。</p></div></div><div className="settings-controls two"><label><span>显示名称</span><input data-no-drag="true" onChange={(event) => setEditor({ ...editor, name: event.target.value })} style={NO_DRAG_REGION_STYLE} value={editor.name} /></label><label><span>标题</span><input data-no-drag="true" onChange={(event) => setEditor({ ...editor, title: event.target.value })} style={NO_DRAG_REGION_STYLE} value={editor.title} /></label><label className="settings-control-wide"><span>已有本地目录</span><input data-no-drag="true" onChange={(event) => setEditor({ ...editor, path: event.target.value })} placeholder="/Users/name/workspace/project" style={NO_DRAG_REGION_STYLE} value={editor.path} /></label><label className="settings-control-wide"><span>说明</span><input data-no-drag="true" onChange={(event) => setEditor({ ...editor, description: event.target.value })} style={NO_DRAG_REGION_STYLE} value={editor.description} /></label></div><div className="operation-actions"><button className="asset-secondary-action" data-no-drag="true" onClick={() => setEditor(null)} style={NO_DRAG_REGION_STYLE} type="button">取消</button><button className="asset-secondary-action" data-no-drag="true" onClick={() => void previewSave()} style={NO_DRAG_REGION_STYLE} type="button">生成保存预览</button></div>{savePreview ? <div className="plan-lines">{savePreview.migratedTargetIds.map((id) => <span key={id}>将迁移未绑定 Target：{id}</span>)}{savePreview.blockingBindings.map((id) => <span className="warning-text" key={id}>阻断绑定：{id}</span>)}{savePreview.warnings.map((warning) => <span className="warning-text" key={warning}>{warning}</span>)}</div> : null}<ApplyConfirmationPanel actionLabel="确认保存项目" canApply={Boolean(savePreview?.canApply)} description="后端会重新校验目录、项目 registry 和 Target binding；不会修改项目目录本身。" isApplying={isApplying} onApply={() => void applySave()} operationError={savePreview?.canApply ? null : operationMessage} result={null} title="保存项目管理记录" /></section> : null}
      {removePreview ? <section className="panel project-management-panel" aria-label="移除项目管理"><div className="section-heading"><div><h3>移除管理：{removePreview.project?.name}</h3><p>只删除本机项目管理记录和无绑定 Target，不删除项目目录或其中任何文件。</p></div></div><div className="plan-lines">{removePreview.migratedTargetIds.map((id) => <span key={id}>将移除无绑定 Target：{id}</span>)}{removePreview.blockingBindings.map((id) => <span className="warning-text" key={id}>必须先解除绑定：{id}</span>)}{removePreview.warnings.map((warning) => <span className="warning-text" key={warning}>{warning}</span>)}</div><div className="operation-actions"><button className="asset-secondary-action" data-no-drag="true" onClick={() => setRemovePreview(null)} style={NO_DRAG_REGION_STYLE} type="button">取消</button></div><ApplyConfirmationPanel actionLabel="确认移除管理" canApply={removePreview.canApply} description="项目目录和未管理的 runtime 配置会保留在本机。" isApplying={isApplying} onApply={() => void applyRemove()} operationError={removePreview.canApply ? null : operationMessage} result={null} title="移除项目管理记录" /></section> : null}
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
    status: project.status === "changed" ? "有变更" : project.status === "needsSync" ? "待同步" : project.status === "invalid" ? "无效" : "正常",
    assets: project.assetCounts.total,
    skills: project.assetCounts.skills,
    commands: project.assetCounts.commands,
    mcps: project.assetCounts.mcps,
    updated: project.updatedAt ?? "未知",
    description: project.description,
    mounts: project.mounts,
  };
}
