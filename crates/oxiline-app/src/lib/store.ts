import { create } from "zustand";

export type View = "today" | "week" | "backlog";

interface UiState {
  date: string; // YYYY-MM-DD selected day
  view: View;
  paletteOpen: boolean;
  preferencesOpen: boolean;
  routineManagerOpen: boolean;
  onboardingOpen: boolean;
  setDate: (d: string) => void;
  setView: (v: View) => void;
  setPaletteOpen: (b: boolean) => void;
  setPreferencesOpen: (b: boolean) => void;
  setRoutineManagerOpen: (b: boolean) => void;
  setOnboardingOpen: (b: boolean) => void;
  shiftDate: (days: number) => void;
  goToToday: () => void;
}

function todayStr(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(
    d.getDate(),
  ).padStart(2, "0")}`;
}

function shift(dateStr: string, days: number): string {
  const [y, m, d] = dateStr.split("-").map(Number);
  const dt = new Date(y, m - 1, d);
  dt.setDate(dt.getDate() + days);
  return `${dt.getFullYear()}-${String(dt.getMonth() + 1).padStart(2, "0")}-${String(
    dt.getDate(),
  ).padStart(2, "0")}`;
}

export const useUi = create<UiState>((set) => ({
  date: todayStr(),
  view: "today",
  paletteOpen: false,
  preferencesOpen: false,
  routineManagerOpen: false,
  onboardingOpen: false,
  setDate: (d) => set({ date: d }),
  setView: (v) => set({ view: v }),
  setPaletteOpen: (b) => set({ paletteOpen: b }),
  setPreferencesOpen: (b) => set({ preferencesOpen: b }),
  setRoutineManagerOpen: (b) => set({ routineManagerOpen: b }),
  setOnboardingOpen: (b) => set({ onboardingOpen: b }),
  shiftDate: (days) => set((s) => ({ date: shift(s.date, days) })),
  goToToday: () => set({ date: todayStr() }),
}));

export { todayStr, shift };
