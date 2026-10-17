import { describe, it, expect } from "vitest";
import { keyName, actionLabel, CONTROL_NAMES } from "../labels";

describe("labels", () => {
  it("names keys, mouse entries and actions", () => {
    expect(CONTROL_NAMES).toHaveLength(6);
    expect(keyName({ type: 2, code: 0x28 })).toBe("Enter");
    expect(keyName({ type: 2, code: 0x04 })).toBe("A");
    expect(keyName({ type: 2, code: 0x68 })).toBe("F13");
    expect(keyName({ type: 7, code: 6 })).toBe("滚轮↓");
    expect(keyName({ type: 9, code: 1 })).toBe("类型9 0x01");
    expect(actionLabel(null, [{ type: 2, code: 0xe3 }, { type: 2, code: 0x04 }])).toBe("⌘A");
    expect(actionLabel(null, [{ type: 2, code: 0 }])).toBe("无");
    expect(actionLabel({ pointerMove: { _0: { dx: 0, dy: 40 } } })).toBe("指针 ↓40");
    expect(actionLabel({ halo: { _0: "main" } }, null, [{ id: "main", name: "主菜单", options: [] }])).toBe("光环「主菜单」");
    expect(actionLabel({ futureAction: { _0: 1 } } as any)).toBe("复杂动作");
  });
});
