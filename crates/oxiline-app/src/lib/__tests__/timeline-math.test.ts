import { describe, it, expect } from "vitest";
import { snapMinute, clampDuration, groupClusters, partitionCluster } from "../timeline-math";
import type { TimelineItem } from "../../types";

const item = (id: string, start: number, dur: number): TimelineItem => ({
  id, is_virtual: false, title: id, start_minute: start, duration_minute: dur,
  category_id: null, is_done: false, is_skipped: false, origin_routine_block_id: null,
});

describe("snapMinute", () => {
  it("rounds to step", () => {
    expect(snapMinute(17, 15)).toBe(15);
    expect(snapMinute(23, 15)).toBe(30);
  });
  it("clamps to [0, 1440-step]", () => {
    expect(snapMinute(-5, 15)).toBe(0);
    expect(snapMinute(2000, 15)).toBe(1425);
  });
});

describe("clampDuration", () => {
  it("floors at minDur and ceilings at dayEndMin - start", () => {
    expect(clampDuration(600, 5, 1320)).toBe(15);
    expect(clampDuration(600, 9999, 1320)).toBe(720);
    expect(clampDuration(600, 45, 1320)).toBe(45);
  });
  it("never goes below minDur even if window smaller", () => {
    expect(clampDuration(1310, 30, 1320, 15)).toBe(10);
  });
});

describe("groupClusters", () => {
  it("groups overlapping, splits disjoint", () => {
    const a = item("a", 540, 60);
    const b = item("b", 555, 60);
    const c = item("c", 660, 30);
    const clusters = groupClusters([c, a, b]);
    expect(clusters).toHaveLength(2);
    expect(clusters[0].map((i) => i.id)).toEqual(["a", "b"]);
    expect(clusters[1].map((i) => i.id)).toEqual(["c"]);
  });
  it("item-count semantics: staggered 4-item cluster (max 3 concurrent) is still ONE cluster", () => {
    const a = item("a", 540, 60);
    const b = item("b", 555, 60);
    const c = item("c", 585, 75);
    const d = item("d", 630, 60);
    const clusters = groupClusters([a, b, c, d]);
    expect(clusters).toHaveLength(1);
    expect(clusters[0]).toHaveLength(4);
    expect(partitionCluster(clusters[0], 3).overflow).toHaveLength(1);
  });
});

describe("partitionCluster", () => {
  it("keeps first `cap` by selection rule, rest overflow", () => {
    const a = item("a", 540, 30);
    const b = item("b", 540, 60);
    const c = item("c", 540, 30);
    const d = item("d", 540, 30);
    const { visible, overflow } = partitionCluster([a, b, c, d], 3);
    expect(visible.map((i) => i.id)).toEqual(["b", "a", "c"]);
    expect(overflow.map((i) => i.id)).toEqual(["d"]);
  });
  it("empty overflow when within cap", () => {
    const { overflow } = partitionCluster([item("a", 0, 30)], 3);
    expect(overflow).toEqual([]);
  });
});
