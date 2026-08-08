// In-app auto-update store + logic. Checks the GitHub Releases
// `latest.json` manifest declared in `tauri.conf.json` → plugins.updater.
// The desktop shell drives check/download/install via the Tauri updater +
// process plugins; browser/dev is a silent no-op.
import { create } from "zustand";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export type UpdateStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "latest"; checkedAt: number }
  | { kind: "available"; version: string; date?: string; notes?: string }
  | { kind: "downloading"; version: string; downloaded: number; contentLength: number }
  | { kind: "error"; message: string };

interface UpdateState {
  status: UpdateStatus;
  /** Query the endpoint for a newer release. No-op outside the Tauri shell. */
  check: () => Promise<void>;
  /** Download + install the pending update, then relaunch. */
  install: () => Promise<void>;
  reset: () => void;
}

export const useUpdate = create<UpdateState>((set, get) => ({
  status: { kind: "idle" },
  check: async () => {
    if (!inTauri) return;
    set({ status: { kind: "checking" } });
    try {
      const u = await check();
      if (!u?.available) {
        set({ status: { kind: "latest", checkedAt: Date.now() } });
        return;
      }
      set({
        status: {
          kind: "available",
          version: u.version,
          date: u.date,
          notes: u.body,
        },
      });
    } catch (e) {
      set({ status: { kind: "error", message: String(e) } });
    }
  },
  install: async () => {
    if (!inTauri) return;
    const cur = get().status;
    const version = cur.kind === "available" ? cur.version : "";
    set({ status: { kind: "downloading", version, downloaded: 0, contentLength: 0 } });
    try {
      const u = await check();
      if (!u?.available) {
        set({ status: { kind: "latest", checkedAt: Date.now() } });
        return;
      }
      let contentLength = 0;
      let downloaded = 0;
      await u.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            contentLength = event.data.contentLength ?? 0;
            set({ status: { kind: "downloading", version: u.version, downloaded: 0, contentLength } });
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            set({ status: { kind: "downloading", version: u.version, downloaded, contentLength } });
            break;
          case "Finished":
            break;
        }
      });
      await relaunch();
    } catch (e) {
      set({ status: { kind: "error", message: String(e) } });
    }
  },
  reset: () => set({ status: { kind: "idle" } }),
}));
