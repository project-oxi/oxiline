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
    <div data-tauri-drag-region className="shrink-0 px-4 pt-9 pb-2">
      {/* Row 1: chevrons + big date title + icons */}
      <div className="flex items-center justify-between gap-2 pb-2.5">
        <div className="flex items-center gap-1">
          <button
            className="rounded p-1 opacity-40 transition hover:bg-sunken hover:opacity-100"
            onClick={() => shiftDate(-1)}
            aria-label="prev day"
          >
            <ChevronLeft size={16} />
          </button>
          <button
            onClick={goToToday}
            className="flex items-baseline gap-1.5 rounded px-1 hover:bg-sunken"
            title={t("nav.today")}
          >
            <span className="text-[18px] font-semibold" style={{ color: "var(--accent-oxide-strong)" }}>
              {yy}
            </span>
            <span className="text-[21px] font-bold tracking-tight" style={{ color: "var(--text-primary)" }}>
              {lang === "ko"
                ? `${mm}월 ${dd}일`
                : titleDt.toLocaleDateString("en-US", { month: "short", day: "numeric" })}
            </span>
            <span className="text-[16px] font-medium" style={{ color: "var(--text-tertiary)" }}>
              {lang === "ko" ? wdKo : titleDt.toLocaleDateString("en-US", { weekday: "short" })}
            </span>
          </button>
          <button
            className="rounded p-1 opacity-40 transition hover:bg-sunken hover:opacity-100"
            onClick={() => shiftDate(1)}
            aria-label="next day"
          >
            <ChevronRight size={16} />
          </button>
        </div>

        <div className="flex items-center gap-1">
          <button
            className="rounded p-1.5 hover:bg-sunken"
            onClick={() => setRoutineManagerOpen(true)}
            aria-label={t("routine.title")}
            title={t("routine.title")}
          >
            <Layers size={16} />
          </button>
          <button
            className="rounded p-1.5 hover:bg-sunken"
            onClick={() => setPaletteOpen(true)}
            aria-label="⌘K"
            title="⌘K"
          >
            <Search size={16} />
          </button>
          <button
            className="rounded p-1.5 hover:bg-sunken"
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
              className="flex flex-1 flex-col items-center gap-1 rounded-lg py-1.5 transition hover:bg-sunken"
            >
              <span
                className="text-[10px] font-semibold"
                style={{ color: isToday ? "var(--accent-oxide-strong)" : "var(--text-tertiary)" }}
              >
                {wdLabel}
              </span>
              <span
                className="flex h-7 w-7 items-center justify-center rounded-full text-[13px] font-semibold transition"
                style={{
                  background: isToday ? "var(--accent-oxide)" : "transparent",
                  color: isToday ? "white" : "var(--text-secondary)",
                  boxShadow: isToday ? "0 2px 8px oklch(0.62 0.1 189 / 0.35)" : undefined,
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
      <div className="flex gap-0.5 rounded-lg p-0.5" style={{ background: "var(--surface-sunken)" }}>
        {tabs.map((tb) => (
          <button
            key={tb.key}
            onClick={() => setView(tb.key)}
            className="flex-1 rounded-md px-3 py-1.5 text-[13px] font-semibold transition"
            style={{
              background: view === tb.key ? "var(--surface-raised)" : "transparent",
              color: view === tb.key ? "var(--text-primary)" : "var(--text-secondary)",
              boxShadow: view === tb.key ? "var(--elevation-card)" : undefined,
            }}
          >
            {tb.label}
          </button>
        ))}
      </div>
    </div>
  );
}
