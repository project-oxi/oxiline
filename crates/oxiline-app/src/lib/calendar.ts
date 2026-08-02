function pad(n: number): string {
  return String(n).padStart(2, "0");
}
function ymd(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}
function parse(s: string): Date {
  const [y, m, d] = s.split("-").map(Number);
  return new Date(y, m - 1, d);
}

/** 6×7 (42-cell) Mon-first grid of YYYY-MM-DD strings for the month of `date`. */
export function monthGrid(date: string): string[] {
  const first = parse(date);
  first.setDate(1);
  const jsDow = first.getDay(); // Sun=0..Sat=6
  const monOffset = jsDow === 0 ? 6 : jsDow - 1; // Mon=0..Sun=6
  const start = new Date(first);
  start.setDate(first.getDate() - monOffset);
  const cells: string[] = [];
  for (let i = 0; i < 42; i++) {
    const d = new Date(start);
    d.setDate(start.getDate() + i);
    cells.push(ymd(d));
  }
  return cells;
}

/** `[firstCell, lastCell]` range for `useTimelineRange` (covers adjacent-month spillover). */
export function monthBounds(date: string): { from: string; to: string } {
  const g = monthGrid(date);
  return { from: g[0], to: g[g.length - 1] };
}
