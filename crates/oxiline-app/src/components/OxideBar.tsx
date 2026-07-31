import { useMemo } from "react";
import type { Category, TimelineItem } from "../types";
import { categoryById, categoryColorMuted } from "../lib/colors";

interface Props {
  items: TimelineItem[];
  categories: Category[];
  dayStartMin: number;
  totalMin: number;
  onClickMinute?: (minute: number) => void;
  compact?: boolean;
}

/** Oxide Bar — a day compressed into one horizontal mini-map (§6.6). */
export function OxideBar({ items, categories, dayStartMin, totalMin, onClickMinute, compact }: Props) {
  const nowMin = useMemo(() => {
    const d = new Date();
    return d.getHours() * 60 + d.getMinutes();
  }, []);

  const segs = items
    .filter((i) => !i.is_skipped && i.start_minute != null && i.duration_minute != null)
    .map((i) => {
      const cat = categoryById(categories, i.category_id);
      return {
        left: ((i.start_minute! - dayStartMin) / totalMin) * 100,
        width: (i.duration_minute! / totalMin) * 100,
        color: categoryColorMuted(cat?.color_hue ?? null),
        done: i.is_done,
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
            opacity: s.done ? 0.5 : 0.85,
          }}
        />
      ))}
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
    </div>
  );
}
