import { describe, it, expect } from "vitest";
import { resizeDuration } from "../resize";

describe("resizeDuration", () => {
  it("snaps to 5 min", () => {
    expect(resizeDuration(60, 8)).toBe(70);
    expect(resizeDuration(60, 7)).toBe(65);
  });
  it("clamps to minimum", () => {
    expect(resizeDuration(60, -55, 15)).toBe(15);
    expect(resizeDuration(60, -200, 15)).toBe(15);
  });
  it("shrinks by delta", () => {
    expect(resizeDuration(120, -30, 15)).toBe(90);
  });
});
