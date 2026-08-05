/**
 * Sidebar (Plan 2 Task 4) — left pane.
 *   1. NowCard — the active recording session (current_record_state) with a
 *      stop control and today/weekly progress.
 *   2. ActivityLibrary — each active activity with a neutral weekly bar
 *      (target tick, 남음/달성/+Xm), drag source for Task 7.
 */
import { useState } from "react";
import { useDraggable } from "@dnd-kit/core";
import { Play, Plus, Square, Trash2 } from "lucide-react";
import {
  useActivities,
  useCompliance,
  useCreateActivity,
  useDeleteActivity,
  useRecordState,
  useStartRecord,
  useStopRecord,
} from "../hooks";
import { complianceLabel, hmm, hueVar } from "../lib/record-format";
import { useUi } from "../lib/store";
import { useContextMenu } from "../lib/context-menu";
import type { Activity, Compliance } from "../types";

export function Sidebar() {
  const stateQ = useRecordState();
  const stop = useStopRecord();
  const start = useStartRecord();
  const { lastActivityId, setSwitcherOpen } = useUi();
  const active = stateQ.data?.active ?? null;

  function handleStart() {
    if (lastActivityId) start.mutate(lastActivityId);
    else setSwitcherOpen(true);
  }

  return (
    <aside className="flex w-[260px] shrink-0 flex-col gap-4 overflow-y-auto border-r border-border bg-surface-sunken p-3">
      <NowCard
        active={active}
        onStart={handleStart}
        onStop={() => stop.mutate(undefined)}
        stopping={stop.isPending}
      />
      <ActivityLibrary />
    </aside>
  );
}

function NowCard({
  active,
  onStart,
  onStop,
  stopping,
}: {
  active: { activity: { name: string; hue_label: string | null }; elapsed_seconds: number } | null;
  onStart: () => void;
  onStop: () => void;
  stopping: boolean;
}) {
  if (!active) {
    return (
      <section className="rounded-lg border border-border bg-surface-raised p-3">
        <div className="flex items-center gap-2 text-[11px] text-text-subtle">
          <span className="inline-block h-1.5 w-1.5 rounded-full bg-text-subtle" />
          지금 녹화 중
        </div>
        <div className="mt-2 text-[13px] text-text-muted">녹화 중인 활동이 없어요</div>
        <button
          onClick={onStart}
          className="mt-2 flex w-full items-center justify-center gap-1.5 rounded-md bg-interactive-primary py-1.5 text-[12px] font-medium text-interactive-primary-foreground transition hover:opacity-90"
        >
          <Play size={13} fill="currentColor" />
          녹화 시작
        </button>
        <div className="mt-1.5 text-center text-[11px] text-text-subtle">⌘⇧A 전환 · ⌘⇧R 빠른 토글</div>
      </section>
    );
  }
  return (
    <section
      className="rounded-lg border p-3"
      style={{ borderColor: hueVar(active.activity.hue_label), background: "var(--color-surface-raised)" }}
    >
      <div className="flex items-center gap-2 text-[11px] text-text-subtle">
        <span
          className="inline-block h-1.5 w-1.5 animate-pulse rounded-full"
          style={{ background: hueVar(active.activity.hue_label) }}
        />
        지금 녹화 중
      </div>
      <div className="mt-1 text-[15px] font-semibold">{active.activity.name}</div>
      <div className="text-[12px] text-text-subtle">{hmm(active.elapsed_seconds)}</div>
      <button
        onClick={onStop}
        disabled={stopping}
        className="mt-2 w-full rounded-md bg-surface-sunken py-1.5 text-[12px] font-medium text-text disabled:opacity-50"
      >
        ⏸ 멈춤
      </button>
    </section>
  );
}

function ActivityLibrary() {
  const activitiesQ = useActivities(true);
  const weekQ = useCompliance("week");
  const createAct = useCreateActivity();
  const byId = new Map(weekQ.data?.map((c) => [c.activity.id, c]));
  const activities = activitiesQ.data ?? [];
  const selectedActivityIds = useUi((state) => state.selectedActivityIds);
  const toggleActivitySelect = useUi((state) => state.toggleActivitySelect);
  const clearActivitySelection = useUi((state) => state.clearActivitySelection);
  const [adding, setAdding] = useState(false);
  const [name, setName] = useState("");

  function submitName() {
    const trimmed = name.trim();
    if (!trimmed) return;
    createAct.mutate({ name: trimmed }, { onSuccess: () => { setName(""); setAdding(false); } });
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col">
      <div className="mb-2 flex items-center justify-between">
        <h2 className="text-[11px] font-semibold uppercase tracking-wide text-text-subtle">활동 · 카드</h2>
        <button
          onClick={() => setAdding((v) => !v)}
          className="flex items-center gap-0.5 rounded px-1 py-0.5 text-[11px] font-medium text-interactive-primary transition hover:bg-surface-sunken"
          aria-label="활동 추가"
          title="활동 추가"
        >
          <Plus size={13} />
          추가
        </button>
      </div>
      {adding && (
        <input
          autoFocus
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") submitName();
            else if (e.key === "Escape") { setAdding(false); setName(""); }
          }}
          onBlur={() => { if (!name.trim()) setAdding(false); }}
          placeholder="활동 이름"
          className="mb-2 rounded bg-surface px-2 py-1 text-[12px] shadow-[var(--input-shadow)] outline-none focus-visible:shadow-[var(--input-shadow-focus)]"
        />
      )}
      {activities.length === 0 && !adding ? (
        <button
          onClick={() => setAdding(true)}
          className="flex flex-1 items-center justify-center gap-1 rounded-lg border border-dashed border-border text-[12px] text-text-subtle transition hover:bg-surface-sunken"
        >
          <Plus size={13} />
          첫 활동 만들기
        </button>
      ) : (
        <div
          className="flex flex-col gap-2"
          onPointerDown={(e) => {
            if (e.target === e.currentTarget) clearActivitySelection();
          }}
        >
          {activities.map((a) => (
            <DraggableActivity
              key={a.id}
              activity={a}
              compliance={byId.get(a.id)}
              onSelect={toggleActivitySelect}
            />
          ))}
        </div>
      )}
    </section>
  );
}

function DraggableActivity({
  activity,
  compliance,
  onSelect,
}: {
  activity: Activity;
  compliance?: Compliance;
  onSelect: (id: string, additive: boolean) => void;
}) {
  const selectedActivityIds = useUi((state) => state.selectedActivityIds);
  const start = useStartRecord();
  const stop = useStopRecord();
  const delAct = useDeleteActivity();
  const recState = useRecordState();
  const isActive = recState.data?.active?.activity.id === activity.id;
  const isSelected = selectedActivityIds.includes(activity.id);
  const ids = isSelected && selectedActivityIds.length > 0 ? selectedActivityIds : [activity.id];
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: `activity-${activity.id}`,
    data: { kind: "activity", activityIds: ids },
  });
  const ratio = compliance?.ratio ?? 0;
  const pct = Math.min(100, Math.round(ratio * 100));
  const target = compliance?.target_seconds ?? null;
  const recorded = compliance?.recorded_seconds ?? 0;
  const surplus = Math.max(0, recorded - (target ?? 0));
  const sub = compliance
    ? target == null
      ? hmm(recorded)
      : compliance.state === "met"
        ? `${hmm(recorded)} · 달성`
        : compliance.state === "over"
          ? `${hmm(recorded)} · ${complianceLabel("over", surplus)}`
          : `${hmm(recorded)} · 남음 ${hmm(target - recorded)}`
    : "—";
  return (
    <div
      ref={setNodeRef}
      {...listeners}
      {...attributes}
      onClick={(e) => onSelect(activity.id, e.metaKey || e.ctrlKey)}
      onContextMenu={(e) => {
        e.preventDefault();
        useContextMenu.getState().show(e.clientX, e.clientY, [
          { kind: "header", label: activity.name },
          {
            kind: "item",
            label: isActive ? "녹화 중지" : "녹화 시작",
            icon: isActive ? Square : Play,
            onSelect: () => (isActive ? stop.mutate() : start.mutate(activity.id)),
          },
          { kind: "separator" },
          { kind: "item", label: "활동 삭제", icon: Trash2, danger: true, onSelect: () => delAct.mutate(activity.id) },
        ]);
      }}
      className={`group cursor-grab rounded-md p-1.5 hover:bg-surface-sunken ${isDragging ? "opacity-40" : ""} ${isSelected ? "ring-2 ring-interactive-primary" : ""}`}
      style={transform ? { transform: `translate3d(${transform.x}px, ${transform.y}px, 0)` } : undefined}
    >
      <div className="flex items-center justify-between text-[12px]">
        <span className="flex items-center gap-1.5">
          <span className="inline-block h-2 w-2 rounded-full" style={{ background: hueVar(activity.hue_label) }} />
          {activity.name}
          {selectedActivityIds.length > 1 && isSelected && (
            <span className="rounded bg-interactive-primary px-1 text-[9px] font-semibold text-text-inverse">{selectedActivityIds.length}</span>
          )}
        </span>
        <span className="flex items-center gap-1">
          <span className="text-[10px] text-text-subtle">
            {activity.target_minutes_weekly ? `주 ${hmm(activity.target_minutes_weekly * 60)}` : ""}
          </span>
          <button
            onPointerDown={(e) => e.stopPropagation()}
            onClick={(e) => { e.stopPropagation(); start.mutate(activity.id); }}
            className="rounded p-0.5 text-text-subtle opacity-0 transition hover:bg-surface hover:text-interactive-primary group-hover:opacity-100"
            aria-label={`${activity.name} 녹화 시작`}
            title="녹화 시작"
          >
            <Play size={12} fill="currentColor" />
          </button>
        </span>
      </div>
      <div className="relative mt-1 h-1.5 rounded-full bg-surface-sunken">
        <div className="h-full rounded-full" style={{ width: `${pct}%`, background: hueVar(activity.hue_label) }} />
        {target != null && (
          <span className="absolute top-1/2 h-2.5 w-0.5 -translate-y-1/2 bg-text-subtle" style={{ left: "100%" }} />
        )}
      </div>
      <div className="mt-0.5 text-[10px] text-text-subtle">{sub}</div>
    </div>
  );
}
