import { AlertTriangle, ArrowDown, ArrowUp, CheckCircle2, GitBranch, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { gitStatus } from "../app/data-api";
import type { GitStatus } from "../app/contracts";
import { StaticActionButton } from "../components/ui/StaticActionButton";

const fallbackGitStatus: GitStatus = {
  repositoryPath: "~/.my-agent-assets",
  isRepository: false,
  statusMessage: "静态预览：尚未读取本地 Git 仓库。",
  branch: "main",
  remote: "origin/main",
  clean: true,
  ahead: 2,
  behind: 1,
  changedFiles: [],
  conflicts: ["assets.yaml"],
  lastSyncedAt: null,
};

export function SyncPage() {
  const [status, setStatus] = useState<GitStatus>(fallbackGitStatus);
  const [stateLabel, setStateLabel] = useState("读取中");

  useEffect(() => {
    let cancelled = false;
    setStateLabel("读取中");
    gitStatus()
      .then((loaded) => {
        if (cancelled) return;
        if (loaded && typeof loaded === "object" && "repositoryPath" in loaded) {
          setStatus(loaded);
          setStateLabel("只读真实数据");
        } else {
          setStatus(fallbackGitStatus);
          setStateLabel("静态预览");
        }
      })
      .catch(() => {
        if (cancelled) return;
        setStatus(fallbackGitStatus);
        setStateLabel("读取失败，使用静态预览");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const cleanLabel = status.clean ? "工作区干净" : `${status.changedFiles.length} 项变更`;
  const conflictLabel = status.conflicts.length > 0 ? `${status.conflicts.length} 项预览` : "0 项";

  return (
    <div className="operation-workspace sync-workspace">
      <section className="panel sync-repository-card">
        <div className="section-heading"><div><h3>本地 Git 仓库</h3><p>{status.repositoryPath}</p></div><span className="healthy-badge"><CheckCircle2 size={13} />{cleanLabel}</span></div>
        <div className="sync-status-grid"><div><GitBranch size={17} /><span><small>当前分支</small><strong>{status.branch || "未检测到"}</strong></span></div><div><RefreshCw size={17} /><span><small>远程仓库</small><strong>{status.remote ?? "未设置"}</strong></span></div><div><ArrowUp size={17} /><span><small>Ahead</small><strong>{status.ahead} commits</strong></span></div><div><ArrowDown size={17} /><span><small>Behind</small><strong>{status.behind} commits</strong></span></div></div>
        <div className="sync-graph"><div className="sync-graph-line"><span className="local-dot" /><strong>本地 {status.branch || "工作区"}</strong><small>{status.statusMessage}</small></div><div className="sync-graph-line"><span /><strong>仓库状态</strong><small>{status.isRepository ? "已识别为本地 Git 仓库" : "未识别为本地 Git 仓库"}</small></div><div className="sync-graph-line"><span className="remote-dot" /><strong>远程仓库</strong><small>{status.remote ?? "未配置 upstream"}</small></div></div>
        <div className="operation-actions"><StaticActionButton className="asset-secondary-action">Pull</StaticActionButton><StaticActionButton className="asset-business-action">Push</StaticActionButton></div>
      </section>

      <div className="detail-two-column sync-lower-grid">
        <section className="panel detail-section"><div className="section-heading"><div><h3>同步历史</h3><p>最近的本地 Git 操作</p></div><span className="preview-label">{stateLabel}</span></div><div className="timeline-list"><div><CheckCircle2 size={14} /><span>读取本地 Git 状态</span><time>{status.lastSyncedAt ?? "刚刚"}</time></div><div><CheckCircle2 size={14} /><span>Pull / Push 保持禁用预览</span><time>只读阶段</time></div><div><CheckCircle2 size={14} /><span>未执行远程同步命令</span><time>安全策略</time></div></div></section>
        <section className="panel detail-section"><div className="section-heading"><div><h3>同步检查</h3><p>执行前风险预览</p></div></div><div className="operation-warning"><AlertTriangle size={17} /><div><strong>{status.statusMessage}</strong><span>{status.conflicts.length > 0 ? `检测到冲突文件：${status.conflicts.join(", ")}` : "当前只读取本地状态，不执行 Pull 或 Push。"}</span></div></div><div className="environment-list"><div><strong>仓库可用</strong><span>{status.isRepository ? "是" : "否"}</span></div><div><strong>未提交变更</strong><span>{status.changedFiles.length} 项</span></div><div><strong>潜在冲突</strong><span>{conflictLabel}</span></div></div></section>
      </div>
    </div>
  );
}
