import type { Behavior, ControlActionMap, Gesture, HostAction, HostKeymap, KeyEntry } from "./keymap";
import { isRotation } from "./keymap";

export type Selection = { layer: number; index: number; gesture: Gesture };
export type Draft = { saved: HostKeymap | null; map: HostKeymap | null; selection: Selection; dirty: boolean };

const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v));
const GKEY = { press: "press", double: "doublePress", long: "longPress" } as const;
const AKEY = { press: "pressAction", double: "doublePressAction", long: "longPressAction" } as const;
const BKEY = { press: "pressBehavior", double: "doublePressBehavior", long: "longPressBehavior" } as const;
const CKEY = { press: "pressTapCount", double: "doublePressTapCount", long: "longPressTapCount" } as const;

export const initialDraft = (): Draft => ({ saved: null, map: null, selection: { layer: 0, index: 0, gesture: "press" }, dirty: false });

/** 收到新的已保存配置：有未保存改动时只更新 saved，不覆盖草稿。 */
export function loadSaved(d: Draft, map: HostKeymap): Draft {
  return { ...d, saved: clone(map), map: d.dirty && d.map ? d.map : clone(map) };
}
export const select = (d: Draft, selection: Selection): Draft => ({ ...d, selection });
export const revert = (d: Draft): Draft => ({ ...d, map: d.saved ? clone(d.saved) : null, dirty: false });
export const markSaved = (d: Draft): Draft => ({ ...d, saved: d.map ? clone(d.map) : null, dirty: false });

export function controlAt(map: HostKeymap, layer: number, index: number): ControlActionMap | undefined {
  if (layer === 0) return map.controls.find((c) => c.index === index);
  return map.layers?.find((l) => l.id === layer)?.controls.find((c) => c.index === index);
}

/** 基础层加该层覆盖的叠加结果，只用于显示。 */
export function resolvedControl(map: HostKeymap, layer: number, index: number): ControlActionMap {
  const base = clone(map.controls.find((c) => c.index === index)!);
  const over = layer === 0 ? undefined : controlAt(map, layer, index);
  if (!over) return base;
  const inh = new Set(over.inheritedGestures ?? []);
  if (!inh.has("press")) Object.assign(base, { press: over.press, pressAction: over.pressAction, pressBehavior: over.pressBehavior, pressTapCount: over.pressTapCount });
  if (!inh.has("doublePress")) Object.assign(base, { doublePress: over.doublePress, doublePressAction: over.doublePressAction, doublePressBehavior: over.doublePressBehavior, doublePressTapCount: over.doublePressTapCount });
  if (!inh.has("longPress")) Object.assign(base, { longPress: over.longPress, longPressAction: over.longPressAction, longPressBehavior: over.longPressBehavior, longPressTapCount: over.longPressTapCount });
  return base;
}

/** 取得可写的控件记录；非基础层缺记录时创建一条全透明的覆盖记录。 */
function writable(map: HostKeymap, layer: number, index: number): ControlActionMap {
  if (layer === 0) return map.controls.find((c) => c.index === index)!;
  map.layers ??= [];
  let l = map.layers.find((x) => x.id === layer);
  if (!l) { l = { id: layer, controls: [] }; map.layers.push(l); map.layers.sort((a, b) => a.id - b.id); }
  let c = l.controls.find((x) => x.index === index);
  if (!c) { c = { index, press: [{ type: 2, code: 0 }], inheritedGestures: ["press", "doublePress", "longPress"] }; l.controls.push(c); }
  return c;
}

function mutate(d: Draft, f: (map: HostKeymap, c: ControlActionMap, g: Gesture) => boolean): Draft {
  if (!d.map) return d;
  const { layer, index, gesture } = d.selection;
  if (isRotation(index) && gesture !== "press") return d;
  const map = clone(d.map);
  const c = writable(map, layer, index);
  if (!f(map, c, gesture)) return d;
  return { ...d, map, dirty: true };
}
const uninherit = (c: ControlActionMap, g: Gesture) => { c.inheritedGestures = (c.inheritedGestures ?? []).filter((x) => x !== GKEY[g]); if (!c.inheritedGestures.length) delete c.inheritedGestures; };

export const assignEntries = (d: Draft, entries: KeyEntry[]) => mutate(d, (_m, c, g) => {
  uninherit(c, g); (c as any)[GKEY[g]] = entries; delete (c as any)[AKEY[g]]; return true;
});
export const assignAction = (d: Draft, action: HostAction) => mutate(d, (_m, c, g) => {
  // MO 只允许按键与旋钮按下的单击 / 长按（与 Rust 校验一致）。
  if ("momentaryLayer" in action && (isRotation(d.selection.index) || g === "double")) return false;
  uninherit(c, g); (c as any)[AKEY[g]] = action; if (g !== "press") delete (c as any)[GKEY[g]]; return true;
});
export const clearGesture = (d: Draft) => mutate(d, (_m, c, g) => {
  uninherit(c, g); delete (c as any)[AKEY[g]];
  if (g === "press") c.press = [{ type: 2, code: 0 }]; else delete (c as any)[GKEY[g]];
  return true;
});
export const setTransparent = (d: Draft) => mutate(d, (_m, c, g) => {
  if (d.selection.layer === 0) return false;
  delete (c as any)[AKEY[g]]; if (g !== "press") delete (c as any)[GKEY[g]];
  const set = new Set(c.inheritedGestures ?? []); set.add(GKEY[g]);
  c.inheritedGestures = (["press", "doublePress", "longPress"] as const).filter((k) => set.has(k));
  return true;
});
export const setBehavior = (d: Draft, b: Behavior) => mutate(d, (_m, c, g) => {
  if (isRotation(d.selection.index) && b === "hold") return false;
  if (g === "double" && b === "hold") return false;
  (c as any)[BKEY[g]] = b; return true;
});
export const setTapCount = (d: Draft, n: number) => mutate(d, (_m, c, g) => {
  (c as any)[CKEY[g]] = Math.min(20, Math.max(2, Math.round(n))); return true;
});
