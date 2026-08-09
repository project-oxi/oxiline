import { useEffect, useRef } from "react";
import { Header } from "./components/Header";
import { RecordTimeline } from "./components/RecordTimeline";
import { CommandPalette } from "./components/CommandPalette";
import { Preferences } from "./components/Preferences";
import { Onboarding } from "./components/Onboarding";
import { Sidebar } from "./components/Sidebar";
import { Inspector } from "./components/Inspector";
import { ContextMenu } from "./components/ContextMenu";
import { ActivitySwitcher } from "./components/ActivitySwitcher";
import { CliNudge } from "./components/CliNudge";
import { UpdateBanner } from "./components/UpdateBanner";
import { useUi } from "./lib/store";
import { useSettings } from "./hooks";
import { useUpdate } from "./lib/updater";
import { api } from "./lib/api";

function useGlobalKeys() {
  const ui = useUi();
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      const typing =
        document.activeElement instanceof HTMLInputElement ||
        document.activeElement instanceof HTMLTextAreaElement ||
        document.activeElement instanceof HTMLSelectElement;

      if (mod && e.key.toLowerCase() === "k") {
        e.preventDefault();
        ui.setPaletteDate(null);
        ui.setPaletteOpen(!ui.paletteOpen);
        return;
      }
      if (mod && e.key === ",") {
        e.preventDefault();
        ui.setPreferencesOpen(true);
        return;
      }
      if (mod && e.key.toLowerCase() === "n") {
        e.preventDefault();
        ui.setPaletteDate(ui.date);
        ui.setPaletteOpen(true);
        return;
      }
      if (mod && e.shiftKey && e.key.toLowerCase() === "a") {
        e.preventDefault();
        ui.setSwitcherOpen(!ui.switcherOpen);
        return;
      }
      if (typing) return;

      if (e.key === "Escape") {
        ui.setPaletteOpen(false);
        ui.setPreferencesOpen(false);
        return;
      }
      if (e.key === "t" || e.key === "T") {
        ui.goToToday();
      } else if (e.key === "ArrowLeft") {
        ui.shiftDate(-1);
      } else if (e.key === "ArrowRight") {
        ui.shiftDate(1);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [ui]);
}

import { DndProvider } from "./lib/dnd";

export default function App() {
  // CLI `oxiline update` writes the `update_request_at` setting; the running
  // app reacts by running the updater, which replaces the whole .app (CLI
  // sidecar included) so GUI + CLI advance together.
  const settings = useSettings();
  const seenUpdateReq = useRef<string | null>(null);
  useEffect(() => {
    const req = (settings.data?.update_request_at as string | undefined) ?? null;
    if (!req || req === seenUpdateReq.current) return;
    seenUpdateReq.current = req;
    // Clear so a later GUI launch doesn't refire on the stale timestamp
    // (and auto-install a version that appeared in the meantime, unattended).
    void api.setSetting("update_request_at", "");
    void useUpdate.getState().check().then(() => {
      if (useUpdate.getState().status.kind === "available") {
        void useUpdate.getState().install();
      }
    });
  }, [settings.data?.update_request_at]);
  useGlobalKeys();

  return (
    <div className="flex h-screen flex-col bg-surface">
      <UpdateBanner />
      <Header />
      <DndProvider>
        <div className="flex flex-1 overflow-hidden">
          <Sidebar />
          <main className="flex flex-1 flex-col overflow-hidden">
            <RecordTimeline />
          </main>
          <Inspector />
        </div>
      </DndProvider>

      <CommandPalette />
      <ActivitySwitcher />
      <Preferences />
      <Onboarding />
      <ContextMenu />
      <CliNudge />
    </div>
  );
}
