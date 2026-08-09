import { describe, it, expect } from "vitest";
import { reduceEvent, initialState, type UpdateStatus, type ProgressEvent } from "./updater";

describe("reduceEvent", () => {
  it("`checking` resets to the checking state", () => {
    const next = reduceEvent(initialState(), { type: "checking" });
    expect(next.kind).toBe("checking");
  });

  it("`latest` records the version and a checkedAt timestamp", () => {
    const before = Date.now();
    const next = reduceEvent(initialState(), {
      type: "latest",
      version: "0.6.1",
    });
    expect(next.kind).toBe("latest");
    if (next.kind === "latest") {
      expect(next.version).toBe("0.6.1");
      expect(next.checkedAt).toBeGreaterThanOrEqual(before);
    }
  });

  it("`available` preserves notes for the Preferences panel", () => {
    const next = reduceEvent(initialState(), {
      type: "available",
      from: "0.6.1",
      to: "0.7.0",
      notes: "OxiLine 0.7.0",
    });
    expect(next.kind).toBe("available");
    if (next.kind === "available") {
      expect(next.version).toBe("0.7.0");
      expect(next.notes).toBe("OxiLine 0.7.0");
    }
  });

  it("`download` keeps the highest pct seen", () => {
    let s: UpdateStatus = initialState();
    s = reduceEvent(s, { type: "download", pct: 0 });
    s = reduceEvent(s, { type: "download", pct: 42 });
    s = reduceEvent(s, { type: "download", pct: 7 });
    expect(s.kind).toBe("downloading");
    if (s.kind === "downloading") {
      expect(s.pct).toBe(42);
    }
  });

  it("`swapping` flips to a transient restarting view", () => {
    const next = reduceEvent(initialState(), { type: "swapping", mode: "app" });
    expect(next.kind).toBe("restarting");
    if (next.kind === "restarting") {
      expect(next.mode).toBe("app");
    }
  });

  it("`error` carries the message verbatim", () => {
    const next = reduceEvent(initialState(), { type: "error", message: "boom" });
    expect(next.kind).toBe("error");
    if (next.kind === "error") expect(next.message).toBe("boom");
  });

  it("full check-then-install sequence walks the states", () => {
    let s = initialState();
    const events: ProgressEvent[] = [
      { type: "checking" },
      { type: "available", from: "0.6.1", to: "0.7.0", notes: "OxiLine 0.7.0" },
      { type: "download", pct: 0 },
      { type: "download", pct: 50 },
      { type: "download", pct: 100 },
      { type: "verifying" },
      { type: "swapping", mode: "app" },
      { type: "done", version: "0.7.0" },
    ];
    for (const e of events) s = reduceEvent(s, e);
    expect(s.kind).toBe("restarting");
    if (s.kind === "restarting") {
      expect(s.version).toBe("0.7.0");
      expect(s.mode).toBe("app");
    }
  });
});
