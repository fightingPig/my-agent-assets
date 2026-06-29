import { AlertTriangle, Archive, BookOpenCheck, FileJson, FolderKanban, History } from "lucide-react";
import { useEffect, useState } from "react";
import { listBackups } from "../app/data-api";
import type { BackupSummary } from "../app/contracts";
import { NO_DRAG_REGION_STYLE } from "../lib/platform";

type BackupItem = {
  id: string;
  title: string;
  created: string;
  size: string;
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
    size: "24 KB",
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
    size: "18 KB",
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
    size: "8 KB",
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
  const selected = backups.find((backup) => backup.id === selectedId) ?? backups[0];
  const totalSize = backups.reduce((sum, backup) => sum + parseDisplayBytes(backup.size), 0);

  useEffect(() => {
    let cancelled = false;
    if (demoMode) {
      setBackups(staticBackups);
      setSelectedId(staticBackups[0].id);
      setListState("Visual QA 示例数据");
      return undefined;
    }

    setBackups([]);
    setSelectedId("");
    setListState("读取中");
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
  }, [demoMode]);

  return (
    <div className="master-detail-workspace backup-workspace">
      <section className="panel master-list-panel">
        <div className="section-heading">
          <div>
            <h3>备份历史</h3>
            <p>Portable / Local backup manifest · {listState}</p>
          </div>
          <span>{backups.length} 份 · {formatBytes(totalSize)}</span>
        </div>
        <div className="master-select-list" role="listbox" aria-label="备份选择">
          {backups.map((backup) => (
            <button
              aria-label={backup.id}
              aria-selected={selectedId === backup.id}
              className={selectedId === backup.id ? "selected" : ""}
              data-no-drag="true"
              key={backup.id}
              onClick={() => setSelectedId(backup.id)}
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
            </div>

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
                <span>
                  先退出 Claude Code、Codex 和相关编辑器；复制当前目标文件作为额外保护；
                  打开 manifest，仅按其中的 originalPath 与 backupPath 逐项恢复。应用不会自动覆盖或恢复任何文件。
                </span>
              </div>
            </div>

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
    size: formatBytes(backup.sizeBytes),
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

function parseDisplayBytes(value: string) {
  const amount = Number.parseFloat(value);
  if (!Number.isFinite(amount)) return 0;
  if (value.endsWith("MB")) return amount * 1024 * 1024;
  if (value.endsWith("KB")) return amount * 1024;
  return amount;
}
