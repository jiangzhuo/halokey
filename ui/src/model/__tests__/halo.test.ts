import { describe, it, expect } from "vitest";
import { slotsFrom, haloFrom, haloProblems, moveSlot, clearSlot } from "../halo";
import type { Halo } from "../keymap";

const kb = (c: number) => ({ keyboard: { _0: [{ type: 2, code: c }] } });

describe("halo editor model", () => {
  it("expands to 8 slots and collapses back to valid options in order", () => {
    const h: Halo = { id: "x", name: "N", options: [{ label: "a", action: kb(4) }, { label: "b", action: kb(5) }] };
    const slots = slotsFrom(h);
    expect(slots).toHaveLength(8);
    expect(slots[1]).toEqual({ label: "b", action: kb(5) });
    expect(slots[7]).toEqual({ label: "", action: null });
    const moved = moveSlot(slots, 1, 1);
    expect(moved[2]!.label).toBe("b");
    expect(moved[1]!.action).toBeNull();
    expect(haloFrom("x", "N", moved).options.map((o) => o.label)).toEqual(["a", "b"]);
    expect(clearSlot(moved, 0)[0]).toEqual({ label: "", action: null });
    expect(slotsFrom(null).every((s) => s.action === null)).toBe(true);
  });
  it("reports problems in Chinese", () => {
    const empty = slotsFrom(null);
    expect(haloProblems("", empty)).toEqual(["名称不能为空", "至少要有一个选项"]);
    const s = slotsFrom(null); s[2] = { label: " ", action: kb(4) };
    expect(haloProblems("主菜单", s)).toEqual(["槽位 3 有动作但没有名字"]);
    expect(haloProblems("x".repeat(81), [{ label: "a", action: kb(4) }, ...empty.slice(1)])).toEqual(["名称不能超过 80 字"]);
    expect(haloProblems("ok", [{ label: "a", action: kb(4) }, ...empty.slice(1)])).toEqual([]);
  });
});
