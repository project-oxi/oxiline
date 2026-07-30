import { useEffect, useRef } from "react";
import { minuteToHHMM } from "../lib/colors";

interface Props {
  pxPerMin: number;
  dayStartMin: number;
}

function nowMinute(): number {
  const d = new Date();
  return d.getHours() * 60 + d.getMinutes();
}

/** The "Now Line" — imperatively slid via requestAnimationFrame so the React
 *  tree is never re-rendered each frame (`04-architecture.md` §4.7). */
export function NowLine({ pxPerMin, dayStartMin }: Props) {
  const lineRef = useRef<HTMLDivElement>(null);
  const dotRef = useRef<HTMLDivElement>(null);
  const labelRef = useRef<HTMLSpanElement>(null);

  useEffect(() => {
    let raf = 0;
    let lastLabel = "";
    const prefersReduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    const tick = () => {
      const m = nowMinute();
      const y = (m - dayStartMin) * pxPerMin;
      if (lineRef.current) lineRef.current.style.transform = `translateY(${y}px)`;
      const label = minuteToHHMM(m);
      if (labelRef.current && label !== lastLabel) {
        labelRef.current.textContent = label;
        lastLabel = label;
      }
      raf = prefersReduced
        ? window.setTimeout(() => requestAnimationFrame(tick), 1000) as unknown as number
        : requestAnimationFrame(tick);
    };
    tick();
    return () => {
      cancelAnimationFrame(raf);
      clearTimeout(raf);
    };
  }, [pxPerMin, dayStartMin]);

  return (
    <div
      ref={lineRef}
      className="pointer-events-none absolute left-0 right-0 z-20"
      style={{ top: 0, willChange: "transform" }}
    >
      <div className="relative h-0">
        <div
          className="absolute left-0 right-0"
          style={{ height: 2, background: "var(--accent-oxide-strong)" }}
        />
        <div
          ref={dotRef}
          className="absolute"
          style={{
            left: 0,
            top: -5,
            width: 10,
            height: 10,
            borderRadius: 999,
            background: "var(--accent-oxide)",
            boxShadow: "0 0 0 4px var(--accent-oxide-subtle)",
            animation: "oxiline-pulse 2s var(--ease-standard) infinite",
          }}
        />
        <span
          ref={labelRef}
          className="absolute font-mono text-[11px]"
          style={{ left: 14, top: -8, color: "var(--accent-oxide-strong)" }}
        />
      </div>
      <style>{`@keyframes oxiline-pulse { 0%,100%{opacity:.85;transform:scale(1)} 50%{opacity:1;transform:scale(1.06)} }`}</style>
    </div>
  );
}
