import type { PlanSlot } from "../types";

/** The slot whose [start, start+duration) contains `nowMin`, else null. */
export function currentSlot(slots: PlanSlot[], nowMin: number): PlanSlot | null {
  return (
    slots.find(
      (s) => nowMin >= s.start_minute && nowMin < s.start_minute + s.duration_minute,
    ) ?? null
  );
}

/** First slot starting strictly after `nowMin` (earliest), else null. */
export function nextSlot(slots: PlanSlot[], nowMin: number): PlanSlot | null {
  return (
    [...slots]
      .filter((s) => s.start_minute > nowMin)
      .sort((a, b) => a.start_minute - b.start_minute)[0] ?? null
  );
}
