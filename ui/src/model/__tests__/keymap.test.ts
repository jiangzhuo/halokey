import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { effectivePress, effectiveDouble, actionKind, type HostKeymap } from "../keymap";

const example = (name: string): HostKeymap =>
  JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples", name), "utf8"));

describe("keymap types", () => {
  it("round-trips every example without losing fields", () => {
    for (const f of ["knob-moves-pointer.json", "switch-apps-and-macro.json", "halo-menu.json", "layer-backlight.json"]) {
      const raw = readFileSync(resolve(__dirname, "../../../../docs/examples", f), "utf8");
      const map: HostKeymap = JSON.parse(raw);
      expect(JSON.parse(JSON.stringify(map))).toEqual(JSON.parse(raw));
      expect(map.controls).toHaveLength(6);
    }
  });
  it("resolves effective actions and keeps unknown kinds", () => {
    const map = example("halo-menu.json");
    const knob = map.controls.find((c) => c.index === 3)!;
    expect(actionKind(effectivePress(knob))).toBe("halo");
    expect(effectiveDouble(knob)).toBeNull();
    const weird = { futureAction: { _0: 1 } } as any;
    expect(actionKind(weird)).toBe("futureAction");
  });
  it("exposes lighting records", () => {
    const map = example("layer-backlight.json");
    expect(map.lighting?.lights).toHaveLength(4);
    expect(map.layers?.[0]?.lighting?.lights[0]?.mode).toBe("breathe");
  });
});
