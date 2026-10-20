// 与 Rust HostKeymap 一一对应的类型；前端只读写这个形状，不做语义校验（校验在 Rust）。
export type KeyEntry = { type: number; code: number };
export type ApplicationTarget = { bundleIdentifier: string; path: string; name: string };
export type MacroStep =
  | { keyboard: { _0: KeyEntry[] } }
  | { application: { _0: ApplicationTarget } }
  | { delay: { _0: number } };
export type PointerDelta = { dx: number; dy: number };
export type HostAction =
  | { keyboard: { _0: KeyEntry[] } }
  | { application: { _0: ApplicationTarget } }
  | { macro: { _0: MacroStep[] } }
  | { library: { _0: string } }
  | { momentaryLayer: { _0: number } }
  | { toggleLayer: { _0: number } }
  | { toLayer: { _0: number } }
  | { oneShotLayer: { _0: number } }
  | { pointerMove: { _0: PointerDelta } }
  | { halo: { _0: string } }
  | { [other: string]: unknown };
export type Behavior = "hold" | "tap" | "burst";
export type Gesture = "press" | "double" | "long";
export const GESTURES: readonly Gesture[] = ["press", "double", "long"];

export type ControlActionMap = {
  index: number;
  press: KeyEntry[];
  doublePress?: KeyEntry[] | null;
  longPress?: KeyEntry[] | null;
  pressAction?: HostAction | null;
  doublePressAction?: HostAction | null;
  longPressAction?: HostAction | null;
  pressBehavior?: Behavior;
  pressTapCount?: number;
  doublePressBehavior?: Behavior;
  doublePressTapCount?: number;
  longPressBehavior?: Behavior;
  longPressTapCount?: number;
  inheritedGestures?: ("press" | "doublePress" | "longPress")[];
};
export type LightMode = "off" | "solid" | "breathe";
export type LightEffect = { mode: LightMode; brightness: number };
/** 四盏灯：键 1、键 2、键 3、旋钮（键 1 的灯在 AU05 上不可见）。 */
export type LayerLighting = { lights: LightEffect[] };
export type HostLayer = { id: number; controls: ControlActionMap[]; lighting?: LayerLighting | null };
export type NamedHostAction = { id: string; name: string; slot: number; action: HostAction };
export type HaloOption = { label: string; action: HostAction };
export type Halo = { id: string; name: string; options: HaloOption[] };
export type HostKeymap = {
  version: number;
  controls: ControlActionMap[];
  layers?: HostLayer[];
  actionLibrary?: NamedHostAction[];
  halos?: Halo[];
  lighting?: LayerLighting | null;
  doublePressWindow: number;
  longPressThreshold: number;
};

export function actionKind(a: HostAction): string {
  return Object.keys(a)[0] ?? "unknown";
}
export function actionValue<T = unknown>(a: HostAction): T {
  return (a as any)[actionKind(a)]?._0 as T;
}
export function effectivePress(c: ControlActionMap): HostAction {
  return c.pressAction ?? { keyboard: { _0: c.press } };
}
export function effectiveDouble(c: ControlActionMap): HostAction | null {
  return c.doublePressAction ?? (c.doublePress ? { keyboard: { _0: c.doublePress } } : null);
}
export function effectiveLong(c: ControlActionMap): HostAction | null {
  return c.longPressAction ?? (c.longPress ? { keyboard: { _0: c.longPress } } : null);
}
export const isRotation = (index: number) => index === 4 || index === 5;

/** 四种切层动作的键名与显示缩写（QMK 叫法）。 */
export const LAYER_ACTIONS = { momentaryLayer: "MO", toggleLayer: "TG", toLayer: "TO", oneShotLayer: "OSL" } as const;
export type LayerActionKind = keyof typeof LAYER_ACTIONS;
export const isLayerAction = (a: HostAction): boolean => actionKind(a) in LAYER_ACTIONS;
