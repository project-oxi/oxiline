import { useEffect } from "react";
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
import { useUi } from "./lib/store";

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
  useGlobalKeys();

  return (
    <div className="flex h-screen flex-col bg-surface">
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
