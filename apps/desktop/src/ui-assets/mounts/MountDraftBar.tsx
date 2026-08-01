import { ArrowRight, RotateCcw } from "lucide-react";
import { NO_DRAG_REGION_STYLE } from "../../lib/platform";
import { useMountDrafts } from "./MountDraftContext";

export function MountDraftBar({ onOpenPreview }: { onOpenPreview?: () => void }) {
  const { drafts, clearDrafts } = useMountDrafts();
  if (drafts.length === 0) return null;

  return (
    <aside className="maa-mount-draft-bar" aria-live="polite">
      <div>
        <i>{drafts.length}</i>
        <span>
          <strong>{drafts.length} 项挂载变更等待预览</strong>
          <small>变更不会立即生效；确认预览后才会写入。</small>
        </span>
      </div>
      <button className="asset-secondary-action" data-no-drag="true" onClick={clearDrafts} style={NO_DRAG_REGION_STYLE} type="button">
        <RotateCcw size={14} />撤销变更
      </button>
      <button className="asset-business-action" data-no-drag="true" onClick={onOpenPreview} style={NO_DRAG_REGION_STYLE} type="button">
        查看挂载预览<ArrowRight size={16} />
      </button>
    </aside>
  );
}
