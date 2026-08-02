import { SNAP_MINUTES } from "./dnd";

/** New duration after dragging a resize handle by `deltaMin` minutes.
 *  Snaps to SNAP_MINUTES (5) and clamps to `min` (default 15). */
export function resizeDuration(currentMin: number, deltaMin: number, min = 15): number {
  const snapped = Math.round((currentMin + deltaMin) / SNAP_MINUTES) * SNAP_MINUTES;
  return Math.max(min, snapped);
}
