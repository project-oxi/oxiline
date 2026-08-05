import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import {
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Play,
  Search,
  Settings as SettingsIcon,
} from "lucide-react";
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

const WD_KO = ["일", "월", "화", "수", "목", "금", "토"];

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
  const anchorRef = useRef<HTMLButtonElement>(null);
  const popBoxRef = useRef<HTMLDivElement>(null);
  const [popPos, setPopPos] = useState<{ left: number; top: number } | null>(null);

  useEffect(() => setCalMonth(date), [date]);

  // Calendar popover: rendered into a body portal (outside the window
  // drag-region) and pinned under the date button.
  useLayoutEffect(() => {
    if (!calOpen) {
      setPopPos(null);
      return;
    }
    const place = () => {
      const r = anchorRef.current?.getBoundingClientRect();
      if (!r) return;
      const left = Math.min(r.left, window.innerWidth - 268 - 8);
      setPopPos({ left: Math.max(8, left), top: r.bottom + 6 });
    };
    place();
    const onDown = (e: PointerEvent) => {
      const target = e.target as Node;
      if (anchorRef.current?.contains(target)) return; // toggle button handles it
      if (popBoxRef.current?.contains(target)) return;
      setCalOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setCalOpen(false);
    };
    window.addEventListener("resize", place);
    window.addEventListener("pointerdown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("resize", place);
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
  const wdKo = WD_KO[titleDt.getDay()];
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
    <div className="shrink-0 select-none border-b border-border bg-surface-raised px-4 pt-1.5">
      {/* Command row — overlaps native traffic lights (Overlay style). The row
          is the window drag region; interactive controls are drag-free islands. */}
      <div data-tauri-drag-region className="flex h-11 items-center gap-3 pl-[56px]">
        {/* Date masthead: ‹ 8월 5일 화 ⌄ › */}
        <div data-tauri-drag-region className="flex shrink-0 items-center gap-0.5">
          <button
            className="rounded-md p-1 text-text-subtle transition hover:bg-surface-sunken hover:text-text"
            onClick={() => shiftDate(-1)}
            aria-label="prev day"
          >
            <ChevronLeft size={16} />
          </button>
          <button
            ref={anchorRef}
            onClick={() => setCalOpen((v) => !v)}
            className="group flex items-center gap-1.5 rounded-lg px-2 py-1.5 transition hover:bg-surface-sunken"
            aria-haspopup="dialog"
            aria-expanded={calOpen}
          >
            <span className="text-[17px] font-bold tracking-tight text-text tabular-nums">
              {lang === "ko"
                ? `${mm}월 ${dd}일`
                : titleDt.toLocaleDateString("en-US", { month: "short", day: "numeric" })}
            </span>
            <span className="text-[12px] font-semibold text-text-muted">
              {lang === "ko" ? wdKo : titleDt.toLocaleDateString("en-US", { weekday: "short" })}
            </span>
            {yy !== Number(today.slice(0, 4)) && (
              <span className="text-[11px] font-medium text-text-subtle">{yy}</span>
            )}
            <ChevronDown
              size={13}
              className="text-text-subtle transition group-hover:text-text-muted"
              aria-hidden="true"
            />
          </button>
          <button
            className="rounded-md p-1 text-text-subtle transition hover:bg-surface-sunken hover:text-text"
            onClick={() => shiftDate(1)}
            aria-label="next day"
          >
            <ChevronRight size={16} />
          </button>
        </div>

        {/* Week strip — one chip per day: weekday · number · micro oxide bar */}
        <div data-tauri-drag-region className="flex min-w-0 flex-1 items-center gap-1">
          {weekDates.map((dStr) => (
            <DayChip
              key={dStr}
              dateStr={dStr}
              lang={lang}
              isToday={dStr === today}
              isSelected={dStr === date}
              onSelect={() => setDate(dStr)}
              records={weekRecs}
              hueById={hueById}
              dayStartMin={dayStartMin}
              totalMin={totalMin}
            />
          ))}
        </div>

        {/* Right cluster: record hero + palette + settings */}
        <div data-tauri-drag-region className="flex shrink-0 items-center gap-1.5">
          <button
            onClick={handleRecordToggle}
            className={`flex h-7 items-center gap-1.5 rounded-full px-3 text-[12px] font-semibold transition hover:opacity-90 ${
              active
                ? "bg-status-error text-text-inverse"
                : "bg-interactive-primary text-interactive-primary-foreground"
            }`}
            title={active ? "멈춤 (⌘⇧R)" : "녹화 시작 (⌘⇧R)"}
            aria-label={active ? "멈춤" : "녹화 시작"}
          >
            {active ? (
              <>
                <span className="relative flex h-1.5 w-1.5" aria-hidden="true">
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-text-inverse opacity-60" />
                  <span className="relative inline-flex h-1.5 w-1.5 rounded-full bg-text-inverse" />
                </span>
                <span className="font-mono tabular-nums">{hmm(active.elapsed_seconds)}</span>
              </>
            ) : (
              <>
                <Play size={12} fill="currentColor" />
                <span>{lang === "en" ? "Record" : "녹화"}</span>
              </>
            )}
          </button>
          <button
            className="rounded-md p-1.5 text-text-muted transition hover:bg-surface-sunken hover:text-text"
            onClick={() => setPaletteOpen(true)}
            aria-label="⌘K"
            title="⌘K"
          >
            <Search size={16} />
          </button>
          <button
            className="rounded-md p-1.5 text-text-muted transition hover:bg-surface-sunken hover:text-text"
            onClick={() => setPreferencesOpen(true)}
            aria-label={t("settings.title")}
            title={t("settings.title")}
          >
            <SettingsIcon size={16} />
          </button>
        </div>
      </div>

      {/* Oxide strip — the day compressed; click to scroll the timeline. */}
      <div className="pb-2 pt-1.5">
        <OxideBar
          records={dayRecs}
          activities={actsQ.data ?? []}
          dayStartMin={dayStartMin}
          totalMin={totalMin}
          onClickMinute={(m) => requestScroll(m)}
          showNow={date === today}
          height={10}
        />
      </div>

      {calOpen &&
        popPos &&
        createPortal(
          <div
            ref={popBoxRef}
            className="date-popover w-[268px]"
            role="dialog"
            aria-label="날짜 선택"
            style={{ position: "fixed", left: popPos.left, top: popPos.top }}
          >
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
          </div>,
          document.body,
        )}
    </div>
  );
}

/** One week-strip chip: weekday label, day number, and a micro oxide bar —
 * the day's records compressed into a 3px strip (identity-consistent with the
 * header oxide bar, replacing scattered hue dots). */
function DayChip({
  dateStr,
  lang,
  isToday,
  isSelected,
  onSelect,
  records,
  hueById,
  dayStartMin,
  totalMin,
}: {
  dateStr: string;
  lang: "ko" | "en";
  isToday: boolean;
  isSelected: boolean;
  onSelect: () => void;
  records: ActivityRecord[];
  hueById: Map<string, string | null>;
  dayStartMin: number;
  totalMin: number;
}) {
  const [cy, cm, cdd] = dateStr.split("-").map(Number);
  const cdt = new Date(cy, cm - 1, cdd);
  const wdLabel =
    lang === "ko" ? WD_KO[cdt.getDay()] : cdt.toLocaleDateString("en-US", { weekday: "narrow" });

  const segs = useMemo(() => {
    const nowMin = new Date().getHours() * 60 + new Date().getMinutes();
    const out: { left: number; width: number; color: string }[] = [];
    for (const r of records) {
      const loc = isoLocal(r.started_at);
      if (loc.date !== dateStr) continue;
      const start = loc.minute;
      const end = r.ended_at ? isoLocal(r.ended_at).minute : nowMin;
      out.push({
        left: ((start - dayStartMin) / totalMin) * 100,
        width: (Math.max(1, end - start) / totalMin) * 100,
        color: hueVar(hueById.get(r.activity_id) ?? null),
      });
    }
    return out;
  }, [records, dateStr, hueById, dayStartMin, totalMin]);

  return (
    <button
      onClick={onSelect}
      className={`flex h-8 min-w-0 flex-1 items-center justify-center gap-1.5 rounded-lg px-1 transition ${
        isSelected
          ? "bg-interactive-primary shadow-[var(--shadow-today-node)]"
          : isToday
            ? "bg-interactive-primary-subtle"
            : "hover:bg-surface-sunken"
      }`}
      aria-label={dateStr}
      aria-current={isToday ? "date" : undefined}
      aria-pressed={isSelected}
    >
      <span
        className={`text-[10px] font-semibold ${
          isSelected
            ? "text-interactive-primary-foreground/70"
            : isToday
              ? "text-interactive-primary"
              : "text-text-subtle"
        }`}
      >
        {wdLabel}
      </span>
      <span
        className={`text-[12px] font-semibold tabular-nums ${
          isSelected
            ? "text-interactive-primary-foreground"
            : isToday
              ? "text-interactive-primary"
              : "text-text-muted"
        }`}
      >
        {cdt.getDate()}
      </span>
      <span
        className={`relative hidden h-[3px] w-6 shrink-0 overflow-hidden rounded-full sm:block ${
          isSelected ? "bg-interactive-primary-foreground/25" : "bg-surface-sunken"
        }`}
        aria-hidden="true"
      >
        {segs.map((s, i) => (
          <span
            key={i}
            className="absolute top-0 h-full"
            style={{
              left: `${Math.max(0, s.left)}%`,
              width: `${Math.max(2, s.width)}%`,
              background: s.color,
            }}
          />
        ))}
      </span>
    </button>
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
