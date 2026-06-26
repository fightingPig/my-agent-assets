import { FolderKanban, Search, SlidersHorizontal } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { listProjects } from "../app/data-api";
import type { ProjectSummary } from "../app/contracts";
import type { ProjectDetailContext } from "../app/detail-context";
import { InspectorFields, InspectorSection, InspectorTags } from "../components/assets/AssetCenterLayout";
import { StaticActionButton } from "../components/ui/StaticActionButton";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";
import { staticProjects, type StaticProject } from "./project-data";

const projectTone = { "正常": "success", "有变更": "warning", "待同步": "neutral" } as const;

type ProjectsListPageProps = {
  onOpenProjectDetail?: (detail: ProjectDetailContext) => void;
};

export function ProjectsListPage({ onOpenProjectDetail }: ProjectsListPageProps = {}) {
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState("all");
  const [projects, setProjects] = useState<readonly StaticProject[]>(staticProjects);
  const [stateLabel, setStateLabel] = useState("读取中");
  const [selectedId, setSelectedId] = useState(staticProjects[0].id);

  useEffect(() => {
    let cancelled = false;
    setStateLabel("读取中");
    listProjects()
      .then((loaded) => {
        if (cancelled) return;
        if (Array.isArray(loaded) && loaded.length > 0) {
          const mapped = loaded.map(toStaticProject);
          setProjects(mapped);
          setSelectedId(mapped[0]?.id ?? "");
          setStateLabel("只读真实数据");
        } else {
          setProjects(staticProjects);
          setSelectedId(staticProjects[0].id);
          setStateLabel("静态预览");
        }
      })
      .catch(() => {
        if (cancelled) return;
        setProjects(staticProjects);
        setSelectedId(staticProjects[0].id);
        setStateLabel("读取失败，使用静态预览");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const visibleProjects = useMemo(() => projects.filter((project) => {
    const matchesStatus = status === "all" || project.status === status;
    const searchable = `${project.name} ${project.title} ${project.path} ${project.description}`.toLocaleLowerCase();
    return matchesStatus && searchable.includes(query.trim().toLocaleLowerCase());
  }), [projects, query, status]);
  const selected = visibleProjects.find((project) => project.id === selectedId) ?? visibleProjects[0];

  return (
    <div className="project-center-layout">
      <section className="panel project-browser" aria-label="项目列表">
        <div className="asset-toolbar">
          <label className="asset-search-field"><Search size={15} /><input aria-label="搜索项目" data-no-drag="true" onChange={(event) => setQuery(event.target.value)} placeholder="搜索项目名称、标题或路径" style={NO_DRAG_REGION_STYLE} type="search" value={query} /></label>
          <label className="asset-filter-field"><SlidersHorizontal size={14} /><select aria-label="项目状态筛选" data-no-drag="true" onChange={(event) => setStatus(event.target.value)} style={NO_DRAG_REGION_STYLE} value={status}><option value="all">全部状态</option><option value="正常">正常</option><option value="有变更">有变更</option><option value="待同步">待同步</option></select></label>
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
          {visibleProjects.length === 0 && <div className="asset-empty-state"><Search size={22} /><strong>没有匹配的项目</strong><span>调整搜索关键词或状态筛选。</span></div>}
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
          <div className="asset-inspector-actions">{onOpenProjectDetail ? <button className="asset-secondary-action" data-no-drag="true" onClick={() => onOpenProjectDetail(selected)} style={NO_DRAG_REGION_STYLE} type="button">查看详情</button> : <StaticActionButton className="asset-secondary-action">扫描项目</StaticActionButton>}<StaticActionButton className="asset-business-action">管理挂载</StaticActionButton></div>
        </> : <div className="asset-inspector-empty"><strong>暂无可检查项目</strong><span>调整筛选后选择一个项目。</span></div>}
      </aside>
    </div>
  );
}

function toStaticProject(project: ProjectSummary): StaticProject {
  return {
    id: project.id,
    name: project.name,
    title: project.title,
    path: project.path,
    status: project.status === "changed" ? "有变更" : project.status === "needsSync" ? "待同步" : "正常",
    assets: project.assetCounts.total,
    skills: project.assetCounts.skills,
    commands: project.assetCounts.commands,
    mcps: project.assetCounts.mcps,
    updated: project.updatedAt ?? "未知",
    description: project.description,
    mounts: project.mounts,
  };
}
