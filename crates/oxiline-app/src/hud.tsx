import { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "./styles.css";
import { onNowUpdate } from "./lib/api";
import { OxideBar } from "./components/OxideBar";
import { useCategories, useTimeline } from "./hooks";
import { todayStr } from "./lib/store";
import type { NowContext } from "./types";
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
  const [ctx, setCtx] = useState<NowContext | null>(null);
  const catsQ = useCategories();
  const tlQ = useTimeline(todayStr());

  useEffect(() => {
    const un = onNowUpdate((c) => {
      setCtx(c);
      syncTheme();
    });
    return () => {
      void un.then((fn) => fn());
    };
  }, []);

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

  const current = ctx?.current ?? null;
  const next = ctx?.next ?? null;

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
          items={tlQ.data ?? []}
          categories={catsQ.data ?? []}
          dayStartMin={5 * 60}
          totalMin={21 * 60}
          compact
        />

        <div className="flex-1">
          {current ? (
            <div>
              <div className="text-[11px] font-medium uppercase text-text-subtle">
                지금 · now
              </div>
              <div className="text-[15px] font-semibold leading-tight text-text">
                {current.title}
              </div>
              <div className="mt-0.5 flex items-baseline justify-between">
                <span className="font-mono text-[13px] text-text-muted">
                  {current.start_minute != null ? minuteToHHMM(current.start_minute) : ""}
                </span>
                {current.remaining_minute != null && (
                  <span className="font-mono text-[20px] text-interactive-primary">
                    {current.remaining_minute}분 남음
                  </span>
                )}
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

        {next ? (
          <div className="border-t border-border pt-1.5 text-[12px] text-text-muted">
            다음 · {next.title}{" "}
            <span className="font-mono text-text-subtle">
              {next.start_minute != null ? minuteToHHMM(next.start_minute) : ""}
              {next.starts_in_minute != null ? ` (${next.starts_in_minute}분 후)` : ""}
            </span>
          </div>
        ) : !current ? (
          <div className="text-[12px] text-text-subtle">
            오늘 예정된 일이 모두 끝났어요
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
