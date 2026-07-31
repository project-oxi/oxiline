import { ChevronLeft, ChevronRight, Search, Settings as SettingsIcon, Layers } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useTimelineRange, useCategories } from "../hooks";
import { useUi, todayStr, shift } from "../lib/store";
import { categoryById, categoryColor } from "../lib/colors";

export function Header() {
  const { t, i18n } = useTranslation();
  const { date, view, setView, setDate, shiftDate, goToToday, setPaletteOpen, setPreferencesOpen, setRoutineManagerOpen } =
    useUi();
  const catsQ = useCategories();
  const lang = i18n.language?.startsWith("en") ? "en" : "ko";

  // Week (Mon–Sun) containing the selected date
  const dow = new Date(date + "T12:00:00").getDay();
  const mondayOffset = dow === 0 ? -6 : 1 - dow;
  const monday = shift(date, mondayOffset);
  const sunday = shift(monday, 6);
  const weekQ = useTimelineRange(monday, sunday);
  const weekCols = weekQ.data ?? [];
  const categories = catsQ.data ?? [];
  const today = todayStr();

  const tabs: { key: typeof view; label: string }[] = [
    { key: "today", label: t("nav.today") },
    { key: "week", label: t("nav.week") },
    { key: "backlog", label: t("nav.backlog") },
    { key: "report", label: t("nav.report") },
  ];

  // Date title parts
  const [yy, mm, dd] = date.split("-").map(Number);
  const titleDt = new Date(yy, mm - 1, dd);
  const wdKo = ["일", "월", "화", "수", "목", "금", "토"][titleDt.getDay()];

  return (
    <div className="shrink-0 select-none px-4 pb-2">
      {/* Titlebar strip — overlaps native traffic lights (Overlay style).
          The strip is the window drag region; interactive buttons are drag-free islands. */}
      <div data-tauri-drag-region className="flex items-center justify-between gap-2 py-1.5 pl-[56px]">
        <div data-tauri-drag-region className="flex items-center gap-1">
          <button
            className="rounded p-1 opacity-40 transition hover:bg-surface-sunken hover:opacity-100"
            onClick={() => shiftDate(-1)}
            aria-label="prev day"
          >
            <ChevronLeft size={16} />
          </button>
          <button
            onClick={goToToday}
            className="flex items-baseline gap-1.5 rounded px-1 hover:bg-surface-sunken"
            title={t("nav.today")}
          >
            <span className="max-[379px]:hidden text-[18px] font-semibold text-interactive-primary">
              {yy}
            </span>
            <span className="text-[21px] font-bold tracking-tight font-display text-text">
              {lang === "ko"
                ? `${mm}월 ${dd}일`
                : titleDt.toLocaleDateString("en-US", { month: "short", day: "numeric" })}
            </span>
            <span className="text-[16px] font-medium text-text-subtle">
              {lang === "ko" ? wdKo : titleDt.toLocaleDateString("en-US", { weekday: "short" })}
            </span>
          </button>
          <button
            className="rounded p-1 opacity-40 transition hover:bg-surface-sunken hover:opacity-100"
            onClick={() => shiftDate(1)}
            aria-label="next day"
          >
            <ChevronRight size={16} />
          </button>
        </div>

        <div data-tauri-drag-region className="flex items-center gap-1">
          <button
            className="rounded p-1.5 hover:bg-surface-sunken"
            onClick={() => setRoutineManagerOpen(true)}
            aria-label={t("routine.title")}
            title={t("routine.title")}
          >
            <Layers size={16} />
          </button>
          <button
            className="rounded p-1.5 hover:bg-surface-sunken"
            onClick={() => setPaletteOpen(true)}
            aria-label="⌘K"
            title="⌘K"
          >
            <Search size={16} />
          </button>
          <button
            className="rounded p-1.5 hover:bg-surface-sunken"
            onClick={() => setPreferencesOpen(true)}
            aria-label={t("settings.title")}
            title={t("settings.title")}
          >
            <SettingsIcon size={16} />
          </button>
        </div>
      </div>

      {/* Row 2: week strip */}
      <div className="flex gap-1 pb-2.5">
        {weekCols.map(({ date: dStr, items }) => {
          const [cy, cm, cdd] = dStr.split("-").map(Number);
          const cdt = new Date(cy, cm - 1, cdd);
          const dayNum = cdt.getDate();
          const wdLabel =
            lang === "ko"
              ? ["일", "월", "화", "수", "목", "금", "토"][cdt.getDay()]
              : cdt.toLocaleDateString("en-US", { weekday: "narrow" });
          const isToday = dStr === today;
          const hues = [
            ...new Set(
              items
                .filter((i) => !i.is_skipped)
                .map((i) => categoryById(categories, i.category_id)?.color_hue ?? null),
            ),
          ].slice(0, 5);
          return (
            <button
              key={dStr}
              onClick={() => {
                setDate(dStr);
                setView("today");
              }}
              className="flex flex-1 flex-col items-center gap-1 rounded-lg py-1.5 transition hover:bg-surface-sunken"
            >
              <span
                className={`text-[10px] font-semibold ${isToday ? "text-interactive-primary" : "text-text-subtle"}`}
              >
                {wdLabel}
              </span>
              <span
                className="flex h-7 w-7 items-center justify-center rounded-full text-[13px] font-semibold transition"
                style={{
                  background: isToday ? "var(--color-interactive-primary)" : "transparent",
                  color: isToday
                    ? "var(--color-interactive-primary-foreground)"
                    : "var(--color-text-muted)",
                  boxShadow: isToday
                    ? "0 2px 8px color-mix(in oklch, var(--color-interactive-primary) 35%, transparent)"
                    : undefined,
                }}
              >
                {dayNum}
              </span>
              <span className="flex h-1.5 items-center gap-0.5">
                {hues.map((h, i) => (
                  <span key={i} className="h-1 w-1 rounded-full" style={{ background: categoryColor(h) }} />
                ))}
              </span>
            </button>
          );
        })}
      </div>

      {/* Row 3: segmented tabs */}
      <div className="flex gap-0.5 rounded-lg bg-surface-sunken p-0.5">
        {tabs.map((tb) => (
          <button
            key={tb.key}
            onClick={() => setView(tb.key)}
            className="flex-1 rounded-md px-3 py-1.5 text-[13px] font-semibold transition"
            style={{
              background: view === tb.key ? "var(--color-surface-raised)" : "transparent",
              color: view === tb.key ? "var(--color-text)" : "var(--color-text-muted)",
              boxShadow: view === tb.key ? "var(--shadow-sm)" : undefined,
            }}
          >
            {tb.label}
          </button>
        ))}
      </div>
    </div>
  );
}
