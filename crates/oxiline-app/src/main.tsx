import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "./styles.css";
import { api, onDbChanged, onOpenPreferences, onOpenQuickAdd } from "./lib/api";
import { initI18n } from "./lib/i18n";
import { applyTheme, watchSystemTheme } from "./lib/theme";
import { useUi } from "./lib/store";
import App from "./App";

async function boot() {
  // Settings drive locale + theme before first paint.
  let settings: Record<string, unknown> = {};
  try {
    settings = await api.getSettings();
  } catch {
    // Running outside Tauri (e.g. plain browser dev) — fall back to defaults.
  }

  const locale = (settings.locale as string | undefined) ?? "system";
  const theme = (settings.theme as string | undefined) ?? "system";
  await initI18n(locale);
  applyTheme(theme as "light" | "dark" | "system");
  localStorage.setItem("oxi-theme", theme as string);
  watchSystemTheme();

  // Onboarding gate (only under Tauri).
  try {
    const done = await api.isOnboardingDone();
    if (!done) useUi.getState().setOnboardingOpen(true);
  } catch {
    /* ignore in browser dev */
  }

  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { staleTime: 1000, refetchOnWindowFocus: true },
    },
  });

  // Any process writing the DB file → invalidate everything (§4.5).
  void onDbChanged(() => queryClient.invalidateQueries());
  void onOpenPreferences(() => useUi.getState().setPreferencesOpen(true));
  void onOpenQuickAdd(() => useUi.getState().setPaletteOpen(true));

  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <QueryClientProvider client={queryClient}>
        <App />
      </QueryClientProvider>
    </React.StrictMode>,
  );
}

void boot();
