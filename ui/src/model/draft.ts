import type { Behavior, ControlActionMap, Gesture, Halo, HostAction, HostKeymap, KeyEntry, LayerLighting, LightEffect } from "./keymap";
import { CONTROL_NAMES } from "./labels";
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

// ---- 分层背光 ----

export const DEFAULT_LIGHTING: LayerLighting = { lights: Array.from({ length: 4 }, () => ({ mode: "off" as const, brightness: 0 })) };

/** 该层自己的记录，否则 Layer 0 的记录，否则 null（不管灯）。 */
export function effectiveLighting(map: HostKeymap, layer: number): LayerLighting | null {
  const own = layer === 0 ? map.lighting : map.layers?.find((l) => l.id === layer)?.lighting;
  return own ?? map.lighting ?? null;
}
export const hasOwnLighting = (map: HostKeymap, layer: number): boolean =>
  !!(layer === 0 ? map.lighting : map.layers?.find((l) => l.id === layer)?.lighting);

/** 修改当前层某盏灯；层没有记录时先用有效背光（或全关）建一条。 */
export function setLightEffect(d: Draft, light: number, patch: Partial<LightEffect>): Draft {
  if (!d.map || light < 0 || light > 3) return d;
  const map = clone(d.map);
  const layer = d.selection.layer;
  const record: LayerLighting = clone(effectiveLighting(map, layer) ?? DEFAULT_LIGHTING);
  const cur = record.lights[light]!;
  record.lights[light] = { mode: patch.mode ?? cur.mode, brightness: Math.min(20, Math.max(0, Math.round(patch.brightness ?? cur.brightness))) };
  writeLighting(map, layer, record);
  return { ...d, map, dirty: true };
}

/** 非基础层：跟随 Layer 0 = 删除本层记录；不跟随 = 以有效背光建一条。 */
export function setFollowBase(d: Draft, follow: boolean): Draft {
  if (!d.map || d.selection.layer === 0) return d;
  const map = clone(d.map);
  const layer = d.selection.layer;
  if (follow) {
    const l = map.layers?.find((x) => x.id === layer);
    if (!l?.lighting) return d;
    delete l.lighting;
    if (!l.controls.length) map.layers = map.layers!.filter((x) => x.id !== layer);
    if (map.layers && !map.layers.length) delete map.layers;
  } else {
    if (hasOwnLighting(map, layer)) return d;
    writeLighting(map, layer, clone(effectiveLighting(map, layer) ?? DEFAULT_LIGHTING));
  }
  return { ...d, map, dirty: true };
}

/** 基础层：删除记录 = 不管灯。 */
export function clearBaseLighting(d: Draft): Draft {
  if (!d.map || d.selection.layer !== 0 || !d.map.lighting) return d;
  const map = clone(d.map);
  delete map.lighting;
  return { ...d, map, dirty: true };
}

function writeLighting(map: HostKeymap, layer: number, record: LayerLighting) {
  if (layer === 0) { map.lighting = record; return; }
  map.layers ??= [];
  let l = map.layers.find((x) => x.id === layer);
  if (!l) { l = { id: layer, controls: [] }; map.layers.push(l); map.layers.sort((a, b) => a.id - b.id); }
  l.lighting = record;
}

// ---- 光环 ----

export function newHaloId(map: HostKeymap): string {
  const used = new Set((map.halos ?? []).map((h) => h.id));
  for (let i = 1; ; i++) if (!used.has(`halo-${i}`)) return `halo-${i}`;
}

export function upsertHalo(d: Draft, halo: Halo): Draft {
  if (!d.map) return d;
  const map = clone(d.map);
  map.halos ??= [];
  const i = map.halos.findIndex((h) => h.id === halo.id);
  if (i >= 0) map.halos[i] = clone(halo); else map.halos.push(clone(halo));
  return { ...d, map, dirty: true };
}

const GESTURE_NAME = { press: "单击", double: "双击", long: "长按" } as const;

/** 谁在用这个光环：基础层的手势动作、其它光环的槽位、各层的手势动作。 */
export function haloReferences(map: HostKeymap, id: string): string[] {
  const refs: string[] = [];
  const isRef = (a: HostAction | null | undefined) => !!a && "halo" in a && (a as { halo: { _0: string } }).halo._0 === id;
  const scan = (controls: ControlActionMap[], prefix: string) => {
    for (const c of controls) {
      const name = CONTROL_NAMES[c.index] ?? `控件 ${c.index}`;
      if (isRef(c.pressAction)) refs.push(`${prefix}${name} › ${GESTURE_NAME.press}`);
      if (isRef(c.doublePressAction)) refs.push(`${prefix}${name} › ${GESTURE_NAME.double}`);
      if (isRef(c.longPressAction)) refs.push(`${prefix}${name} › ${GESTURE_NAME.long}`);
    }
  };
  scan(map.controls, "");
  for (const h of map.halos ?? []) h.options.forEach((o, i) => { if (isRef(o.action)) refs.push(`光环「${h.name}」槽位 ${i + 1}`); });
  for (const l of map.layers ?? []) scan(l.controls, `Layer ${l.id} · `);
  return refs;
}

export function deleteHalo(d: Draft, id: string): { draft: Draft; blockedBy: string[] } {
  if (!d.map) return { draft: d, blockedBy: [] };
  const blockedBy = haloReferences(d.map, id);
  if (blockedBy.length) return { draft: d, blockedBy };
  const map = clone(d.map);
  map.halos = (map.halos ?? []).filter((h) => h.id !== id);
  if (!map.halos.length) delete map.halos;
  return { draft: { ...d, map, dirty: true }, blockedBy: [] };
}

/** 光环选项允许的动作：不能引用功能库、不能切层、不能套自己（与 Rust Halo::validate 一致）。 */
export function haloActionAllowed(action: HostAction, editingId: string | null): boolean {
  if ("library" in action || "momentaryLayer" in action) return false;
  if ("halo" in action && (action as { halo: { _0: string } }).halo._0 === editingId) return false;
  return true;
}
