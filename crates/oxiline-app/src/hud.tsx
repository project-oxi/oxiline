import { useEffect, useMemo, useState } from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";
import { Play, Square } from "lucide-react";
import { OxideBar } from "./components/OxideBar";
import { useActivities, useCompliance, useDayRecords, useRecordState, useSlots, useStartRecord, useStopRecord } from "./hooks";
import { api } from "./lib/api";
import { todayStr } from "./lib/store";
import { hmm, hueVar } from "./lib/record-format";
import { isoLocal } from "./lib/record-time";
import { currentSlot, nextSlot } from "./lib/now-next";
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
  const startRec = useStartRecord();
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

  // The HUD shows no context menu, but still suppress the platform's native
  // webview menu so a stray right-click is a clean no-op.
  useEffect(() => {
    const h = (e: MouseEvent) => e.preventDefault();
    document.addEventListener("contextmenu", h);
    return () => document.removeEventListener("contextmenu", h);
  }, []);

  const [tick, setTick] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setTick((t) => t + 1), 1000);
    return () => clearInterval(id);
  }, []);
  const baseNowMin = new Date().getHours() * 60 + new Date().getMinutes();
  const nowMin = baseNowMin + Math.floor(tick / 60);
  const active = stateQ.data?.active ?? null;
  const cur = currentSlot(slotsQ.data ?? [], nowMin);
  const nxt = nextSlot(slotsQ.data ?? [], nowMin);
  const weekComp = active
    ? (weekQ.data ?? []).find((c) => c.activity.id === active.activity.id)
    : undefined;
  const openMain = () => {
    void api.showMainWindow();
  };
  // Today's total recorded time (the live session grows with `tick`). Gives
  const todayTotalSec = useMemo(() => {
    const today = todayStr();
    const nowMs = Date.now();
    return (recsQ.data ?? [])
      .filter((r) => isoLocal(r.started_at).date === today)
      .reduce((sum, r) => {
        const end = r.ended_at ? new Date(r.ended_at).getTime() : nowMs;
        return sum + Math.max(0, (end - new Date(r.started_at).getTime()) / 1000);
      }, 0);
  }, [recsQ.data, tick]);
  // Idle-but-scheduled: which activity to launch from the HUD ("지금 시작").
  const curTarget = cur?.resolved_by?.activity_id ?? cur?.options[0]?.id ?? null;

  return (
    <div className="h-screen w-screen p-2.5">
      <div
        role="button"
        tabIndex={0}
        onClick={openMain}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            openMain();
          }
        }}
        className="flex h-full w-full cursor-pointer flex-col gap-2 rounded-2xl border border-border p-3 outline-none transition-shadow hover:shadow-[var(--shadow-lg)] focus-visible:ring-2 focus-visible:ring-interactive-primary"
        style={{
          background: "var(--color-surface-raised)",
          boxShadow: "var(--shadow-lg)",
          ...(active
            ? { borderLeft: `3px solid ${hueVar(active.activity.hue_label)}`, paddingLeft: 11 }
            : null),
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
              <div className="flex items-center gap-1.5 text-[13px] font-medium leading-tight text-text">
                <span
                  aria-hidden
                  className="inline-block h-2 w-2 shrink-0 animate-pulse rounded-full"
                  style={{ background: hueVar(active.activity.hue_label) }}
                />
                <span className="truncate">{active.activity.name}</span>
              </div>
              <div className="mt-1 flex items-baseline gap-1.5">
                <span className="font-mono text-[22px] font-semibold leading-none tabular-nums text-text">
                  {hmm(active.elapsed_seconds + tick)}
                </span>
                <span className="text-[11px] text-text-subtle">경과</span>
              </div>
              {(weekComp?.target_seconds ?? 0) > 0 && (
                <div className="mt-1.5">
                  <div className="flex items-baseline justify-between text-[11px]">
                    <span className="text-text-muted">이번 주</span>
                    <span className="font-mono text-text-subtle">
                      {hmm(weekComp!.recorded_seconds)}/{hmm(weekComp!.target_seconds!)}
                    </span>
                  </div>
                  <div
                    className="mt-1 h-1 overflow-hidden rounded-full"
                    style={{ background: "var(--color-surface-sunken)" }}
                  >
                    <div
                      className="h-full rounded-full transition-[width] duration-1000 ease-linear"
                      style={{
                        width: `${Math.min(100, Math.round((weekComp!.ratio ?? 0) * 100))}%`,
                        background: hueVar(active.activity.hue_label),
                      }}
                    />
                  </div>
                </div>
              )}
              <button
                onPointerDown={(e) => e.stopPropagation()}
                onClick={(e) => {
                  e.stopPropagation();
                  stopRec.mutate();
                }}
                className="mt-2 flex w-full items-center justify-center gap-1.5 rounded-md bg-status-error-subtle py-1 text-[12px] font-medium text-status-error transition hover:bg-status-error hover:text-text-inverse"
              >
                <Square size={12} fill="currentColor" />
                멈춤
              </button>
            </div>
          ) : cur ? (
            <div>
              <div className="text-[10px] font-medium uppercase tracking-wide text-text-subtle">
                지금 예정
              </div>
              <div className="truncate text-[15px] font-semibold leading-tight text-text">
                {cur.options[0]?.name ?? "계획"}
                {cur.options.length > 1 ? " OR" : ""}
              </div>
              <button
                disabled={!curTarget}
                onPointerDown={(e) => e.stopPropagation()}
                onClick={(e) => {
                  e.stopPropagation();
                  if (curTarget) startRec.mutate(curTarget);
                }}
                className="mt-2 flex w-full items-center justify-center gap-1.5 rounded-md bg-interactive-primary py-1 text-[12px] font-medium text-interactive-primary-foreground transition hover:opacity-90 disabled:opacity-40"
              >
                <Play size={12} fill="currentColor" />
                지금 시작
              </button>
            </div>
          ) : (
            <div>
              <div className="text-[10px] font-medium uppercase tracking-wide text-text-subtle">
                지금
              </div>
              <div className="text-[15px] font-semibold leading-tight text-text">자유 시간</div>
              <div className="mt-0.5 text-[11px] text-text-subtle">
                오늘 <span className="font-mono tabular-nums">{hmm(todayTotalSec)}</span> 기록
              </div>
            </div>
          )}
        </div>

        {nxt && (
          <div className="border-t border-border pt-1.5 text-[11px] text-text-muted">
            다음 · {nxt.options[0]?.name ?? "계획"}
            {nxt.options.length > 1 ? " OR" : ""}{" "}
            <span className="font-mono text-text-subtle">
              {minuteToHHMM(nxt.start_minute)} ({nxt.start_minute - nowMin}분 후)
            </span>
          </div>
        )}
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
