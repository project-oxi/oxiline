import type { Category } from "../types";

// Category color: fixed L/C with variable hue (06-design-system.md §6.2).
// Light: L=0.62 C=0.09 / Dark: L=0.74 C=0.11; "other" is achromatic (C=0).
const LIGHT = "0.62 0.09";
const DARK = "0.74 0.11";
const LIGHT_ACHROME = "0.62 0";
const DARK_ACHROME = "0.74 0";

function isDark(): boolean {
  return document.documentElement.getAttribute("data-theme") === "dark";
}

/** Foreground/category fill color for a hue. */
export function categoryColor(hue: number | null): string {
  const dark = isDark();
  if (hue === null || Number.isNaN(hue)) {
    return `oklch(${dark ? DARK_ACHROME : LIGHT_ACHROME} 0)`;
  }
  return `oklch(${dark ? DARK : LIGHT} ${hue})`;
}

/** Low-saturation variant for small spaces (oxide bar / HUD). */
export function categoryColorMuted(hue: number | null): string {
  const dark = isDark();
  if (hue === null || Number.isNaN(hue)) {
    return `oklch(${dark ? DARK_ACHROME : LIGHT_ACHROME} 0)`;
  }
  const lc = dark ? "0.74 0.088" : "0.62 0.072";
  return `oklch(${lc} ${hue})`;
}

export function categoryById(
  categories: Category[],
  id: string | null | undefined,
): Category | undefined {
  if (!id) return undefined;
  return categories.find((c) => c.id === id);
}

export function minuteToHHMM(min: number): string {
  const h = Math.floor(min / 60);
  const m = min % 60;
  return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}`;
}

export function rangeLabel(start: number | null, dur: number | null): string {
  if (start == null || dur == null) return "";
  const end = Math.min(start + dur, 1440);
  return `${minuteToHHMM(start)}–${minuteToHHMM(end)}`;
}

export const WEEKDAY_KEYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"] as const;
export type WeekdayKey = (typeof WEEKDAY_KEYS)[number];

export const MASK_DAILY = 0b1111111;
export const MASK_WEEKDAYS = 0b0001111;
export const MASK_WEEKENDS = 0b1100000;
