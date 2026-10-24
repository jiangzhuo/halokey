import { actionKind, actionValue, LAYER_ACTIONS, type LayerActionKind, type HostAction, type KeyEntry, type Halo, type ApplicationTarget, type PointerDelta, type NamedHostAction } from "./keymap";
import { t, pick } from "../lib/i18n";

// 本文件的中文片段都经 t() 查表；英文表见 lib/i18n.ts。

export const CONTROL_NAMES = ["顶部按键", "中间按键", "底部按键", "旋钮按下", "右旋", "左旋"] as const;
export const LIGHT_NAMES = ["键 1", "键 2", "键 3", "旋钮"] as const;
const SPECIAL: Record<number, string> = {
  0x00: "无", 0x01: "无效码", 0x28: "Enter", 0x29: "Esc", 0x2a: "⌫", 0x2b: "Tab", 0x2c: "Space", 0x2d: "−", 0x2e: "=", 0x2f: "[", 0x30: "]",
  0x31: "\\", 0x32: "Non-US #", 0x33: ";", 0x34: "'", 0x35: "`", 0x36: ",", 0x37: ".", 0x38: "/", 0x39: "Caps Lock",
  0x46: "Print Screen", 0x47: "Scroll Lock", 0x48: "Pause", 0x49: "Insert", 0x4a: "Home", 0x4b: "Page Up", 0x4c: "Delete", 0x4d: "End",
  0x4e: "Page Down", 0x4f: "→", 0x50: "←", 0x51: "↓", 0x52: "↑", 0x53: "Num Lock", 0x54: "KP /", 0x55: "KP *", 0x56: "KP −", 0x57: "KP +",
  0x58: "KP Enter", 0x62: "KP 0", 0x63: "KP .", 0x65: "Menu",
  0xe0: "左 Control", 0xe1: "左 Shift", 0xe2: "左 Alt", 0xe3: "左 Super", 0xe4: "右 Control", 0xe5: "右 Shift", 0xe6: "右 Alt", 0xe7: "右 Super",
};
const MOUSE = ["左键", "右键", "中键", "鼠标键 4", "鼠标键 5", "滚轮↑", "滚轮↓"];
const MOD_SYMBOL: Record<number, string> = { 0xe0: "⌃", 0xe1: "⇧", 0xe2: "⌥", 0xe3: "⌘" };

export function keyName(e: KeyEntry): string {
  if (e.type === 7) return MOUSE[e.code] ? t(MOUSE[e.code]!) : `${t("鼠标")} 0x${e.code.toString(16).padStart(2, "0")}`;
  if (e.type !== 2) return `${t("类型")}${e.type} 0x${e.code.toString(16).padStart(2, "0")}`;
  const c = e.code;
  if (SPECIAL[c]) return t(SPECIAL[c]!);
  if (c >= 0x04 && c <= 0x1d) return String.fromCharCode(65 + c - 4);
  if (c >= 0x1e && c <= 0x27) return String((c - 0x1d) % 10);
  if (c >= 0x3a && c <= 0x45) return `F${c - 0x39}`;
  if (c >= 0x59 && c <= 0x61) return `KP ${c - 0x58}`;
  if (c >= 0x68 && c <= 0x73) return `F${c - 0x68 + 13}`;
  return `0x${c.toString(16).padStart(2, "0")}`;
}

export function entriesLabel(entries: KeyEntry[]): string {
  const active = entries.filter((e) => !(e.type === 2 && e.code === 0));
  if (!active.length) return t("无");
  const mods = active.filter((e) => e.type === 2 && MOD_SYMBOL[e.code]).map((e) => MOD_SYMBOL[e.code]).join("");
  const rest = active.filter((e) => !(e.type === 2 && MOD_SYMBOL[e.code])).map(keyName).join(" + ");
  return mods && rest ? `${mods}${rest}` : mods || rest;
}

/** 动作的显示文字；`halos` / `library` 用来把引用解析成名字。 */
/** "切换到 X"：日文语序是"X に切替"。 */
export const switchTo = (name: string) => pick({ zh: `切换到 ${name}`, en: `Switch to ${name}`, ja: `${name} に切替` });

export function actionLabel(a: HostAction | null, entries?: KeyEntry[] | null, halos?: Halo[], library?: NamedHostAction[]): string {
  if (!a) return entries ? entriesLabel(entries) : t("无");
  switch (actionKind(a)) {
    case "keyboard": return entriesLabel(actionValue<KeyEntry[]>(a));
    case "application": return switchTo(actionValue<ApplicationTarget>(a).name);
    case "macro": { const n = actionValue<unknown[]>(a).length; return pick({ zh: `宏（${n} 步）`, en: `Macro (${n} steps)`, ja: `マクロ（${n} ステップ）` }); }
    case "library": {
      const id = actionValue<string>(a);
      const item = library?.find((i) => i.id === id);
      if (!item) return `${t("动作库")} ${String(id).slice(0, 8)}`;
      if ("application" in item.action) return switchTo(item.name);
      return pick({ zh: `宏「${item.name}」`, en: `Macro “${item.name}”`, ja: `マクロ「${item.name}」` });
    }
    case "momentaryLayer": case "toggleLayer": case "toLayer": case "oneShotLayer":
      return `${LAYER_ACTIONS[actionKind(a) as LayerActionKind]}(${actionValue<number>(a)})`;
    case "pointerMove": {
      const { dx, dy } = actionValue<PointerDelta>(a);
      const parts = [dx ? `${dx > 0 ? "→" : "←"}${Math.abs(dx)}` : "", dy ? `${dy > 0 ? "↓" : "↑"}${Math.abs(dy)}` : ""].filter(Boolean);
      return `${t("指针")} ${parts.join(" ")}`;
    }
    case "halo": { const id = actionValue<string>(a); const name = halos?.find((h) => h.id === id)?.name ?? id; return pick({ zh: `光环「${name}」`, en: `Halo “${name}”`, ja: `ハロー「${name}」` }); }
    default: return t("复杂动作");
  }
}
