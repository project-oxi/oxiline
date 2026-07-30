import { useTranslation } from "react-i18next";
import { Trash2, CalendarPlus, GripVertical } from "lucide-react";
import { useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import { useBacklog, useDeleteTask, useUpdateTask, useSetTaskDone } from "../hooks";
import { announce } from "../lib/a11y";
import { useUi } from "../lib/store";

export function BacklogView() {
  const { t } = useTranslation();
  const q = useBacklog();
  const { date } = useUi();
  const items = q.data ?? [];
  const del = useDeleteTask();

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
      <ul className="space-y-1">
        {items.map((it) => (
          <DraggableBacklogRow key={it.id} item={it} date={date} del={del} />
        ))}
      </ul>
    </div>
  );
}

function DraggableBacklogRow({
  item,
  date,
  del,
}: {
  item: { id: string; title: string; is_done: boolean; start_minute?: number | null };
  date: string;
  del: { mutate: (id: string) => void };
}) {
  const upd = useUpdateTask();
  const { t } = useTranslation();
  const setDone = useSetTaskDone();
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: `backlog:${item.id}`,
    data: { kind: "backlog", task: item },
  });
  // The dnd container is the single focusable unit (§7.10: Enter toggles
  // done, Backspace/Delete removes).
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
      className="group flex items-center gap-2 rounded-md px-2 py-2 hover:bg-sunken"
      style={{
        transform: CSS.Translate.toString(transform),
        opacity: isDragging ? 0.5 : 1,
        cursor: "grab",
      }}
      aria-label={`${item.title}${item.is_done ? `, ${t("a11y.done")}` : ""}`}
      aria-describedby={undefined}
      onKeyDown={onKeyDown}
    >
      <GripVertical size={14} className="shrink-0 opacity-30" style={{ color: "var(--text-tertiary)" }} />
      <button
        tabIndex={-1}
        onClick={(e) => { e.stopPropagation(); del.mutate(item.id); }}
        className="rounded p-1 opacity-0 transition group-hover:opacity-100 group-focus-within:opacity-100 hover:bg-border-subtle"
        aria-label={t("common.delete")}
        onPointerDown={(e) => e.stopPropagation()}
      >
        <Trash2 size={14} style={{ color: "var(--text-tertiary)" }} />
      </button>
      <span
        className="flex-1 truncate text-[13px]"
        style={{
          textDecoration: item.is_done ? "line-through" : "none",
          opacity: item.is_done ? 0.6 : 1,
        }}
      >
        {item.title}
      </span>
      <button
        tabIndex={-1}
        className="rounded p-1 opacity-0 transition group-hover:opacity-100 group-focus-within:opacity-100 hover:bg-border-subtle"
        aria-label={t("backlog.scheduleToday")}
        onClick={(e) => {
          e.stopPropagation();
          upd.mutate({
            id: item.id,
            date,
            startMinute: item.start_minute ?? null,
          });
        }}
        onPointerDown={(e) => e.stopPropagation()}
      >
        <CalendarPlus size={14} style={{ color: "var(--accent-oxide)" }} />
      </button>
    </li>
  );
}
