import { describe, it, expect } from "vitest";
import { wedgePath, wedgeCenter } from "../wedge";

describe("wedge geometry", () => {
  it("places the first wedge centred on 12 o'clock and goes clockwise", () => {
    const c0 = wedgeCenter(8, 0, 100);
    expect(Math.abs(c0.x)).toBeLessThan(1e-9);
    expect(c0.y).toBeCloseTo(-100);
    const c2 = wedgeCenter(8, 2, 100);
    expect(c2.x).toBeCloseTo(100);
    expect(Math.abs(c2.y)).toBeLessThan(1e-9);
  });
  it("draws a full ring for a single option and valid arcs for two", () => {
    const one = wedgePath(1, 0, 100, 60);
    expect(one.startsWith("M")).toBe(true);
    expect((one.match(/A/g) ?? []).length).toBe(4);
    const two = wedgePath(2, 1, 100, 60);
    expect(two).toContain("A 100 100 0 0 1");
    expect(two).toContain("A 60 60 0 0 0");
    expect(two.endsWith("Z")).toBe(true);
  });
});
