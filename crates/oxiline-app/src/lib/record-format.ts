/** Recording-layer display formatting shared by Sidebar + Inspector (Plan 2).
 *
 * Neutral by design: compliance states never use failure language
 * (실패/깨짐/놓침); over is a surplus ("초과 +Xm"). Mirrors the CLI lang.rs. */
import type { ComplianceState } from "../types";

/** Seconds → "Xh Ym" / "Ym" / "0m" (minute resolution). */
export function hmm(secs: number): string {
  const m = Math.round(secs / 60);
  const h = Math.floor(m / 60);
  const mm = m % 60;
  return h > 0 ? `${h}h ${mm}m` : `${mm}m`;
}

/** Neutral state label; `surplusSecs` only used for the over case. */
export function complianceLabel(state: ComplianceState, surplusSecs = 0): string {
  switch (state) {
    case "under":
      return "미달";
    case "met":
      return "달성";
    case "over":
      return `초과 +${hmm(surplusSecs)}`;
    case "unbudgeted":
      return "목표 없음";
  }
}

export function hueVar(hue: string | null): string {
  return hue ? `var(--color-hue-${hue})` : "var(--color-interactive-primary)";
}
