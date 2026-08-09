// In-app auto-update store + logic. The GUI is a thin view of the
// `oxiline` CLI engine (`doc/10-updater.md`). It spawns the bundled
// `oxiline` sidecar with `--json-progress` and parses the NDJSON
// progress events on stdout. The CLI is the only place that downloads,
// verifies, and swaps releases.
import { create } from "zustand";
import { Command } from "@tauri-apps/plugin-shell";
import { relaunch } from "@tauri-apps/plugin-process";

const inTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Discriminated event matching the CLI `--json-progress` NDJSON contract. */
export type ProgressEvent =
  | { type: "checking" }
  | { type: "current"; version: string }
  | { type: "available"; from: string; to: string; notes: string }
  | { type: "latest"; version: string }
  | { type: "download"; pct: number }
  | { type: "verifying" }
  | { type: "swapping"; mode: "app" | "standalone" }
  | { type: "done"; version: string }
  | { type: "error"; message: string };

/** Extends the existing `UpdateStatus` with a transient "restarting" step. */
export type UpdateStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | {
      kind: "latest";
      checkedAt: number;
      version?: string;
    }
  | {
      kind: "available";
      version: string;
      date?: string;
      notes?: string;
    }
  | {
      kind: "downloading";
      version: string;
      downloaded: number;
      contentLength: number;
      pct: number;
    }
  | {
      kind: "restarting";
      mode: "app" | "standalone";
      version: string;
    }
  | { kind: "error"; message: string };

export function initialState(): UpdateStatus {
  return { kind: "idle" };
}

/**
 * Pure reducer: collapse a single NDJSON event into the next state.
 * Exported separately so the unit tests can pin the schema without
 * running Tauri.
 */
export function reduceEvent(
  state: UpdateStatus,
  ev: ProgressEvent,
): UpdateStatus {
  switch (ev.type) {
    case "checking":
      return { kind: "checking" };
    case "latest":
      return { kind: "latest", checkedAt: Date.now(), version: ev.version };
    case "available":
      return { kind: "available", version: ev.to, notes: ev.notes };
    case "download": {
      const version =
        state.kind === "available"
          ? state.version
          : state.kind === "downloading"
            ? state.version
            : "";
      const contentLength =
        state.kind === "downloading" ? state.contentLength : 0;
      const downloaded =
        state.kind === "downloading"
          ? Math.max(state.downloaded, 1)
          : 0;
      // The CLI emits monotonic-increasing percentages (it suppresses
      // regressions in its own `download` stream); the GUI sees the
      // same monotonic stream. We take the max anyway so a stray
      // out-of-order event (none today) cannot visibly rewind the
      // progress bar.
      const prevPct = state.kind === "downloading" ? state.pct : -1;
      return {
        kind: "downloading",
        version,
        downloaded,
        contentLength,
        pct: Math.max(prevPct, ev.pct),
      };
    }
    case "swapping": {
      const version =
        state.kind === "available"
          ? state.version
          : state.kind === "downloading"
            ? state.version
            : "";
      return { kind: "restarting", mode: ev.mode, version };
    }
    case "done": {
      // The CLI has finished the swap and will shortly signal relaunch
      // via the watched `update_request_at` setting. The GUI's
      // `App.tsx` watcher calls `relaunch()` itself; we just keep
      // state coherent.
      return state.kind === "restarting"
        ? { ...state, version: ev.version }
        : { kind: "latest", checkedAt: Date.now(), version: ev.version };
    }
    case "error":
      return { kind: "error", message: ev.message };
    case "verifying":
    case "current":
      return state; // intermediate; no UI change
  }
}

interface UpdateState {
  status: UpdateStatus;
  check: () => Promise<void>;
  install: () => Promise<void>;
  reset: () => void;
}

// The CLI sidecar name. Tauri auto-injects the platform-suffixed binary
// from `binaries/oxiline-<target-triple>` declared in `tauri.conf.json`
// `bundle.externalBin`; the plugin's `Command.sidecar` API takes the
// base name.
const SIDECAR = "binaries/oxiline";

/** Spawn the CLI as a sidecar and stream its NDJSON progress events. */
async function runUpgrade(args: string[]): Promise<ProgressEvent[]> {
  if (!inTauri) return [];
  const cmd = Command.sidecar(SIDECAR, args);
  const events: ProgressEvent[] = [];
  cmd.stdout.on("data", (line: string) => {
    const trimmed = line.trim();
    if (!trimmed) return;
    try {
      const ev = JSON.parse(trimmed) as ProgressEvent;
      events.push(ev);
    } catch {
      // Ignore non-JSON lines (e.g. human-readable `print!` output)
      // when `--json-progress` is set; the contract is one JSON per
      // line.
    }
  });
  const result = await cmd.execute();
  if (result.code !== 0) {
    const last = events[events.length - 1];
    const msg =
      last && last.type === "error"
        ? last.message
        : `oxiline upgrade exited with code ${result.code}`;
    throw new Error(msg);
  }
  return events;
}

export const useUpdate = create<UpdateState>((set, get) => ({
  status: initialState(),
  check: async () => {
    if (!inTauri) return;
    set({ status: { kind: "checking" } });
    try {
      const events = await runUpgrade([
        "upgrade",
        "--check",
        "--yes",
        "--json-progress",
      ]);
      let status: UpdateStatus = { kind: "checking" };
      for (const ev of events) status = reduceEvent(status, ev);
      // `--check` may exit without ever emitting `latest`/`available`
      // if the fetch failed; the thrown error above catches that case.
      if (status.kind === "checking") {
        status = { kind: "latest", checkedAt: Date.now() };
      }
      set({ status });
    } catch (e) {
      set({ status: { kind: "error", message: String(e) } });
    }
  },
  install: async () => {
    if (!inTauri) return;
    const cur = get().status;
    if (cur.kind !== "available") return;
    set({
      status: {
        kind: "downloading",
        version: cur.version,
        downloaded: 0,
        contentLength: 0,
        pct: 0,
      },
    });
    try {
      const events = await runUpgrade(["upgrade", "--yes", "--json-progress"]);
      let status: UpdateStatus = {
        kind: "downloading",
        version: cur.version,
        downloaded: 0,
        contentLength: 0,
        pct: 0,
      };
      for (const ev of events) status = reduceEvent(status, ev);
      set({ status });
      // The CLI has written `update_request_at`; `App.tsx` watches
      // that and calls `relaunch()` itself. We do not relaunch
      // from here to keep the single-writer invariant.
    } catch (e) {
      set({ status: { kind: "error", message: String(e) } });
    }
  },
  reset: () => set({ status: initialState() }),
}));

/** Re-export `relaunch` so the App-level watcher can call it. */
export { relaunch };
