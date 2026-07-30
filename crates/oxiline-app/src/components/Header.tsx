import { ChevronLeft, ChevronRight, Search, Settings as SettingsIcon, Layers } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useTimeline, useCategories, useSettings } from "../hooks";
import { useUi } from "../lib/store";
import { OxideBar } from "./OxideBar";

function num(v: unknown, d: number): number {
  return typeof v === "number" ? v : d;
}

function localeDateLabel(dateStr: string, lang: string): string {
  const [y, m, d] = dateStr.split("-").map(Number);
  const dt = new Date(y, m - 1, d);
  const wd = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"][dt.getDay()];
  const wdKo = ["일", "월", "화", "수", "목", "금", "토"][dt.getDay()];
  if (lang === "ko") {
    return `${y}년 ${m}월 ${d}일 (${wdKo})`;
  }
  const wdEn = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][dt.getDay()];
  void wd;
  return `${wdEn}, ${dt.toLocaleDateString("en-US", { month: "short" })} ${d}`;
}

export function Header() {
  const { t, i18n } = useTranslation();
  const { date, view, setView, shiftDate, goToToday, setPaletteOpen, setPreferencesOpen, setRoutineManagerOpen } =
    useUi();
  const catsQ = useCategories();
  const tlQ = useTimeline(date);
  const settingsQ = useSettings();

  const dayStart = num(settingsQ.data?.day_start_hour, 5);
  const dayEnd = num(settingsQ.data?.day_end_hour, 26);
  const dayStartMin = dayStart * 60;
  const totalMin = (dayEnd - dayStart) * 60;

  const tabs: { key: typeof view; label: string }[] = [
    { key: "today", label: t("nav.today") },
    { key: "week", label: t("nav.week") },
    { key: "backlog", label: t("nav.backlog") },
  ];

  return (
    <div data-tauri-drag-region className="shrink-0 px-3 pt-9">
      <div className="flex items-center justify-between gap-2">
        <div className="group flex items-center gap-1">
          <button
            className="rounded p-1 opacity-0 transition group-hover:opacity-100 hover:bg-sunken"
            onClick={() => shiftDate(-1)}
            aria-label="prev day"
          >
            <ChevronLeft size={16} />
          </button>
          <button
            className="rounded px-1 text-[15px] font-semibold hover:bg-sunken"
            onClick={goToToday}
            title={t("nav.today")}
          >
            {localeDateLabel(date, i18n.language)}
          </button>
          <button
            className="rounded p-1 opacity-0 transition group-hover:opacity-100 hover:bg-sunken"
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

      <div className="mt-2 mb-2 px-1">
        <OxideBar
          items={tlQ.data ?? []}
          categories={catsQ.data ?? []}
          dayStartMin={dayStartMin}
          totalMin={totalMin}
        />
      </div>

      <div className="flex items-center gap-1 border-b border-border-subtle pb-1.5">
        {tabs.map((tb) => (
          <button
            key={tb.key}
            onClick={() => setView(tb.key)}
            className="rounded-md px-3 py-1 text-[13px] font-medium transition"
            style={{
              background: view === tb.key ? "var(--surface-sunken)" : "transparent",
              color: view === tb.key ? "var(--text-primary)" : "var(--text-secondary)",
            }}
          >
            {tb.label}
          </button>
        ))}
      </div>
    </div>
  );
}
