import { useEffect, useRef } from "react";
import { minuteToHHMM } from "../lib/colors";

interface Props {
  pxPerMin: number;
  dayStartMin: number;
  spineX: number;
}

function nowMinute(): number {
  const d = new Date();
  return d.getHours() * 60 + d.getMinutes();
}

/** The "Now Line" — imperatively slid via requestAnimationFrame so the React
 *  tree is never re-rendered each frame (`04-architecture.md` §4.7). */
export function NowLine({ pxPerMin, dayStartMin, spineX }: Props) {
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
      className="pointer-events-none absolute z-20"
      style={{ top: 0, left: spineX, right: 0, willChange: "transform" }}
    >
      <div className="relative h-0">
        <div
          className="absolute"
          style={{
            left: 6,
            right: 0,
            height: 1.5,
            background:
              "linear-gradient(90deg, var(--color-interactive-primary), transparent)",
          }}
        />
        <div
          ref={dotRef}
          className="absolute"
          style={{
            left: -5,
            top: -5,
            width: 10,
            height: 10,
            borderRadius: 999,
            background: "var(--color-interactive-primary)",
            boxShadow:
              "0 0 0 4px color-mix(in oklch, var(--color-interactive-primary) 22%, transparent)",
            animation: "oxiline-pulse 2s var(--ease-out) infinite",
          }}
        />
        <span
          ref={labelRef}
          className="absolute font-mono text-[10px]"
          style={{ left: 10, top: -7, color: "var(--color-interactive-primary)" }}
        />
      </div>
      <style>{`@keyframes oxiline-pulse { 0%,100%{opacity:.85;transform:scale(1)} 50%{opacity:1;transform:scale(1.06)} }`}</style>
    </div>
  );
}
