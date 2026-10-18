import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { initialDraft, loadSaved, select, assignEntries, assignAction, clearGesture, setTransparent, setBehavior, setTapCount, revert, markSaved, controlAt, resolvedControl, effectiveLighting, hasOwnLighting, setLightEffect, setFollowBase, clearBaseLighting, newHaloId, upsertHalo, deleteHalo, haloReferences, haloActionAllowed, newUuid, nextSlot, upsertLibraryItem, deleteLibraryItem, libraryReferences, libraryItems } from "../draft";
import type { NamedHostAction } from "../keymap";
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

describe("draft halos", () => {
  const loadHalo = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/halo-menu.json"), "utf8"));
  it("allocates ids and upserts", () => {
    let d = loadSaved(initialDraft(), loadHalo());
    expect(newHaloId(d.map!)).toBe("halo-1");
    d = upsertHalo(d, { id: "halo-1", name: "新建", options: [{ label: "A", action: { keyboard: { _0: [{ type: 2, code: 4 }] } } }] });
    expect(d.dirty).toBe(true);
    expect(d.map!.halos!.map((h) => h.id)).toEqual(["main", "more", "halo-1"]);
    expect(newHaloId(d.map!)).toBe("halo-2");
    d = upsertHalo(d, { id: "main", name: "改名", options: d.map!.halos![0]!.options });
    expect(d.map!.halos![0]!.name).toBe("改名");
    expect(d.map!.halos).toHaveLength(3);
  });
  it("lists references and refuses to delete a referenced halo", () => {
    const m = loadHalo();
    expect(haloReferences(m, "main")).toEqual(["旋钮按下 › 单击", "光环「更多」槽位 2"]); // 示例里"更多"有一项回到主菜单
    expect(haloReferences(m, "more")).toEqual(["光环「主菜单」槽位 7"]);
    let d = loadSaved(initialDraft(), m);
    const r = deleteHalo(d, "more");
    expect(r.blockedBy).toEqual(["光环「主菜单」槽位 7"]);
    expect(r.draft.map!.halos).toHaveLength(2);
    expect(r.draft.dirty).toBe(false);
    d = upsertHalo(d, { id: "halo-1", name: "孤儿", options: [{ label: "x", action: { keyboard: { _0: [{ type: 2, code: 4 }] } } }] });
    const r2 = deleteHalo(d, "halo-1");
    expect(r2.blockedBy).toEqual([]);
    expect(r2.draft.map!.halos!.map((h) => h.id)).toEqual(["main", "more"]);
  });
  it("finds references inside layers", () => {
    let d = loadSaved(initialDraft(), loadHalo());
    d = assignAction(select(d, { layer: 2, index: 1, gesture: "long" }), { halo: { _0: "more" } });
    expect(haloReferences(d.map!, "more")).toEqual(["光环「主菜单」槽位 7", "Layer 2 · 中间按键 › 长按"]);
  });
  it("blocks self-reference, library and layer actions in halo options", () => {
    expect(haloActionAllowed({ halo: { _0: "x" } }, "x")).toBe(false);
    expect(haloActionAllowed({ halo: { _0: "y" } }, "x")).toBe(true);
    expect(haloActionAllowed({ library: { _0: "a" } }, null)).toBe(false);
    expect(haloActionAllowed({ momentaryLayer: { _0: 1 } }, null)).toBe(false);
    expect(haloActionAllowed({ pointerMove: { _0: { dx: 1, dy: 0 } } }, null)).toBe(true);
  });
});

describe("draft action library", () => {
  const loadApps = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/switch-apps-and-macro.json"), "utf8"));
  const app = (id: string, slot: number, name = "X"): NamedHostAction => ({ id, name, slot, action: { application: { _0: { bundleIdentifier: "x", path: "", name } } } });
  it("generates uuids and allocates slots per kind", () => {
    expect(newUuid()).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
    let d = loadSaved(initialDraft(), load());
    expect(nextSlot(d.map!, "application")).toBe(0);
    d = upsertLibraryItem(d, app("11111111-1111-4111-8111-111111111111", 0, "A"));
    d = upsertLibraryItem(d, app("22222222-2222-4222-8222-222222222222", 2, "C"));
    expect(nextSlot(d.map!, "application")).toBe(1);
    expect(nextSlot(d.map!, "macro")).toBe(0);
    expect(libraryItems(d.map!, "application").map((i) => i.name)).toEqual(["A", "C"]);
    expect(d.dirty).toBe(true);
    for (let s = 0; s < 16; s++) d = upsertLibraryItem(d, app(`33333333-3333-4333-8333-${String(s).padStart(12, "0")}`, s));
    expect(nextSlot(d.map!, "application")).toBeNull();
  });
  it("lists references and blocks deleting a referenced item", () => {
    let d = loadSaved(initialDraft(), load());
    const id = "44444444-4444-4444-8444-444444444444";
    d = upsertLibraryItem(d, app(id, 0, "Chrome"));
    d = assignAction(select(d, { layer: 0, index: 0, gesture: "long" }), { library: { _0: id } });
    d = assignAction(select(d, { layer: 1, index: 2, gesture: "press" }), { library: { _0: id } });
    expect(libraryReferences(d.map!, id)).toEqual(["顶部按键 › 长按", "Layer 1 · 底部按键 › 单击"]);
    const r = deleteLibraryItem(d, id);
    expect(r.blockedBy).toHaveLength(2);
    expect(r.draft.map!.actionLibrary).toHaveLength(1);
    d = clearGesture(select(d, { layer: 0, index: 0, gesture: "long" }));
    d = clearGesture(select(d, { layer: 1, index: 2, gesture: "press" }));
    const r2 = deleteLibraryItem(d, id);
    expect(r2.blockedBy).toEqual([]);
    expect(r2.draft.map!.actionLibrary).toBeUndefined();
  });
  it("keeps the example's inline actions untouched", () => {
    const m = loadApps();
    expect(libraryItems(m, "application")).toEqual([]);
    expect(nextSlot(m, "macro")).toBe(0);
  });
});
