import {
  Activity,
  AlertTriangle,
  Blocks,
  BookOpen,
  CircleCheck,
  FolderKanban,
  ListChecks,
  TerminalSquare,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useState } from "react";
import {
  gitStatus,
  doctorReport,
  consistencyRepairApply,
  consistencyRepairPreview,
  initializationApply,
  initializationPreview,
  listAuditLog,
  listAssets,
  listBackups,
  listProjects,
  recoveryStatus,
  safeCommandErrorMessage,
} from "../app/data-api";
import type {
  AppInfo,
  AuditLogEntry,
  AssetSummary,
  ConsistencyRepairAction,
  ConsistencyRepairPreview,
  DoctorReport,
  GitStatus,
  InitializationPreview,
  ProjectSummary,
  RecoveryStatus,
} from "../app/contracts";
import type { PageId } from "../app/pages";
import { DEFAULT_ASSET_CENTER_PATH, DEFAULT_GIT_REMOTE_NAME } from "../app/defaults";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";
import { statusToneForLabel } from "../ui-assets";
import {
  projects as demoProjects,
  recentActivity as demoRecentActivity,
  stats as demoStats,
  systemChecks as demoSystemChecks,
} from "../mock-data";

type DashboardPageProps = {
  appInfo: AppInfo;
  demoMode?: boolean;
  onPageChange?: (page: PageId) => void;
  visualQaState?: string;
};

type DashboardStat = {
  label: string;
  value: number;
  change: string;
  icon: LucideIcon;
  tone: "green" | "blue" | "violet" | "amber";
};

const emptyGitStatus: GitStatus = {
  repositoryPath: DEFAULT_ASSET_CENTER_PATH,
  isRepository: false,
  statusMessage: "尚未读取本地 Git 仓库。",
  branch: "",
  remoteName: DEFAULT_GIT_REMOTE_NAME,
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

const unknownRecoveryStatus: RecoveryStatus = {
  writesBlocked: true,
  journals: [],
  recentRecoveries: [],
  message: "无法确认事务恢复状态；为安全起见请暂停写入并查看诊断。",
};

export function DashboardPage({ appInfo, demoMode = false, onPageChange, visualQaState }: DashboardPageProps) {
  const [assets, setAssets] = useState<readonly AssetSummary[]>([]);
  const [projects, setProjects] = useState<readonly ProjectSummary[]>([]);
  const [backupCount, setBackupCount] = useState(0);
  const [repository, setRepository] = useState<GitStatus>(emptyGitStatus);
  const [recovery, setRecovery] = useState<RecoveryStatus>(
    demoMode && visualQaState !== "recovery-error"
      ? healthyRecoveryStatus
      : unknownRecoveryStatus,
  );
  const [auditEntries, setAuditEntries] = useState<readonly AuditLogEntry[]>([]);
  const [doctor, setDoctor] = useState<DoctorReport | null>(null);
  const [repairPreview, setRepairPreview] = useState<ConsistencyRepairPreview | null>(null);
  const [repairMessage, setRepairMessage] = useState("");
  const [repairBusy, setRepairBusy] = useState(false);
  const [initialization, setInitialization] = useState<InitializationPreview | null>(null);
  const [showInitializationPreview, setShowInitializationPreview] = useState(false);
  const [initializationMessage, setInitializationMessage] = useState("");
  const [initializationBusy, setInitializationBusy] = useState(false);
  const [reloadKey, setReloadKey] = useState(0);
  const [stateLabel, setStateLabel] = useState(demoMode ? "Visual QA 示例数据" : "读取中");

  useEffect(() => {
    if (demoMode) {
      setStateLabel("Visual QA 示例数据");
      return undefined;
    }

    let cancelled = false;
    setStateLabel("读取中");
    Promise.allSettled([
      listAssets({ assetType: null }),
      listProjects(),
      listBackups(),
      gitStatus(),
      recoveryStatus(),
      listAuditLog(),
      initializationPreview(),
      doctorReport(),
    ])
      .then(([loadedAssets, loadedProjects, loadedBackups, loadedRepository, loadedRecovery, loadedAuditEntries, loadedInitialization, loadedDoctor]) => {
        if (cancelled) return;
        setAssets(settledValue(loadedAssets, []));
        setProjects(settledValue(loadedProjects, []));
        setBackupCount(settledValue(loadedBackups, []).length);
        setRepository(settledValue(loadedRepository, emptyGitStatus));
        setRecovery(settledValue(loadedRecovery, unknownRecoveryStatus));
        setAuditEntries(settledValue(loadedAuditEntries, []));
        setInitialization(settledValue(loadedInitialization, null));
        setDoctor(settledValue(loadedDoctor, null));
        const failedReads = [
          loadedAssets,
          loadedProjects,
          loadedBackups,
          loadedRepository,
          loadedRecovery,
          loadedAuditEntries,
          loadedInitialization,
          loadedDoctor,
        ].filter((result) => result.status === "rejected").length;
        setStateLabel(
          failedReads === 0 ? "只读真实数据" : `部分读取失败（${failedReads} 项）`,
        );
      });
    return () => {
      cancelled = true;
    };
  }, [demoMode, reloadKey]);

  const handleInitializationPreview = async () => {
    setInitializationBusy(true);
    setInitializationMessage("");
    try {
      const preview = await initializationPreview();
      setInitialization(preview);
      setShowInitializationPreview(true);
    } catch (error) {
      setInitializationMessage(`初始化预览失败：${errorMessage(error)}`);
    } finally {
      setInitializationBusy(false);
    }
  };

  const handleInitializationApply = async () => {
    if (!initialization?.canApply || initialization.alreadyInitialized) return;
    setInitializationBusy(true);
    setInitializationMessage("");
    try {
      const result = await initializationApply({
        previewId: initialization.previewId,
        previewGeneratedAtEpochSeconds: initialization.generatedAtEpochSeconds,
      });
      setInitializationMessage(
        result.created ? "资产中心初始化完成。" : "资产中心已初始化，无需重复创建。",
      );
      setShowInitializationPreview(false);
      setReloadKey((value) => value + 1);
    } catch (error) {
      setInitializationMessage(`初始化失败：${errorMessage(error)}`);
    } finally {
      setInitializationBusy(false);
    }
  };

  const handleRepairPreview = async (assetId: string, action: ConsistencyRepairAction) => {
    setRepairBusy(true);
    setRepairMessage("");
    try {
      setRepairPreview(await consistencyRepairPreview({ assetId, action }));
    } catch (error) {
      setRepairMessage(`一致性修复预览失败：${errorMessage(error)}`);
    } finally {
      setRepairBusy(false);
    }
  };

  const handleRepairApply = async () => {
    if (!repairPreview?.canApply) return;
    setRepairBusy(true);
    setRepairMessage("");
    try {
      const result = await consistencyRepairApply({
        previewId: repairPreview.previewId,
        previewGeneratedAtEpochSeconds: repairPreview.generatedAtEpochSeconds,
        request: repairPreview.request,
      });
      setRepairMessage(`一致性修复已完成：${result.assetId}`);
      setRepairPreview(null);
      setReloadKey((value) => value + 1);
    } catch (error) {
      setRepairMessage(`一致性修复失败：${errorMessage(error)}`);
    } finally {
      setRepairBusy(false);
    }
  };


  const stats = demoMode ? demoStats : realStats(assets, projects);
  const statPages: Record<string, PageId> = {
    Skills: "skills",
    Commands: "commands",
    "MCP Servers": "mcp",
    项目: "projects",
  };
  const visibleProjects = demoMode
    ? demoProjects
    : projects.slice(0, 3).map((project) => ({
      name: project.name,
      path: project.path,
      assets: project.assetCounts.total,
      state: project.status === "changed" ? "有变更" : project.status === "needsSync" ? "待同步" : "正常",
    }));
  const recentActivities = demoMode
    ? demoRecentActivity
    : auditEntries.slice(-5).reverse().map((entry) => ({
      title: auditOperationLabel(entry.operationType),
      meta: auditOutcomeLabel(entry.outcome),
      time: formatAuditTime(entry.occurredAtEpochSeconds),
      icon: entry.outcome === "completed" ? CircleCheck : AlertTriangle,
      tone: entry.outcome === "completed" ? "green" : "amber",
    }));
  const mountCount = assets.reduce((total, asset) => total + asset.mountTargets.length, 0);
  const conflictCount = assets.filter((asset) => asset.status === "conflict").length
    + (doctor?.contentDiagnostics.length ?? 0);
  const runtimeCheck = (id: "claude_runtime" | "codex_runtime", label: string) => {
    const check = doctor?.checks.find((item) => item.id === id);
    return {
      label,
      detail: check?.message ?? "未读取本机 Runtime 诊断。",
      status: check ? doctorStatusLabel(check.status) : "未读取",
    };
  };
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
    runtimeCheck("claude_runtime", "Claude Code Runtime"),
    runtimeCheck("codex_runtime", "Codex Runtime"),
    {
      label: "挂载关系",
      detail: `${mountCount} 条本机 asset-to-target binding`,
      status: mountCount > 0 ? "已挂载" : "暂无",
    },
    {
      label: "备份历史",
      detail: `${backupCount} 份 portable / local backup`,
      status: backupCount > 0 ? "可查看" : "暂无",
    },
    {
      label: "事务恢复",
      detail: recovery.message,
      status: recovery.writesBlocked ? "写入已阻止" : "正常",
    },
    {
      label: "冲突与一致性",
      detail: conflictCount === 0 ? "canonical 资产没有待处理冲突或诊断" : `${conflictCount} 项需要处理`,
      status: conflictCount > 0 ? "需处理" : "正常",
    },
  ];

  return (
    <div className="dashboard-page">
      <section className="stats-grid" aria-label="资产统计">
        {stats.map((stat) => {
          const Icon = stat.icon;
          return (
            <button className="stat-card" data-no-drag="true" key={stat.label} onClick={() => onPageChange?.(statPages[stat.label])} style={NO_DRAG_REGION_STYLE} type="button">
              <div className={`icon-box ${stat.tone}`}><Icon size={20} /></div>
              <div className="stat-copy"><span>{stat.label}</span><strong>{stat.value}</strong><small>{stat.change}</small></div>
            </button>
          );
        })}
      </section>

      <div className="dashboard-grid">
        <section className="panel activity-panel">
          <div className="panel-header">
            <div><h2>最近活动</h2><p>资产中心的最新变更</p></div>
            {onPageChange ? <button className="text-button" data-no-drag="true" onClick={() => onPageChange("scan")} style={NO_DRAG_REGION_STYLE} type="button">扫描资产</button> : null}
          </div>
          <div className="activity-list">
            {recentActivities.map((item) => {
              const Icon = item.icon;
              return (
                <div className="activity-item" key={item.title}>
                  <div className={`activity-icon ${item.tone}`}><Icon size={16} /></div>
                  <div className="activity-copy"><strong>{item.title}</strong><span>{item.meta}</span></div>
                  <time>{item.time}</time>
                </div>
              );
            })}
            {!demoMode && recentActivities.length === 0 && (
              <div className="asset-empty-state">
                <Activity size={22} />
                <strong>暂无活动记录</strong>
                <span>完成扫描、导入或同步后，真实活动会显示在这里。</span>
              </div>
            )}
          </div>
        </section>

        <section className="panel projects-panel">
          <div className="panel-header">
            <div><h2>已维护项目</h2><p>显式添加的本地运行目标</p></div>
            {onPageChange ? <button className="text-button" data-no-drag="true" onClick={() => onPageChange("projects")} style={NO_DRAG_REGION_STYLE} type="button">管理项目</button> : null}
          </div>
          <div className="project-list">
            {visibleProjects.map((project) => (
              <button className="project-item" data-no-drag="true" key={project.name} onClick={() => onPageChange?.("projects")} style={NO_DRAG_REGION_STYLE} type="button">
                <div className="project-folder"><FolderKanban size={18} /></div>
                <div className="project-copy"><strong>{project.name}</strong><span>{project.path}</span></div>
                <div className="project-meta"><span>{project.assets} 项资产</span><small className={project.state === "正常" ? "ok" : "pending"}>{project.state}</small></div>
              </button>
            ))}
            {visibleProjects.length === 0 && (
              <div className="asset-empty-state">
                <FolderKanban size={22} />
                <strong>尚未维护项目</strong>
                <span>在项目列表添加已有本地目录后会显示在这里。</span>
              </div>
            )}
          </div>
        </section>

        <section className="panel health-panel">
          <div className="panel-header">
            <div><h2>系统状态</h2><p>{demoMode ? "Visual QA 示例环境" : "本机只读运行环境"}</p></div>
            <div className="initialization-actions">
              {onPageChange ? <button className="text-button" data-no-drag="true" onClick={() => onPageChange("backups")} style={NO_DRAG_REGION_STYLE} type="button">查看备份</button> : null}
              <span className={`healthy-badge ${statusToneForLabel(stateLabel)}`}><CircleCheck size={14} />{stateLabel}</span>
            </div>
          </div>
          <div className="check-grid">
            {systemChecks.map((check) => (
              <div className="check-item" key={check.label}>
                <div className={`check-icon ${statusToneForLabel(check.status)}`}><ListChecks size={17} /></div>
                <div><strong>{check.label}</strong><span>{check.detail}</span></div>
                <small className={statusToneForLabel(check.status)}>{check.status}</small>
              </div>
            ))}
          </div>
          <div className="backend-strip">
            <Activity size={15} />
            <span>Tauri 后端</span>
            <strong>{appInfo.backendReady ? "已连接" : "浏览器预览"}</strong>
            <code>{appInfo.platform} · {appInfo.arch} · v{appInfo.version}</code>
          </div>
          {!demoMode && initialization && !initialization.alreadyInitialized && (
            <section className="initialization-panel" aria-label="资产中心初始化">
              <div className="initialization-copy">
                <AlertTriangle size={17} />
                <div>
                  <strong>资产中心尚未初始化</strong>
                  <span>{initialization.assetCenterPath}</span>
                </div>
              </div>
              {!showInitializationPreview ? (
                <button
                  className="asset-secondary-action"
                  data-no-drag="true"
                  disabled={initializationBusy}
                  onClick={handleInitializationPreview}
                  style={NO_DRAG_REGION_STYLE}
                  type="button"
                >
                  {initializationBusy ? "正在检查…" : "预览初始化"}
                </button>
              ) : (
                <div className="initialization-preview">
                  <p>将创建 {initialization.plannedPaths.length} 个目录或文件，并初始化本地 Git main 分支。</p>
                  {initialization.warnings.map((warning) => (
                    <p className="initialization-warning" key={warning}>{warning}</p>
                  ))}
                  <div className="initialization-actions">
                    <button
                      className="asset-secondary-action"
                      data-no-drag="true"
                      disabled={initializationBusy}
                      onClick={() => setShowInitializationPreview(false)}
                      style={NO_DRAG_REGION_STYLE}
                      type="button"
                    >
                      取消
                    </button>
                    <button
                      className="asset-business-action"
                      data-no-drag="true"
                      disabled={initializationBusy || !initialization.canApply}
                      onClick={handleInitializationApply}
                      style={NO_DRAG_REGION_STYLE}
                      type="button"
                    >
                      {initializationBusy ? "正在初始化…" : "确认初始化"}
                    </button>
                  </div>
                </div>
              )}
              {initializationMessage && <p className="initialization-message">{initializationMessage}</p>}
            </section>
          )}
          {!demoMode && initializationMessage && initialization?.alreadyInitialized && (
            <p className="initialization-message">{initializationMessage}</p>
          )}
          {!demoMode && doctor && doctor.contentDiagnostics.length > 0 && (
            <section className="initialization-panel" aria-label="资产一致性修复">
              <div className="initialization-copy">
                <AlertTriangle size={17} />
                <div>
                  <strong>检测到资产索引与 canonical 内容不一致</strong>
                  <span>不会自动修复。请选择一项操作后先查看高风险 Preview。</span>
                </div>
              </div>
              <div className="initialization-preview">
                {doctor.contentDiagnostics.map((diagnostic) => (
                  <div className="repair-diagnostic" key={diagnostic.assetId}>
                    <strong>{diagnostic.assetId}</strong>
                    <p>{diagnostic.message ?? diagnostic.state}</p>
                    {diagnostic.state === "missing_content" && (
                      <button className="asset-secondary-action" data-no-drag="true" disabled={repairBusy} onClick={() => handleRepairPreview(diagnostic.assetId, "remove_missing_registry_record")} style={NO_DRAG_REGION_STYLE} type="button">预览移除陈旧索引</button>
                    )}
                    {diagnostic.state === "unregistered" && (
                      <div className="initialization-actions">
                        <button className="asset-secondary-action" data-no-drag="true" disabled={repairBusy} onClick={() => handleRepairPreview(diagnostic.assetId, "register_unregistered_content")} style={NO_DRAG_REGION_STYLE} type="button">预览重新登记</button>
                        <button className="asset-business-action danger-action" data-no-drag="true" disabled={repairBusy} onClick={() => handleRepairPreview(diagnostic.assetId, "delete_unregistered_content")} style={NO_DRAG_REGION_STYLE} type="button">预览删除孤立内容</button>
                      </div>
                    )}
                    {diagnostic.state === "invalid_content" && <p className="initialization-warning">损坏内容仅供诊断，系统不会自动覆盖或删除。</p>}
                  </div>
                ))}
                {repairPreview && (
                  <div className="repair-diagnostic">
                    <strong>修复 Preview: {repairPreview.diagnostic.assetId}</strong>
                    {repairPreview.plannedEffects.map((effect) => <p key={effect}>{effect}</p>)}
                    {repairPreview.warnings.map((warning) => <p className="initialization-warning" key={warning}>{warning}</p>)}
                    <div className="initialization-actions">
                      <button className="asset-secondary-action" data-no-drag="true" disabled={repairBusy} onClick={() => setRepairPreview(null)} style={NO_DRAG_REGION_STYLE} type="button">取消</button>
                      <button className="asset-business-action danger-action" data-no-drag="true" disabled={repairBusy || !repairPreview.canApply} onClick={handleRepairApply} style={NO_DRAG_REGION_STYLE} type="button">确认修复</button>
                    </div>
                  </div>
                )}
              </div>
              {repairMessage && <p className="initialization-message">{repairMessage}</p>}
            </section>
          )}
        </section>
      </div>
    </div>
  );
}

function auditOperationLabel(operation: string) {
  const labels: Record<string, string> = {
    import: "导入资产",
    batch_import: "批量导入资产",
    adopt: "导入并接管资产",
    mount: "挂载资产",
    unmount: "解除挂载",
    delete_asset: "删除资产",
    mcp_save: "保存 MCP",
    settings_save: "保存设置",
    git_sync: "Git 同步",
    target_add: "添加目标",
    target_remove: "移除目标",
    backup_delete: "删除备份",
    consistency_repair: "修复资产一致性",
    diagnostic_export: "导出诊断包",
  };
  return labels[operation] ?? "本地资产操作";
}

function auditOutcomeLabel(outcome: AuditLogEntry["outcome"]) {
  return outcome === "completed" ? "已完成" : outcome === "recovered" ? "已恢复" : "需要恢复";
}

function formatAuditTime(epochSeconds: number) {
  if (!Number.isFinite(epochSeconds) || epochSeconds <= 0) return "刚刚";
  return new Date(epochSeconds * 1000).toLocaleString("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function realStats(assets: readonly AssetSummary[], projects: readonly ProjectSummary[]): DashboardStat[] {
  const count = (assetType: AssetSummary["assetType"]) =>
    assets.filter((asset) => asset.assetType === assetType).length;
  return [
    { label: "Skills", value: count("skill"), change: "本地真实数据", icon: BookOpen, tone: "green" },
    { label: "Commands", value: count("command"), change: "本地真实数据", icon: TerminalSquare, tone: "blue" },
    { label: "MCP Servers", value: count("mcp"), change: "本地真实数据", icon: Blocks, tone: "violet" },
    { label: "项目", value: projects.length, change: "已维护项目", icon: FolderKanban, tone: "amber" },
  ];
}

function doctorStatusLabel(status: "ok" | "warning" | "error") {
  if (status === "ok") return "正常";
  if (status === "warning") return "需注意";
  return "异常";
}

function settledValue<T>(result: PromiseSettledResult<T>, fallback: T): T {
  return result.status === "fulfilled" && result.value != null ? result.value : fallback;
}

function errorMessage(error: unknown) {
  return safeCommandErrorMessage(
    error,
    "本地概览操作未完成。请查看系统状态或导出诊断包后重试。",
  );
}
