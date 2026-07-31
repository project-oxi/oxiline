import type { Category } from "../types";

// Oxi label palette (DESIGN.md §3.2): six canonical hues share L≈0.70–0.75,
// C≈0.12–0.15 so any label has equal visual weight. Categories use an arbitrary
// hue with the canonical L/C; dark mode raises L only (+0.05, never C/H).
// A hue of null/NaN/<0 is the achromatic sentinel (C=0) — used by builtin "other".
const LIGHT = "0.72 0.14";
const DARK = "0.77 0.13";
const LIGHT_ACHROME = "0.62 0";
const DARK_ACHROME = "0.72 0";

/** The six canonical label hues (DESIGN.md §3.2), available to the picker. */
export const CATEGORY_HUES = {
  red: 25,
  amber: 75,
  green: 145,
  teal: 195,
  blue: 250,
  purple: 310,
} as const;

/** Foreground/category fill color for a hue. */
export function categoryColor(hue: number | null): string {
  const dark = document.documentElement.classList.contains("dark");
  if (hue == null || Number.isNaN(hue) || hue < 0)
    return `oklch(${dark ? DARK_ACHROME : LIGHT_ACHROME} 0)`;
  return `oklch(${dark ? DARK : LIGHT} ${hue})`;
}

/** Low-saturation variant for small spaces (oxide bar / HUD). */
export function categoryColorMuted(hue: number | null): string {
  const dark = document.documentElement.classList.contains("dark");
  if (hue == null || Number.isNaN(hue) || hue < 0)
    return `oklch(${dark ? DARK_ACHROME : LIGHT_ACHROME} 0)`;
  return `oklch(${dark ? "0.77 0.10" : "0.72 0.11"} ${hue})`;
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

/** Human duration: 90 → "1시간 30분" / "1h 30m"; 45 → "45분" / "45m"; 60 → "1시간" / "1h". */
export function formatDuration(min: number, lang: "ko" | "en" = "ko"): string {
  if (min <= 0) return lang === "ko" ? "0분" : "0m";
  const h = Math.floor(min / 60);
  const m = min % 60;
  if (lang === "ko") {
    return [h && `${h}시간`, m && `${m}분`].filter(Boolean).join(" ");
  }
  return [h && `${h}h`, m && `${m}m`].filter(Boolean).join(" ");
}

export const WEEKDAY_KEYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"] as const;
export type WeekdayKey = (typeof WEEKDAY_KEYS)[number];

export const MASK_DAILY = 0b1111111;
export const MASK_WEEKDAYS = 0b0001111;
export const MASK_WEEKENDS = 0b1100000;
