import { describe, it, expect } from "vitest";
import { swapOrder } from "../tray-slot-order";

describe("swapOrder", () => {
  it("swaps adjacent slots and renormalizes order", () => {
    const slots = [
      { kind: "now_recording", on: true,  order: 0 },
      { kind: "now_next",      on: true,  order: 1 },
      { kind: "state_dot",     on: false, order: 2 },
    ];
    const r = swapOrder(slots, 0, 1);
    expect(r[0].kind).toBe("now_next");
    expect(r[1].kind).toBe("now_recording");
    expect(r.map((s) => s.order)).toEqual([0, 1, 2]);
  });

  it("is a no-op at the boundaries (up at index 0)", () => {
    const slots = [
      { kind: "now_recording", on: true, order: 0 },
      { kind: "now_next",      on: true, order: 1 },
    ];
    expect(swapOrder(slots, 0, -1)).toEqual(slots);
  });

  it("is a no-op at the boundaries (down at last index)", () => {
    const slots = [
      { kind: "now_recording", on: true, order: 0 },
      { kind: "now_next",      on: true, order: 1 },
    ];
    expect(swapOrder(slots, 1, 1)).toEqual(slots);
  });

  it("preserves the `on` field of every slot", () => {
    const slots = [
      { kind: "now_recording", on: true,  order: 0 },
      { kind: "now_next",      on: false, order: 1 },
      { kind: "state_dot",     on: true,  order: 2 },
    ];
    const r = swapOrder(slots, 1, 1);
    expect(r[0].on).toBe(true);
    expect(r[1].on).toBe(true);  // state_dot was at index 2 with on=true
    expect(r[2].on).toBe(false); // now_next was at index 1 with on=false
  });
});
