import { describe, it, expect } from "vitest";
import { usageFromCode, isModifier, comboEntries, MODIFIER_USAGES } from "../hidcodes";
import { isAllowedKeyboardCode } from "../codes";

describe("hid codes", () => {
  it("maps browser codes to HID usages", () => {
    expect(usageFromCode("KeyA")).toBe(0x04);
    expect(usageFromCode("KeyZ")).toBe(0x1d);
    expect(usageFromCode("Digit1")).toBe(0x1e);
    expect(usageFromCode("Digit0")).toBe(0x27);
    expect(usageFromCode("Enter")).toBe(0x28);
    expect(usageFromCode("Escape")).toBe(0x29);
    expect(usageFromCode("Backspace")).toBe(0x2a);
    expect(usageFromCode("Tab")).toBe(0x2b);
    expect(usageFromCode("Space")).toBe(0x2c);
    expect(usageFromCode("Minus")).toBe(0x2d);
    expect(usageFromCode("BracketLeft")).toBe(0x2f);
    expect(usageFromCode("Backslash")).toBe(0x31);
    expect(usageFromCode("Semicolon")).toBe(0x33);
    expect(usageFromCode("Quote")).toBe(0x34);
    expect(usageFromCode("Backquote")).toBe(0x35);
    expect(usageFromCode("Comma")).toBe(0x36);
    expect(usageFromCode("Slash")).toBe(0x38);
    expect(usageFromCode("CapsLock")).toBe(0x39);
    expect(usageFromCode("F1")).toBe(0x3a);
    expect(usageFromCode("F12")).toBe(0x45);
    expect(usageFromCode("F13")).toBe(0x68);
    expect(usageFromCode("F24")).toBe(0x73);
    expect(usageFromCode("PrintScreen")).toBe(0x46);
    expect(usageFromCode("Insert")).toBe(0x49);
    expect(usageFromCode("ArrowRight")).toBe(0x4f);
    expect(usageFromCode("ArrowUp")).toBe(0x52);
    expect(usageFromCode("NumLock")).toBe(0x53);
    expect(usageFromCode("NumpadDivide")).toBe(0x54);
    expect(usageFromCode("NumpadEnter")).toBe(0x58);
    expect(usageFromCode("Numpad1")).toBe(0x59);
    expect(usageFromCode("Numpad0")).toBe(0x62);
    expect(usageFromCode("NumpadDecimal")).toBe(0x63);
    expect(usageFromCode("ContextMenu")).toBe(0x65);
    expect(usageFromCode("ControlLeft")).toBe(0xe0);
    expect(usageFromCode("MetaRight")).toBe(0xe7);
    expect(usageFromCode("Unidentified")).toBeNull();
    expect(usageFromCode("MediaPlayPause")).toBeNull();
    expect(usageFromCode("F25")).toBeNull();
    expect(usageFromCode("")).toBeNull();
  });
  it("only produces codes Rust accepts", () => {
    const codes = ["KeyA", "Digit5", "F13", "F24", "Numpad5", "ContextMenu", "MetaLeft", "Space", "ArrowDown"];
    for (const c of codes) expect(isAllowedKeyboardCode(usageFromCode(c)!), c).toBe(true);
  });
  it("builds combo entries with sorted modifiers first", () => {
    expect(MODIFIER_USAGES).toEqual([0xe0, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7]);
    expect(isModifier(0xe3)).toBe(true);
    expect(isModifier(0x04)).toBe(false);
    expect(comboEntries([0xe3, 0xe0, 0xe3], 0x04)).toEqual([{ type: 2, code: 0xe0 }, { type: 2, code: 0xe3 }, { type: 2, code: 0x04 }]);
    expect(comboEntries([], 0x28)).toEqual([{ type: 2, code: 0x28 }]);
    expect(comboEntries([0xe1], null)).toEqual([{ type: 2, code: 0xe1 }]);
  });
});
