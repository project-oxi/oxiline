import { useEffect, useRef, useState } from "react";
import { ChevronLeft, ChevronRight, Play, Search, Settings as SettingsIcon, Square } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  useActivities,
  useRecordState,
  useRecordsRange,
  useSettings,
  useStartRecord,
  useStopRecord,
} from "../hooks";
import { useUi, todayStr, shift } from "../lib/store";
import { hmm, hueVar } from "../lib/record-format";
import { isoLocal } from "../lib/record-time";
import type { ActivityRecord } from "../types";
import { monthBounds, monthGrid, shiftMonth } from "../lib/calendar";
import { OxideBar } from "./OxideBar";

export function Header() {
  const { t, i18n } = useTranslation();
  const {
    date,
    setDate,
    shiftDate,
    setPaletteOpen,
    setPreferencesOpen,
    setSwitcherOpen,
    lastActivityId,
    requestScroll,
  } = useUi();
  const actsQ = useActivities(false);
  const settingsQ = useSettings();
  const stateQ = useRecordState();
  const startRec = useStartRecord();
  const stopRec = useStopRecord();
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
  const today = todayStr();
  const bounds = monthBounds(calMonth);
  const weekDates = Array.from({ length: 7 }, (_, i) => shift(monday, i));
  const weekRecs = useRecordsRange(monday, sunday).data ?? [];
  const monthRecs = useRecordsRange(bounds.from, bounds.to).data ?? [];
  const hueById = new Map((actsQ.data ?? []).map((a) => [a.id, a.hue_label] as const));
  const [calYy, calMm] = calMonth.split("-").map(Number);

  // Date title parts
  const [yy, mm, dd] = date.split("-").map(Number);
  const titleDt = new Date(yy, mm - 1, dd);
  const wdKo = ["일", "월", "화", "수", "목", "금", "토"][titleDt.getDay()];
  // Oxide Bar (day minimap) — reflects the selected date. day_start/end from
  // settings; fall back to 05:00–26:00 like the timeline.
  const dayStart =
    typeof settingsQ.data?.day_start_hour === "number" ? settingsQ.data.day_start_hour : 5;
  const dayEnd =
    typeof settingsQ.data?.day_end_hour === "number" ? settingsQ.data.day_end_hour : 26;
  const dayStartMin = dayStart * 60;
  const totalMin = (dayEnd - dayStart) * 60;
  const dayRecs = weekRecs.filter((r) => isoLocal(r.started_at).date === date);
  const active = stateQ.data?.active ?? null;

  function handleRecordToggle() {
    if (active) {
      stopRec.mutate();
    } else if (lastActivityId) {
      startRec.mutate(lastActivityId);
    } else {
      setSwitcherOpen(true);
    }
  }

  return (
    <div className="shrink-0 select-none border-b border-border bg-surface-raised px-4 pb-2 pt-2">
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
              <span className="text-[13px] text-text-muted" aria-hidden="true">⌄</span>
            </button>
            {calOpen && (
              <div className="date-popover w-[268px]" role="dialog" aria-label="날짜 선택">
                <div className="mb-2 flex items-center gap-1">
                  <button className="rounded px-2 py-1 text-text-muted hover:bg-surface-sunken" onClick={() => setCalMonth(shiftMonth(calMonth, -1))} aria-label="이전 달">‹</button>
                  <span className="min-w-0 flex-1 text-center text-[13px] font-semibold text-text">{calYy}년 {calMm}월</span>
                  <button className="rounded px-2 py-1 text-text-muted hover:bg-surface-sunken" onClick={() => setCalMonth(shiftMonth(calMonth, 1))} aria-label="다음 달">›</button>
                  <button className="ml-1 rounded px-2 py-1 text-[11px] font-medium text-interactive-primary hover:bg-surface-sunken" onClick={() => { setCalMonth(today); setDate(today); setCalOpen(false); }}>오늘</button>
                </div>
                <div className="date-popover-grid mb-1 text-center text-[10px] font-semibold text-text-subtle">
                  {["월", "화", "수", "목", "금", "토", "일"].map((label) => <span key={label}>{label}</span>)}
                </div>
                <div className="date-popover-grid">
                  {monthGrid(calMonth).map((cell) => {
                    const hues = dateHues(monthRecs, cell, hueById);
                    const isOtherMonth = cell.slice(0, 7) !== calMonth.slice(0, 7);
                    const isToday = cell === today;
                    const isSelected = cell === date;
                    return <button key={cell} className={`date-popover-cell transition hover:bg-surface-sunken ${isOtherMonth ? "text-text-subtle/40" : "text-text-muted"} ${isToday ? "bg-interactive-primary text-interactive-primary-foreground" : ""} ${isSelected ? "ring-2 ring-interactive-primary ring-offset-1 ring-offset-surface-raised" : ""}`} onClick={() => { setDate(cell); setCalOpen(false); }} aria-label={cell} aria-current={isToday ? "date" : undefined}>
                      <span>{Number(cell.slice(8, 10))}</span>
                      <span className="flex h-1.5 items-center gap-0.5">{hues.map((hue, index) => <span key={index} className="h-1 w-1 rounded-full" style={{ background: hueVar(hue) }} />)}</span>
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
            onClick={handleRecordToggle}
            className={`flex items-center gap-1.5 rounded px-2 py-1 text-[12px] font-medium transition hover:bg-surface-sunken ${
              active ? "text-status-error" : "text-text-muted"
            }`}
            title={active ? "멈춤 (⌘⇧R)" : "녹화 시작 (⌘⇧R)"}
            aria-label={active ? "멈춤" : "녹화 시작"}
          >
            {active ? (
              <>
                <Square size={13} fill="currentColor" />
                <span className="font-mono tabular-nums">{hmm(active.elapsed_seconds)}</span>
              </>
            ) : (
              <>
                <Play size={13} fill="currentColor" />
                <span>{lang === "en" ? "Record" : "녹화"}</span>
              </>
            )}
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
        {weekDates.map((dStr) => {
          const [cy, cm, cdd] = dStr.split("-").map(Number);
          const cdt = new Date(cy, cm - 1, cdd);
          const dayNum = cdt.getDate();
          const wdLabel =
            lang === "ko"
              ? ["일", "월", "화", "수", "목", "금", "토"][cdt.getDay()]
              : cdt.toLocaleDateString("en-US", { weekday: "narrow" });
          const isToday = dStr === today;
          const hues = dateHues(weekRecs, dStr, hueById);
          return (
            <button
              key={dStr}
              onClick={() => {
                setDate(dStr);
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
                  <span key={i} className="h-1 w-1 rounded-full" style={{ background: hueVar(h) }} />
                ))}
              </span>
            </button>
          );
        })}
      </div>

      {/* Row 3: Oxide Bar — day minimap (signature visual, §6.6). Click a
          position to scroll the timeline there. */}
      <div className="px-1 pt-0.5">
        <OxideBar
          records={dayRecs}
          activities={actsQ.data ?? []}
          dayStartMin={dayStartMin}
          totalMin={totalMin}
          onClickMinute={(m) => requestScroll(m)}
          showNow={date === today}
        />
      </div>
    </div>
  );
}

function dateHues(
  records: ActivityRecord[],
  dateStr: string,
  hueById: Map<string, string | null>,
): (string | null)[] {
  const hues = new Set<string | null>();
  for (const r of records) {
    if (isoLocal(r.started_at).date === dateStr) hues.add(hueById.get(r.activity_id) ?? null);
  }
  return [...hues].slice(0, 5);
}
