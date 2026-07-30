import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useTimeline, useCategories, useCreateTask, useSettings } from "../hooks";
import { useUi } from "../lib/store";
import { useDroppable } from "@dnd-kit/core";
import { BlockView } from "./BlockView";
import { NowLine } from "./NowLine";
import type { TimelineItem } from "../types";

interface Lane {
  item: TimelineItem;
  col: number;
  columns: number;
}

// Greedy lane assignment: items that overlap in time share a row, split into N
// columns (Google-Calendar style, §7.1).
function layout(items: TimelineItem[]): Lane[] {
  const timed = items.filter((i) => i.start_minute != null && i.duration_minute != null);
  const sorted = [...timed].sort((a, b) => a.start_minute! - b.start_minute!);
  const laid: Lane[] = [];
  let i = 0;
  while (i < sorted.length) {
    let j = i;
    let clusterEnd = sorted[i].start_minute! + sorted[i].duration_minute!;
    const cluster: TimelineItem[] = [sorted[i]];
    while (j + 1 < sorted.length && sorted[j + 1].start_minute! < clusterEnd) {
      j++;
      cluster.push(sorted[j]);
      clusterEnd = Math.max(
        clusterEnd,
        sorted[j].start_minute! + sorted[j].duration_minute!,
      );
    }
    const colEnds: number[] = [];
    const startIndex = laid.length;
    for (const it of cluster) {
      const start = it.start_minute!;
      const end = start + it.duration_minute!;
      let col = colEnds.findIndex((ce) => ce <= start);
      if (col === -1) {
        col = colEnds.length;
        colEnds.push(end);
      } else {
        colEnds[col] = end;
      }
      laid.push({ item: it, col, columns: 0 });
    }
    const cols = colEnds.length;
    for (let k = startIndex; k < laid.length; k++) laid[k].columns = cols;
    i = j + 1;
  }
  return laid;
}

function num(v: unknown, d: number): number {
  return typeof v === "number" ? v : d;
}

function nowMin(): number {
  const d = new Date();
  return d.getHours() * 60 + d.getMinutes();
}

export function DayTimeline() {
  const { t } = useTranslation();
  const { date } = useUi();
  const settingsQ = useSettings();
  const catsQ = useCategories();
  const tlQ = useTimeline(date);
  const create = useCreateTask();
  const [adding, setAdding] = useState<{ minute: number } | null>(null);
  const [draft, setDraft] = useState("");

  const dayStart = num(settingsQ.data?.day_start_hour, 5);
  const dayEnd = num(settingsQ.data?.day_end_hour, 26);
  const dayStartMin = dayStart * 60;
  const totalMin = (dayEnd - dayStart) * 60;
  const pxPerMin = 56 / 60;
  const heightPx = totalMin * pxPerMin;

  const items = tlQ.data ?? [];
  const laid = useMemo(() => layout(items), [items]);

  const hours = useMemo(() => {
    const arr: number[] = [];
    for (let h = dayStart; h <= dayEnd; h++) arr.push(h);
    return arr;
  }, [dayStart, dayEnd]);

  const workloadMin = items
    .filter((i) => !i.is_skipped && i.duration_minute != null)
    .reduce((s, i) => s + (i.duration_minute ?? 0), 0);
  const warn = num(settingsQ.data?.workload_warning_minutes, 600);
  const tight = warn > 0 && workloadMin > warn;

  return (
    <div className="flex h-full flex-col">
      <div className="relative flex-1 overflow-y-auto px-2 pb-6">
        <div className="relative" style={{ height: heightPx }}>
          {/* hour rail */}
          {hours.map((h) => {
            const top = (h * 60 - dayStartMin) * pxPerMin;
            const label = `${String(h % 24).padStart(2, "0")}:00`;
            return (
              <div key={h} className="pointer-events-none absolute left-0 right-0" style={{ top }}>
                <div className="flex items-center">
                  <span
                    className="w-14 shrink-0 pr-2 text-right font-mono text-[11px]"
                    style={{ color: "var(--text-tertiary)" }}
                  >
                    {label}
                  </span>
                  <div className="h-px flex-1" style={{ background: "var(--border-subtle)" }} />
                </div>
              </div>
            );
          })}

          {/* blocks */}
          {laid.map(({ item, col, columns }) => {
            const start = item.start_minute!;
            const dur = item.duration_minute!;
            const top = (start - dayStartMin) * pxPerMin;
            const height = dur * pxPerMin;
            const past = start + dur <= nowMin();
            return (
              <BlockView
                key={item.id}
                item={item}
                categories={catsQ.data ?? []}
                left={col}
                columns={columns}
                top={top}
                height={height}
                past={past}
              />
            );
          })}

          {/* inline quick-add on an empty slot */}
          {adding && (
            <div
              className="absolute left-16 right-2 z-30 rounded-md border p-1.5"
              style={{
                top: (adding.minute - dayStartMin) * pxPerMin,
                borderColor: "var(--accent-oxide)",
                background: "var(--surface-raised)",
                boxShadow: "var(--elevation-panel)",
              }}
            >
              <input
                autoFocus
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                placeholder={t("palette.placeholder")}
                className="w-full bg-transparent text-[13px] outline-none"
                onKeyDown={(e) => {
                  if (e.key === "Enter" && draft.trim()) {
                    create.mutate({
                      date,
                      title: draft.trim(),
                      categoryId: null,
                      startMinute: adding.minute,
                      durationMinute: 30,
                      notes: null,
                    });
                    setAdding(null);
                    setDraft("");
                  }
                  if (e.key === "Escape") {
                    setAdding(null);
                    setDraft("");
                  }
                }}
                onBlur={() => {
                  setAdding(null);
                  setDraft("");
                }}
              />
            </div>
          )}

          {/* click empty time to add + droppable area */}
          <DropZone dayStartMin={dayStartMin} pxPerMin={pxPerMin} date={date} heightPx={heightPx} onAdd={(minute) => setAdding({ minute })} />

          <NowLine pxPerMin={pxPerMin} dayStartMin={dayStartMin} />
        </div>
      </div>

      {/* workload inline (Sunsama-quiet, §7.1) */}
      <div
        className="flex items-center justify-center gap-1.5 border-t border-border-subtle px-3 py-1.5 text-[12px]"
        style={{ color: tight ? "var(--signal-rust)" : "var(--text-secondary)" }}
      >
        <span>{t("timeline.planned", { n: workloadMin })}</span>
        <span style={{ color: "var(--text-tertiary)" }}>·</span>
        <span>{tight ? t("timeline.workloadTight") : t("timeline.workloadEasy")}</span>
      </div>
    </div>
  );
}

/** Droppable area over the timeline for drag-and-drop scheduling. */
function DropZone({
  dayStartMin,
  pxPerMin,
  date,
  heightPx,
  onAdd,
}: {
  dayStartMin: number;
  pxPerMin: number;
  date: string;
  heightPx: number;
  onAdd: (minute: number) => void;
}) {
  const { setNodeRef, isOver } = useDroppable({
    id: "timeline-slot",
    data: { kind: "timeline-slot", date, pxPerMin, dayStartMin },
  });

  return (
    <div
      ref={setNodeRef}
      className="absolute left-0 right-0"
      style={{
        top: 0,
        height: heightPx,
        zIndex: 0,
        background: isOver ? "var(--accent-oxide-subtle)" : undefined,
        transition: "background var(--motion-sweep) var(--ease-standard)",
      }}
      onClick={(e) => {
        const rect = (e.currentTarget as HTMLDivElement).getBoundingClientRect();
        const y = e.clientY - rect.top;
        const minute = Math.round(y / pxPerMin + dayStartMin);
        onAdd(Math.max(0, Math.min(1439, minute)));
      }}
    />
  );
}