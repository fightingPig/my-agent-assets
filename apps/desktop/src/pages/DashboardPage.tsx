import {
  Activity,
  Blocks,
  BookOpen,
  CircleCheck,
  FolderKanban,
  ListChecks,
  TerminalSquare,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useState } from "react";
import { gitStatus, listAssets, listProjects, recoveryStatus } from "../app/data-api";
import type {
  AppInfo,
  AssetSummary,
  GitStatus,
  ProjectSummary,
  RecoveryStatus,
} from "../app/contracts";
import {
  projects as demoProjects,
  recentActivity as demoRecentActivity,
  stats as demoStats,
  systemChecks as demoSystemChecks,
} from "../mock-data";

type DashboardPageProps = {
  appInfo: AppInfo;
  demoMode?: boolean;
};

type DashboardStat = {
  label: string;
  value: number;
  change: string;
  icon: LucideIcon;
  tone: "green" | "blue" | "violet" | "amber";
};

const emptyGitStatus: GitStatus = {
  repositoryPath: "~/.my-agent-assets",
  isRepository: false,
  statusMessage: "尚未读取本地 Git 仓库。",
  branch: "",
  remoteName: "origin",
  clean: true,
  ahead: 0,
  behind: 0,
  changedFiles: [],
  conflicts: [],
  syncableChanges: [],
  blockedChanges: [],
  lastSyncedAt: null,
};

const healthyRecoveryStatus: RecoveryStatus = {
  writesBlocked: false,
  journals: [],
  recentRecoveries: [],
  message: "没有未完成事务。",
};

export function DashboardPage({ appInfo, demoMode = false }: DashboardPageProps) {
  const [assets, setAssets] = useState<readonly AssetSummary[]>([]);
  const [projects, setProjects] = useState<readonly ProjectSummary[]>([]);
  const [repository, setRepository] = useState<GitStatus>(emptyGitStatus);
  const [recovery, setRecovery] = useState<RecoveryStatus>(healthyRecoveryStatus);
  const [stateLabel, setStateLabel] = useState(demoMode ? "Visual QA 示例数据" : "读取中");

  useEffect(() => {
    if (demoMode) {
      setStateLabel("Visual QA 示例数据");
      return undefined;
    }

    let cancelled = false;
    setStateLabel("读取中");
    Promise.all([listAssets({ assetType: null }), listProjects(), gitStatus(), recoveryStatus()])
      .then(([loadedAssets, loadedProjects, loadedRepository, loadedRecovery]) => {
        if (cancelled) return;
        setAssets(loadedAssets);
        setProjects(loadedProjects);
        setRepository(loadedRepository);
        setRecovery(loadedRecovery);
        setStateLabel("只读真实数据");
      })
      .catch((error) => {
        if (cancelled) return;
        setAssets([]);
        setProjects([]);
        setRepository(emptyGitStatus);
        setRecovery(healthyRecoveryStatus);
        setStateLabel(`读取失败：${errorMessage(error)}`);
      });
    return () => {
      cancelled = true;
    };
  }, [demoMode]);

  const stats = demoMode ? demoStats : realStats(assets, projects);
  const visibleProjects = demoMode
    ? demoProjects
    : projects.slice(0, 3).map((project) => ({
      name: project.name,
      path: project.path,
      assets: project.assetCounts.total,
      state: project.status === "changed" ? "有变更" : project.status === "needsSync" ? "待同步" : "正常",
    }));
  const systemChecks = demoMode ? demoSystemChecks : [
    {
      label: "资产中心",
      detail: repository.repositoryPath,
      status: repository.isRepository ? "可用" : "未初始化",
    },
    {
      label: "Git",
      detail: repository.statusMessage,
      status: repository.isRepository ? (repository.clean ? "正常" : "有变更") : "未连接",
    },
    {
      label: "Claude Runtime",
      detail: `${assets.length} 项资产已读取`,
      status: appInfo.backendReady ? "已连接" : "未连接",
    },
    {
      label: "事务恢复",
      detail: recovery.message,
      status: recovery.writesBlocked ? "写入已阻止" : "正常",
    },
  ];

  return (
    <>
      <section className="stats-grid" aria-label="资产统计">
        {stats.map((stat) => {
          const Icon = stat.icon;
          return (
            <article className="stat-card" key={stat.label}>
              <div className={`icon-box ${stat.tone}`}><Icon size={20} /></div>
              <div className="stat-copy"><span>{stat.label}</span><strong>{stat.value}</strong><small>{stat.change}</small></div>
            </article>
          );
        })}
      </section>

      <div className="dashboard-grid">
        <section className="panel activity-panel">
          <div className="panel-header"><div><h2>最近活动</h2><p>资产中心的最新变更</p></div></div>
          <div className="activity-list">
            {(demoMode ? demoRecentActivity : []).map((item) => {
              const Icon = item.icon;
              return (
                <div className="activity-item" key={item.title}>
                  <div className={`activity-icon ${item.tone}`}><Icon size={16} /></div>
                  <div className="activity-copy"><strong>{item.title}</strong><span>{item.meta}</span></div>
                  <time>{item.time}</time>
                </div>
              );
            })}
            {!demoMode && (
              <div className="asset-empty-state">
                <Activity size={22} />
                <strong>暂无活动记录</strong>
                <span>完成扫描、导入或同步后，真实活动会显示在这里。</span>
              </div>
            )}
          </div>
        </section>

        <section className="panel projects-panel">
          <div className="panel-header"><div><h2>常用项目</h2><p>最近访问的运行目标</p></div></div>
          <div className="project-list">
            {visibleProjects.map((project) => (
              <div className="project-item" key={project.name}>
                <div className="project-folder"><FolderKanban size={18} /></div>
                <div className="project-copy"><strong>{project.name}</strong><span>{project.path}</span></div>
                <div className="project-meta"><span>{project.assets} 项资产</span><small className={project.state === "正常" ? "ok" : "pending"}>{project.state}</small></div>
              </div>
            ))}
            {visibleProjects.length === 0 && (
              <div className="asset-empty-state">
                <FolderKanban size={22} />
                <strong>未发现本地项目</strong>
                <span>项目扫描根目录下出现可识别项目后会显示在这里。</span>
              </div>
            )}
          </div>
        </section>

        <section className="panel health-panel">
          <div className="panel-header"><div><h2>系统状态</h2><p>{demoMode ? "Visual QA 示例环境" : "本机只读运行环境"}</p></div><span className="healthy-badge"><CircleCheck size={14} />{stateLabel}</span></div>
          <div className="check-grid">
            {systemChecks.map((check) => (
              <div className="check-item" key={check.label}>
                <div className="check-icon"><ListChecks size={17} /></div>
                <div><strong>{check.label}</strong><span>{check.detail}</span></div>
                <small>{check.status}</small>
              </div>
            ))}
          </div>
          <div className="backend-strip">
            <Activity size={15} />
            <span>Tauri 后端</span>
            <strong>{appInfo.backendReady ? "已连接" : "浏览器预览"}</strong>
            <code>{appInfo.platform} · {appInfo.arch} · v{appInfo.version}</code>
          </div>
        </section>
      </div>
    </>
  );
}

function realStats(assets: readonly AssetSummary[], projects: readonly ProjectSummary[]): DashboardStat[] {
  const count = (assetType: AssetSummary["assetType"]) =>
    assets.filter((asset) => asset.assetType === assetType).length;
  return [
    { label: "Skills", value: count("skill"), change: "本地真实数据", icon: BookOpen, tone: "green" },
    { label: "Commands", value: count("command"), change: "本地真实数据", icon: TerminalSquare, tone: "blue" },
    { label: "MCP Servers", value: count("mcp"), change: "本地真实数据", icon: Blocks, tone: "violet" },
    { label: "项目", value: projects.length, change: "扫描根目录", icon: FolderKanban, tone: "amber" },
  ];
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "无法读取本地概览数据。";
}
