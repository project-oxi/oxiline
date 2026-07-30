import { Check, GripVertical } from "lucide-react";
import { useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import { useTranslation } from "react-i18next";
import type { Category, TimelineItem } from "../types";
import { categoryById, categoryColor, rangeLabel } from "../lib/colors";
import { useSetTaskDone } from "../hooks";

interface Props {
  item: TimelineItem;
  categories: Category[];
  left: number; // column index
  columns: number; // # of concurrent lanes in this group
  pxPerMin: number;
  top: number;
  height: number;
  past: boolean;
}

export function BlockView({ item, categories, left, columns, pxPerMin, top, height, past }: Props) {
  const { t } = useTranslation();
  const done = useSetTaskDone();
  const cat = categoryById(categories, item.category_id);
  const color = categoryColor(cat?.color_hue ?? null);

  const widthPct = 100 / columns;
  const leftPct = left * widthPct;
  const accent = item.is_done ? "var(--signal-success)" : color;

  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: `block:${item.id}`,
    data: { kind: "block", item: { id: item.id, start_minute: item.start_minute } },
  });

  // Resize handle: separate draggable at the bottom.
  const {
    attributes: resizeAttr,
    listeners: resizeListeners,
    setNodeRef: resizeRef,
  } = useDraggable({
    id: `resize:${item.id}`,
    data: { kind: "resize", item, pxPerMin },
  });

  const style: React.CSSProperties = {
    top,
    height: Math.max(height, 22),
    left: `calc(${leftPct}% + ${leftPct > 0 ? 4 : 0}px)`,
    width: `calc(${widthPct}% - 4px)`,
    borderLeft: `4px solid ${accent}`,
    opacity: isDragging ? 0.5 : item.is_virtual && !item.is_done ? 0.92 : 1,
    boxShadow: past && !item.is_done ? "inset 3px 0 0 var(--signal-rust)" : undefined,
    transform: CSS.Translate.toString(transform),
    zIndex: isDragging ? 999 : undefined,
    transition: isDragging
      ? undefined
      : `opacity var(--motion-sweep) var(--ease-standard),
          filter var(--motion-sweep) var(--ease-standard)`,
    filter: past ? "saturate(0.4)" : undefined,
    cursor: "grab",
  };

  return (
    <div ref={setNodeRef} style={style} className="absolute rounded-md border border-border-subtle bg-raised group" title={item.title} {...attributes} {...listeners}>
      <button
        className="flex h-full w-full flex-col justify-start px-2 py-1 text-left"
        onClick={(e) => { e.stopPropagation(); done.mutate({ id: item.id, done: !item.is_done }); }}
        onPointerDown={(e) => e.stopPropagation()}
      >
        <span className="flex items-center gap-1">
          <span
            className="inline-flex h-4 w-4 shrink-0 items-center justify-center rounded-full border"
            style={{
              borderColor: item.is_done ? "var(--signal-success)" : "var(--border-default)",
              background: item.is_done ? "var(--signal-success)" : "transparent",
            }}
          >
            {item.is_done && <Check size={11} color="white" />}
          </span>
          <span
            className="truncate text-[13px] font-medium"
            style={{
              textDecoration: item.is_done ? "line-through" : "none",
              color: item.is_done ? "var(--text-secondary)" : "var(--text-primary)",
            }}
          >
            {item.title}
          </span>
        </span>
        {height > 44 && (
          <span
            className="mt-0.5 font-mono text-[11px] leading-tight"
            style={{ color: "var(--text-tertiary)" }}
          >
            {rangeLabel(item.start_minute, item.duration_minute)}
          </span>
        )}
        {height > 64 && item.duration_minute != null && (
          <span
            className="mt-0.5 font-mono text-[11px]"
            style={{ color: "var(--text-tertiary)" }}
          >
            {t("common.minutes", { n: item.duration_minute })}
          </span>
        )}
      </button>
      {/* Resize handle — only visible on hover */}
      <div
        ref={resizeRef}
        {...resizeAttr}
        {...resizeListeners}
        className="absolute bottom-0 left-0 right-0 h-[6px] cursor-ns-resize opacity-0 transition group-hover:opacity-100"
        style={{ background: "var(--accent-oxide)" }}
      />
    </div>
  );
}
