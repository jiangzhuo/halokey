import type { KeyEntry } from "./keymap";
import { keyName } from "./labels";

export type Keycap = { label: string; entry: KeyEntry };
export type CategoryId = "common" | "letters" | "fkeys" | "keypad" | "mouse" | "combo" | "layer" | "app" | "macro" | "halo" | "pointer";
export type Category = { id: CategoryId; name: string; keycaps?: Keycap[] };

// 与 crates/proto keyconfig::is_allowed_keyboard_code 保持一致。
export function isAllowedKeyboardCode(code: number): boolean {
  return code === 0 || code === 1 || (code >= 0x04 && code <= 0x63) || code === 0x65 || (code >= 0x68 && code <= 0x73) || (code >= 0xe0 && code <= 0xe7);
}
const kb = (code: number, label?: string): Keycap => ({ label: label ?? keyName({ type: 2, code }), entry: { type: 2, code } });
const range = (from: number, to: number) => Array.from({ length: to - from + 1 }, (_, i) => kb(from + i));

export const CATEGORIES: Category[] = [
  { id: "common", name: "常用", keycaps: [kb(0x28), kb(0x29), kb(0x2a), kb(0x2b), kb(0x2c), kb(0x39), kb(0x65), kb(0x46), kb(0x47), kb(0x48), kb(0x49), kb(0x4a), kb(0x4b), kb(0x4c), kb(0x4d), kb(0x4e), kb(0x4f), kb(0x50), kb(0x51), kb(0x52), ...range(0xe0, 0xe7), kb(0x01)] },
  { id: "letters", name: "字母", keycaps: [...range(0x04, 0x1d), ...range(0x1e, 0x27), ...range(0x2d, 0x38)] },
  { id: "fkeys", name: "功能键", keycaps: [...range(0x3a, 0x45), ...range(0x68, 0x73)] },
  { id: "keypad", name: "数字键盘", keycaps: range(0x53, 0x63) },
  { id: "mouse", name: "鼠标", keycaps: Array.from({ length: 7 }, (_, code) => ({ label: keyName({ type: 7, code }), entry: { type: 7, code } })) },
  { id: "combo", name: "组合键" },
  { id: "layer", name: "Layer" },
  { id: "app", name: "APP" },
  { id: "macro", name: "宏" },
  { id: "halo", name: "光环" },
  { id: "pointer", name: "指针" },
];
