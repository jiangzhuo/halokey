import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { initialDraft, loadSaved, select, assignEntries, assignAction, clearGesture, setTransparent, setBehavior, setTapCount, revert, markSaved, controlAt, resolvedControl, effectiveLighting, hasOwnLighting, setLightEffect, setFollowBase, clearBaseLighting } from "../draft";
import type { HostKeymap } from "../keymap";

const load = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/halo-menu.json"), "utf8"));

describe("draft reducer", () => {
  it("loads, selects, assigns and tracks dirty", () => {
    let d = loadSaved(initialDraft(), load());
    expect(d.dirty).toBe(false);
    d = select(d, { layer: 0, index: 1, gesture: "press" });
    d = assignEntries(d, [{ type: 2, code: 0x2b }]);
    expect(d.dirty).toBe(true);
    expect(controlAt(d.map!, 0, 1)!.press).toEqual([{ type: 2, code: 0x2b }]);
    expect(controlAt(d.map!, 0, 1)!.pressAction).toBeUndefined();
    d = assignAction(d, { pointerMove: { _0: { dx: 20, dy: 0 } } });
    expect(controlAt(d.map!, 0, 1)!.pressAction).toEqual({ pointerMove: { _0: { dx: 20, dy: 0 } } });
    d = revert(d);
    expect(d.dirty).toBe(false);
    expect(controlAt(d.map!, 0, 1)!.press).toEqual(load().controls[1]!.press);
  });
  it("does not overwrite a dirty draft when a new saved map arrives", () => {
    let d = loadSaved(initialDraft(), load());
    d = assignEntries(select(d, { layer: 0, index: 0, gesture: "press" }), [{ type: 2, code: 0x04 }]);
    const incoming = load(); incoming.doublePressWindow = 0.3;
    d = loadSaved(d, incoming);
    expect(d.map!.doublePressWindow).toBe(0.25);
    expect(d.saved!.doublePressWindow).toBe(0.3);
    d = markSaved(d);
    expect(d.dirty).toBe(false);
  });
  it("creates layer overrides on demand and supports transparency", () => {
    let d = loadSaved(initialDraft(), load());
    d = select(d, { layer: 2, index: 1, gesture: "long" });
    d = assignEntries(d, [{ type: 2, code: 0x29 }]);
    const layer = d.map!.layers!.find((l) => l.id === 2)!;
    expect(layer.controls).toHaveLength(1);
    expect(layer.controls[0]!.longPress).toEqual([{ type: 2, code: 0x29 }]);
    expect(layer.controls[0]!.inheritedGestures).toEqual(["press", "doublePress"]);
    expect(resolvedControl(d.map!, 2, 1).press).toEqual(load().controls[1]!.press);
    d = setTransparent(select(d, { layer: 2, index: 1, gesture: "long" }));
    expect(controlAt(d.map!, 2, 1)!.inheritedGestures).toEqual(["press", "doublePress", "longPress"]);
    expect(controlAt(d.map!, 2, 1)!.longPress).toBeUndefined();
  });
  it("clears gestures, rejects rotation extras and clamps tap counts", () => {
    let d = loadSaved(initialDraft(), load());
    d = clearGesture(select(d, { layer: 0, index: 0, gesture: "press" }));
    expect(controlAt(d.map!, 0, 0)!.press).toEqual([{ type: 2, code: 0 }]);
    d = clearGesture(select(d, { layer: 0, index: 0, gesture: "double" }));
    expect(controlAt(d.map!, 0, 0)!.doublePress).toBeUndefined();
    const before = JSON.stringify(d.map);
    d = assignEntries(select(d, { layer: 0, index: 4, gesture: "long" }), [{ type: 2, code: 0x04 }]);
    expect(JSON.stringify(d.map)).toBe(before);
    d = setBehavior(select(d, { layer: 0, index: 4, gesture: "press" }), "burst");
    expect(controlAt(d.map!, 0, 4)!.pressBehavior).toBe("burst");
    const beforeHold = JSON.stringify(d.map);
    d = setBehavior(select(d, { layer: 0, index: 4, gesture: "press" }), "hold");
    d = setBehavior(select(d, { layer: 0, index: 0, gesture: "double" }), "hold");
    expect(JSON.stringify(d.map)).toBe(beforeHold);
    d = setTapCount(select(d, { layer: 0, index: 0, gesture: "press" }), 99);
    expect(controlAt(d.map!, 0, 0)!.pressTapCount).toBe(20);
  });
  it("refuses momentary layer on rotation and double press", () => {
    let d = loadSaved(initialDraft(), load());
    const before = JSON.stringify(d.map);
    d = assignAction(select(d, { layer: 0, index: 4, gesture: "press" }), { momentaryLayer: { _0: 1 } });
    d = assignAction(select(d, { layer: 0, index: 0, gesture: "double" }), { momentaryLayer: { _0: 1 } });
    expect(JSON.stringify(d.map)).toBe(before);
    expect(d.dirty).toBe(false);
    d = assignAction(select(d, { layer: 0, index: 0, gesture: "long" }), { momentaryLayer: { _0: 2 } });
    expect(controlAt(d.map!, 0, 0)!.longPressAction).toEqual({ momentaryLayer: { _0: 2 } });
  });
});

describe("draft lighting", () => {
  const loadLit = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/layer-backlight.json"), "utf8"));
  it("resolves effective lighting per layer", () => {
    const m = loadLit();
    expect(effectiveLighting(m, 0)!.lights[0]!.mode).toBe("solid");
    expect(effectiveLighting(m, 1)!.lights[0]!.mode).toBe("breathe");
    expect(effectiveLighting(m, 2)!.lights[0]!.mode).toBe("solid");
    expect(hasOwnLighting(m, 2)).toBe(false);
    expect(effectiveLighting(load(), 0)).toBeNull();
  });
  it("edits create a record for the selected layer and mark dirty", () => {
    let d = loadSaved(initialDraft(), loadLit());
    d = setLightEffect(select(d, { layer: 2, index: 0, gesture: "press" }), 3, { mode: "breathe", brightness: 7 });
    expect(d.dirty).toBe(true);
    const l2 = d.map!.layers!.find((l) => l.id === 2)!;
    expect(l2.lighting!.lights[3]).toEqual({ mode: "breathe", brightness: 7 });
    expect(l2.lighting!.lights[0]).toEqual({ mode: "solid", brightness: 10 }); // 其余灯复制自有效背光
    expect(l2.controls).toEqual([]);
  });
  it("follow toggles create and delete the record cleanly", () => {
    let d = loadSaved(initialDraft(), loadLit());
    d = select(d, { layer: 1, index: 0, gesture: "press" });
    d = setFollowBase(d, true);
    expect(d.map!.layers!.find((l) => l.id === 1)!.lighting).toBeUndefined();
    d = setFollowBase(d, false);
    expect(d.map!.layers!.find((l) => l.id === 1)!.lighting!.lights[0]).toEqual({ mode: "solid", brightness: 10 });
    const before = JSON.stringify(d.map);
    d = setFollowBase(select(d, { layer: 0, index: 0, gesture: "press" }), true);
    expect(JSON.stringify(d.map)).toBe(before);
  });
  it("base layer can drop its record and edits start from all-off when nothing exists", () => {
    let d = loadSaved(initialDraft(), loadLit());
    d = clearBaseLighting(select(d, { layer: 0, index: 0, gesture: "press" }));
    expect(d.map!.lighting).toBeUndefined();
    d = setLightEffect(d, 0, { brightness: 12 });
    expect(d.map!.lighting!.lights[0]).toEqual({ mode: "off", brightness: 12 });
    expect(d.map!.lighting!.lights[1]).toEqual({ mode: "off", brightness: 0 });
  });
  it("clamps brightness to 0–20", () => {
    let d = loadSaved(initialDraft(), loadLit());
    d = setLightEffect(d, 0, { brightness: 99 });
    expect(d.map!.lighting!.lights[0]!.brightness).toBe(20);
    d = setLightEffect(d, 0, { brightness: -4 });
    expect(d.map!.lighting!.lights[0]!.brightness).toBe(0);
  });
});
