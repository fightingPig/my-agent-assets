import { Plus, Search, ShieldCheck } from "lucide-react";
import type { PageMetadata } from "../../app/pages";
import { NO_DRAG_REGION_STYLE } from "../../lib/platform";

type PageHeaderProps = {
  page: PageMetadata;
  shortcuts: {
    globalSearch: string;
  };
};

export function PageHeader({ page, shortcuts }: PageHeaderProps) {
  return (
    <div className="page-header">
      <div className="page-heading">
        <h1>{page.title}</h1>
        <p className="page-subtitle">{page.subtitle}</p>
      </div>
      <div className="page-header-actions">
        <button
          className="search-button"
          style={NO_DRAG_REGION_STYLE}
          title={`全局搜索 ${shortcuts.globalSearch}`}
        >
          <Search size={16} />
          <span>搜索</span>
          <kbd>{shortcuts.globalSearch}</kbd>
        </button>
        <button className="preview-button" style={NO_DRAG_REGION_STYLE} title="当前使用预览数据">
          <ShieldCheck size={14} />
          预览数据
        </button>
        <button className="primary-button" style={NO_DRAG_REGION_STYLE}>
          <Plus size={16} />
          快速操作
        </button>
      </div>
    </div>
  );
}
