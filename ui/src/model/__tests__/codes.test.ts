import { describe, it, expect } from "vitest";
import { CATEGORIES, isAllowedKeyboardCode } from "../codes";

describe("codes", () => {
  it("lists categories in the agreed order", () => {
    expect(CATEGORIES.map((c) => c.id)).toEqual(["common", "letters", "fkeys", "keypad", "mouse", "combo", "layer", "app", "macro", "halo", "pointer"]);
  });
  it("keycaps use only allowed codes and have no duplicates", () => {
    const seen = new Set<string>();
    for (const c of CATEGORIES) for (const k of c.keycaps ?? []) {
      const key = `${k.entry.type}:${k.entry.code}`;
      expect(seen.has(key), key).toBe(false);
      seen.add(key);
      if (k.entry.type === 2) expect(isAllowedKeyboardCode(k.entry.code), key).toBe(true);
      else expect(k.entry.type).toBe(7);
    }
    expect(seen.size).toBeGreaterThan(90);
  });
  it("mirrors the Rust allowed-code predicate", () => {
    expect(isAllowedKeyboardCode(0x00)).toBe(true);
    expect(isAllowedKeyboardCode(0x02)).toBe(false);
    expect(isAllowedKeyboardCode(0x64)).toBe(false);
    expect(isAllowedKeyboardCode(0x65)).toBe(true);
    expect(isAllowedKeyboardCode(0x73)).toBe(true);
    expect(isAllowedKeyboardCode(0x74)).toBe(false);
    expect(isAllowedKeyboardCode(0xe7)).toBe(true);
  });
});
