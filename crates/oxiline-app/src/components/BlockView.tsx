import { Check } from "lucide-react";
import { useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import { useTranslation } from "react-i18next";
import type { Category, TimelineItem } from "../types";
import { categoryById, categoryColor, rangeLabel } from "../lib/colors";
import { useSetTaskDone, useDeleteTask, useSetTaskSkipped } from "../hooks";
import { announce } from "../lib/a11y";

interface Props {
  item: TimelineItem;
  categories: Category[];
  left: number;
  columns: number;
  top: number;
  height: number;
  past: boolean;
}

export function BlockView({ item, categories, left, columns, top, height, past }: Props) {
  const { t } = useTranslation();
  const done = useSetTaskDone();
  const del = useDeleteTask();
  const skip = useSetTaskSkipped();
  const cat = categoryById(categories, item.category_id);
  const color = categoryColor(cat?.color_hue ?? null);

  const widthPct = 100 / columns;
  const leftPct = left * widthPct;
  const accent = item.is_done ? "var(--signal-success)" : color;

  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: `block:${item.id}`,
    data: { kind: "block", item: { id: item.id, start_minute: item.start_minute } },
  });

  const style: React.CSSProperties = {
    top,
    height: Math.max(height, 22),
    left: `calc(${leftPct}% + ${leftPct > 0 ? 4 : 0}px)`,
    width: `calc(${widthPct}% - 4px)`,
    background: item.is_done
      ? "color-mix(in oklch, var(--signal-success) 6%, var(--surface-raised))"
      : `color-mix(in oklch, ${accent} 8%, var(--surface-raised))`,
    opacity: isDragging
      ? 0.5
      : item.is_virtual && !item.is_done
        ? 0.92
        : past && !item.is_done
          ? 0.55
          : 1,
    transform: CSS.Translate.toString(transform),
    zIndex: isDragging ? 999 : 2,
    transition: isDragging
      ? undefined
      : `transform var(--motion-base) var(--ease-standard),
          box-shadow var(--motion-base) var(--ease-standard)`,
    cursor: "grab",
  };
  // The dnd container is the single focusable unit (§7.10: Enter toggles
  // done, Backspace/Delete skips a routine occurrence or deletes a manual task).
  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      done.mutate({ id: item.id, done: !item.is_done });
      announce(item.is_done ? t("a11y.undone") : t("a11y.markedDone"));
    } else if (e.key === "Backspace" || e.key === "Delete") {
      e.preventDefault();
      if (item.origin_routine_block_id) {
        skip.mutate({ id: item.id, skipped: true });
        announce(t("a11y.skipped"));
      } else {
        del.mutate(item.id);
        announce(t("a11y.deleted"));
      }
    }
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`absolute overflow-hidden rounded-lg ${
        isDragging
          ? "shadow-[var(--elevation-panel)]"
          : "shadow-[var(--elevation-card)] hover:shadow-[var(--elevation-panel)]"
      }`}
      title={item.title}
      {...attributes}
      {...listeners}
      aria-label={`${item.title}${item.is_done ? `, ${t("a11y.done")}` : ""}`}
      aria-describedby={undefined}
      onKeyDown={onKeyDown}
    >
      <div
        role="presentation"
        tabIndex={-1}
        className="flex h-full w-full flex-col justify-start px-2 py-1 text-left"
        onClick={(e) => { e.stopPropagation(); done.mutate({ id: item.id, done: !item.is_done }); }}
        >
        <span className="flex items-center gap-1">
          <span
            className="inline-flex h-4 w-4 shrink-0 items-center justify-center rounded-full border"
            style={{
              borderColor: item.is_done ? "var(--signal-success)" : accent,
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
      </div>
    </div>
  );
}
