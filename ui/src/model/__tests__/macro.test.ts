import { describe, it, expect } from "vitest";
import { stepLabel, macroProblems, moveStep, removeStep, insertStep } from "../macro";
import type { MacroStep } from "../keymap";
import { setLang } from "../../lib/i18n";

const kb: MacroStep = { keyboard: { _0: [{ type: 2, code: 0xe0 }, { type: 2, code: 0x17 }] } };
const ap: MacroStep = { application: { _0: { bundleIdentifier: "google-chrome", path: "", name: "Chrome" } } };
const dl: MacroStep = { delay: { _0: 0.3 } };

describe("macro model", () => {
  it("labels steps", () => {
    setLang("zh");
    expect(stepLabel(kb)).toBe("⌃T");
    expect(stepLabel(ap)).toBe("切换到 Chrome");
    expect(stepLabel(dl)).toBe("等待 0.3 秒");
    setLang("en");
    expect(stepLabel(dl)).toBe("Wait 0.3 s");
    setLang("ja");
    expect(stepLabel(dl)).toBe("0.3 秒待つ");
    setLang("zh");
  });
  it("reports problems", () => {
    expect(macroProblems("", [])).toEqual(["名称不能为空", "至少要有一步"]);
    expect(macroProblems("m", [{ delay: { _0: 0.01 } }])).toEqual(["第 1 步等待时间应为 0.05–10 秒"]);
    expect(macroProblems("m", [{ keyboard: { _0: [] } }])).toEqual(["第 1 步没有按键"]);
    expect(macroProblems("m", Array.from({ length: 33 }, () => dl))).toEqual(["最多 32 步"]);
    expect(macroProblems("m", [kb, dl, ap])).toEqual([]);
  });
  it("moves, removes and inserts without mutating", () => {
    const steps = [kb, dl, ap];
    expect(moveStep(steps, 0, 1).map((s) => Object.keys(s)[0])).toEqual(["delay", "keyboard", "application"]);
    expect(moveStep(steps, 0, -1)).toBe(steps);
    expect(removeStep(steps, 1).length).toBe(2);
    expect(insertStep(steps, 1, dl).map((s) => Object.keys(s)[0])).toEqual(["keyboard", "delay", "delay", "application"]);
    expect(steps).toHaveLength(3);
  });
});
