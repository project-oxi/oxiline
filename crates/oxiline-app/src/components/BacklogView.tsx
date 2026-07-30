import { useTranslation } from "react-i18next";
import { Trash2, CalendarPlus } from "lucide-react";
import { useBacklog, useDeleteTask, useUpdateTask } from "../hooks";
import { useUi } from "../lib/store";

export function BacklogView() {
  const { t } = useTranslation();
  const q = useBacklog();
  const del = useDeleteTask();
  const upd = useUpdateTask();
  const { date } = useUi();
  const items = q.data ?? [];

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
          <li
            key={it.id}
            className="group flex items-center gap-2 rounded-md px-2 py-2 hover:bg-sunken"
          >
            <button
              onClick={() => del.mutate(it.id)}
              className="rounded p-1 opacity-0 transition group-hover:opacity-100 hover:bg-border-subtle"
              aria-label={t("task.delete")}
            >
              <Trash2 size={14} style={{ color: "var(--text-tertiary)" }} />
            </button>
            <span
              className="flex-1 truncate text-[13px]"
              style={{
                textDecoration: "line-through",
                opacity: 0.6,
              }}
              hidden={!it.is_done}
            >
              {it.title}
            </span>
            <span
              className="flex-1 truncate text-[13px]"
              hidden={it.is_done}
            >
              {it.title}
            </span>
            <button
              className="rounded p-1 opacity-0 transition group-hover:opacity-100 hover:bg-border-subtle"
              aria-label={t("backlog.scheduleToday")}
              title={t("backlog.scheduleToday")}
              onClick={() =>
                upd.mutate({
                  id: it.id,
                  date,
                  startMinute: it.start_minute ?? null,
                })
              }
            >
              <CalendarPlus size={14} style={{ color: "var(--accent-oxide)" }} />
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
