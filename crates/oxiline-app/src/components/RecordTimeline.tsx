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
import { useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { useDroppable } from "@dnd-kit/core";
import {
  useActivities,
  useCreateActivity,
  useCreatePlan,
  useDayRecords,
  useDeletePlan,
  useDeleteRecord,
  useEditRecord,
  useMovePlan,
  useRecordState,
  useResizePlan,
  useSettings,
  useSlots,
  useStartRecord,
  useStopRecord,
} from "../hooks";
import { X } from "lucide-react";
import { todayStr, useUi } from "../lib/store";
import { snapMinute, SNAP_MINUTES } from "../lib/dnd";
import { resizeDuration } from "../lib/resize";
import { packColumns } from "../lib/layout";
import { useTranslation } from "react-i18next";
import { formatDuration } from "../lib/colors";
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
  // Workload tone shift (§doc/07 §7.1): sum today's planned minutes and
  // compare to `workload_warning_minutes` (0 disables). Tight = at/over.
  const { t, i18n } = useTranslation();
  const plannedMin = useMemo(
    () => slots.reduce((acc, s) => acc + (s.duration_minute ?? 0), 0),
    [slots],
  );
  const warningMin = num(settingsQ.data?.workload_warning_minutes, 600);
  const tight = warningMin > 0 && plannedMin >= warningMin;
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

      {warningMin > 0 && (
        <div className="flex items-center justify-between border-b border-border bg-surface-sunken px-3 py-1 text-[11px] tabular-nums">
          <span className={tight ? "text-status-warning" : "text-text-subtle"}>
            {t("timeline.plannedDur", { dur: formatDuration(plannedMin, i18n.language as "ko" | "en") })}{" "}
            ·{" "}
            {tight ? t("timeline.workloadTight") : t("timeline.workloadEasy")}
          </span>
        </div>
      )}
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
            {showPlan && <PlanLane slots={slots} dayStartMin={dayStartMin} totalMin={totalMin} />}
            {showAct && (
              <ActualLane
                records={records}
                hueById={hueById}
                nameById={nameById}
                dayStartMin={dayStartMin}
                divider={both}
                totalMin={totalMin}
              />
            )}
            {showNow && <NowLine top={nowTop} />}
          </div>
        </div>
      </div>
    </div>
  );
}

function PlanLane({ slots, dayStartMin, totalMin }: { slots: PlanSlot[]; dayStartMin: number; totalMin: number }) {
  const date = useUi((s) => s.date);
  const createPlan = useCreatePlan();
  const createActivity = useCreateActivity();
  const activities = useActivities(false).data ?? [];
  const [draft, setDraft] = useState<{ startMinute: number; durationMinute: number } | null>(null);
  const [rubber, setRubber] = useState<{ startMinute: number; durationMinute: number } | null>(null);
  const dragRef = useRef<{ startY: number; startMinute: number; moved: boolean } | null>(null);
  // Column-pack overlapping plan cards (Google-Calendar style). Each card
  // keeps its Y (time) position; overlapping cards fan out side-by-side.
  const layout = useMemo(
    () =>
      packColumns(
        slots.map((s) => ({
          start: s.start_minute,
          end: s.start_minute + s.duration_minute,
        })),
      ),
    [slots],
  );

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
      {slots.map((s, idx) => (
        <PlanCard
          key={s.plan_id}
          s={s}
          col={layout[idx]?.col ?? 0}
          cols={layout[idx]?.cols ?? 1}
          dayStartMin={dayStartMin}
          totalMin={totalMin}
        />
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
      className="absolute inset-x-1 z-20 overflow-hidden rounded-lg border border-interactive-primary/70 bg-surface-raised px-2.5 py-2 shadow-[var(--shadow-lg)]"
      style={{
        top: (draft.startMinute - dayStartMin) * PX_PER_MIN,
        minHeight: 62,
        height: Math.max(62, draft.durationMinute * PX_PER_MIN),
      }}
    >
      <div className="mb-1 flex items-center gap-1.5 text-[11px]">
        <span className="rounded bg-interactive-primary-subtle px-1.5 py-0.5 font-mono font-medium tabular-nums text-interactive-primary">
          {hhmm(draft.startMinute)}–{hhmm(endMinute)}
        </span>
        <span className="text-text-subtle">{draft.durationMinute}분</span>
        <span className="ml-auto flex items-center gap-1 text-text-subtle">
          <kbd className="rounded border border-border bg-surface px-1 font-mono text-[9px]">⏎</kbd>
          <kbd className="rounded border border-border bg-surface px-1 font-mono text-[9px]">esc</kbd>
        </span>
      </div>
      <input
        autoFocus
        placeholder="활동 이름"
        onKeyDown={(e) => {
          if (e.key === "Enter") onCommit((e.currentTarget as HTMLInputElement).value);
          else if (e.key === "Escape") onCancel();
        }}
        onBlur={(e) => {
          const v = e.target.value.trim();
          if (v) onCommit(v);
          else onCancel();
        }}
        className="w-full bg-transparent text-[13px] font-medium text-text outline-none placeholder:font-normal placeholder:text-text-subtle"
      />
    </div>
  );
}

function PlanCard({
  s,
  col,
  cols,
  dayStartMin,
  totalMin,
}: {
  s: PlanSlot;
  col: number;
  cols: number;
  dayStartMin: number;
  totalMin: number;
}) {
  const { setNodeRef, isOver } = useDroppable({
    id: `plan-${s.plan_id}`,
    data: { kind: "plan-slot", planId: s.plan_id },
  });
  const [dragDur, setDragDur] = useState<number | null>(null);
  const [dragStart, setDragStart] = useState<number | null>(null);
  const resize = useResizePlan();
  const move = useMovePlan();
  const del = useDeletePlan();
  const start = useStartRecord();
  const stop = useStopRecord();
  const state = useRecordState();
  // Toggle target: the resolved (executed) option if present, else the first
  // option. OR plans without a resolved pick default to the first option.
  const toggleTarget =
    s.resolved_by?.activity_id ?? s.options[0]?.id ?? null;
  const toggleRecording = () => {
    if (!toggleTarget) return;
    if (state.data?.active?.activity.id === toggleTarget) stop.mutate();
    else start.mutate(toggleTarget);
  };

  const startMin = dragStart ?? s.start_minute;
  const top = (startMin - dayStartMin) * PX_PER_MIN;
  const height = (dragDur ?? s.duration_minute) * PX_PER_MIN;
  const maxStart = dayStartMin + totalMin - s.duration_minute;
  // Resolved → the slot has been fulfilled by an actual record. Use the
  // resolved option's hue as a left rail so the plan card visually lines up
  // with its corresponding ActualBlock (same 3px hue stripe).
  const resolvedOption = s.resolved_by
    ? s.options.find((o) => o.id === s.resolved_by?.activity_id) ?? null
    : null;
  const resolvedHue = resolvedOption?.hue_label ?? null;

  return (
    <div
      ref={setNodeRef}
      className={`group absolute overflow-hidden rounded-md border border-dashed ${s.is_resolved ? "border-interactive-primary/55" : "border-border-strong"} p-1.5 outline-none transition-shadow focus-visible:ring-2 focus-visible:ring-interactive-primary ${
        isOver ? "ring-2 ring-interactive-primary" : ""
      } ${dragStart != null ? "z-30 cursor-grabbing border-interactive-primary/70 shadow-[var(--shadow-lg)]" : "cursor-grab hover:shadow-[var(--shadow-md)]"}`}
      style={{
        top,
        height,
        left: `calc(${(col / cols) * 100}% + 4px)`,
        width: `calc(${(1 / cols) * 100}% - 8px)`,
        ...(s.is_resolved && resolvedHue
          ? { borderLeft: `3px solid ${hueVar(resolvedHue)}`, paddingLeft: 4 }
          : null),
      }}
      role="button"
      tabIndex={0}
      aria-label={`${s.options.map((o) => o.name).join(" / ")} 계획 — Enter로 녹화`}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        if (toggleTarget) e.currentTarget.focus();
        const startY = e.clientY;
        const orig = s.start_minute;
        const ref = { current: orig };
        (e.currentTarget as Element).setPointerCapture(e.pointerId);
        const onMove = (ev: PointerEvent) => {
          const next = snapMinute(Math.min(maxStart, Math.max(dayStartMin, orig + (ev.clientY - startY) / PX_PER_MIN)));
          ref.current = next;
          setDragStart(next);
        };
        const onUp = () => {
          window.removeEventListener("pointermove", onMove);
          window.removeEventListener("pointerup", onUp);
          window.removeEventListener("pointercancel", onUp);
          setDragStart(null);
          if (ref.current !== orig) {
            move.mutate({
              planId: s.plan_id,
              startMinute: ref.current,
              durationMinute: s.duration_minute,
              weekdayMask: s.weekday_mask,
            });
          }
        };
        window.addEventListener("pointermove", onMove);
        window.addEventListener("pointerup", onUp);
        window.addEventListener("pointercancel", onUp);
      }}
      onKeyDown={(e) => {
        if (e.key !== "Enter" && e.key !== " ") return;
        e.preventDefault();
        toggleRecording();
      }}
    >
      <div className="mb-1 flex items-center justify-between text-[10px] text-text-subtle">
        <span className="tabular-nums">
          {hhmm(startMin)} · {Math.round(s.duration_minute)}m
          {dragStart != null && dragStart !== s.start_minute && (
            <span className="ml-1 text-interactive-primary">→ {hhmm(dragStart)}</span>
          )}
        </span>
        <span className="flex items-center gap-1">
          {s.options.length > 1 && <span className="font-semibold">OR</span>}
          <button
            onPointerDown={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.stopPropagation();
              del.mutate(s.plan_id);
            }}
            className="rounded p-0.5 text-text-subtle opacity-0 transition hover:bg-status-error-subtle hover:text-status-error group-hover:opacity-100"
            aria-label="삭제"
            title="삭제"
          >
            <X size={11} />
          </button>
        </span>
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
              {picked && <span className="text-[10px] text-text-subtle">● →실행</span>}
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
          const mv = (ev: PointerEvent) => {
            const next = resizeDuration(startDur, (ev.clientY - startY) / PX_PER_MIN);
            durRef.current = next;
            setDragDur(next);
          };
          const finish = () => {
            window.removeEventListener("pointermove", mv);
            window.removeEventListener("pointerup", finish);
            window.removeEventListener("pointercancel", finish);
            if (durRef.current !== startDur) {
              resize.mutate({ planId: s.plan_id, durationMinute: durRef.current });
            }
            setDragDur(null);
          };
          window.addEventListener("pointermove", mv);
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
  totalMin,
  divider,
}: {
  records: { r: ActivityRecord; start: number }[];
  hueById: Map<string, string | null>;
  nameById: Map<string, string>;
  dayStartMin: number;
  totalMin: number;
  divider: boolean;
}) {
  return (
    <div className={`relative ${divider ? "border-l border-border" : ""}`}>
      {records.map(({ r, start }) => {
        const end = r.ended_at
          ? isoLocal(r.ended_at).minute
          : new Date().getHours() * 60 + new Date().getMinutes();
        return (
          <ActualBlock
            key={r.id}
            r={r}
            start={start}
            end={end}
            live={r.ended_at === null}
            name={nameById.get(r.activity_id) ?? r.activity_id}
            hue={hueVar(hueById.get(r.activity_id) ?? null)}
            dayStartMin={dayStartMin}
            totalMin={totalMin}
          />
        );
      })}
    </div>
  );
}

/** A single actual-record block. Dragging the body moves the whole block by an
 *  absolute UTC delta (preserves duration + DST offset); 5-min snap; clamped to
 *  the day window. Live (open) records are fixed — stop the session first. */
function ActualBlock({
  r,
  start,
  end,
  live,
  name,
  hue,
  dayStartMin,
  totalMin,
}: {
  r: ActivityRecord;
  start: number;
  end: number;
  live: boolean;
  name: string;
  hue: string;
  dayStartMin: number;
  totalMin: number;
}) {
  const edit = useEditRecord();
  const del = useDeleteRecord();
  const [dragDelta, setDragDelta] = useState(0);
  const dragging = dragDelta !== 0;
  const top = (start - dayStartMin + dragDelta) * PX_PER_MIN;
  const height = Math.max(16, (end - start) * PX_PER_MIN);
  return (
    <div
      className={`group absolute left-1 right-1 overflow-hidden rounded-md p-1.5 text-[11px] transition-shadow ${
        live
          ? ""
          : dragging
            ? "z-30 cursor-grabbing shadow-[var(--shadow-lg)]"
            : "cursor-grab hover:shadow-[var(--shadow-md)]"
      }`}
      style={{
        top,
        height,
        background: `color-mix(in oklch, ${hue} 22%, var(--color-surface-raised))`,
        borderLeft: `3px solid ${hue}`,
      }}
      onPointerDown={(e) => {
        if (live || e.button !== 0) return;
        const startY = e.clientY;
        const ref = { current: 0 };
        (e.currentTarget as Element).setPointerCapture(e.pointerId);
        const onMove = (ev: PointerEvent) => {
          const snapped = snapMinute((ev.clientY - startY) / PX_PER_MIN);
          // Keep the block start inside [dayStartMin, dayEnd - duration].
          const newStartMin = start + snapped;
          const clamped = Math.min(
            dayStartMin + totalMin - (end - start),
            Math.max(dayStartMin, newStartMin),
          );
          ref.current = clamped - start;
          setDragDelta(ref.current);
        };
        const onUp = () => {
          window.removeEventListener("pointermove", onMove);
          window.removeEventListener("pointerup", onUp);
          window.removeEventListener("pointercancel", onUp);
          setDragDelta(0);
          if (ref.current !== 0) {
            const ns = new Date(new Date(r.started_at).getTime() + ref.current * 60000).toISOString();
            const ne = r.ended_at
              ? new Date(new Date(r.ended_at).getTime() + ref.current * 60000).toISOString()
              : null;
            edit.mutate({ recordId: r.id, startedAt: ns, endedAt: ne });
          }
        };
        window.addEventListener("pointermove", onMove);
        window.addEventListener("pointerup", onUp);
        window.addEventListener("pointercancel", onUp);
      }}
    >
      <div className="flex items-center justify-between gap-1 font-medium">
        <span className="flex min-w-0 items-center gap-1">
          {live && <span className="inline-block h-1.5 w-1.5 shrink-0 animate-pulse rounded-full bg-status-error" />}
          <span className="truncate">{name}</span>
        </span>
        {!live && (
          <button
            onPointerDown={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.stopPropagation();
              del.mutate(r.id);
            }}
            className="shrink-0 rounded p-0.5 text-text-subtle opacity-0 transition hover:bg-status-error-subtle hover:text-status-error group-hover:opacity-100"
            aria-label="삭제"
            title="삭제"
          >
            <X size={11} />
          </button>
        )}
      </div>
      <div className="text-text-subtle">
        {hhmm(start + dragDelta)}–{live ? "" : hhmm(end + dragDelta)}
      </div>
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
