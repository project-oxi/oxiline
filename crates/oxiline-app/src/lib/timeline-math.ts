import type { TimelineItem } from "../types";

/** Round `m` to the nearest `step` minutes; clamp into [0, 1440 - step]. */
export function snapMinute(m: number, step: number): number {
  const snapped = Math.round(m / step) * step;
  return Math.max(0, Math.min(1440 - step, snapped));
}

/** Clamp a duration so the block stays >= minDur and never ends past dayEndMin.
 * Ceiling wins at the edge: when the window to dayEnd < minDur, the block ends
 * exactly at dayEnd (returns the window) rather than forcing minDur past it.
 * Spec §6: dayEnd ceiling is hard. Outer Math.min caps at dayEnd.
 */
export function clampDuration(
  start: number,
  dur: number,
  dayEndMin: number,
  minDur = 15,
): number {
  const maxDur = dayEndMin - start;
  return Math.min(Math.max(dur, minDur), maxDur);
}

/** Partition time-ranged items into maximal overlap clusters (start-asc). */
export function groupClusters(items: TimelineItem[]): TimelineItem[][] {
  const timed = items
    .filter((i) => i.start_minute != null && i.duration_minute != null)
    .slice()
    .sort((a, b) => a.start_minute! - b.start_minute!);
  const clusters: TimelineItem[][] = [];
  let runEnd = -1;
  for (const it of timed) {
    const start = it.start_minute!;
    const end = start + it.duration_minute!;
    if (clusters.length === 0 || start >= runEnd) {
      clusters.push([it]);
      runEnd = end;
    } else {
      clusters[clusters.length - 1].push(it);
      runEnd = Math.max(runEnd, end);
    }
  }
  return clusters;
}

/** Selection rule (spec §4): start asc, then duration desc, then id asc. */
function bySelection(a: TimelineItem, b: TimelineItem): number {
  if (a.start_minute! !== b.start_minute!) return a.start_minute! - b.start_minute!;
  const da = a.duration_minute!;
  const db = b.duration_minute!;
  if (da !== db) return db - da;
  return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
}

/** Split one cluster into `cap` visible (selection order) + the overflow tail. */
export function partitionCluster(
  cluster: TimelineItem[],
  cap: number,
): { visible: TimelineItem[]; overflow: TimelineItem[] } {
  const sorted = cluster.slice().sort(bySelection);
  return { visible: sorted.slice(0, cap), overflow: sorted.slice(cap) };
}
