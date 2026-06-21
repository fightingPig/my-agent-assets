import { Plus, Search, ShieldCheck } from "lucide-react";
import { NO_DRAG_REGION_STYLE } from "../../lib/platform";

type PageHeaderProps = {
  shortcuts: {
    globalSearch: string;
    pageSearch: string;
  };
};

export function PageHeader({ shortcuts }: PageHeaderProps) {
  return (
    <div className="page-header">
      <div className="page-heading">
        <h1>首页</h1>
        <p className="page-subtitle">集中查看资产、项目和本地运行环境。</p>
      </div>
      <div className="page-header-actions">
        <button
          className="search-button"
          style={NO_DRAG_REGION_STYLE}
          title={`页面搜索 ${shortcuts.pageSearch}`}
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
