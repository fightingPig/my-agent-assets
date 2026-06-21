import { ChevronRight, Search, SlidersHorizontal, type LucideIcon } from "lucide-react";
import { useMemo, useState, type ReactNode } from "react";
import { NO_DRAG_REGION_STYLE } from "../../lib/platform";

export type AssetStatusTone = "success" | "warning" | "neutral";

export type AssetCenterItem = {
  id: string;
  name: string;
  summary: string;
  status: string;
  statusTone: AssetStatusTone;
  scope: string;
  path: string;
  icon: LucideIcon;
  searchTerms?: readonly string[];
};

type AssetCenterLayoutProps<T extends AssetCenterItem> = {
  items: readonly T[];
  itemLabel: string;
  searchPlaceholder: string;
  actionLabel: string;
  renderInspector: (item: T) => ReactNode;
};

export type InspectorField = {
  label: string;
  value: string;
};

function matchesSearch(item: AssetCenterItem, query: string) {
  const searchable = [item.name, item.summary, item.scope, item.path, ...(item.searchTerms ?? [])]
    .join(" ")
    .toLocaleLowerCase();
  return searchable.includes(query.trim().toLocaleLowerCase());
}

export function AssetCenterLayout<T extends AssetCenterItem>({
  items,
  itemLabel,
  searchPlaceholder,
  actionLabel,
  renderInspector,
}: AssetCenterLayoutProps<T>) {
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [selectedId, setSelectedId] = useState(items[0]?.id ?? "");
  const statuses = useMemo(() => [...new Set(items.map((item) => item.status))], [items]);
  const visibleItems = useMemo(
    () => items.filter((item) =>
      (statusFilter === "all" || item.status === statusFilter) && matchesSearch(item, query)),
    [items, query, statusFilter],
  );
  const selectedItem = visibleItems.find((item) => item.id === selectedId) ?? visibleItems[0];

  return (
    <div className="asset-center-layout">
      <section className="panel asset-browser" aria-label={`${itemLabel}列表`}>
        <div className="asset-toolbar">
          <label className="asset-search-field">
            <Search size={15} />
            <input
              aria-label={`搜索${itemLabel}`}
              data-no-drag="true"
              onChange={(event) => setQuery(event.target.value)}
              placeholder={searchPlaceholder}
              style={NO_DRAG_REGION_STYLE}
              type="search"
              value={query}
            />
          </label>
          <label className="asset-filter-field">
            <SlidersHorizontal size={14} />
            <select
              aria-label={`${itemLabel}状态筛选`}
              data-no-drag="true"
              onChange={(event) => setStatusFilter(event.target.value)}
              style={NO_DRAG_REGION_STYLE}
              value={statusFilter}
            >
              <option value="all">全部状态</option>
              {statuses.map((status) => <option key={status} value={status}>{status}</option>)}
            </select>
          </label>
        </div>

        <div className="asset-list-heading">
          <span>{itemLabel}</span>
          <small>{visibleItems.length} / {items.length}</small>
        </div>

        <div className="asset-list" role="listbox" aria-label={`${itemLabel}选择`}>
          {visibleItems.map((item) => {
            const Icon = item.icon;
            const isSelected = selectedItem?.id === item.id;
            return (
              <button
                aria-label={item.name}
                aria-selected={isSelected}
                className={`asset-list-row ${isSelected ? "selected" : ""}`}
                data-no-drag="true"
                key={item.id}
                onClick={() => setSelectedId(item.id)}
                role="option"
                style={NO_DRAG_REGION_STYLE}
                type="button"
              >
                <span className="asset-list-icon"><Icon size={17} /></span>
                <span className="asset-list-copy">
                  <strong>{item.name}</strong>
                  <small>{item.summary}</small>
                  <span>{item.scope} · {item.path}</span>
                </span>
                <span className={`asset-status ${item.statusTone}`}>{item.status}</span>
                <ChevronRight className="asset-row-chevron" size={15} />
              </button>
            );
          })}
          {visibleItems.length === 0 && (
            <div className="asset-empty-state">
              <Search size={22} />
              <strong>没有匹配的{itemLabel}</strong>
              <span>调整搜索关键词或状态筛选。</span>
            </div>
          )}
        </div>
      </section>

      <aside className="panel asset-inspector" aria-label={`${itemLabel}检查器`}>
        {selectedItem ? (
          <>
            <div className="asset-inspector-header">
              <div className="asset-inspector-title">
                <span className="asset-list-icon"><selectedItem.icon size={18} /></span>
                <div><small>{itemLabel}详情</small><h2>{selectedItem.name}</h2></div>
              </div>
              <span className={`asset-status ${selectedItem.statusTone}`}>{selectedItem.status}</span>
            </div>
            <div className="asset-inspector-content">{renderInspector(selectedItem)}</div>
            <div className="asset-inspector-actions">
              <button aria-disabled="true" className="asset-secondary-action" data-no-drag="true" disabled style={NO_DRAG_REGION_STYLE} type="button">更多操作</button>
              <button aria-disabled="true" className="asset-business-action" data-no-drag="true" disabled style={NO_DRAG_REGION_STYLE} type="button">{actionLabel}</button>
            </div>
          </>
        ) : (
          <div className="asset-inspector-empty"><strong>暂无可检查资产</strong><span>左侧出现匹配结果后，这里将显示详情。</span></div>
        )}
      </aside>
    </div>
  );
}

export function InspectorFields({ fields }: { fields: readonly InspectorField[] }) {
  return (
    <dl className="asset-inspector-fields">
      {fields.map((field) => <div key={field.label}><dt>{field.label}</dt><dd>{field.value}</dd></div>)}
    </dl>
  );
}

export function InspectorSection({ title, children }: { title: string; children: ReactNode }) {
  return <section className="asset-inspector-section"><h3>{title}</h3>{children}</section>;
}

export function InspectorCode({ label, children }: { label: string; children: string }) {
  return <div className="asset-code-preview"><div>{label}</div><pre><code>{children}</code></pre></div>;
}

export function InspectorTags({ tags }: { tags: readonly string[] }) {
  return <div className="asset-tag-list">{tags.map((tag) => <span key={tag}>{tag}</span>)}</div>;
}
