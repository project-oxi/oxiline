import { create } from "zustand";


interface UiState {
  date: string; // YYYY-MM-DD selected day
  paletteOpen: boolean;
  preferencesOpen: boolean;
  switcherOpen: boolean;
  onboardingOpen: boolean;
  paletteDate: string | null;
  selectedActivityIds: string[];
  // Last activity that was recording — the resume target for the global
  // quick-record toggle (⌘⇧R) and the header transport's "start" action.
  lastActivityId: string | null;
  // OxideBar click → scroll the timeline to this minute. `token` bumps each
  // request so the same minute re-triggers the effect.
  scrollTarget: { minute: number; token: number } | null;
  setDate: (d: string) => void;
  setPaletteOpen: (b: boolean) => void;
  setPreferencesOpen: (b: boolean) => void;
  setSwitcherOpen: (b: boolean) => void;
  setOnboardingOpen: (b: boolean) => void;
  setPaletteDate: (d: string | null) => void;
  toggleActivitySelect: (id: string, additive: boolean) => void;
  clearActivitySelection: () => void;
  setLastActivityId: (id: string | null) => void;
  requestScroll: (minute: number) => void;
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
  paletteOpen: false,
  preferencesOpen: false,
  switcherOpen: false,
  onboardingOpen: false,
  paletteDate: null,
  selectedActivityIds: [],
  lastActivityId: null,
  scrollTarget: null,
  setDate: (d) => set({ date: d }),
  setPaletteOpen: (b) => set({ paletteOpen: b }),
  setPreferencesOpen: (b) => set({ preferencesOpen: b }),
  setSwitcherOpen: (b) => set({ switcherOpen: b }),
  setOnboardingOpen: (b) => set({ onboardingOpen: b }),
  setPaletteDate: (d) => set({ paletteDate: d }),
  toggleActivitySelect: (id, additive) =>
    set((state) => ({
      selectedActivityIds: additive
        ? state.selectedActivityIds.includes(id)
          ? state.selectedActivityIds.filter((selectedId) => selectedId !== id)
          : [...state.selectedActivityIds, id]
        : [id],
    })),
  clearActivitySelection: () => set({ selectedActivityIds: [] }),
  setLastActivityId: (id) => set({ lastActivityId: id }),
  requestScroll: (minute) =>
    set((state) => ({
      scrollTarget: { minute, token: (state.scrollTarget?.token ?? 0) + 1 },
    })),
  shiftDate: (days) => set((s) => ({ date: shift(s.date, days) })),
  goToToday: () => set({ date: todayStr() }),
}));

export { todayStr, shift };
