import { Check } from "lucide-react";
import { useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import { useTranslation } from "react-i18next";
import { useRef, useState } from "react";
import type { Category, TimelineItem } from "../types";
import { categoryById, categoryColor, rangeLabel } from "../lib/colors";
import { useSetTaskDone, useDeleteTask, useSetTaskSkipped, useUpdateTask } from "../hooks";
import { api } from "../lib/api";
import { clampDuration } from "../lib/timeline-math";
import { announce } from "../lib/a11y";
interface Props {
  item: TimelineItem;
  categories: Category[];
  left: number;
  columns: number;
  top: number;
  height: number;
  past: boolean;
  dayEndMin: number;
  pxPerMin: number;
}

export function BlockView({ item, categories, left, columns, top, height, past, dayEndMin, pxPerMin }: Props) {
  const { t } = useTranslation();
  const done = useSetTaskDone();
  const del = useDeleteTask();
  const skip = useSetTaskSkipped();
  const upd = useUpdateTask();
  const [previewDur, setPreviewDur] = useState<number | null>(null);
  const drag = useRef<{ startY: number; startDur: number } | null>(null);

  async function commitDuration(dur: number) {
    let id = item.id;
    if (item.is_virtual) id = await api.materializeIfVirtual(item.id);
    upd.mutate({ id, durationMinute: dur });
  }

  function onResizeDown(e: React.PointerEvent) {
    e.stopPropagation(); // dnd-kit 이동 드래그 발화 방지
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
    drag.current = { startY: e.clientY, startDur: item.duration_minute ?? 30 };
  }
  function onResizeMove(e: React.PointerEvent) {
    if (!drag.current) return;
    const deltaMin = (e.clientY - drag.current.startY) / pxPerMin;
    const start = item.start_minute!;
    const newDur = drag.current.startDur + deltaMin;
    const dur = clampDuration(start, Math.round(newDur / 15) * 15, dayEndMin, 15);
    setPreviewDur(dur);
  }
  function onPointerCancel(e: React.PointerEvent) {
    if (!drag.current) return;
    (e.currentTarget as Element).releasePointerCapture(e.pointerId);
    drag.current = null;
    setPreviewDur(null);
  }
  function onResizeUp(e: React.PointerEvent) {
    if (!drag.current) return;
    (e.currentTarget as Element).releasePointerCapture(e.pointerId);
    drag.current = null;
    if (previewDur == null) {
      setPreviewDur(null);
      return;
    }
    const dur = previewDur;
    setPreviewDur(null);
    commitDuration(dur);
  }
  const cat = categoryById(categories, item.category_id);
  const color = categoryColor(cat?.color_hue ?? null);

  const widthPct = 100 / columns;
  const leftPct = left * widthPct;
  const accent = item.is_done ? "var(--color-status-success)" : color;

  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: `block:${item.id}`,
    data: { kind: "block", item: { id: item.id, start_minute: item.start_minute } },
  });

  const effDur = previewDur ?? item.duration_minute ?? 0;
  const effHeight = Math.max(effDur * pxPerMin, 22);

  const style: React.CSSProperties = {


    top,
    height: effHeight,
    left: `calc(${leftPct}% + ${leftPct > 0 ? 4 : 0}px)`,
    width: `calc(${widthPct}% - 4px)`,
    background: item.is_done
      ? "color-mix(in oklch, var(--color-status-success) 8%, var(--color-block-bg))"
      : "var(--color-block-bg)",
    opacity: isDragging
      ? 0.5
      : item.is_virtual && !item.is_done
        ? 0.92
        : past && !item.is_done
          ? 0.55
          : 1,
    transform: isDragging
      ? `${CSS.Translate.toString(transform)} scale(1.02)`
      : CSS.Translate.toString(transform),
    zIndex: isDragging ? 999 : 2,
    boxShadow: isDragging ? "var(--shadow-block-drag)" : undefined,
    transition: isDragging
      ? undefined
      : `transform var(--duration-base) var(--ease-out),
          box-shadow var(--duration-base) var(--ease-out)`,
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
    } else if (e.shiftKey && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
      e.preventDefault();
      const step = e.key === "ArrowUp" ? -15 : 15;
      const start = item.start_minute!;
      const dur = clampDuration(start, (item.duration_minute ?? 30) + step, dayEndMin, 15);
      commitDuration(dur);
      announce(t("a11y.resized", { n: dur }));
    }
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className="absolute overflow-hidden rounded-lg border border-[var(--color-block-border)] hover:border-[var(--color-border-strong)] shadow-[var(--shadow-block-rest)] hover:shadow-[var(--shadow-block-hover)]"
      title={item.title}
      {...attributes}
      {...listeners}
      aria-label={`${item.title}${item.is_done ? `, ${t("a11y.done")}` : ""}`}
      aria-describedby={undefined}
      onKeyDown={onKeyDown}
    >
      <div
        aria-hidden
        className="absolute left-0 top-0 bottom-0"
        style={{ width: "var(--tl-rail-width)", background: accent }}
      />
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
              borderColor: item.is_done ? "var(--color-status-success)" : accent,
              background: item.is_done ? "var(--color-status-success)" : "transparent",
            }}
          >
            {item.is_done && <Check size={11} color="white" />}
          </span>
          <span
            className="truncate text-[13px] font-medium"
            style={{
              textDecoration: item.is_done ? "line-through" : "none",
              color: item.is_done ? "var(--color-text-muted)" : "var(--color-text)",
            }}
          >
            {item.title}
          </span>
        </span>
        {height > 44 && (
          <span
            className="mt-0.5 font-mono text-[11px] leading-tight"
            style={{ color: "var(--color-text-subtle)" }}
          >
            {rangeLabel(item.start_minute, previewDur ?? item.duration_minute)}
          </span>
        )}
        {height > 64 && effDur != null && effDur > 0 && (
          <span
            className="mt-0.5 font-mono text-[11px]"
            style={{ color: "var(--color-text-subtle)" }}
          >
            {t("common.minutes", { n: effDur })}
          </span>
        )}
      </div>
      <div
        role="separator"
        aria-orientation="horizontal"
        onPointerDown={onResizeDown}
        onPointerMove={onResizeMove}
        onPointerUp={onResizeUp}
        onPointerCancel={onPointerCancel}
        className="absolute bottom-0 left-0 right-0 flex h-2 cursor-ns-resize items-end justify-center pb-0.5 opacity-40 hover:opacity-100"
        style={{ touchAction: "none" }}
      >
        <span className="h-[2px] w-6 rounded-full" style={{ background: "var(--color-text-subtle)" }} />
      </div>
    </div>
  );
}
