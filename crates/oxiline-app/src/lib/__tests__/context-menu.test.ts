import { beforeEach, describe, expect, it } from "vitest";
import { clampMenuPosition, useContextMenu } from "../context-menu";

describe("clampMenuPosition", () => {
  const vp = { width: 1000, height: 800 };
  const menu = { width: 200, height: 160 };

  it("leaves position unchanged when the menu fits down-right", () => {
    expect(clampMenuPosition(100, 100, menu, vp)).toEqual({ x: 100, y: 100 });
  });

  it("flips left when the menu would overflow the right edge", () => {
    // x=900 → 900+200=1100 > 1000-8 → flip to x-menu = 700.
    const r = clampMenuPosition(900, 100, menu, vp);
    expect(r.x).toBe(700);
    expect(r.y).toBe(100);
  });

  it("flips up when the menu would overflow the bottom edge", () => {
    // y=700 → 700+160=860 > 800-8 → flip to y-menu = 540.
    const r = clampMenuPosition(100, 700, menu, vp);
    expect(r.x).toBe(100);
    expect(r.y).toBe(540);
  });
  it("clamps to the viewport edge when flipping still overflows", () => {
    // Menu taller than the usable area: maxY = max(8, 800-8-790) = 8. Any
    // cursor flips up past the top, so it clamps to the margin floor (8).
    const tall = { width: 200, height: 790 };
    const r = clampMenuPosition(100, 780, tall, vp);
    expect(r.y).toBe(8);
  });

  it("never returns a coordinate below the margin", () => {
    const r = clampMenuPosition(0, 0, menu, vp);
    expect(r.x).toBeGreaterThanOrEqual(8);
    expect(r.y).toBeGreaterThanOrEqual(8);
  });
});

describe("useContextMenu store", () => {
  beforeEach(() => useContextMenu.getState().close());

  it("opens at coordinates with items, then closes", () => {
    useContextMenu.getState().show(10, 20, [{ kind: "separator" }]);
    const open = useContextMenu.getState();
    expect(open.open).toBe(true);
    expect(open.x).toBe(10);
    expect(open.y).toBe(20);
    expect(open.items).toHaveLength(1);

    useContextMenu.getState().close();
    expect(useContextMenu.getState().open).toBe(false);
    expect(useContextMenu.getState().items).toHaveLength(0);
  });

  it("replaces an open menu on a second show", () => {
    useContextMenu.getState().show(1, 2, [
      { kind: "header", label: "a" },
      { kind: "separator" },
    ]);
    useContextMenu.getState().show(3, 4, [{ kind: "separator" }]);
    const s = useContextMenu.getState();
    expect(s.x).toBe(3);
    expect(s.items).toHaveLength(1);
  });
});
