import { useEffect } from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";
import { Square } from "lucide-react";
import { OxideBar } from "./components/OxideBar";
import { useActivities, useCompliance, useDayRecords, useRecordState, useSlots, useStopRecord } from "./hooks";
import { todayStr } from "./lib/store";
import { currentSlot, nextSlot } from "./lib/now-next";
import { hmm, hueVar } from "./lib/record-format";
import { applyTheme, type ThemeMode } from "./lib/theme";

function minuteToHHMM(min: number): string {
  return `${String(Math.floor(min / 60)).padStart(2, "0")}:${String(min % 60).padStart(2, "0")}`;
}

/** Re-apply the persisted theme to *this* window's <html>. The HUD is a
 * separate, long-lived webview that is only shown/hidden (never reloaded), so
 * its FOUC class can drift from the main window. */
function syncTheme() {
  applyTheme((localStorage.getItem("oxi-theme") as ThemeMode) ?? "system");
}

function HudCard() {
  const actsQ = useActivities(false);
  const recsQ = useDayRecords(todayStr());
  const stateQ = useRecordState();
  const stopRec = useStopRecord();
  const slotsQ = useSlots(todayStr());
  const weekQ = useCompliance("week");
  const qc = useQueryClient();

  // Refresh on show — the Rust side emits "oxiline://hud-show" each time the
  // global shortcut fires, so the queries (recordState / slots / compliance)
  // re-fetch and the card reflects the latest state even though the window
  // was hidden rather than reloaded.
  useEffect(() => {
    const un = listen("oxiline://hud-show", () => {
      void qc.invalidateQueries();
    });
    return () => {
      void un.then((f) => f());
    };
  }, [qc]);

  // Keep the HUD's theme in lock-step with the main window: sync once on mount
  // (the main window mirrors the DB theme into localStorage during boot, which
  // may run after our FOUC), and again whenever it changes (Preferences writes
  // localStorage, which fires a `storage` event in this window).
  useEffect(() => {
    syncTheme();
    const onStorage = (e: StorageEvent) => {
      if (e.key === null || e.key === "oxi-theme") syncTheme();
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  const nowMin = new Date().getHours() * 60 + new Date().getMinutes();
  const active = stateQ.data?.active ?? null;
  const cur = currentSlot(slotsQ.data ?? [], nowMin);
  const nxt = nextSlot(slotsQ.data ?? [], nowMin);
  const weekComp = active
    ? (weekQ.data ?? []).find((c) => c.activity.id === active.activity.id)
    : undefined;

  return (
    <div className="h-screen w-screen p-2.5">
      <div
        className="flex h-full w-full flex-col gap-2 rounded-2xl border border-border p-3"
        style={{
          background: "var(--color-surface-raised)",
          boxShadow: "var(--shadow-lg)",
        }}
      >
        <OxideBar
          records={recsQ.data ?? []}
          activities={actsQ.data ?? []}
          dayStartMin={5 * 60}
          totalMin={21 * 60}
          compact
        />

        <div className="flex-1">
          {active ? (
            <div>
              <div className="text-[15px] font-semibold leading-tight text-text">
                <span aria-hidden style={{ color: hueVar(active.activity.hue_label) }}>● </span>
                {active.activity.name}
              </div>
              {(weekComp?.target_seconds ?? 0) > 0 ? (
                <div className="mt-1.5">
                  <div className="flex items-baseline justify-between text-[12px]">
                    <span className="text-text-muted">
                      {hmm(active.elapsed_seconds)} 경과
                    </span>
                    <span className="font-mono text-text-subtle">
                      {hmm(weekComp!.recorded_seconds)}/{hmm(weekComp!.target_seconds!)}
                    </span>
                  </div>
                  <div
                    className="mt-1 h-1 overflow-hidden rounded-full"
                    style={{ background: "var(--color-surface-sunken)" }}
                  >
                    <div
                      className="h-full rounded-full"
                      style={{
                        width: `${Math.min(100, Math.round((weekComp!.ratio ?? 0) * 100))}%`,
                        background: hueVar(active.activity.hue_label),
                      }}
                    />
                  </div>
                </div>
              ) : (
                <div className="mt-1 text-[12px] text-text-muted">
                  {hmm(active.elapsed_seconds)} 경과
                </div>
              )}
              <button
                onClick={() => stopRec.mutate()}
                className="mt-2 flex w-full items-center justify-center gap-1.5 rounded-md py-1 text-[12px] font-medium text-status-error transition hover:bg-status-error-subtle"
              >
                <Square size={12} fill="currentColor" />
                멈춤
              </button>
            </div>
          ) : cur ? (
            <div>
              <div className="text-[11px] font-medium uppercase text-text-subtle">
                지금 예정
              </div>
              <div className="text-[15px] font-semibold leading-tight text-text">
                {cur.options[0]?.name ?? "계획"}
                {cur.options.length > 1 ? " OR" : ""}
              </div>
            </div>
          ) : (
            <div>
              <div className="text-[11px] font-medium uppercase text-text-subtle">
                지금
              </div>
              <div className="text-[15px] font-semibold text-text">
                자유 시간
              </div>
            </div>
          )}
        </div>

        {nxt ? (
          <div className="border-t border-border pt-1.5 text-[12px] text-text-muted">
            다음 · {nxt.options[0]?.name ?? "계획"}
            {nxt.options.length > 1 ? " OR" : ""}{" "}
            <span className="font-mono text-text-subtle">
              {minuteToHHMM(nxt.start_minute)} ({nxt.start_minute - nowMin}분 후)
            </span>
          </div>
        ) : null}
      </div>
    </div>
  );
}

const queryClient = new QueryClient();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <QueryClientProvider client={queryClient}>
    <HudCard />
  </QueryClientProvider>,
);
