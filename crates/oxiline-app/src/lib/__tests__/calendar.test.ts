import { describe, it, expect } from "vitest";
import { monthGrid, monthBounds, shiftMonth } from "../calendar";

describe("monthGrid (Mon-first)", () => {
  it("2026-08 starts Mon 2026-07-27 and spans 42 cells", () => {
    const g = monthGrid("2026-08-15");
    expect(g).toHaveLength(42);
    expect(g[0]).toBe("2026-07-27"); // 2026-08-01 is a Saturday → Mon offset 5
    expect(g).toContain("2026-08-01");
    expect(g).toContain("2026-08-31");
  });
  it("bounds = first..last cell", () => {
    const b = monthBounds("2026-08-15");
    expect(b.from).toBe("2026-07-27");
    expect(b.to).toBe("2026-09-06"); // 42 cells from 07-27
  });
  it("handles December→January wrap", () => {
    const g = monthGrid("2026-12-10");
    expect(g).toContain("2026-12-01");
    expect(g).toContain("2027-01-01");
  });
});

describe("shiftMonth", () => {
  it("moves one calendar month, day-normalized", () => {
    expect(shiftMonth("2026-05-02", -1)).toBe("2026-04-01");
    expect(shiftMonth("2026-01-15", 1)).toBe("2026-02-01");
    expect(shiftMonth("2026-03-31", -1)).toBe("2026-02-01"); // no overflow
  });
  it("never skips a month (no day drift)", () => {
    let d = "2026-08-15";
    const visited: string[] = [];
    for (let i = 0; i < 6; i++) {
      d = shiftMonth(d, -1);
      visited.push(d.slice(0, 7));
    }
    expect(visited).toEqual(["2026-07", "2026-06", "2026-05", "2026-04", "2026-03", "2026-02"]);
  });
});
