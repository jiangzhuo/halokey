import { describe, it, expect } from "vitest";
import { keyName, actionLabel, CONTROL_NAMES } from "../labels";
import { setLang } from "../../lib/i18n";

describe("labels", () => {
  it("names keys, mouse entries and actions", () => {
    setLang("zh");
    expect(CONTROL_NAMES).toHaveLength(6);
    expect(keyName({ type: 2, code: 0x28 })).toBe("Enter");
    expect(keyName({ type: 2, code: 0x04 })).toBe("A");
    expect(keyName({ type: 2, code: 0x68 })).toBe("F13");
    expect(keyName({ type: 7, code: 6 })).toBe("滚轮↓");
    expect(keyName({ type: 9, code: 1 })).toBe("类型9 0x01");
    expect(actionLabel(null, [{ type: 2, code: 0xe3 }, { type: 2, code: 0x04 }])).toBe("⌘A");
    expect(actionLabel(null, [{ type: 2, code: 0 }])).toBe("无");
    expect(actionLabel({ pointerMove: { _0: { dx: 0, dy: 40 } } })).toBe("指针 ↓40");
    expect(actionLabel({ momentaryLayer: { _0: 1 } })).toBe("MO(1)");
    expect(actionLabel({ toggleLayer: { _0: 2 } })).toBe("TG(2)");
    expect(actionLabel({ toLayer: { _0: 0 } })).toBe("TO(0)");
    expect(actionLabel({ oneShotLayer: { _0: 3 } })).toBe("OSL(3)");
    expect(actionLabel({ halo: { _0: "main" } }, null, [{ id: "main", name: "主菜单", options: [] }])).toBe("光环「主菜单」");
    expect(actionLabel({ futureAction: { _0: 1 } } as any)).toBe("复杂动作");
  });
  it("resolves library references to the item's name", () => {
    setLang("zh");
    const lib = [
      { id: "aaaaaaaa-0000-4000-8000-000000000000", name: "Chrome", slot: 0, action: { application: { _0: { bundleIdentifier: "google-chrome", path: "", name: "Chrome" } } } },
      { id: "bbbbbbbb-0000-4000-8000-000000000000", name: "打开并输入", slot: 0, action: { macro: { _0: [{ delay: { _0: 0.3 } }] } } },
    ];
    expect(actionLabel({ library: { _0: lib[0]!.id } }, null, [], lib)).toBe("切换到 Chrome");
    expect(actionLabel({ library: { _0: lib[1]!.id } }, null, [], lib)).toBe("宏「打开并输入」");
    expect(actionLabel({ library: { _0: "cccccccc-0000-4000-8000-000000000000" } }, null, [], lib)).toBe("动作库 cccccccc");
    setLang("en");
    try { expect(actionLabel({ library: { _0: lib[1]!.id } }, null, [], lib)).toBe("Macro “打开并输入”"); } finally { setLang("zh"); }
    setLang("ja");
    try {
      expect(actionLabel({ library: { _0: lib[1]!.id } }, null, [], lib)).toBe("マクロ「打开并输入」");
      expect(actionLabel({ library: { _0: lib[0]!.id } }, null, [], lib)).toBe("Chrome に切替");
      expect(actionLabel({ macro: { _0: [1, 2] } } as any)).toBe("マクロ（2 ステップ）");
      expect(actionLabel({ halo: { _0: "main" } }, null, [{ id: "main", name: "主菜单", options: [] }])).toBe("ハロー「主菜单」");
      expect(keyName({ type: 2, code: 0xe0 })).toBe("左 Ctrl");
    } finally { setLang("zh"); }
  });
  it("speaks English when the UI language is English", () => {
    setLang("en");
    try {
      expect(keyName({ type: 7, code: 6 })).toBe("Wheel ↓");
      expect(keyName({ type: 2, code: 0xe0 })).toBe("Left Control");
      expect(keyName({ type: 2, code: 0x01 })).toBe("Invalid");
      expect(actionLabel(null, [{ type: 2, code: 0 }])).toBe("None");
      expect(actionLabel({ pointerMove: { _0: { dx: 0, dy: 40 } } })).toBe("Pointer ↓40");
      expect(actionLabel({ halo: { _0: "main" } }, null, [{ id: "main", name: "主菜单", options: [] }])).toBe("Halo “主菜单”");
      expect(actionLabel({ macro: { _0: [1, 2] } } as any)).toBe("Macro (2 steps)");
      expect(actionLabel({ application: { _0: { name: "Chrome", path: "", bundleIdentifier: "" } } })).toBe("Switch to Chrome");
      expect(actionLabel({ futureAction: { _0: 1 } } as any)).toBe("Complex action");
    } finally { setLang("zh"); }
  });
});
