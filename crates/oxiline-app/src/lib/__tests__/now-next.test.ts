import { describe, it, expect } from "vitest";
import { currentSlot, nextSlot } from "../now-next";
import type { PlanSlot } from "../../types";

function slot(id: string, start: number, dur: number): PlanSlot {
  return { plan_id: id, date: "2026-08-02", start_minute: start, duration_minute: dur, options: [], is_resolved: false, resolved_by: null, weekday_mask: 0 };
}

describe("now/next derivation", () => {
  const slots = [slot("a", 600, 60), slot("b", 720, 30), slot("c", 900, 45)]; // 10:00-11:00, 12:00-12:30, 15:00-15:45
  it("current = slot containing now (inclusive start, exclusive end)", () => {
    expect(currentSlot(slots, 600)?.plan_id).toBe("a");
    expect(currentSlot(slots, 659)?.plan_id).toBe("a");
    expect(currentSlot(slots, 660)).toBeNull();
  });
  it("next = first slot strictly after now", () => {
    expect(nextSlot(slots, 600)?.plan_id).toBe("b");
    expect(nextSlot(slots, 659)?.plan_id).toBe("b");
    expect(nextSlot(slots, 945)).toBeNull();
  });
  it("empty slots → null", () => {
    expect(currentSlot([], 0)).toBeNull();
    expect(nextSlot([], 0)).toBeNull();
  });
});
