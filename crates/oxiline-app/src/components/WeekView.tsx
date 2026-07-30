import { useTranslation } from "react-i18next";
import { useUi, todayStr, shift } from "../lib/store";
import { useTimelineRange, useCategories } from "../hooks";
import { categoryById, categoryColorMuted } from "../lib/colors";
import type { Category, TimelineItem } from "../types";

function weekdayLabel(date: string, locale: string): string {
  const d = new Date(date + "T12:00:00");
  return d.toLocaleDateString(locale, { weekday: "short" });
}

function monthDay(date: string, locale: string): string {
  const d = new Date(date + "T12:00:00");
  return d.toLocaleDateString(locale, { month: "short", day: "numeric" });
}

function MiniDay({
  date,
  items,
  onJump,
  locale,
  categories,
  isToday,
  isLast,
}: {
  date: string;
  items: TimelineItem[];
  onJump: (d: string) => void;
  locale: string;
  categories: Category[];
  isToday: boolean;
  isLast: boolean;
}) {
  const { t } = useTranslation();
  const total = items
    .filter((i) => !i.is_skipped && i.duration_minute != null)
    .reduce((s, i) => s + (i.duration_minute ?? 0), 0);
  return (
    <button
      onClick={() => onJump(date)}
      className={`flex min-w-0 flex-1 flex-col gap-1 p-2 text-left transition-shadow${
        isToday ? " bg-sunken" : ""
      }${isLast ? "" : " border-r border-border-subtle"}`}
    >
      <div className="flex items-baseline justify-between">
        <span className="font-mono text-[11px]" style={{ color: "var(--text-tertiary)" }}>
          {weekdayLabel(date, locale)}
        </span>
        <span className="text-[11px]" style={{ color: "var(--text-secondary)" }}>
          {monthDay(date, locale)}
        </span>
      </div>
      {items.length === 0 ? (
        <span className="text-[10px]" style={{ color: "var(--text-tertiary)" }}>
          {t("week.empty")}
        </span>
      ) : (
        <div className="flex flex-col gap-0.5">
          {items.slice(0, 5).map((it) => {
            const cat = categoryById(categories, it.category_id);
            const col = categoryColorMuted(cat?.color_hue ?? null);
            return (
              <div
                key={it.id}
                className="truncate rounded-sm px-1 py-[1px] text-[10px]"
                style={{
                  background: it.is_done ? "var(--signal-success-subtle)" : col,
                  color: "var(--text-primary)",
                  textDecoration: it.is_done ? "line-through" : "none",
                  borderLeft: `2px solid ${it.is_done ? "var(--signal-success)" : col}`,
                }}
                title={it.title}
              >
                {it.start_minute != null
                  ? `${Math.floor(it.start_minute / 60).toString().padStart(2, "0")}:${(it.start_minute % 60).toString().padStart(2, "0")} `
                  : ""}
                {it.title}
              </div>
            );
          })}
          {items.length > 5 && (
            <span className="text-[10px]" style={{ color: "var(--text-tertiary)" }}>
              +{items.length - 5}
            </span>
          )}
        </div>
      )}
      <span className="mt-auto font-mono text-[10px]" style={{ color: "var(--text-tertiary)" }}>
        {t("week.workload", { n: total })}
      </span>
    </button>
  );
}

export function WeekView() {
  const { t, i18n } = useTranslation();
  const locale = i18n.language === "en" ? "en" : "ko";
  const ui = useUi();
  const today = todayStr();
  const from = shift(today, -3);
  const to = shift(today, 3);
  const tlQ = useTimelineRange(from, to);
  const catsQ = useCategories();
  const categories = catsQ.data ?? [];
  const cols = tlQ.data ?? [];
  const len = cols.length;

  return (
    <div className="flex h-full flex-col">
      <div
        className="border-b border-border-subtle px-3 py-2 text-[12px]"
        style={{ color: "var(--text-secondary)" }}
      >
        {t("week.title")}
      </div>
      <div className="flex flex-1 overflow-hidden">
        {cols.map(({ date, items }, idx) => (
          <MiniDay
            key={date}
            date={date}
            items={items}
            locale={locale}
            categories={categories}
            isToday={date === today}
            isLast={idx === len - 1}
            onJump={(d) => {
              ui.setDate(d);
              ui.setView("today");
            }}
          />
        ))}
      </div>
    </div>
  );
}
