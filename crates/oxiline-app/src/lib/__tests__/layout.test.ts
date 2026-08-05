import { describe, it, expect } from "vitest";
import { packColumns, type Rect } from "../layout";

const r = (start: number, end: number): Rect => ({ start, end });

describe("packColumns", () => {
  it("returns empty for no items", () => {
    expect(packColumns([])).toEqual([]);
  });

  it("packs non-overlapping items into a single column", () => {
    // 09:00–10:00, 10:00–11:00, 11:00–12:00 — touch but don't overlap
    const items = [r(540, 600), r(600, 660), r(660, 720)];
    const out = packColumns(items);
    expect(out).toEqual([
      { col: 0, cols: 1 },
      { col: 0, cols: 1 },
      { col: 0, cols: 1 },
    ]);
  });

  it("splits two fully-overlapping items across two columns", () => {
    const items = [r(540, 600), r(540, 600)];
    const out = packColumns(items);
    expect(out).toEqual([
      { col: 0, cols: 2 },
      { col: 1, cols: 2 },
    ]);
  });

  it("packs three mutually-overlapping items across three columns", () => {
    const items = [r(540, 660), r(570, 630), r(540, 600)];
    const out = packColumns(items);
    // Greedy coloring picks deterministic columns per sort order, so assert
    // the semantic guarantee: all share cols=3 and occupy distinct columns.
    expect(out.every((p) => p.cols === 3)).toBe(true);
    expect(new Set(out.map((p) => p.col))).toEqual(new Set([0, 1, 2]));
  });
  it("reuses a column once an earlier item has ended (chain overlap)", () => {
    // A∩B overlap, B∩C overlap, but A∩C do not → C reuses A's column.
    const items = [r(540, 630), r(600, 690), r(660, 750)];
    const out = packColumns(items);
    expect(out).toEqual([
      { col: 0, cols: 2 },
      { col: 1, cols: 2 },
      { col: 0, cols: 2 },
    ]);
  });

  it("separates disjoint overlap groups into independent clusters", () => {
    // Cluster 1: A 09–10, B 09–10. Cluster 2: C 14–15, D 14–15.
    const items = [r(540, 600), r(540, 600), r(840, 900), r(840, 900)];
    const out = packColumns(items);
    expect(out).toEqual([
      { col: 0, cols: 2 },
      { col: 1, cols: 2 },
      { col: 0, cols: 2 },
      { col: 1, cols: 2 },
    ]);
  });

  it("preserves input order in the output regardless of internal sort", () => {
    // Given out of time-order; output index must align with input index.
    const items = [r(840, 900), r(540, 600), r(660, 750)];
    const out = packColumns(items);
    expect(out.length).toBe(items.length);
    // All non-overlapping → single column each
    expect(out).toEqual([
      { col: 0, cols: 1 },
      { col: 0, cols: 1 },
      { col: 0, cols: 1 },
    ]);
  });
});
