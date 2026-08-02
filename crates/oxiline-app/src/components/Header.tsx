import { useEffect, useRef, useState } from "react";
import { ChevronLeft, ChevronRight, Search, Settings as SettingsIcon, Layers } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useTimelineRange, useCategories } from "../hooks";
import { useUi, todayStr, shift } from "../lib/store";
import { categoryById, categoryColor } from "../lib/colors";
import { monthBounds, monthGrid, shiftMonth } from "../lib/calendar";

export function Header() {
  const { t, i18n } = useTranslation();
  const { date, view, setView, setDate, shiftDate, setPaletteOpen, setPreferencesOpen, setRoutineManagerOpen } = useUi();
  const catsQ = useCategories();
  const lang = i18n.language?.startsWith("en") ? "en" : "ko";

  const [calOpen, setCalOpen] = useState(false);
  const [calMonth, setCalMonth] = useState(date);
  const popRef = useRef<HTMLDivElement>(null);
  useEffect(() => setCalMonth(date), [date]);
  useEffect(() => {
    if (!calOpen) return;
    const onDown = (e: PointerEvent) => {
      if (popRef.current && !popRef.current.contains(e.target as Node)) setCalOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setCalOpen(false);
    };
    window.addEventListener("pointerdown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointerdown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [calOpen]);
  // Week (Mon–Sun) containing the selected date
  const dow = new Date(date + "T12:00:00").getDay();
  const mondayOffset = dow === 0 ? -6 : 1 - dow;
  const monday = shift(date, mondayOffset);
  const sunday = shift(monday, 6);
  const weekQ = useTimelineRange(monday, sunday);
  const weekCols = weekQ.data ?? [];
  const categories = catsQ.data ?? [];
  const today = todayStr();
  const bounds = monthBounds(calMonth);
  const monthQ = useTimelineRange(bounds.from, bounds.to);
  const byDate = new Map((monthQ.data ?? []).map((column) => [column.date, column.items] as const));
  const [calYy, calMm] = calMonth.split("-").map(Number);

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
    <div className="shrink-0 select-none px-4 pb-2 pt-2">
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
          <div ref={popRef} className="relative">
            <button
              onClick={() => setCalOpen((v) => !v)}
              className="flex items-baseline gap-1.5 rounded px-1 hover:bg-surface-sunken"
              aria-haspopup="dialog"
              aria-expanded={calOpen}
            >
              <span className="text-[18px] font-semibold tracking-tight text-text">
                {lang === "ko" ? `${mm}월 ${dd}일` : titleDt.toLocaleDateString("en-US", { month: "short", day: "numeric" })}
              </span>
              <span className="text-[12px] font-medium text-text-muted">
                {lang === "ko" ? wdKo + "요일" : titleDt.toLocaleDateString("en-US", { weekday: "short" })}
              </span>
              {yy !== Number(today.slice(0, 4)) && <span className="text-[11px] font-medium text-text-subtle">{yy}</span>}
              <span className="text-[10px] text-text-subtle" aria-hidden="true">⌄</span>
            </button>
            {calOpen && (
              <div className="date-popover w-[268px]" role="dialog" aria-label="날짜 선택">
                <div className="mb-2 flex items-center gap-1">
                  <button className="rounded px-2 py-1 text-text-muted hover:bg-surface-sunken" onClick={() => setCalMonth(shiftMonth(calMonth, -1))} aria-label="이전 달">‹</button>
                  <span className="min-w-0 flex-1 text-center text-[13px] font-semibold text-text">{calYy}년 {calMm}월</span>
                  <button className="rounded px-2 py-1 text-text-muted hover:bg-surface-sunken" onClick={() => setCalMonth(shiftMonth(calMonth, 1))} aria-label="다음 달">›</button>
                  <button className="ml-1 rounded px-2 py-1 text-[11px] font-medium text-interactive-primary hover:bg-surface-sunken" onClick={() => { setCalMonth(today); setDate(today); setView("today"); setCalOpen(false); }}>오늘</button>
                </div>
                <div className="date-popover-grid mb-1 text-center text-[10px] font-semibold text-text-subtle">
                  {["월", "화", "수", "목", "금", "토", "일"].map((label) => <span key={label}>{label}</span>)}
                </div>
                <div className="date-popover-grid">
                  {monthGrid(calMonth).map((cell) => {
                    const hues = [...new Set((byDate.get(cell) ?? []).filter((item) => !item.is_skipped).map((item) => categoryById(categories, item.category_id)?.color_hue ?? null))].slice(0, 5);
                    const isOtherMonth = cell.slice(0, 7) !== calMonth.slice(0, 7);
                    const isToday = cell === today;
                    const isSelected = cell === date;
                    return <button key={cell} className={`date-popover-cell transition hover:bg-surface-sunken ${isOtherMonth ? "text-text-subtle/40" : "text-text-muted"} ${isToday ? "bg-interactive-primary text-interactive-primary-foreground" : ""} ${isSelected ? "ring-2 ring-interactive-primary ring-offset-1 ring-offset-surface-raised" : ""}`} onClick={() => { setDate(cell); setView("today"); setCalOpen(false); }} aria-label={cell} aria-current={isToday ? "date" : undefined}>
                      <span>{Number(cell.slice(8, 10))}</span>
                      <span className="flex h-1.5 items-center gap-0.5">{hues.map((hue, index) => <span key={index} className="h-1 w-1 rounded-full" style={{ background: categoryColor(hue) }} />)}</span>
                    </button>;
                  })}
                </div>
              </div>
            )}
          </div>
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
                className={`flex h-7 w-7 items-center justify-center rounded-full text-[13px] font-semibold transition ${
                  isToday
                    ? "bg-interactive-primary text-interactive-primary-foreground shadow-[var(--shadow-today-node)]"
                    : "text-text-muted"
                }`}
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

      {/* Row 3: underline tabs (DESIGN.md §6.10) */}
      <div role="tablist" className="flex gap-1 border-b border-line">
        {tabs.map((tb) => {
          const on = view === tb.key;
          return (
            <button
              key={tb.key}
              role="tab"
              aria-selected={on}
              onClick={() => setView(tb.key)}
              className={`-mb-px border-b-2 px-3 py-2 text-[13px] transition ${
                on
                  ? "border-interactive-primary text-text font-semibold"
                  : "border-transparent text-text-muted font-medium hover:text-text"
              }`}
            >
              {tb.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}
