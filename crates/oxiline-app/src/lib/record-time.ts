//! Records store UTC instants; the timetable + markers use LOCAL wall-clock.
//! Shared conversion (previously duplicated privately in RecordTimeline /
//! Inspector). Callers read `.date` (YYYY-MM-DD) or `.minute` (of-day) as
//! needed — no per-field wrappers.

/** ISO UTC instant → local {date (YYYY-MM-DD), minute-of-day}. */
export function isoLocal(iso: string): { date: string; minute: number } {
  const d = new Date(iso);
  return {
    date: `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`,
    minute: d.getHours() * 60 + d.getMinutes(),
  };
}
