import { Check, ChevronDown, SlidersHorizontal } from "lucide-react";
import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { NO_DRAG_REGION_STYLE } from "../../lib/platform";

type StatusFilterMenuProps = {
  itemLabel: string;
  statuses: readonly string[];
  value: string;
  onChange: (value: string) => void;
};

/**
 * Reusable status filter for desktop management workspaces.
 *
 * The menu stays inside the application layer instead of delegating to a
 * platform select popup, which keeps spacing, focus, and overlay geometry
 * consistent on macOS and Windows.
 */
export function StatusFilterMenu({ itemLabel, statuses, value, onChange }: StatusFilterMenuProps) {
  const [open, setOpen] = useState(false);
  const fieldRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const optionRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const listboxId = `${useId().replace(/:/g, "")}-options`;
  const options = useMemo(
    () => [
      { value: "all", label: "全部状态" },
      ...statuses.filter((status) => status !== "all").map((status) => ({ value: status, label: status })),
    ],
    [statuses],
  );
  const selectedIndex = Math.max(0, options.findIndex((option) => option.value === value));

  useEffect(() => {
    if (!open) return undefined;
    const handlePointerDown = (event: PointerEvent) => {
      if (!fieldRef.current?.contains(event.target as Node)) setOpen(false);
    };
    const handleKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
        triggerRef.current?.focus();
      }
    };
    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    optionRefs.current[selectedIndex]?.focus();
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open, selectedIndex]);

  const moveSelection = (direction: 1 | -1) => {
    const nextIndex = Math.min(options.length - 1, Math.max(0, selectedIndex + direction));
    const nextOption = options[nextIndex];
    if (!nextOption) return;
    onChange(nextOption.value);
    optionRefs.current[nextIndex]?.focus();
  };

  const handleTriggerKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setOpen(true);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      setOpen((current) => !current);
    }
  };

  const handleOptionKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      moveSelection(1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      moveSelection(-1);
    } else if (event.key === "Home") {
      event.preventDefault();
      const first = options[0];
      if (first) {
        onChange(first.value);
        optionRefs.current[0]?.focus();
      }
    } else if (event.key === "End") {
      event.preventDefault();
      const lastIndex = options.length - 1;
      const last = options[lastIndex];
      if (last) {
        onChange(last.value);
        optionRefs.current[lastIndex]?.focus();
      }
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      setOpen(false);
      triggerRef.current?.focus();
    }
  };

  return (
    <div className={`asset-filter-field${open ? " is-open" : ""}`} ref={fieldRef}>
      <SlidersHorizontal aria-hidden="true" size={14} />
      <button
        aria-controls={listboxId}
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-label={`${itemLabel}状态筛选`}
        className="asset-filter-trigger"
        data-no-drag="true"
        onClick={() => setOpen((current) => !current)}
        onKeyDown={handleTriggerKeyDown}
        ref={triggerRef}
        role="combobox"
        style={NO_DRAG_REGION_STYLE}
        type="button"
      >
        <span>{options[selectedIndex]?.label ?? "全部状态"}</span>
        <ChevronDown aria-hidden="true" size={15} />
      </button>
      {open ? (
        <div className="asset-filter-menu" id={listboxId} role="listbox" aria-label={`${itemLabel}状态选项`}>
          {options.map((option, index) => (
            <button
              aria-selected={value === option.value}
              className={value === option.value ? "selected" : ""}
              data-no-drag="true"
              key={option.value}
              onClick={() => {
                onChange(option.value);
                setOpen(false);
                triggerRef.current?.focus();
              }}
              onKeyDown={handleOptionKeyDown}
              ref={(node) => { optionRefs.current[index] = node; }}
              role="option"
              style={NO_DRAG_REGION_STYLE}
              type="button"
            >
              <span>{option.label}</span>
              {value === option.value ? <Check aria-hidden="true" size={15} /> : null}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}
