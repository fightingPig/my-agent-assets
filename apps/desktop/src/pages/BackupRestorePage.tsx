import { AlertTriangle, Archive, BookOpenCheck, FileJson, FolderKanban, FolderOpen, History } from "lucide-react";
import { useEffect, useState } from "react";
import {
  backupDeleteApply,
  backupDeletePreview,
  listBackups,
  revealBackupManifest,
  settingsLoad,
} from "../app/data-api";
import type { BackupDeletePreview, BackupSummary } from "../app/contracts";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

type BackupItem = {
  id: string;
  title: string;
  created: string;
  createdAtEpochSeconds: number | null;
  size: string;
  sizeBytes: number;
  entryCount: number;
  manifestPath: string;
  runtimeRoot: string;
  paths: string[];
  class: string;
  operation: string;
  sensitiveConfigRisk: boolean;
  warnings: string[];
};

const staticBackups: readonly BackupItem[] = [
  {
    id: "backup-20260621-1842",
    title: "扫描导入前",
    created: "今天 18:42",
    createdAtEpochSeconds: null,
    size: "24 KB",
    sizeBytes: 24 * 1024,
    entryCount: 3,
    manifestPath: "~/.my-agent-assets/backups/local/backup-20260621-1842/manifest.json",
    runtimeRoot: "~",
    paths: ["~/.claude/skills/review", "~/workspace/project-a/.mcp.json", "~/.claude/commands/deploy-prod.md"],
    class: "local",
    operation: "adopt",
    sensitiveConfigRisk: true,
    warnings: [],
  },
  {
    id: "backup-20260620-0915",
    title: "挂载变更前",
    created: "昨天 09:15",
    createdAtEpochSeconds: null,
    size: "18 KB",
    sizeBytes: 18 * 1024,
    entryCount: 2,
    manifestPath: "~/.my-agent-assets/backups/local/backup-20260620-0915/manifest.json",
    runtimeRoot: "~",
    paths: ["~/workspace/my-app/.claude/skills/react-review", "~/workspace/my-app/.mcp.json"],
    class: "local",
    operation: "mount",
    sensitiveConfigRisk: true,
    warnings: [],
  },
  {
    id: "backup-20260618-1630",
    title: "资产移除前",
    created: "3 天前",
    createdAtEpochSeconds: null,
    size: "8 KB",
    sizeBytes: 8 * 1024,
    entryCount: 1,
    manifestPath: "~/.my-agent-assets/backups/local/backup-20260618-1630/manifest.json",
    runtimeRoot: "~",
    paths: ["~/.claude/commands/format-code.md"],
    class: "portable",
    operation: "delete",
    sensitiveConfigRisk: false,
    warnings: [],
  },
];

export function BackupRestorePage({ demoMode = false }: { demoMode?: boolean }) {
  const [backups, setBackups] = useState<readonly BackupItem[]>(demoMode ? staticBackups : []);
  const [selectedId, setSelectedId] = useState(demoMode ? staticBackups[0].id : "");
  const [listState, setListState] = useState("读取中");
  const [revealState, setRevealState] = useState("");
  const [deletePreview, setDeletePreview] = useState<BackupDeletePreview | null>(null);
  const [deleteState, setDeleteState] = useState("");
  const [deleteBusy, setDeleteBusy] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const [warningThresholdBytes, setWarningThresholdBytes] = useState(1024 * 1024 * 1024);
  const selected = backups.find((backup) => backup.id === selectedId) ?? backups[0];
  const totalSize = backups.reduce((sum, backup) => sum + backup.sizeBytes, 0);
  const oldest = backups
    .filter((backup) => backup.createdAtEpochSeconds !== null)
    .sort((left, right) => (left.createdAtEpochSeconds ?? 0) - (right.createdAtEpochSeconds ?? 0))[0];
  const overCapacity = totalSize >= warningThresholdBytes;

  useEffect(() => {
    let cancelled = false;
    if (demoMode) {
      setBackups(staticBackups);
      setSelectedId(staticBackups[0].id);
      setListState("Visual QA 示例数据");
      setDeletePreview(null);
      setDeleteState("");
      return undefined;
    }

    setBackups([]);
    setSelectedId("");
    setListState("读取中");
    setDeletePreview(null);
    setDeleteState("");
    listBackups()
      .then((loaded) => {
        if (cancelled) return;
        if (Array.isArray(loaded) && loaded.length > 0) {
          const mapped = loaded.map(toBackupItem);
          setBackups(mapped);
          setSelectedId(mapped[0]?.id ?? "");
          setListState("只读真实数据");
        } else {
          setListState("未发现本地备份");
        }
      })
      .catch((error) => {
        if (cancelled) return;
        setListState(`读取失败：${errorMessage(error)}`);
      });

    return () => {
      cancelled = true;
    };
  }, [demoMode, refreshKey]);

  useEffect(() => {
    if (demoMode) {
      setWarningThresholdBytes(1024 * 1024 * 1024);
      return;
    }
    let cancelled = false;
    settingsLoad()
      .then((settings) => {
        if (!cancelled && Number.isFinite(settings.backupWarningThresholdBytes)) {
          setWarningThresholdBytes(settings.backupWarningThresholdBytes);
        }
      })
      .catch(() => {
        if (!cancelled) setWarningThresholdBytes(1024 * 1024 * 1024);
      });
    return () => {
      cancelled = true;
    };
  }, [demoMode]);

  const handleRevealManifest = async () => {
    if (!selected || demoMode) return;
    setRevealState("正在打开文件管理器…");
    try {
      const result = await revealBackupManifest({ entryId: selected.id });
      setRevealState(`已在文件管理器中显示：${result.manifestPath}`);
    } catch (error) {
      setRevealState(`无法显示 manifest：${errorMessage(error)}`);
    }
  };

  const handlePreviewDelete = async () => {
    if (!selected || demoMode) return;
    setDeleteBusy(true);
    setDeleteState("正在生成删除影响预览…");
    try {
      const preview = await backupDeletePreview({ entryId: selected.id });
      setDeletePreview(preview);
      setDeleteState(preview.canApply ? "删除计划已生成，请确认高风险影响。" : "删除计划被安全检查阻止。");
    } catch (error) {
      setDeletePreview(null);
      setDeleteState(`无法生成删除计划：${errorMessage(error)}`);
    } finally {
      setDeleteBusy(false);
    }
  };

  const handleApplyDelete = async () => {
    if (!deletePreview?.canApply || !selected || demoMode) return;
    setDeleteBusy(true);
    setDeleteState("正在永久删除选中的备份…");
    try {
      const result = await backupDeleteApply({
        previewId: deletePreview.previewId,
        previewGeneratedAtEpochSeconds: deletePreview.generatedAtEpochSeconds,
        request: { entryId: selected.id },
      });
      setDeletePreview(null);
      setDeleteState(result.warnings[0] ?? "选中的备份已删除。");
      setRefreshKey((current) => current + 1);
    } catch (error) {
      setDeleteState(`删除失败：${errorMessage(error)}`);
    } finally {
      setDeleteBusy(false);
    }
  };

  return (
    <div className="master-detail-workspace backup-workspace">
      <section className="panel master-list-panel">
        <div className="section-heading">
          <div>
            <h3>备份历史</h3>
            <p>Portable / Local backup manifest · {listState}</p>
          </div>
          <span>{backups.length} 份 · {formatBytes(totalSize)} · 最早：{oldest?.created ?? "暂无"}</span>
        </div>
        {overCapacity ? (
          <div className="operation-warning">
            <AlertTriangle size={17} />
            <div><strong>备份总量超过提醒阈值</strong><span>当前 {formatBytes(totalSize)}，阈值 {formatBytes(warningThresholdBytes)}。请人工检查并删除不再需要的备份；应用不会自动清理。</span></div>
          </div>
        ) : null}
        <div className="master-select-list" role="listbox" aria-label="备份选择">
          {backups.map((backup) => (
            <button
              aria-label={backup.id}
              aria-selected={selectedId === backup.id}
              className={selectedId === backup.id ? "selected" : ""}
              data-no-drag="true"
              key={backup.id}
              onClick={() => {
                setSelectedId(backup.id);
                setDeletePreview(null);
                setDeleteState("");
              }}
              role="option"
              style={NO_DRAG_REGION_STYLE}
              type="button"
            >
              <span className="skeleton-icon"><Archive size={16} /></span>
              <span>
                <strong>{backup.title}</strong>
                <small>{backup.created} · {backup.size} · {backup.entryCount} 项</small>
              </span>
            </button>
          ))}
          {backups.length === 0 && (
            <div className="asset-empty-state">
              <History size={22} />
              <strong>暂无备份历史</strong>
              <span>执行需要安全备份的操作后，manifest 会显示在这里。</span>
            </div>
          )}
        </div>
      </section>

      <section className="panel master-inspector-panel">
        {selected ? (
          <>
            <div className="section-heading">
              <div><h3>{selected.title}</h3><p>{selected.id}</p></div>
              <span className="healthy-badge">只读历史</span>
            </div>

            <div className="restore-summary">
              <FileJson size={18} />
              <div>
                <strong>Manifest 文件</strong>
                <code>{selected.manifestPath}</code>
              </div>
              {!demoMode ? (
                <button
                  className="asset-secondary-action"
                  data-no-drag="true"
                  onClick={handleRevealManifest}
                  style={NO_DRAG_REGION_STYLE}
                  type="button"
                >
                  <FolderOpen size={15} />
                  在文件管理器中显示
                </button>
              ) : null}
            </div>
            {revealState ? <p className="backup-reveal-state" role="status">{revealState}</p> : null}

            {!demoMode ? (
              <section className="backup-delete-panel">
                <div className="operation-warning">
                  <AlertTriangle size={17} />
                  <div>
                    <strong>删除这份备份</strong>
                    <span>会永久移除 manifest 和备份内容，应用内没有自动 Restore。请先查看受影响路径和手动恢复说明。</span>
                  </div>
                </div>
                <div className="operation-actions">
                  <button
                    className="asset-secondary-action"
                    data-no-drag="true"
                    disabled={deleteBusy}
                    onClick={handlePreviewDelete}
                    style={NO_DRAG_REGION_STYLE}
                    type="button"
                  >
                    预览删除影响
                  </button>
                  {deletePreview ? (
                    <button
                      className="asset-business-action danger-action"
                      data-no-drag="true"
                      disabled={deleteBusy || !deletePreview.canApply}
                      onClick={handleApplyDelete}
                      style={NO_DRAG_REGION_STYLE}
                      type="button"
                    >
                      确认永久删除
                    </button>
                  ) : null}
                </div>
                {deletePreview ? (
                  <div className={`backup-delete-preview ${deletePreview.canApply ? "" : "blocked"}`}>
                    <strong>{deletePreview.canApply ? "删除计划" : "删除已阻止"}</strong>
                    <span>{deletePreview.backupPath} · {formatBytes(deletePreview.sizeBytes)} · {deletePreview.entryCount} 项</span>
                    {deletePreview.plannedEffects.map((effect) => <p key={effect}>{effect}</p>)}
                    {deletePreview.warnings.map((warning) => <p className="warning-text" key={warning}>提示：{warning}</p>)}
                  </div>
                ) : null}
                {deleteState ? <p className="backup-reveal-state" role="status">{deleteState}</p> : null}
              </section>
            ) : null}

            <section className="affected-paths">
              <h4>记录的文件路径</h4>
              {selected.paths.length > 0 ? selected.paths.map((path) => (
                <div key={path}><FolderKanban size={14} /><code>{path}</code></div>
              )) : (
                <p>当前列表合同尚未返回 manifest 条目，请从上述 manifest 路径查看完整清单。</p>
              )}
            </section>

            <div className="operation-warning neutral">
              <BookOpenCheck size={17} />
              <div>
                <strong>手动恢复说明</strong>
                <span>应用不会自动覆盖或恢复任何历史文件。请按以下步骤人工核对。</span>
              </div>
            </div>
            <ol className="manual-restore-steps">
              <li>退出 Claude Code、Codex 以及正在使用相关配置的编辑器。</li>
              <li>点击“在文件管理器中显示”，打开并阅读 manifest 的操作类型和受影响路径。</li>
              <li>先复制当前目标文件或目录，作为恢复前的额外保护。</li>
              <li>仅按 manifest 记录的原路径与备份内容逐项复制；不要整目录覆盖资产中心。</li>
              <li>重新启动相关客户端，运行 <code>maa doctor</code>，确认 registry、mount 和 runtime 状态。</li>
            </ol>

            <section className="affected-paths">
              <h4>备份信息</h4>
              <div><History size={14} /><span>创建时间：{selected.created}</span></div>
              <div><Archive size={14} /><span>大小：{selected.size}，条目：{selected.entryCount}，runtime root：{selected.runtimeRoot}</span></div>
              <div><Archive size={14} /><span>类型：{selected.class}，操作：{selected.operation}</span></div>
            </section>
            {selected.sensitiveConfigRisk ? (
              <div className="operation-warning">
                <AlertTriangle size={17} />
                <div><strong>可能包含敏感 MCP 配置</strong><span>该备份可能包含 token、密码或 header；提交 Git 前请确认远程仓库为 Private。</span></div>
              </div>
            ) : null}
            {selected.warnings.map((warning) => (
              <p className="warning-text" key={warning}>{warning}</p>
            ))}
          </>
        ) : (
          <div className="asset-inspector-empty">
            <Archive size={22} />
            <strong>暂无备份详情</strong>
            <span>选择真实备份记录后，这里会显示 manifest 路径和手动恢复说明。</span>
          </div>
        )}
      </section>
    </div>
  );
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "无法读取备份历史。";
}

function toBackupItem(backup: BackupSummary): BackupItem {
  const created = backup.createdAtEpochSeconds
    ? new Date(backup.createdAtEpochSeconds * 1000).toLocaleString()
    : backup.createdAt || "未知时间";
  return {
    id: backup.id,
    title: backup.label,
    created,
    createdAtEpochSeconds: backup.createdAtEpochSeconds ?? null,
    size: formatBytes(backup.sizeBytes),
    sizeBytes: backup.sizeBytes,
    entryCount: backup.entryCount,
    manifestPath: backup.manifestPath ?? `~/.my-agent-assets/backups/${backup.id}/manifest.json`,
    runtimeRoot: backup.runtimeRoot ?? "请查看 manifest",
    paths: backup.affectedPaths ?? [],
    class: backup.class ?? "legacy",
    operation: backup.operation ?? "legacy",
    sensitiveConfigRisk: backup.sensitiveConfigRisk ?? false,
    warnings: backup.warnings ?? [],
  };
}

function formatBytes(sizeBytes: number) {
  if (sizeBytes >= 1024 * 1024) return `${(sizeBytes / 1024 / 1024).toFixed(1)} MB`;
  if (sizeBytes >= 1024) return `${(sizeBytes / 1024).toFixed(1)} KB`;
  return `${sizeBytes} B`;
}
