// Theme application (DESIGN.md §8).
//
// `.dark` class on <html> is the single light/dark trigger. The storage key is
// `oxi-theme` (localStorage); the DB `settings.theme` is the source of truth and
// is mirrored here so the inline FOUC script in index.html can pre-apply the
// class before first paint.

export type ThemeMode = "light" | "dark" | "system";

const STORAGE_KEY = "oxi-theme";

/** Resolve a mode to a concrete light/dark decision and toggle `.dark`. */
export function applyTheme(mode: ThemeMode) {
  const dark =
    mode === "dark" ||
    (mode === "system" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.classList.toggle("dark", dark);
}

let mqlUnlisten: (() => void) | null = null;

/** Subscribe to OS theme changes; only reacts while in `system` mode. */
export function watchSystemTheme() {
  if (mqlUnlisten) mqlUnlisten();
  const mql = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = () => {
    if (localStorage.getItem(STORAGE_KEY) === "system") applyTheme("system");
  };
  mql.addEventListener("change", handler);
  mqlUnlisten = () => mql.removeEventListener("change", handler);
}

/** Persist the mode and apply it immediately. */
export function setThemeMode(mode: ThemeMode) {
  localStorage.setItem(STORAGE_KEY, mode);
  applyTheme(mode);
}
