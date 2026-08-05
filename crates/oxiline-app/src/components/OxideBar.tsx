import { useMemo } from "react";
import type { Activity, ActivityRecord } from "../types";
import { hueVar } from "../lib/record-format";
import { isoLocal } from "../lib/record-time";

interface Props {
  records: ActivityRecord[];
  activities: Activity[];
  dayStartMin: number;
  totalMin: number;
  onClickMinute?: (minute: number) => void;
  compact?: boolean;
  /** Render the "now" marker. The HUD always shows it; the main-window bar
   * only shows it for the selected date when that date is today. */
  showNow?: boolean;
}

/** Oxide Bar — a day compressed into one horizontal mini-map (§6.6). Segments
 * are actual records (what happened), colored by each record's activity hue. */
export function OxideBar({ records, activities, dayStartMin, totalMin, onClickMinute, compact, showNow = true }: Props) {
  const nowMin = useMemo(() => {
    const d = new Date();
    return d.getHours() * 60 + d.getMinutes();
  }, []);

  const hueById = useMemo(
    () => new Map(activities.map((a) => [a.id, a.hue_label] as const)),
    [activities],
  );

  const segs = records.map((r) => {
    const start = isoLocal(r.started_at).minute;
    const end = r.ended_at ? isoLocal(r.ended_at).minute : nowMin;
    return {
      left: ((start - dayStartMin) / totalMin) * 100,
      width: (Math.max(1, end - start) / totalMin) * 100,
      color: hueVar(hueById.get(r.activity_id) ?? null),
    };
  });

  const nowPct = ((nowMin - dayStartMin) / totalMin) * 100;

  return (
    <div
      className="relative w-full overflow-hidden rounded-full"
      style={{ height: compact ? 6 : 8, background: "var(--color-surface-sunken)" }}
      onClick={(e) => {
        if (!onClickMinute) return;
        const rect = (e.currentTarget as HTMLDivElement).getBoundingClientRect();
        const pct = (e.clientX - rect.left) / rect.width;
        onClickMinute(Math.round(dayStartMin + pct * totalMin));
      }}
    >
      {segs.map((s, idx) => (
        <div
          key={idx}
          className="absolute top-0 h-full"
          style={{
            left: `${Math.max(0, s.left)}%`,
            width: `${Math.max(0.5, s.width)}%`,
            background: s.color,
            opacity: 0.85,
          }}
        />
      ))}
      {showNow && (
        <div
          className="absolute top-1/2 z-10"
          style={{
            left: `${Math.min(100, Math.max(0, nowPct))}%`,
            transform: "translate(-50%, -50%)",
            width: compact ? 6 : 8,
            height: compact ? 6 : 8,
            borderRadius: 999,
            background: "var(--color-interactive-primary)",
            boxShadow:
              "0 0 0 3px color-mix(in oklch, var(--color-interactive-primary) 22%, transparent)",
          }}
        />
      )}
    </div>
  );
}
