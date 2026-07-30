import { useTranslation } from "react-i18next";
import { Trash2, CalendarPlus, GripVertical } from "lucide-react";
import { useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import { useBacklog, useCategories, useDeleteTask, useUpdateTask, useSetTaskDone } from "../hooks";
import { announce } from "../lib/a11y";
import { useUi } from "../lib/store";
import { categoryById, categoryColor } from "../lib/colors";
import type { Category, Task } from "../types";

export function BacklogView() {
  const { t } = useTranslation();
  const q = useBacklog();
  const catsQ = useCategories();
  const { date } = useUi();
  const items = q.data ?? [];
  const del = useDeleteTask();
  const categories = catsQ.data ?? [];

  if (items.length === 0) {
    return (
      <div className="flex h-full items-center justify-center px-6 text-center">
        <p className="text-[13px]" style={{ color: "var(--text-tertiary)" }}>
          {t("backlog.empty")}
        </p>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto px-3 py-2">
      <p className="mb-2 px-1 text-[12px] font-medium" style={{ color: "var(--text-secondary)" }}>
        {t("backlog.title")} ({items.length})
      </p>
      <ul className="space-y-1.5">
        {items.map((it) => (
          <DraggableBacklogRow
            key={it.id}
            item={it}
            date={date}
            del={del}
            categories={categories}
          />
        ))}
      </ul>
    </div>
  );
}

function DraggableBacklogRow({
  item,
  date,
  del,
  categories,
}: {
  item: Task;
  date: string;
  del: { mutate: (id: string) => void };
  categories: Category[];
}) {
  const upd = useUpdateTask();
  const { t } = useTranslation();
  const setDone = useSetTaskDone();
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: `backlog:${item.id}`,
    data: { kind: "backlog", task: item },
  });

  const cat = categoryById(categories, item.category_id);
  const dotColor = categoryColor(cat?.color_hue ?? null);

  // The row is the single focusable unit (§7.10: Enter toggles done,
  // Backspace/Delete removes). Action icons are mouse-only (aria-hidden) so the
  // role=button container never nests another interactive control.
  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      setDone.mutate({ id: item.id, done: !item.is_done });
      announce(item.is_done ? t("a11y.undone") : t("a11y.markedDone"));
    } else if (e.key === "Backspace" || e.key === "Delete") {
      e.preventDefault();
      del.mutate(item.id);
      announce(t("a11y.deleted"));
    }
  };

  return (
    <li
      ref={setNodeRef}
      {...attributes}
      {...listeners}
      className="group flex items-center gap-2.5 rounded-lg border border-border-subtle bg-raised px-2.5 py-2 outline-none transition-shadow focus-within:[box-shadow:var(--elevation-card)]"
      style={{
        transform: CSS.Translate.toString(transform),
        opacity: isDragging ? 0.5 : 1,
        cursor: "grab",
      }}
      aria-label={`${item.title}${item.is_done ? `, ${t("a11y.done")}` : ""}`}
      aria-describedby={undefined}
      onKeyDown={onKeyDown}
    >
      <span
        aria-hidden={true}
        className="inline-block h-2.5 w-2.5 shrink-0 rounded-full"
        style={{
          background: cat ? dotColor : "transparent",
          border: cat ? "none" : "2px solid var(--border-default)",
        }}
      />
      <span
        className="min-w-0 flex-1 truncate text-[13px]"
        style={{
          textDecoration: item.is_done ? "line-through" : "none",
          color: item.is_done ? "var(--text-secondary)" : "var(--text-primary)",
        }}
      >
        {item.title}
      </span>

      {/* mouse-only actions; keyboard equivalents live on the row (§7.10) */}
      <span
        aria-hidden={true}
        tabIndex={-1}
        onClick={(e) => {
          e.stopPropagation();
          upd.mutate({
            id: item.id,
            date,
            startMinute: item.start_minute ?? null,
          });
        }}
        onPointerDown={(e) => e.stopPropagation()}
        className="cursor-pointer rounded p-1 text-[var(--accent-oxide)] opacity-0 transition group-hover:opacity-100 group-focus-within:opacity-100 hover:bg-sunken"
        title={t("backlog.scheduleToday")}
      >
        <CalendarPlus size={14} />
      </span>
      <span
        aria-hidden={true}
        tabIndex={-1}
        onClick={(e) => {
          e.stopPropagation();
          del.mutate(item.id);
        }}
        onPointerDown={(e) => e.stopPropagation()}
        className="cursor-pointer rounded p-1 opacity-0 transition group-hover:opacity-100 group-focus-within:opacity-100 hover:bg-sunken"
        style={{ color: "var(--text-tertiary)" }}
        title={t("common.delete")}
      >
        <Trash2 size={14} />
      </span>
      <GripVertical
        size={14}
        className="shrink-0 opacity-20 group-hover:opacity-40"
        style={{ color: "var(--text-tertiary)" }}
      />
    </li>
  );
}
