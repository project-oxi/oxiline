/**
 * RecordTimeline (Plan 2 Task 3) — the two-lane recording timetable.
 *
 * Mode toggle `[계획|실제|둘 다]` over a shared local-wall-clock hour axis:
 *   - 계획  : plan lane only — `PlanSlot` choice groups (dashed/hollow options,
 *            OR marker, picked→executed when resolved).
 *   - 실제  : actual lane only — recorded `ActivityRecord` blocks (solid/filled)
 *            + the now-line for the live session.
 *   - 둘 다 : both lanes side by side (plan | actual) per the converged mockup.
 *
 * Records are UTC instants; positions use LOCAL minute-of-day. Plans already
 * store local minutes. No `is_done` anywhere — completion = a record existing.
 */
import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { useDroppable } from "@dnd-kit/core";
import {
  useActivities,
  useCreateActivity,
  useCreatePlan,
  useDayRecords,
  useResizePlan,
  useSettings,
  useSlots,
} from "../hooks";
import { todayStr, useUi } from "../lib/store";
import { snapMinute, SNAP_MINUTES } from "../lib/dnd";
import { resizeDuration } from "../lib/resize";
import type { ActivityRecord, PlanSlot } from "../types";

type Mode = "plan" | "act" | "both";

const PX_PER_MIN = 64 / 60;

function num(v: unknown, d: number): number {
  return typeof v === "number" ? v : d;
}

function hueVar(hue: string | null): string {
  return hue ? `var(--color-hue-${hue})` : "var(--color-interactive-primary)";
}

/** ISO UTC instant → local {date, minute-of-day}. */
function isoLocal(iso: string): { date: string; minute: number } {
  const d = new Date(iso);
  return {
    date: `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(
      d.getDate(),
    ).padStart(2, "0")}`,
    minute: d.getHours() * 60 + d.getMinutes(),
  };
}

function hhmm(min: number): string {
  const h = Math.floor(min / 60);
  const m = min % 60;
  return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}`;
}

export function RecordTimeline() {
  const date = useUi((s) => s.date);
  const [mode, setMode] = useState<Mode>("both");
  const settingsQ = useSettings();
  const slotsQ = useSlots(date);
  const recordsQ = useDayRecords(date);
  const activitiesQ = useActivities(false);

  const dayStart = num(settingsQ.data?.day_start_hour, 5);
  const dayEnd = num(settingsQ.data?.day_end_hour, 26);
  const dayStartMin = dayStart * 60;
  const totalMin = (dayEnd - dayStart) * 60;
  const heightPx = totalMin * PX_PER_MIN;

  const hours: number[] = [];
  for (let h = dayStart; h <= dayEnd; h++) hours.push(h);

  const slots = slotsQ.data ?? [];
  const hueById = new Map(activitiesQ.data?.map((a) => [a.id, a.hue_label] as const));
  const nameById = new Map(activitiesQ.data?.map((a) => [a.id, a.name] as const));
  const records = (recordsQ.data ?? [])
    .filter((r) => isoLocal(r.started_at).date === date)
    .map((r) => ({ r, start: isoLocal(r.started_at).minute }));

  const showPlan = mode !== "act";
  const showAct = mode !== "plan";
  const both = mode === "both";
  const { setNodeRef, isOver } = useDroppable({
    id: "record-timeline",
    data: { kind: "timeline-slot", date, pxPerMin: PX_PER_MIN, dayStartMin },
  });

  // now-line (today only)
  const nowMin = new Date().getHours() * 60 + new Date().getMinutes();
  const nowTop = (nowMin - dayStartMin) * PX_PER_MIN;
  const showNow = date === todayStr() && nowMin >= dayStartMin && nowMin <= dayStartMin + totalMin;
  // OxideBar click (Header) → scroll this lane to the requested minute.
  const scrollTarget = useUi((s) => s.scrollTarget);
  const scrollRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!scrollTarget || !scrollRef.current) return;
    const top = (scrollTarget.minute - dayStartMin) * PX_PER_MIN;
    const el = scrollRef.current;
    el.scrollTo({ top: Math.max(0, top - el.clientHeight / 2), behavior: "smooth" });
  }, [scrollTarget, dayStartMin]);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <div className="inline-flex rounded-md bg-surface-sunken p-0.5 text-[12px]">
          {(["plan", "act", "both"] as Mode[]).map((m) => (
            <button
              key={m}
              onClick={() => setMode(m)}
              className={`rounded-[5px] px-3 py-1 transition ${
                mode === m
                  ? "bg-surface-raised font-medium text-text shadow-[var(--shadow-sm)] ring-1 ring-border-strong"
                  : "text-text-subtle hover:text-text-muted"
              }`}
            >
              {m === "plan" ? "계획" : m === "act" ? "실제" : "둘 다"}
            </button>
          ))}
        </div>
        <span className="text-[11px] text-text-subtle">계획은 OR(택1) · 실제는 기록</span>
      </div>

      {both && (
        <div
          className="grid border-b border-border px-3 text-[11px] text-text-subtle"
          style={{ gridTemplateColumns: "1fr 1fr" }}
        >
          <span className="py-1">계획 · 선택지</span>
          <span className="border-l border-border py-1 pl-2">실제 · 기록</span>
        </div>
      )}

      <div ref={scrollRef} className="relative flex-1 overflow-y-auto">
        <div className="relative flex" style={{ height: heightPx }}>
          <div className="relative w-12 shrink-0 border-r border-border">
            {hours.map((h) => (
              <div
                key={h}
                className="absolute right-1 text-[10px] text-text-subtle"
                style={{ top: (h * 60 - dayStartMin) * PX_PER_MIN - 6 }}
              >
                {hhmm(h * 60)}
              </div>
            ))}
          </div>

          <div
            ref={setNodeRef}
            className={`${both ? "relative grid flex-1" : "relative flex-1"} ${isOver ? "ring-2 ring-inset ring-interactive-primary/40" : ""}`}
            style={both ? { gridTemplateColumns: "1fr 1fr" } : undefined}
          >
            {showPlan && <PlanLane slots={slots} dayStartMin={dayStartMin} />}
            {showAct && (
              <ActualLane
                records={records}
                hueById={hueById}
                nameById={nameById}
                dayStartMin={dayStartMin}
                divider={both}
              />
            )}
            {showNow && <NowLine top={nowTop} />}
          </div>
        </div>
      </div>
    </div>
  );
}

function PlanLane({ slots, dayStartMin }: { slots: PlanSlot[]; dayStartMin: number }) {
  const date = useUi((s) => s.date);
  const createPlan = useCreatePlan();
  const createActivity = useCreateActivity();
  const activities = useActivities(false).data ?? [];
  const [draft, setDraft] = useState<{ startMinute: number; durationMinute: number } | null>(null);
  const [rubber, setRubber] = useState<{ startMinute: number; durationMinute: number } | null>(null);
  const dragRef = useRef<{ startY: number; startMinute: number; moved: boolean } | null>(null);

  function minuteFromY(el: HTMLElement, clientY: number): number {
    const rect = el.getBoundingClientRect();
    return snapMinute(Math.round(dayStartMin + (clientY - rect.top) / PX_PER_MIN));
  }

  function commitDraft(title: string) {
    if (!draft) return;
    const trimmed = title.trim();
    const { startMinute, durationMinute } = draft;
    setDraft(null);
    if (!trimmed) return;
    const existing = activities.find(
      (a) => a.name.trim().toLowerCase() === trimmed.toLowerCase(),
    );
    const base = {
      date,
      start_minute: startMinute,
      duration_minute: durationMinute,
      weekday_mask: 0,
      title: null as string | null,
    };
    if (existing) {
      createPlan.mutate({ ...base, activity_ids: [existing.id] });
    } else {
      createActivity.mutate(
        { name: trimmed },
        { onSuccess: (a) => createPlan.mutate({ ...base, activity_ids: [a.id] }) },
      );
    }
  }

  // Empty-surface gesture: a click makes a 30-min draft; a drag rubber-bands a
  // custom span. Both resolve to the same inline DraftBlock.
  function onPointerDown(e: ReactPointerEvent<HTMLDivElement>) {
    if (draft || e.target !== e.currentTarget) return;
    const startMinute = minuteFromY(e.currentTarget, e.clientY);
    dragRef.current = { startY: e.clientY, startMinute, moved: false };
    e.currentTarget.setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: ReactPointerEvent<HTMLDivElement>) {
    const d = dragRef.current;
    if (!d) return;
    if (Math.abs(e.clientY - d.startY) > 6) d.moved = true;
    if (d.moved) {
      const cur = minuteFromY(e.currentTarget, e.clientY);
      const start = Math.min(d.startMinute, cur);
      const dur = Math.max(SNAP_MINUTES, snapMinute(Math.abs(cur - d.startMinute)));
      setRubber({ startMinute: start, durationMinute: dur });
    }
  }

  function onPointerUp() {
    const d = dragRef.current;
    dragRef.current = null;
    if (!d) return;
    if (rubber) {
      setDraft(rubber);
      setRubber(null);
    } else {
      setDraft({ startMinute: d.startMinute, durationMinute: 30 });
    }
  }

  return (
    <div
      className="relative"
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
    >
      {slots.map((s) => (
        <PlanCard key={s.plan_id} s={s} dayStartMin={dayStartMin} />
      ))}
      {rubber && (
        <div
          className="pointer-events-none absolute inset-x-1 rounded-md border border-dashed border-interactive-primary bg-interactive-primary-subtle"
          style={{
            top: (rubber.startMinute - dayStartMin) * PX_PER_MIN,
            height: rubber.durationMinute * PX_PER_MIN,
          }}
        />
      )}
      {draft && (
        <DraftBlock
          draft={draft}
          dayStartMin={dayStartMin}
          onCommit={commitDraft}
          onCancel={() => setDraft(null)}
        />
      )}
    </div>
  );
}

function DraftBlock({
  draft,
  dayStartMin,
  onCommit,
  onCancel,
}: {
  draft: { startMinute: number; durationMinute: number };
  dayStartMin: number;
  onCommit: (title: string) => void;
  onCancel: () => void;
}) {
  const endMinute = draft.startMinute + draft.durationMinute;
  return (
    <div
      className="absolute inset-x-1 z-20 overflow-hidden rounded-lg border border-interactive-primary/70 bg-surface-raised p-2 shadow-[var(--shadow-lg)]"
      style={{
        top: (draft.startMinute - dayStartMin) * PX_PER_MIN,
        minHeight: 104,
        height: Math.max(104, draft.durationMinute * PX_PER_MIN),
      }}
    >
      <div className="mb-1.5 flex items-center gap-1.5 text-[11px]">
        <span className="rounded bg-interactive-primary-subtle px-1.5 py-0.5 font-mono font-medium tabular-nums text-interactive-primary">
          {hhmm(draft.startMinute)}–{hhmm(endMinute)}
        </span>
        <span className="text-text-subtle">{draft.durationMinute}분</span>
      </div>
      <input
        autoFocus
        placeholder="활동 이름 · 엔터로 추가"
        onKeyDown={(e) => {
          if (e.key === "Enter") onCommit((e.currentTarget as HTMLInputElement).value);
          else if (e.key === "Escape") onCancel();
        }}
        onBlur={(e) => {
          const v = e.target.value.trim();
          if (v) onCommit(v);
          else onCancel();
        }}
        className="w-full rounded-md bg-surface px-2 py-1.5 text-[13px] text-text outline-none shadow-[var(--input-shadow)] transition placeholder:text-text-subtle focus-visible:shadow-[var(--input-shadow-focus)]"
      />
      <div className="mt-1 flex items-center justify-end gap-2 text-[10px] text-text-subtle">
        <span><kbd className="rounded border border-border bg-surface px-1 font-mono">⏎</kbd> 저장</span>
        <span><kbd className="rounded border border-border bg-surface px-1 font-mono">esc</kbd> 취소</span>
      </div>
    </div>
  );
}

function PlanCard({ s, dayStartMin }: { s: PlanSlot; dayStartMin: number }) {
  const { setNodeRef, isOver } = useDroppable({
    id: `plan-${s.plan_id}`,
    data: { kind: "plan-slot", planId: s.plan_id },
  });
  const [dragDur, setDragDur] = useState<number | null>(null);
  const resize = useResizePlan();
  const top = (s.start_minute - dayStartMin) * PX_PER_MIN;
  const height = (dragDur ?? s.duration_minute) * PX_PER_MIN;
  return (
    <div
      ref={setNodeRef}
      className={`group absolute left-1 right-1 overflow-hidden rounded-md border border-dashed border-border-strong p-1.5 ${isOver ? "ring-2 ring-interactive-primary" : ""}`}
      style={{ top, height }}
    >
      <div className="mb-1 flex items-center justify-between text-[10px] text-text-subtle">
        <span>
          {hhmm(s.start_minute)} · {Math.round(s.duration_minute)}m
        </span>
        {s.options.length > 1 && <span className="font-semibold">OR</span>}
      </div>
      <div className="flex flex-col gap-0.5">
        {s.options.map((o) => {
          const picked = s.resolved_by?.activity_id === o.id;
          return (
            <div key={o.id} className="flex items-center gap-1.5 text-[12px]">
              <span
                className="inline-block h-2 w-2 rounded-full border"
                style={{ borderColor: hueVar(o.hue_label), background: picked ? hueVar(o.hue_label) : "transparent" }}
              />
              <span className={picked ? "text-text" : "text-text-muted"}>{o.name}</span>
              {picked && <span className="text-[10px] text-text-subtle">→실행</span>}
            </div>
          );
        })}
      </div>
      <div
        className="absolute inset-x-0 bottom-0 h-1.5 cursor-ns-resize opacity-0 transition group-hover:opacity-100"
        onPointerDown={(e) => {
          e.stopPropagation();
          const startY = e.clientY;
          const startDur = s.duration_minute;
          const durRef = { current: startDur };
          (e.currentTarget as Element).setPointerCapture(e.pointerId);
          const move = (ev: PointerEvent) => {
            const next = resizeDuration(startDur, (ev.clientY - startY) / PX_PER_MIN);
            durRef.current = next;
            setDragDur(next);
          };
          const finish = () => {
            window.removeEventListener("pointermove", move);
            window.removeEventListener("pointerup", finish);
            window.removeEventListener("pointercancel", finish);
            if (durRef.current !== startDur) {
              resize.mutate({ planId: s.plan_id, durationMinute: durRef.current });
            }
            setDragDur(null);
          };
          window.addEventListener("pointermove", move);
          window.addEventListener("pointerup", finish);
          window.addEventListener("pointercancel", finish);
        }}
      />
    </div>
  );
}

function ActualLane({
  records,
  hueById,
  nameById,
  dayStartMin,
  divider,
}: {
  records: { r: ActivityRecord; start: number }[];
  hueById: Map<string, string | null>;
  nameById: Map<string, string>;
  dayStartMin: number;
  divider: boolean;
}) {
  return (
    <div className={`relative ${divider ? "border-l border-border" : ""}`}>
      {records.map(({ r, start }) => {
        const end = r.ended_at ? isoLocal(r.ended_at).minute : new Date().getHours() * 60 + new Date().getMinutes();
        const top = (start - dayStartMin) * PX_PER_MIN;
        const height = Math.max(16, (end - start) * PX_PER_MIN);
        const live = r.ended_at === null;
        return (
          <div
            key={r.id}
            className="absolute left-1 right-1 overflow-hidden rounded-md p-1.5 text-[11px]"
            style={{
              top,
              height,
              background: `color-mix(in oklch, ${hueVar(hueById.get(r.activity_id) ?? null)} 22%, var(--color-surface-raised))`,
              borderLeft: `3px solid ${hueVar(hueById.get(r.activity_id) ?? null)}`,
            }}
          >
            <div className="flex items-center gap-1 font-medium">
              {live && <span className="inline-block h-1.5 w-1.5 animate-pulse rounded-full bg-status-error" />}
              <span className="truncate">{nameById.get(r.activity_id) ?? r.activity_id}</span>
            </div>
            <div className="text-text-subtle">
              {hhmm(start)}–{live ? "" : hhmm(end)}
            </div>
          </div>
        );
      })}
    </div>
  );
}

/** Now-line — spans every lane (plan and/or actual) at the current minute. */
function NowLine({ top }: { top: number }) {
  const hh = new Date().getHours();
  const mm = new Date().getMinutes();
  return (
    <div className="pointer-events-none absolute inset-x-0 z-10" style={{ top }}>
      <div className="border-t border-status-error" />
      <span className="absolute -top-2 right-0 rounded bg-status-error px-1 font-mono text-[9px] font-medium text-text-inverse">
        {String(hh).padStart(2, "0")}:{String(mm).padStart(2, "0")}
      </span>
    </div>
  );
}
