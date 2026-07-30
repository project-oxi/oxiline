// Theme application: `light` | `dark` | `system` (06-design-system.md §6.8).

export type ThemeMode = "light" | "dark" | "system";

export function applyTheme(mode: ThemeMode) {
  const dark =
    mode === "dark" ||
    (mode === "system" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.setAttribute("data-theme", dark ? "dark" : "light");
}

let mqlUnlisten: (() => void) | null = null;

export function watchSystemTheme() {
  if (mqlUnlisten) mqlUnlisten();
  const mql = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = () => {
    const current =
      (document.documentElement.getAttribute("data-theme") as
        | "light"
        | "dark"
        | null) ?? "light";
    // Only react if the app is in system mode.
    if (sessionStorage.getItem("oxiline-theme") === "system") {
      applyTheme("system");
      void current;
    }
  };
  mql.addEventListener("change", handler);
  mqlUnlisten = () => mql.removeEventListener("change", handler);
}

export function setThemeMode(mode: ThemeMode) {
  sessionStorage.setItem("oxiline-theme", mode);
  applyTheme(mode);
}
