/**
 * Sidebar (Plan 2 Task 4) — left pane.
 *   1. NowCard — the active recording session (current_record_state) with a
 *      stop control and today/weekly progress.
 *   2. ActivityLibrary — each active activity with a neutral weekly bar
 *      (target tick, 남음/달성/+Xm), drag source for Task 7.
 */
import { useState } from "react";
import { useActivities, useCompliance, useRecordState, useStopRecord } from "../hooks";
import { complianceLabel, hmm, hueVar } from "../lib/record-format";
import { useDraggable } from "@dnd-kit/core";
import type { Activity, Compliance } from "../types";

export function Sidebar() {
  const stateQ = useRecordState();
  const stop = useStopRecord();
  const active = stateQ.data?.active ?? null;

  return (
    <aside className="flex w-[260px] shrink-0 flex-col gap-4 overflow-y-auto p-3">
      <NowCard
        active={active}

        onStop={() => stop.mutate(undefined)}
        stopping={stop.isPending}
      />
      <ActivityLibrary />
    </aside>
  );
}

function NowCard({
  active,
  onStop,
  stopping,
}: {
  active: { activity: { name: string; hue_label: string | null }; elapsed_seconds: number } | null;

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
        <div className="mt-1 text-[11px] text-text-subtle">⌘⇧A 로 빠르게 전환</div>
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
  const byId = new Map(weekQ.data?.map((c) => [c.activity.id, c]));
  const activities = activitiesQ.data ?? [];
  const [selected, setSelected] = useState<Set<string>>(new Set());

  function handleSelect(id: string, additive: boolean) {
    if (additive) {
      setSelected((prev) => {
        const next = new Set(prev);
        if (next.has(id)) next.delete(id);
        else next.add(id);
        return next;
      });
    } else {
      setSelected(new Set([id]));
    }
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col">
      <div className="mb-2 flex items-center justify-between">
        <h2 className="text-[11px] font-semibold uppercase tracking-wide text-text-subtle">활동 · 카드</h2>
        <span className="text-[10px] text-text-subtle">드래그 → 배치</span>
      </div>
      {activities.length === 0 ? (
        <div className="flex flex-1 items-center justify-center rounded-lg border border-dashed border-border text-[12px] text-text-subtle">
          활동을 추가하세요
        </div>
      ) : (
        <div
          className="flex flex-col gap-2"
          onPointerDown={(e) => {
            if (e.target === e.currentTarget) setSelected(new Set());
          }}
        >
          {activities.map((a) => (
            <DraggableActivity
              key={a.id}
              activity={a}
              compliance={byId.get(a.id)}
              selectedSet={selected}
              onSelect={handleSelect}
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
  selectedSet,
  onSelect,
}: {
  activity: Activity;
  compliance?: Compliance;
  selectedSet: Set<string>;
  onSelect: (id: string, additive: boolean) => void;
}) {
  const isSelected = selectedSet.has(activity.id);
  const ids = isSelected && selectedSet.size > 0 ? [...selectedSet] : [activity.id];
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
      className={`cursor-grab rounded-md p-1.5 hover:bg-surface-sunken ${isDragging ? "opacity-40" : ""} ${isSelected ? "ring-2 ring-interactive-primary" : ""}`}
      style={transform ? { transform: `translate3d(${transform.x}px, ${transform.y}px, 0)` } : undefined}
    >
      <div className="flex items-center justify-between text-[12px]">
        <span className="flex items-center gap-1.5">
          <span className="inline-block h-2 w-2 rounded-full" style={{ background: hueVar(activity.hue_label) }} />
          {activity.name}
          {selectedSet.size > 1 && isSelected && (
            <span className="rounded bg-interactive-primary px-1 text-[9px] font-semibold text-text-inverse">{selectedSet.size}</span>
          )}
        </span>
        <span className="text-[10px] text-text-subtle">
          {activity.target_minutes_weekly ? `주 ${hmm(activity.target_minutes_weekly * 60)}` : ""}
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
