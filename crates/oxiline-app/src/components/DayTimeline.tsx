import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check } from "lucide-react";
import { useTimeline, useCategories, useCreateTask, useSettings } from "../hooks";
import { useUi } from "../lib/store";
import { useDroppable } from "@dnd-kit/core";
import { BlockView } from "./BlockView";
import { NowLine } from "./NowLine";
import { OxideBar } from "./OxideBar";
import { formatDuration, minuteToHHMM, categoryById, categoryColor, rangeLabel } from "../lib/colors";
import { clampDuration, snapMinute, groupClusters, partitionCluster } from "../lib/timeline-math";
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

/** Snap a minute to the nearest SLOT boundary so quick-add lands on a clean slot. */
const SLOT = 15;
function snap(m: number): number {
  return Math.max(0, Math.min(1440 - SLOT, Math.round(m / SLOT) * SLOT));
}

const GUTTER_PX = 44;
const SPINE_X = 54;
const LANE_GAP = 12;

export function DayTimeline() {
  const { t, i18n } = useTranslation();
  const { date } = useUi();
  const settingsQ = useSettings();
  const catsQ = useCategories();
  const tlQ = useTimeline(date);
  const create = useCreateTask();
  const [adding, setAdding] = useState<{ minute: number; durationMinute: number } | null>(null);
  const [creating, setCreating] = useState<{ startMin: number; curMin: number } | null>(null);
  const [draft, setDraft] = useState("");
  const [hover, setHover] = useState<number | null>(null);
  const [overflowOpen, setOverflowOpen] = useState<string | null>(null);

  const dayStart = num(settingsQ.data?.day_start_hour, 5);
  const dayEnd = num(settingsQ.data?.day_end_hour, 26);
  const dayStartMin = dayStart * 60;
  const totalMin = (dayEnd - dayStart) * 60;
  const pxPerMin = 64 / 60;
  const heightPx = totalMin * pxPerMin;
  const lang = i18n.language?.startsWith("en") ? "en" : "ko";

  const items = tlQ.data ?? [];
  const laid = useMemo(() => layout(items), [items]);
  // Overlap soft-cap (spec §4, T2): cap = item-count (cluster.length > 3).
  // capOverride forces the visible items into a 3-column equal-width layout so
  // they don't render at layout()'s raw columns (e.g. 4) with a phantom gap.
  const timed = items.filter((i) => i.start_minute != null && i.duration_minute != null);
  const { overflowIds, capOverride } = useMemo(() => {
    const ids = new Set<string>();
    const override = new Map<string, { col: number; columns: number }>();
    for (const cluster of groupClusters(timed)) {
      if (cluster.length > 3) {
        const { visible, overflow } = partitionCluster(cluster, 3);
        overflow.forEach((it) => ids.add(it.id));
        visible.forEach((it, idx) => override.set(it.id, { col: idx, columns: 3 }));
      }
    }
    return { overflowIds: ids, capOverride: override };
  }, [items, timed]);

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

  const laneLeft = SPINE_X + LANE_GAP;
  return (
    <div className="flex h-full flex-col px-3 pb-3">
      <div
        className="flex flex-1 flex-col overflow-hidden rounded-2xl"
        style={{ background: "var(--color-surface-raised)", boxShadow: "var(--shadow-lg)" }}
      >
        {/* oxide handle — day minimap as card grabber */}
        <div className="px-3 pt-2.5 pb-1.5">
          <OxideBar
            items={items}
            categories={catsQ.data ?? []}
            dayStartMin={dayStartMin}
            totalMin={totalMin}
            compact
            onClickMinute={(m) => setAdding({ minute: snap(m), durationMinute: 30 })}
          />
        </div>

        <div className="relative flex-1 overflow-y-auto px-2 pb-6">
          <div className="relative" style={{ height: heightPx }}>
            {/* spine line */}
            <div
              className="pointer-events-none absolute"
              style={{ left: SPINE_X - 1, top: 0, bottom: 0, width: 2, background: "var(--color-border)" }}
            />

            {/* quiet time labels — no gridlines */}
            {hours.map((h) => {
              const top = (h * 60 - dayStartMin) * pxPerMin;
              const label = `${String(h % 24).padStart(2, "0")}:00`;
              return (
                <span
                  key={h}
                  className="pointer-events-none absolute font-mono text-[10px]"
                  style={{
                    left: 0,
                    width: GUTTER_PX,
                    top,
                    textAlign: "right",
                    paddingRight: 8,
                    color: "var(--color-text-subtle)",
                    transform: "translateY(-5px)",
                  }}
                >
                  {label}
                </span>
              );
            })}

            {/* spine nodes — one per block, colored by category */}
            {laid.map(({ item }) => {
              if (overflowIds.has(item.id)) return null;
              const start = item.start_minute!;
              const dur = item.duration_minute!;
              const nodeTop = (start - dayStartMin) * pxPerMin;
              const cat = categoryById(catsQ.data ?? [], item.category_id);
              const nodeColor = categoryColor(cat?.color_hue ?? null);
              const isPastUndone = !item.is_done && start + dur <= nowMin();
              const fill = item.is_done
                ? "var(--color-status-success)"
                : isPastUndone
                  ? "var(--color-surface-raised)"
                  : nodeColor;
              const ring = item.is_done
                ? "var(--color-status-success)"
                : isPastUndone
                  ? "var(--color-status-error)"
                  : "var(--color-surface-raised)";
              return (
                <div
                  key={`node-${item.id}`}
                  className="pointer-events-none absolute z-10 flex items-center justify-center rounded-full"
                  style={{
                    left: SPINE_X - 7,
                    top: nodeTop - 7,
                    width: 14,
                    height: 14,
                    background: fill,
                    border: `2px solid ${ring}`,
                  }}
                >
                  {item.is_done && <Check size={9} color="white" strokeWidth={3} />}
                </div>
              );
            })}

            {/* content lane — right of the spine */}
            <div className="absolute bottom-0 right-0 top-0" style={{ left: laneLeft }}>
              {/* hover slot hint */}
              {hover != null && !adding && (
                <div
                  className="pointer-events-none absolute left-0 right-0 z-[1] flex items-center rounded-md"
                  style={{
                    top: (hover - dayStartMin) * pxPerMin,
                    height: SLOT * pxPerMin,
                    background: "color-mix(in oklch, var(--color-interactive-primary) 14%, transparent)",
                  }}
                >
                  <span
                    className="ml-1 font-mono text-[11px] font-medium"
                    style={{ color: "var(--color-interactive-primary)" }}
                  >
                    + {minuteToHHMM(hover)}
                  </span>
                </div>
              )}

              {/* blocks */}
              {laid.map(({ item, col, columns }) => {
                if (overflowIds.has(item.id)) return null;
                const ov = capOverride.get(item.id);
                const effCol = ov?.col ?? col;
                const effColumns = ov?.columns ?? columns;
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
                    left={effCol}
                    columns={effColumns}
                    top={top}
                    height={height}
                    past={past}
                    dayEndMin={dayEnd * 60}
                    pxPerMin={pxPerMin}
                  />
                );
              })}

              {/* overflow chips — right-edge badge for capped clusters (>3 items) */}
              {groupClusters(timed)
                .filter((c) => c.length > 3)
                .map((cluster) => {
                  const { overflow } = partitionCluster(cluster, 3);
                  const start = Math.min(...cluster.map((i) => i.start_minute!));
                  const top = (start - dayStartMin) * pxPerMin;
                  const chipId = `overflow:${start}`;
                  const moreLabel = t("timeline.more", {
                    n: overflow.length,
                    defaultValue: "더 보기",
                  });
                  return (
                    <div
                      key={chipId}
                      className="pointer-events-auto absolute right-0 z-[4]"
                      style={{ top }}
                    >
                      <button
                        onClick={() =>
                          setOverflowOpen((id) => (id === chipId ? null : chipId))
                        }
                        className="rounded-full border border-border bg-surface-raised px-2 py-0.5 text-[11px] font-medium shadow-sm"
                        style={{ color: "var(--color-text-muted)" }}
                      >
                        +{overflow.length} {moreLabel}
                      </button>
                      {overflowOpen === chipId && (
                        <div className="absolute right-0 top-6 z-30 w-56 rounded-lg border border-border bg-surface-raised p-1 shadow-lg">
                          <p className="px-2 py-1 text-[11px] font-semibold uppercase text-text-subtle">
                            {t("timeline.overlapTitle", { defaultValue: "겹친 일정" })}
                          </p>
                          {overflow.map((it) => (
                            <div
                              key={it.id}
                              className="flex items-center justify-between gap-2 rounded px-2 py-1 hover:bg-surface-sunken"
                            >
                              <span className="truncate text-[12px]">{it.title}</span>
                              <span className="shrink-0 font-mono text-[10px] text-text-subtle">
                                {rangeLabel(it.start_minute, it.duration_minute)}
                              </span>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  );
                })}

              {/* drag-to-create rubber band */}
              {creating && (
                <div
                  className="pointer-events-none absolute left-0 right-0 z-[5] rounded-md border"
                  style={{
                    top: (Math.min(creating.startMin, creating.curMin) - dayStartMin) * pxPerMin,
                    height: Math.abs(creating.curMin - creating.startMin) * pxPerMin,
                    background:
                      "color-mix(in oklch, var(--color-interactive-primary) 14%, transparent)",
                    borderColor: "var(--color-interactive-primary)",
                  }}
                >
                  <span
                    className="ml-1 font-mono text-[11px] font-medium"
                    style={{ color: "var(--color-interactive-primary)" }}
                  >
                    {minuteToHHMM(Math.min(creating.startMin, creating.curMin))}–{minuteToHHMM(Math.max(creating.startMin, creating.curMin))}
                  </span>
                </div>
              )}

              {/* quick-add composer */}
              {adding && (
                <div
                  className="absolute left-0 right-0 z-30 flex items-center gap-2 rounded-lg border px-2 py-1.5"
                  style={{
                    top: (adding.minute - dayStartMin) * pxPerMin,
                    borderColor: "var(--color-interactive-primary)",
                    background: "var(--color-surface-raised)",
                    boxShadow: "var(--shadow-lg)",
                  }}
                >
                  <span
                    className="shrink-0 rounded-md px-1.5 py-0.5 font-mono text-[11px] font-medium"
                    style={{
                      background: "var(--color-interactive-primary-subtle)",
                      color: "var(--color-interactive-primary)",
                    }}
                  >
                    {minuteToHHMM(adding.minute)} · {formatDuration(adding.durationMinute, lang as "ko" | "en")}
                  </span>
                  <input
                    autoFocus
                    value={draft}
                    onChange={(e) => setDraft(e.target.value)}
                    placeholder={t("palette.placeholder")}
                    className="flex-1 bg-transparent text-[13px] outline-none"
                    onKeyDown={(e) => {
                      if (e.key === "Enter" && draft.trim()) {
                        create.mutate({
                          date,
                          title: draft.trim(),
                          categoryId: null,
                          durationMinute: adding.durationMinute,
                          startMinute: adding.minute,
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

              <DropZone
                dayStartMin={dayStartMin}
                pxPerMin={pxPerMin}
                date={date}
                heightPx={heightPx}
                dayEndMin={dayEnd * 60}
                onCompose={(minute, durationMinute) => setAdding({ minute, durationMinute })}
                onHover={setHover}
                onPreviewChange={setCreating}
              />
            </div>

            <NowLine pxPerMin={pxPerMin} dayStartMin={dayStartMin} spineX={SPINE_X} />
          </div>
        </div>

        {/* workload footer */}
        <div
          className="flex items-center justify-center gap-1.5 border-t border-border px-3 py-1.5 text-[12px]"
          style={{ color: tight ? "var(--color-status-error)" : "var(--color-text-muted)" }}
        >
          <span>{t("timeline.plannedDur", { dur: formatDuration(workloadMin, lang as "ko" | "en") })}</span>
          <span style={{ color: "var(--color-text-subtle)" }}>·</span>
          <span>{tight ? t("timeline.workloadTight") : t("timeline.workloadEasy")}</span>
        </div>
      </div>
    </div>
  );
}

function nowMin(): number {
  const d = new Date();
  return d.getHours() * 60 + d.getMinutes();
}
/** Droppable area over the timeline for drag-and-drop + click-to-add. Sits below
 *  the blocks (z-1) so existing blocks still receive their own clicks. */
function DropZone({
  dayStartMin,
  pxPerMin,
  date,
  heightPx,
  dayEndMin,
  onCompose,
  onHover,
  onPreviewChange,
}: {
  dayStartMin: number;
  pxPerMin: number;
  date: string;
  heightPx: number;
  dayEndMin: number;
  onCompose: (minute: number, durationMinute: number) => void;
  onHover: (minute: number | null) => void;
  onPreviewChange: (c: { startMin: number; curMin: number } | null) => void;
}) {
  const { setNodeRef, isOver } = useDroppable({
    id: "timeline-slot",
    data: { kind: "timeline-slot", date, pxPerMin, dayStartMin },
  });
  const creatingRef = useRef<{ startMin: number; startClientY: number; curMin: number } | null>(null);
  const didDragRef = useRef(false);

  function minuteAt(e: { clientY: number; currentTarget: EventTarget | null }): number {
    const rect = (e.currentTarget as HTMLDivElement).getBoundingClientRect();
    const y = e.clientY - rect.top;
    return Math.max(0, Math.min(1439, Math.round(y / pxPerMin + dayStartMin)));
  }

  function onPointerDown(e: React.PointerEvent) {
    const m = snapMinute(minuteAt(e), 15);
    creatingRef.current = { startMin: m, startClientY: e.clientY, curMin: m };
    didDragRef.current = false;
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
  }
  function onPointerMove(e: React.PointerEvent) {
    if (!creatingRef.current) {
      onHover(snap(minuteAt(e)));
      return;
    }
    if (!didDragRef.current && Math.abs(e.clientY - creatingRef.current.startClientY) > 8) {
      didDragRef.current = true;
    }
    if (didDragRef.current) {
      const m = snapMinute(minuteAt(e), 15);
      creatingRef.current.curMin = m;
      onPreviewChange({ startMin: creatingRef.current.startMin, curMin: m });
    }
  }
  function onPointerUp(e: React.PointerEvent) {
    const c = creatingRef.current;
    const target = e.currentTarget as Element;
    if (target.hasPointerCapture(e.pointerId)) target.releasePointerCapture(e.pointerId);
    creatingRef.current = null;
    if (c && didDragRef.current) {
      const start = Math.min(c.startMin, c.curMin);
      const end = Math.max(c.startMin, c.curMin);
      const dur = clampDuration(start, Math.max(15, end - start), dayEndMin, 15);
      onPreviewChange(null);
      // Leave didDragRef set: the synthetic click fires AFTER pointerup and
      // the onClick guard below consumes + clears it (noDouble-fire).
      onCompose(start, dur);
      return;
    }
    didDragRef.current = false;
    onPreviewChange(null);
    // pure click (didDragRef=false) → onClick handles it
  }
  function onPointerCancel(e: React.PointerEvent) {
    const target = e.currentTarget as Element;
    if (target.hasPointerCapture(e.pointerId)) target.releasePointerCapture(e.pointerId);
    creatingRef.current = null;
    didDragRef.current = false;
    onPreviewChange(null);
  }
  function onClick(e: React.MouseEvent) {
    if (didDragRef.current) {
      didDragRef.current = false;
      onPreviewChange(null);
      return;
    }
    onPreviewChange(null);
    onHover(null);
    onCompose(snapMinute(minuteAt(e), 15), 30);
  }

  return (
    <div
      ref={setNodeRef}
      className="absolute left-0 right-0 cursor-crosshair"
      style={{
        top: 0,
        height: heightPx,
        zIndex: 1,
        touchAction: "none",
        background: isOver ? "var(--color-interactive-primary-subtle)" : undefined,
        transition: "background var(--duration-slow) var(--ease-out)",
      }}
      onMouseMove={(e) => { if (!creatingRef.current) onHover(snap(minuteAt(e))); }}
      onMouseLeave={() => { if (!creatingRef.current) onHover(null); }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerCancel}
      onClick={onClick}
    />
  );
}
