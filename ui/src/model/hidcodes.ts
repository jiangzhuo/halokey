import type { KeyEntry } from "./keymap";

// 浏览器 KeyboardEvent.code → USB HID 键盘页 usage。只列 Rust `is_allowed_keyboard_code` 接受的键。
const FIXED: Record<string, number> = {
  Enter: 0x28, Escape: 0x29, Backspace: 0x2a, Tab: 0x2b, Space: 0x2c, Minus: 0x2d, Equal: 0x2e, BracketLeft: 0x2f, BracketRight: 0x30,
  Backslash: 0x31, IntlHash: 0x32, Semicolon: 0x33, Quote: 0x34, Backquote: 0x35, Comma: 0x36, Period: 0x37, Slash: 0x38, CapsLock: 0x39,
  PrintScreen: 0x46, ScrollLock: 0x47, Pause: 0x48, Insert: 0x49, Home: 0x4a, PageUp: 0x4b, Delete: 0x4c, End: 0x4d, PageDown: 0x4e,
  ArrowRight: 0x4f, ArrowLeft: 0x50, ArrowDown: 0x51, ArrowUp: 0x52, NumLock: 0x53, NumpadDivide: 0x54, NumpadMultiply: 0x55,
  NumpadSubtract: 0x56, NumpadAdd: 0x57, NumpadEnter: 0x58, Numpad0: 0x62, NumpadDecimal: 0x63, ContextMenu: 0x65,
  ControlLeft: 0xe0, ShiftLeft: 0xe1, AltLeft: 0xe2, MetaLeft: 0xe3, ControlRight: 0xe4, ShiftRight: 0xe5, AltRight: 0xe6, MetaRight: 0xe7,
};

/** 浏览器键码 → HID usage；不认识（媒体键、IME、`Unidentified`）返回 null。 */
export function usageFromCode(code: string): number | null {
  if (!code) return null;
  if (code in FIXED) return FIXED[code]!;
  let m = /^Key([A-Z])$/.exec(code);
  if (m) return 0x04 + (m[1]!.charCodeAt(0) - 65);
  m = /^Digit([0-9])$/.exec(code);
  if (m) return m[1] === "0" ? 0x27 : 0x1e + (Number(m[1]) - 1);
  m = /^F([0-9]{1,2})$/.exec(code);
  if (m) {
    const n = Number(m[1]);
    if (n >= 1 && n <= 12) return 0x3a + n - 1;
    if (n >= 13 && n <= 24) return 0x68 + n - 13;
    return null;
  }
  m = /^Numpad([1-9])$/.exec(code);
  if (m) return 0x59 + Number(m[1]) - 1;
  return null;
}

export const MODIFIER_USAGES: readonly number[] = [0xe0, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7];
export const isModifier = (usage: number): boolean => usage >= 0xe0 && usage <= 0xe7;

/** 修饰键升序去重在前、主键在后，与 Rust 的 press_order 一致。 */
export function comboEntries(modifiers: Iterable<number>, main: number | null): KeyEntry[] {
  const mods = [...new Set(modifiers)].filter(isModifier).sort((a, b) => a - b);
  const entries: KeyEntry[] = mods.map((code) => ({ type: 2, code }));
  if (main !== null) entries.push({ type: 2, code: main });
  return entries;
}
