import { useEffect } from "react";
import { Header } from "./components/Header";
import { DayTimeline } from "./components/DayTimeline";
import { BacklogView } from "./components/BacklogView";
import { WeekView } from "./components/WeekView";
import { ReportView } from "./components/ReportView";
import { RoutineManager } from "./components/RoutineManager";
import { CommandPalette } from "./components/CommandPalette";
import { Preferences } from "./components/Preferences";
import { Onboarding } from "./components/Onboarding";
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
      if (typing) return;

      if (e.key === "Escape") {
        ui.setPaletteOpen(false);
        ui.setPreferencesOpen(false);
        ui.setRoutineManagerOpen(false);
        return;
      }
      if (e.key === "t" || e.key === "T") {
        ui.goToToday();
      } else if (e.key === "ArrowLeft") {
        ui.shiftDate(-1);
      } else if (e.key === "ArrowRight") {
        ui.shiftDate(1);
      } else if (e.key === "1") {
        ui.setView("today");
      } else if (e.key === "2") {
        ui.setView("week");
      } else if (e.key === "3") {
        ui.setView("backlog");
      } else if (e.key === "4") {
        ui.setView("report");
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [ui]);
}

import { DndProvider } from "./lib/dnd";

export default function App() {
  useGlobalKeys();
  const view = useUi((s) => s.view);

  return (
    <div className="flex h-screen flex-col bg-surface">
      <Header />
      <DndProvider>
        <div className="flex flex-1 flex-col overflow-hidden">
          {view === "today" && <DayTimeline />}
          {view === "week" && <WeekView />}
          {view === "backlog" && <BacklogView />}
          {view === "report" && <ReportView />}
        </div>
      </DndProvider>

      <CommandPalette />
      <Preferences />
      <RoutineManager />
      <Onboarding />
    </div>
  );
}
