# 组合键录制、APP 编辑器、宏编辑器 · 第 4 期实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 键码面板的"组合键 / APP / 宏"三个分类从占位变成可用：录真实键盘得到组合键；维护功能库里的应用（可从当前窗口列表挑）与宏（APP / 等待 / 组合键三种步骤，可录制）；分配给手势仍走现有的 `library` 引用。

**Architecture:** 功能库就是键位文件里的 `actionLibrary[]`（`NamedHostAction { id: UUID, name, slot 0–15, action: application | macro }`），编辑全部走草稿 reducer。组合键录制是纯前端：`KeyboardEvent.code → HID usage` 查表，修饰键用 `0xE0–0xE7`。窗口列表由 Rust 新命令 `list_windows` 用 X11 EWMH 读 `_NET_CLIENT_LIST` + `WM_CLASS` + `_NET_WM_NAME` + `_NET_WM_PID`（复用 `crates/wm` 的连接代码）；非 Linux 返回空列表与原因。APP 编辑器是键位页上的对话框，宏编辑器是整页（与光环编辑器同构）。

**Tech Stack:** Svelte 5 + TypeScript + vitest（`ui/`），Rust（`crates/wm`、`apps/halokey`，x11rb 已是 wm 的依赖）。

**Spec:** `docs/superpowers/specs/2026-10-17-settings-ui-redesign-design.md` §4.4（组合键 / APP / 宏三段）、§8 `list_windows`、§12 第 4 期。

## Global Constraints

- 功能库规则不变（Rust `NamedHostAction::validate`）：名称 1–80 字、`slot` 0–15、`id` 必须是 UUID（8-4-4-4-12）、动作只能是 `application` 或 `macro`；应用名称 ≤ 200 字且 `bundleIdentifier` / `path` 至少一个非空；宏 1–32 步，`delay` 0.05–10 秒；键盘条目经 `validate_entries`（键码表见 `isAllowedKeyboardCode`）。前端编辑器先按同样规则给中文提示，Rust 校验是最后一道门。
- Linux 上 `ApplicationTarget.bundleIdentifier` = WM_CLASS 的 class 或 instance（不区分大小写），`path` = 可执行文件路径，`name` 只做显示（`crates/wm/src/lib.rs` 文件头）。
- 组合键：最多 4 个修饰键 + 1 个主键；条目顺序 = 修饰键在前（按 `0xE0..0xE7` 升序）、主键最后，与 `press_order` 一致。
- 分类顺序与面板布局不变；组合键分类在手势模式与动作模式（光环 / 宏编辑器复用）都可用；APP / 宏分类在动作模式下产出内联动作（第 3 期已定）。
- 新增字符串全部经 `t()` 并进英文表；i18n 扫描测试会拦截 `.svelte` 模板里的中文字面量。
- 每个任务结束：`npm test`、`npm run check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`、`python3 tools/check_docs.py` 全绿；提交英文，末尾 `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`。
- 不新增依赖（`crypto.randomUUID()` 是 WebKitGTK / Node 22 都有的内置 API）。
- 开发：`cd ui && npm run dev`（5177）+ `cargo run -p halokey-app --no-default-features`；发布：`npm run build` 后 `cargo build --release -p halokey-app`。

## Review Focus

1. **录制时按到不认识的键（IME 组合键、媒体键、`Unidentified`）**：不能把 `null` 写进条目，录制框要提示"不支持这个键"。→ Task 1 `usageFromCode` 返回 `null` 的测试 + Task 3 录制框忽略并提示。
2. **删除被手势引用的 APP / 宏**：文件不能变成 Rust 会拒绝的状态（`MissingLibraryAction`）。→ Task 2 `deleteLibraryItem` 返回引用列表并拒绝。
3. **同类动作已有 16 个（slot 0–15 用完）再"添加"**：要提示，不能生成 slot 16。→ Task 2 `nextSlot` 返回 `null` 的测试 + Task 5/6 编辑器提示。
4. **窗口列表拿不到（Wayland、无 X 连接）**：APP 编辑器仍可手填；提示原因但不报错弹窗。→ Task 4 Rust 返回 `{ windows: [], error }`，Task 5 测试 mock 返回 error 时编辑器仍能保存。
5. **宏录制模式下设备自己注入的按键被录进去**（halokey 窗口有焦点时按设备键，注入的按键会被录制框捕获）：录制时显示"录制中会捕获设备注入的按键"提示，并提供"清空最后一步"。→ Task 6 录制框文案测试（功能上不可区分，只能提示）。

---

## 文件结构

```
ui/src/model/hidcodes.ts              KeyboardEvent.code → HID usage；修饰键集合；usageFromCode
ui/src/model/__tests__/hidcodes.test.ts
ui/src/model/draft.ts                 newUuid / nextSlot / upsertLibraryItem / deleteLibraryItem / libraryReferences
ui/src/model/__tests__/draft.test.ts  追加
ui/src/components/ComboRecorder.svelte  录制框 + 手动勾修饰键 + 主键选择 + "分配组合键"
ui/src/components/Palette.svelte      组合键 / APP / 宏分类接入；onEditApp / onEditMacro 入口
ui/src/components/AppEditor.svelte    对话框：名称、WM_CLASS、可执行路径、窗口列表
ui/src/pages/MacroEditor.svelte       整页：步骤列表（APP / 等待 / 组合键）、上下移、删除、录制模式
ui/src/pages/Keymap.svelte            透传 onEditApp / onEditMacro；挂 AppEditor
ui/src/App.svelte                     路由加 "macro"
ui/src/lib/tauri.ts                   api.listWindows + mock
ui/src/lib/i18n.ts                    新字符串
crates/wm/src/x11.rs                  pub fn list_windows() -> Vec<WindowInfo>
crates/wm/src/lib.rs                  pub struct WindowInfo；pub fn list_windows() 跨平台入口
apps/halokey/Cargo.toml / src/main.rs halokey-wm 依赖；命令 list_windows
docs/03-config-format.md (+zh)        actionLibrary 小节补"编辑器与窗口列表"一句
```

---

### Task 1: `KeyboardEvent.code` → HID 键码表

**Files:**
- Create: `ui/src/model/hidcodes.ts`
- Test: `ui/src/model/__tests__/hidcodes.test.ts`

**Interfaces:**
- `usageFromCode(code: string): number | null`（`"KeyA"` → `0x04`，`"Digit1"` → `0x1E`，`"Enter"` → `0x28`，`"F13"` → `0x68`，`"Numpad1"` → `0x59`，`"ControlLeft"` → `0xE0`，未知 → `null`）
- `MODIFIER_USAGES: readonly number[]` = `[0xE0, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7]`；`isModifier(usage: number): boolean`
- `comboEntries(modifiers: Iterable<number>, main: number | null): KeyEntry[]`（修饰键升序去重在前，主键在后；主键为 `null` 时只有修饰键）

- [ ] **Step 1: 写失败的测试**

```ts
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
```

- [ ] **Step 2: 运行确认失败** — `cd ui && npx vitest run src/model/__tests__/hidcodes.test.ts` → 找不到模块。

- [ ] **Step 3: 实现**

```ts
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

export function usageFromCode(code: string): number | null {
  if (!code) return null;
  if (code in FIXED) return FIXED[code]!;
  let m = /^Key([A-Z])$/.exec(code);
  if (m) return 0x04 + (m[1]!.charCodeAt(0) - 65);
  m = /^Digit([0-9])$/.exec(code);
  if (m) return m[1] === "0" ? 0x27 : 0x1e + (Number(m[1]) - 1);
  m = /^F([0-9]{1,2})$/.exec(code);
  if (m) { const n = Number(m[1]); if (n >= 1 && n <= 12) return 0x3a + n - 1; if (n >= 13 && n <= 24) return 0x68 + n - 13; return null; }
  m = /^Numpad([1-9])$/.exec(code);
  if (m) return 0x59 + Number(m[1]) - 1;
  return null;
}

export const MODIFIER_USAGES: readonly number[] = [0xe0, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7];
export const isModifier = (usage: number) => usage >= 0xe0 && usage <= 0xe7;

/** 修饰键升序去重在前、主键在后，与 Rust 的 press_order 一致。 */
export function comboEntries(modifiers: Iterable<number>, main: number | null): KeyEntry[] {
  const mods = [...new Set(modifiers)].filter(isModifier).sort((a, b) => a - b);
  const entries: KeyEntry[] = mods.map((code) => ({ type: 2, code }));
  if (main !== null) entries.push({ type: 2, code: main });
  return entries;
}
```

- [ ] **Step 4: 运行测试** → 全绿；`npm run check` 0 错。
- [ ] **Step 5: 提交** `git add ui && git commit -m "feat(ui): browser key codes to HID usages and combo entry builder"`（带 Co-Authored-By）。

---

### Task 2: 草稿 reducer — 功能库的增删改与引用检查

**Files:**
- Modify: `ui/src/model/draft.ts`
- Test: `ui/src/model/__tests__/draft.test.ts`

**Interfaces:**
- `newUuid(): string`（`crypto.randomUUID()`）
- `nextSlot(map: HostKeymap, kind: "application" | "macro"): number | null`（同类里最小未用的 0–15；满了 `null`）
- `upsertLibraryItem(d: Draft, item: NamedHostAction): Draft`（按 `id` 替换或追加；dirty）
- `libraryReferences(map: HostKeymap, id: string): string[]`（与 `haloReferences` 同格式：`"顶部按键 › 单击"`、`"Layer 2 · 中间按键 › 长按"`；光环选项不会引用功能库，不扫）
- `deleteLibraryItem(d: Draft, id: string): { draft: Draft; blockedBy: string[] }`
- `libraryItems(map: HostKeymap, kind: "application" | "macro"): NamedHostAction[]`（按 slot 排序）

- [ ] **Step 1: 写失败的测试（追加到 `draft.test.ts`）**

```ts
import { newUuid, nextSlot, upsertLibraryItem, deleteLibraryItem, libraryReferences, libraryItems } from "../draft";
import type { NamedHostAction } from "../keymap";
// …import 列表

describe("draft action library", () => {
  const loadApps = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/switch-apps-and-macro.json"), "utf8"));
  const app = (id: string, slot: number, name = "X"): NamedHostAction => ({ id, name, slot, action: { application: { _0: { bundleIdentifier: "x", path: "", name } } } });
  it("generates uuids and allocates slots per kind", () => {
    expect(newUuid()).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
    let d = loadSaved(initialDraft(), load());
    expect(nextSlot(d.map!, "application")).toBe(0);
    d = upsertLibraryItem(d, app("11111111-1111-4111-8111-111111111111", 0, "A"));
    d = upsertLibraryItem(d, app("22222222-2222-4222-8222-222222222222", 2, "C"));
    expect(nextSlot(d.map!, "application")).toBe(1);
    expect(nextSlot(d.map!, "macro")).toBe(0);
    expect(libraryItems(d.map!, "application").map((i) => i.name)).toEqual(["A", "C"]);
    expect(d.dirty).toBe(true);
    for (let s = 0; s < 16; s++) d = upsertLibraryItem(d, app(`33333333-3333-4333-8333-${String(s).padStart(12, "0")}`, s));
    expect(nextSlot(d.map!, "application")).toBeNull();
  });
  it("lists references and blocks deleting a referenced item", () => {
    let d = loadSaved(initialDraft(), load());
    const id = "44444444-4444-4444-8444-444444444444";
    d = upsertLibraryItem(d, app(id, 0, "Chrome"));
    d = assignAction(select(d, { layer: 0, index: 0, gesture: "long" }), { library: { _0: id } });
    d = assignAction(select(d, { layer: 1, index: 2, gesture: "press" }), { library: { _0: id } });
    expect(libraryReferences(d.map!, id)).toEqual(["顶部按键 › 长按", "Layer 1 · 底部按键 › 单击"]);
    const r = deleteLibraryItem(d, id);
    expect(r.blockedBy).toHaveLength(2);
    expect(r.draft.map!.actionLibrary).toHaveLength(1);
    d = clearGesture(select(d, { layer: 0, index: 0, gesture: "long" }));
    d = clearGesture(select(d, { layer: 1, index: 2, gesture: "press" }));
    const r2 = deleteLibraryItem(d, id);
    expect(r2.blockedBy).toEqual([]);
    expect(r2.draft.map!.actionLibrary).toBeUndefined();
  });
  it("keeps the example's inline actions untouched", () => {
    const m = loadApps();
    expect(libraryItems(m, "application")).toEqual([]);
    expect(nextSlot(m, "macro")).toBe(0);
  });
});
```

- [ ] **Step 2: 运行确认失败** → 函数不存在。

- [ ] **Step 3: 实现（追加到 `draft.ts`）**

```ts
import type { NamedHostAction } from "./keymap";

// ---- 功能库（应用 / 宏）----

export const newUuid = (): string => crypto.randomUUID();

const kindOf = (item: NamedHostAction): "application" | "macro" | null => "application" in item.action ? "application" : "macro" in item.action ? "macro" : null;

export function libraryItems(map: HostKeymap, kind: "application" | "macro"): NamedHostAction[] {
  return (map.actionLibrary ?? []).filter((i) => kindOf(i) === kind).sort((a, b) => a.slot - b.slot);
}

/** 同类动作内最小的空编号 0–15；满了返回 null。 */
export function nextSlot(map: HostKeymap, kind: "application" | "macro"): number | null {
  const used = new Set(libraryItems(map, kind).map((i) => i.slot));
  for (let s = 0; s < 16; s++) if (!used.has(s)) return s;
  return null;
}

export function upsertLibraryItem(d: Draft, item: NamedHostAction): Draft {
  if (!d.map) return d;
  const map = clone(d.map);
  map.actionLibrary ??= [];
  const i = map.actionLibrary.findIndex((x) => x.id === item.id);
  if (i >= 0) map.actionLibrary[i] = clone(item); else map.actionLibrary.push(clone(item));
  return { ...d, map, dirty: true };
}

/** 谁在用这个功能库项：基础层与各层的手势动作（光环选项不能引用功能库）。 */
export function libraryReferences(map: HostKeymap, id: string): string[] {
  const refs: string[] = [];
  const isRef = (a: HostAction | null | undefined) => !!a && "library" in a && (a as { library: { _0: string } }).library._0 === id;
  const scan = (controls: ControlActionMap[], prefix: string) => {
    for (const c of controls) {
      const name = CONTROL_NAMES[c.index] ?? `控件 ${c.index}`;
      if (isRef(c.pressAction)) refs.push(`${prefix}${name} › ${GESTURE_NAME.press}`);
      if (isRef(c.doublePressAction)) refs.push(`${prefix}${name} › ${GESTURE_NAME.double}`);
      if (isRef(c.longPressAction)) refs.push(`${prefix}${name} › ${GESTURE_NAME.long}`);
    }
  };
  scan(map.controls, "");
  for (const l of map.layers ?? []) scan(l.controls, `Layer ${l.id} · `);
  return refs;
}

export function deleteLibraryItem(d: Draft, id: string): { draft: Draft; blockedBy: string[] } {
  if (!d.map) return { draft: d, blockedBy: [] };
  const blockedBy = libraryReferences(d.map, id);
  if (blockedBy.length) return { draft: d, blockedBy };
  const map = clone(d.map);
  map.actionLibrary = (map.actionLibrary ?? []).filter((i) => i.id !== id);
  if (!map.actionLibrary.length) delete map.actionLibrary;
  return { draft: { ...d, map, dirty: true }, blockedBy: [] };
}
```

`GESTURE_NAME` 与 `clone` 已在文件里（第 3 期）。`crypto.randomUUID` 在 jsdom（Node 22）里可用；若 `npm run check` 抱怨类型，`lib` 里加 `"DOM"`（`@tsconfig/svelte` 已含）。

- [ ] **Step 4: 运行测试** → 全绿。
- [ ] **Step 5: 提交** `git add ui && git commit -m "feat(ui): action-library upsert, slots, references and delete in the draft reducer"`。

---

### Task 3: 组合键录制框 + 面板"组合键"分类

**Files:**
- Create: `ui/src/components/ComboRecorder.svelte`
- Modify: `ui/src/components/Palette.svelte`, `ui/src/lib/i18n.ts`
- Test: `ui/src/components/__tests__/combo.test.svelte.ts`

**Interfaces:**
- `ComboRecorder` props：`{ onAssign: (entries: KeyEntry[]) => void; assignLabel?: string }`。
  - 录制框 `button.rec`（可聚焦，`data-role="combo-rec"`）：聚焦后 `keydown` 一律 `preventDefault`；修饰键加入集合，非修饰键成为主键（替换）；`usageFromCode` 返回 `null` 时不改状态并显示 `t("不支持这个键")`；`Escape`（在录制框里）清空。
  - 手动区：8 个修饰键复选（`data-mod={usage}`）+ 主键 `<select data-role="combo-main">`（选项来自 `CATEGORIES` 中 common / letters / fkeys / keypad 的键帽，用 `keyName`）。
  - 预览：`entriesLabel(comboEntries(mods, main))`；"分配组合键"按钮（`data-role="combo-assign"`）在没有任何键时禁用；点击 `onAssign(comboEntries(...))` 并清空。
- Palette：`category === "combo"` 渲染 `<ComboRecorder onAssign={emitEntries} />`（手势模式写草稿，动作模式回调）。

- [ ] **Step 1: 写失败的测试**

```ts
import { describe, it, expect, afterEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import ComboRecorder from "../ComboRecorder.svelte";
import { setLang } from "../../lib/i18n";
import type { KeyEntry } from "../../model/keymap";

let app: ReturnType<typeof mount> | undefined;
afterEach(() => { if (app) unmount(app); app = undefined; document.body.innerHTML = ""; });
const key = (el: Element, code: string, extra: KeyboardEventInit = {}) => { const ev = new KeyboardEvent("keydown", { code, bubbles: true, cancelable: true, ...extra }); el.dispatchEvent(ev); return ev; };

describe("combo recorder", () => {
  it("records modifiers + one main key, ignores unknown keys, assigns sorted entries", () => {
    setLang("zh");
    const got: KeyEntry[][] = [];
    app = mount(ComboRecorder, { target: document.body, props: { onAssign: (e: KeyEntry[]) => got.push(e) } });
    flushSync();
    const rec = document.querySelector("[data-role=combo-rec]")!;
    const assign = document.querySelector("[data-role=combo-assign]") as HTMLButtonElement;
    expect(assign.disabled).toBe(true);
    expect(key(rec, "MetaLeft").defaultPrevented).toBe(true);
    key(rec, "ShiftLeft");
    key(rec, "KeyA");
    flushSync();
    expect(document.body.textContent).toContain("⇧⌘A");
    key(rec, "MediaPlayPause");
    flushSync();
    expect(document.body.textContent).toContain("不支持这个键");
    key(rec, "KeyB");
    flushSync();
    expect(document.body.textContent).toContain("⇧⌘B");
    assign.click();
    flushSync();
    expect(got).toEqual([[{ type: 2, code: 0xe1 }, { type: 2, code: 0xe3 }, { type: 2, code: 0x05 }]]);
    expect(assign.disabled).toBe(true);
  });
  it("supports manual modifiers and main key, and Escape clears", () => {
    setLang("zh");
    const got: KeyEntry[][] = [];
    app = mount(ComboRecorder, { target: document.body, props: { onAssign: (e: KeyEntry[]) => got.push(e) } });
    flushSync();
    (document.querySelector("input[data-mod='224']") as HTMLInputElement).click();
    const main = document.querySelector("select[data-role=combo-main]") as HTMLSelectElement;
    main.value = "40"; main.dispatchEvent(new Event("change", { bubbles: true }));
    flushSync();
    expect(document.body.textContent).toContain("⌃Enter");
    (document.querySelector("[data-role=combo-assign]") as HTMLButtonElement).click();
    flushSync();
    expect(got[0]).toEqual([{ type: 2, code: 0xe0 }, { type: 2, code: 0x28 }]);
    const rec = document.querySelector("[data-role=combo-rec]")!;
    key(rec, "KeyC"); flushSync();
    key(rec, "Escape"); flushSync();
    expect((document.querySelector("[data-role=combo-assign]") as HTMLButtonElement).disabled).toBe(true);
  });
});
```

- [ ] **Step 2: 运行确认失败** → 找不到模块。

- [ ] **Step 3: `ComboRecorder.svelte`**

```svelte
<script lang="ts">
  import { t } from "../lib/i18n";
  import { usageFromCode, isModifier, comboEntries, MODIFIER_USAGES } from "../model/hidcodes";
  import { CATEGORIES } from "../model/codes";
  import { keyName, entriesLabel } from "../model/labels";
  import type { KeyEntry } from "../model/keymap";
  let { onAssign, assignLabel }: { onAssign: (entries: KeyEntry[]) => void; assignLabel?: string } = $props();
  let mods = $state<Set<number>>(new Set());
  let main = $state<number | null>(null);
  let hint = $state("");
  const entries = $derived(comboEntries(mods, main));
  const MAIN_KEYS = CATEGORIES.filter((c) => ["common", "letters", "fkeys", "keypad"].includes(c.id)).flatMap((c) => c.keycaps ?? []).filter((k) => !isModifier(k.entry.code) && k.entry.code !== 0);
  function onKey(e: KeyboardEvent) {
    e.preventDefault();
    if (e.code === "Escape") { mods = new Set(); main = null; hint = ""; return; }
    const usage = usageFromCode(e.code);
    if (usage === null) { hint = t("不支持这个键"); return; }
    hint = "";
    if (isModifier(usage)) { const s = new Set(mods); s.add(usage); mods = s; } else main = usage;
  }
  function toggleMod(u: number) { const s = new Set(mods); if (s.has(u)) s.delete(u); else s.add(u); mods = s; }
  function assign() { if (!entries.length) return; onAssign(entries); mods = new Set(); main = null; hint = ""; }
</script>

<div class="combo">
  <button class="rec" data-role="combo-rec" onkeydown={onKey}>
    <span class="preview">{entries.length ? entriesLabel(entries) : t("点这里，然后按下组合键")}</span>
    <small>{hint || t("录制：先按修饰键再按主键；Esc 清空")}</small>
  </button>
  <div class="manual">
    {#each MODIFIER_USAGES as u}
      <label><input type="checkbox" data-mod={u} checked={mods.has(u)} onchange={() => toggleMod(u)} /> {keyName({ type: 2, code: u })}</label>
    {/each}
    <select data-role="combo-main" value={main === null ? "" : String(main)} onchange={(e) => { const v = (e.target as HTMLSelectElement).value; main = v === "" ? null : Number(v); }}>
      <option value="">{t("主键…")}</option>
      {#each MAIN_KEYS as k}<option value={String(k.entry.code)}>{k.label}</option>{/each}
    </select>
    <button class="primary" data-role="combo-assign" disabled={!entries.length} onclick={assign}>{assignLabel ?? t("分配组合键")}</button>
  </div>
</div>

<style>
  .combo { display: flex; flex-direction: column; gap: 12px; }
  .rec { display: flex; flex-direction: column; align-items: center; gap: 4px; padding: 18px; border: 1px dashed var(--muted); border-radius: 10px; background: var(--card); min-height: 80px; }
  .rec:focus { outline: 2px solid var(--accent); border-color: transparent; }
  .preview { font-size: 18px; font-weight: 600; }
  .rec small { color: var(--muted); font-size: 12px; }
  .manual { display: flex; flex-wrap: wrap; gap: 10px 14px; align-items: center; font-size: 13px; }
  .manual label { display: inline-flex; align-items: center; gap: 4px; }
</style>
```

`entriesLabel` 要从 `labels.ts` 导出（第 2 期已 `export`）。英文表补：`"点这里，然后按下组合键": "Click here, then press the combo"`, `"录制：先按修饰键再按主键；Esc 清空": "Recording: modifiers first, then the main key; Esc clears"`, `"不支持这个键": "That key is not supported"`, `"主键…": "Main key…"`, `"分配组合键": "Assign combo"`。

- [ ] **Step 4: Palette 接入**

`{:else}` 分支之前加：

```svelte
    {:else if category === "combo"}
      <ComboRecorder onAssign={emitEntries} />
```

并 `import ComboRecorder from "./ComboRecorder.svelte";`。

- [ ] **Step 5: 运行** `npm test && npm run check && npm run build`；浏览器里：键位页 → 组合键 → 点录制框按 Ctrl+Shift+T → "分配组合键" → 卡片显示 `⌃⇧T`。
- [ ] **Step 6: 提交** `git add ui && git commit -m "feat(ui): combo recorder with manual fallback in the palette"`。

---

### Task 4: Rust `list_windows`

**Files:**
- Modify: `crates/wm/src/lib.rs`, `crates/wm/src/x11.rs`, `apps/halokey/Cargo.toml`, `apps/halokey/src/main.rs`, `ui/src/lib/tauri.ts`
- Test: `crates/wm/src/lib.rs` 的 `#[cfg(test)]`（纯函数 `WindowInfo::dedup_sorted`），`ui/src/lib/__tests__/tauri.test.ts`

**Interfaces:**
- `halokey_wm::WindowInfo { pub class: String, pub instance: String, pub title: String, pub exe: String }`（serde `camelCase`）
- `halokey_wm::list_windows() -> Result<Vec<WindowInfo>, String>`：Linux 走 X11（`_NET_CLIENT_LIST` 的每个窗口读 `WM_CLASS`、`_NET_WM_NAME`（UTF8_STRING，退化到 `WM_NAME`）、`_NET_WM_PID` → `/proc/<pid>/exe`），去掉 class 为空的，按 class 去重（保留第一个标题），按 class 排序；非 Linux `Err("此平台还没有窗口列表")`。
- `WindowInfo::dedup_sorted(items: Vec<WindowInfo>) -> Vec<WindowInfo>`（纯函数，可测）。
- Tauri 命令 `list_windows() -> ListWindows { windows: Vec<WindowInfo>, error: Option<String> }`（`async`，不碰设备线程）。
- 前端 `api.listWindows(): Promise<{ windows: WindowInfo[]; error: string | null }>`；mock 返回两个假窗口 `[{ class: "google-chrome", instance: "google-chrome", title: "Chrome", exe: "/usr/bin/google-chrome" }, { class: "org.gnome.TextEditor", instance: "gnome-text-editor", title: "Text Editor", exe: "/usr/bin/gnome-text-editor" }]`。

- [ ] **Step 1: 写失败的测试**

`crates/wm/src/lib.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_sorted_keeps_first_title_per_class_and_sorts() {
        let w = |class: &str, title: &str| WindowInfo {
            class: class.into(),
            instance: class.to_ascii_lowercase(),
            title: title.into(),
            exe: String::new(),
        };
        let out = WindowInfo::dedup_sorted(vec![w("Zed", "z1"), w("chrome", "c1"), w("Zed", "z2"), w("", "no class")]);
        assert_eq!(out.len(), 2);
        assert_eq!((out[0].class.as_str(), out[0].title.as_str()), ("chrome", "c1"));
        assert_eq!((out[1].class.as_str(), out[1].title.as_str()), ("Zed", "z1"));
    }
}
```

`ui/src/lib/__tests__/tauri.test.ts` 追加：

```ts
  it("mock window list has classes", async () => {
    const r = await api.listWindows();
    expect(r.error).toBeNull();
    expect(r.windows.map((w) => w.class)).toContain("google-chrome");
  });
```

Run: `cargo test -p halokey-wm 2>&1 | grep -E "^error|cannot find"`；`npx vitest run src/lib/__tests__/tauri.test.ts` → 都失败。

- [ ] **Step 2: 实现 wm**

`crates/wm/Cargo.toml` 加 `serde = { version = "1", features = ["derive"] }`（workspace 里已有 serde，版本同其它 crate）。`lib.rs`：

```rust
use serde::{Deserialize, Serialize};

/// 当前打开的一个顶层窗口，供 APP 编辑器挑选。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    /// WM_CLASS 的 class（Linux 上写进 `bundleIdentifier`）。
    pub class: String,
    pub instance: String,
    pub title: String,
    /// `/proc/<pid>/exe`，拿不到为空。
    pub exe: String,
}

impl WindowInfo {
    /// 去掉没有 class 的，按 class 去重（保留先出现的标题），按 class 不区分大小写排序。
    pub fn dedup_sorted(items: Vec<WindowInfo>) -> Vec<WindowInfo> {
        let mut seen = std::collections::HashSet::new();
        let mut out: Vec<WindowInfo> = items
            .into_iter()
            .filter(|w| !w.class.is_empty())
            .filter(|w| seen.insert(w.class.to_ascii_lowercase()))
            .collect();
        out.sort_by_key(|w| w.class.to_ascii_lowercase());
        out
    }
}

/// 列出当前顶层窗口；没有后端时返回原因。
pub fn list_windows() -> Result<Vec<WindowInfo>, String> {
    #[cfg(target_os = "linux")]
    {
        x11::list_windows().map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("此平台还没有窗口列表".to_string())
    }
}
```

`x11.rs` 加（独立短连接，不复用 `X11Launcher`，因为启动器实例在设备线程里）：

```rust
/// 枚举 `_NET_CLIENT_LIST` 里的窗口：WM_CLASS、标题、可执行文件。
pub fn list_windows() -> Result<Vec<crate::WindowInfo>, Box<dyn std::error::Error>> {
    let (conn, screen) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen].root;
    let atom = |name: &[u8]| -> Result<u32, Box<dyn std::error::Error>> { Ok(conn.intern_atom(false, name)?.reply()?.atom) };
    let net_client_list = atom(b"_NET_CLIENT_LIST")?;
    let net_wm_name = atom(b"_NET_WM_NAME")?;
    let utf8_string = atom(b"UTF8_STRING")?;
    let net_wm_pid = atom(b"_NET_WM_PID")?;
    let windows: Vec<Window> = conn
        .get_property(false, root, net_client_list, AtomEnum::WINDOW, 0, 4096)?
        .reply()?
        .value32()
        .map(|v| v.collect())
        .unwrap_or_default();
    let mut out = Vec::new();
    for w in windows {
        let (class, instance) = match WmClass::get(&conn, w).ok().and_then(|c| c.reply().ok()).flatten() {
            Some(c) => (String::from_utf8_lossy(c.class()).into_owned(), String::from_utf8_lossy(c.instance()).into_owned()),
            None => continue,
        };
        let title = conn
            .get_property(false, w, net_wm_name, utf8_string, 0, 1024)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| String::from_utf8_lossy(&r.value).into_owned())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                conn.get_property(false, w, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 1024)
                    .ok()
                    .and_then(|c| c.reply().ok())
                    .map(|r| String::from_utf8_lossy(&r.value).into_owned())
            })
            .unwrap_or_default();
        let exe = conn
            .get_property(false, w, net_wm_pid, AtomEnum::CARDINAL, 0, 1)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().and_then(|mut v| v.next()))
            .and_then(|pid| std::fs::read_link(format!("/proc/{pid}/exe")).ok())
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        out.push(crate::WindowInfo { class, instance, title, exe });
    }
    Ok(crate::WindowInfo::dedup_sorted(out))
}
```

（`WmClass::get(...).reply()` 返回 `Result<Option<WmClass>>`，所以 `.ok().flatten()` 的写法要按实际类型调整；x11rb 0.14 的 `WmClass::get` 返回 `Result<WmClassCookie>`，`reply()` 返回 `Result<Option<WmClass>>`。）

- [ ] **Step 3: 应用命令与前端桥**

`apps/halokey/Cargo.toml` 加 `halokey-wm = { path = "../../crates/wm" }`。`main.rs`：

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListWindows {
    windows: Vec<halokey_wm::WindowInfo>,
    error: Option<String>,
}

/// 当前顶层窗口列表，供 APP 编辑器挑选；拿不到时给原因而不是失败。
#[tauri::command]
async fn list_windows() -> ListWindows {
    match halokey_wm::list_windows() {
        Ok(windows) => ListWindows { windows, error: None },
        Err(e) => ListWindows { windows: Vec::new(), error: Some(e) },
    }
}
```

注册到 `generate_handler!`。`tauri.ts`：

```ts
export type WindowInfo = { class: string; instance: string; title: string; exe: string };
// api 里：
  listWindows: (): Promise<{ windows: WindowInfo[]; error: string | null }> => isTauri() ? invoke("list_windows") : Promise.resolve({ windows: [{ class: "google-chrome", instance: "google-chrome", title: "Chrome", exe: "/usr/bin/google-chrome" }, { class: "org.gnome.TextEditor", instance: "gnome-text-editor", title: "Text Editor", exe: "/usr/bin/gnome-text-editor" }], error: null }),
```

- [ ] **Step 4: 验证** `cargo test -p halokey-wm`、`cargo clippy --workspace --all-targets -- -D warnings`、`npm test`；本机：`cargo run -p halokey-cli -- --help` 不需要改。真机：下一任务的 APP 编辑器里点"从窗口列表选"能看到当前打开的窗口（GNOME/X11）。
- [ ] **Step 5: 提交** `git add -A && git commit -m "feat(wm,app): list top-level X11 windows for the APP editor"`。

---

### Task 5: APP 编辑器对话框 + 面板"APP"分类

**Files:**
- Create: `ui/src/components/AppEditor.svelte`
- Modify: `ui/src/components/Palette.svelte`, `ui/src/pages/Keymap.svelte`, `ui/src/lib/i18n.ts`
- Test: `ui/src/pages/__tests__/app-editor.test.svelte.ts`

**Interfaces:**
- `AppEditor` props：`{ draft: Draft (bindable); itemId: string | null; onClose: (savedId: string | null) => void }`；模态对话框（`role="dialog"`，遮罩点外面 = 取消）。字段：名称（`data-role="app-name"`）、WM_CLASS（`data-role="app-class"`，写进 `bundleIdentifier`）、可执行文件路径（`data-role="app-path"`）；"从当前窗口选"区：挂载时 `api.listWindows()`，列表 `button[data-window=<class>]`，点一下把 class / 标题（若名称为空）/ exe 填进去；拿不到时显示 `error` 文案但其余照常。保存：名称非空且 ≤ 200 字，class 与 path 至少一个非空，否则中文提示；新建时 `slot = nextSlot(map, "application")`，满了提示"应用最多 16 个"；`upsertLibraryItem` → `onClose(id)`。删除（编辑已有）→ `deleteLibraryItem`，被引用则显示引用列表。
- Palette 新 props：`onEditApp?: (id: string | null) => void`、`onEditMacro?: (id: string | null) => void`；APP 分类卡片 `A{slot}` + 名称 + 小字 class，右侧 ✎（`data-edit-app={id}`），末尾"＋ 添加 APP"（`data-role="new-app"`）；手势模式点卡片 = `assignAction({ library: { _0: id } })`，动作模式 = 内联 `item.action`（第 3 期已做）。
- `Keymap.svelte`：持有 `appEditing: { id: string | null } | null`，渲染 `<AppEditor>`；`onClose(savedId)` 若是新建且 `savedId` 非空 → `assignAction(draft, { library: { _0: savedId } })` 到当前选中手势。

- [ ] **Step 1: 写失败的测试**

```ts
import { describe, it, expect, afterEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import Keymap from "../Keymap.svelte";
import { initialDraft, loadSaved, type Draft } from "../../model/draft";
import type { HostKeymap } from "../../model/keymap";
import { setLang } from "../../lib/i18n";

const load = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/halo-menu.json"), "utf8"));
let app: ReturnType<typeof mount> | undefined;
afterEach(() => { if (app) unmount(app); app = undefined; document.body.innerHTML = ""; });
const buttons = () => [...document.querySelectorAll("button")] as HTMLButtonElement[];
const byText = (t: string) => buttons().find((b) => b.textContent?.trim() === t);
const tick = () => new Promise((r) => setTimeout(r, 0));
const setInput = (el: HTMLInputElement, v: string) => { el.value = v; el.dispatchEvent(new Event("input", { bubbles: true })); };

describe("APP editor", () => {
  it("adds an app from the window list and assigns it to the selected gesture", async () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null } });
    flushSync();
    byText("APP")!.click();
    flushSync();
    (document.querySelector("[data-role=new-app]") as HTMLButtonElement).click();
    await tick(); await tick();
    flushSync();
    expect(document.querySelector("[role=dialog]")).toBeTruthy();
    (document.querySelector("button[data-window='google-chrome']") as HTMLButtonElement).click();
    flushSync();
    expect((document.querySelector("input[data-role=app-class]") as HTMLInputElement).value).toBe("google-chrome");
    expect((document.querySelector("input[data-role=app-name]") as HTMLInputElement).value).toBe("Chrome");
    byText("保存")!.click();
    flushSync();
    const lib = draft.v.map!.actionLibrary!;
    expect(lib).toHaveLength(1);
    expect(lib[0]!.slot).toBe(0);
    expect(lib[0]!.action).toEqual({ application: { _0: { bundleIdentifier: "google-chrome", path: "/usr/bin/google-chrome", name: "Chrome" } } });
    expect(draft.v.map!.controls[0]!.pressAction).toEqual({ library: { _0: lib[0]!.id } });
    expect(document.querySelector("[role=dialog]")).toBeNull();
    expect(document.body.textContent).toContain("A0");
  });
  it("validates and blocks deleting a referenced app", async () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null } });
    flushSync();
    byText("APP")!.click(); flushSync();
    (document.querySelector("[data-role=new-app]") as HTMLButtonElement).click();
    await tick(); flushSync();
    byText("保存")!.click(); flushSync();
    expect(document.body.textContent).toContain("名称不能为空");
    setInput(document.querySelector("input[data-role=app-name]") as HTMLInputElement, "编辑器");
    byText("保存")!.click(); flushSync();
    expect(document.body.textContent).toContain("WM_CLASS 和路径至少填一个");
    setInput(document.querySelector("input[data-role=app-class]") as HTMLInputElement, "gnome-text-editor");
    byText("保存")!.click(); flushSync();
    const id = draft.v.map!.actionLibrary![0]!.id;
    (document.querySelector(`button[data-edit-app='${id}']`) as HTMLButtonElement).click();
    await tick(); flushSync();
    byText("删除")!.click(); flushSync();
    expect(document.body.textContent).toContain("顶部按键 › 单击");
    expect(draft.v.map!.actionLibrary).toHaveLength(1);
  });
});
```

- [ ] **Step 2: 运行确认失败**。

- [ ] **Step 3: `AppEditor.svelte`**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../lib/i18n";
  import { api, type WindowInfo } from "../lib/tauri";
  import { newUuid, nextSlot, upsertLibraryItem, deleteLibraryItem, type Draft } from "../model/draft";
  import type { ApplicationTarget } from "../model/keymap";

  let { draft = $bindable(), itemId, onClose }: { draft: Draft; itemId: string | null; onClose: (savedId: string | null) => void } = $props();
  // svelte-ignore state_referenced_locally
  const existing = itemId ? (draft.map?.actionLibrary ?? []).find((i) => i.id === itemId) ?? null : null;
  const target: ApplicationTarget | null = existing && "application" in existing.action ? (existing.action as { application: { _0: ApplicationTarget } }).application._0 : null;
  let name = $state(existing?.name ?? "");
  let cls = $state(target?.bundleIdentifier ?? "");
  let path = $state(target?.path ?? "");
  let windows = $state<WindowInfo[]>([]);
  let listError = $state("");
  let problems = $state<string[]>([]);
  let blocked = $state<string[]>([]);
  onMount(async () => { try { const r = await api.listWindows(); windows = r.windows; listError = r.error ?? ""; } catch (e) { listError = String(e); } });

  function pick(w: WindowInfo) { cls = w.class; if (!path) path = w.exe; if (!name.trim()) name = w.title || w.class; }
  function save() {
    const p: string[] = [];
    if (!name.trim()) p.push(t("名称不能为空")); else if ([...name.trim()].length > 200) p.push(t("名称不能超过 200 字"));
    if (!cls.trim() && !path.trim()) p.push(t("WM_CLASS 和路径至少填一个"));
    problems = p;
    if (p.length) return;
    const id = existing?.id ?? newUuid();
    const slot = existing?.slot ?? nextSlot(draft.map!, "application");
    if (slot === null) { problems = [t("应用最多 16 个")]; return; }
    draft = upsertLibraryItem(draft, { id, name: name.trim(), slot, action: { application: { _0: { bundleIdentifier: cls.trim(), path: path.trim(), name: name.trim() } } } });
    onClose(id);
  }
  function remove() {
    if (!existing) return;
    const r = deleteLibraryItem(draft, existing.id);
    if (r.blockedBy.length) { blocked = r.blockedBy; return; }
    draft = r.draft; onClose(null);
  }
</script>

<div class="backdrop" role="presentation" onmousedown={(e) => { if (e.target === e.currentTarget) onClose(null); }}>
  <div class="dialog" role="dialog" aria-modal="true">
    <h3>{existing ? t("编辑 APP") : t("添加 APP")}</h3>
    <label>{t("名称")}<input data-role="app-name" bind:value={name} placeholder={t("显示名称")} /></label>
    <label>WM_CLASS<input data-role="app-class" bind:value={cls} placeholder="google-chrome" /></label>
    <label>{t("可执行文件路径")}<input data-role="app-path" bind:value={path} placeholder="/usr/bin/google-chrome" /></label>
    <div class="windows">
      <div class="muted">{t("从当前打开的窗口选")}{#if listError}：{listError}{/if}</div>
      <div class="list">
        {#each windows as w}<button data-window={w.class} onclick={() => pick(w)}><b>{w.class}</b><small>{w.title}</small></button>{/each}
        {#if !windows.length && !listError}<span class="muted">{t("没有找到窗口")}</span>{/if}
      </div>
    </div>
    {#if problems.length}<p class="bad">{problems.join("；")}</p>{/if}
    {#if blocked.length}<p class="bad">{t("被以下位置引用，无法删除")}：{blocked.join("、")}</p>{/if}
    <footer>
      {#if existing}<button onclick={remove}>{t("删除")}</button>{/if}
      <button onclick={() => onClose(null)}>{t("取消")}</button>
      <button class="primary" onclick={save}>{t("保存")}</button>
    </footer>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, .5); display: flex; align-items: center; justify-content: center; z-index: 50; }
  .dialog { width: 560px; max-width: calc(100vw - 32px); background: var(--panel); border: 1px solid var(--line); border-radius: 12px; padding: 18px 20px; display: flex; flex-direction: column; gap: 10px; box-shadow: 0 16px 48px rgba(0, 0, 0, .5); }
  h3 { margin: 0 0 4px; font-size: 15px; }
  label { display: grid; grid-template-columns: 120px 1fr; align-items: center; gap: 10px; font-size: 13px; color: var(--muted); }
  label input { width: 100%; }
  .windows { border-top: 1px solid var(--line); padding-top: 10px; }
  .list { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 8px; max-height: 180px; overflow: auto; }
  .list button { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; min-width: 150px; max-width: 240px; }
  .list small { color: var(--muted); font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 220px; }
  .muted { color: var(--muted); font-size: 13px; }
  .bad { color: var(--bad); font-size: 13px; margin: 0; }
  footer { display: flex; justify-content: flex-end; gap: 10px; margin-top: 6px; }
</style>
```

- [ ] **Step 4: Palette 的 APP / 宏分类卡片**

把现有 `category === "app" || category === "macro"` 分支改为：

```svelte
    {:else if category === "app" || category === "macro"}
      {@const items = category === "app" ? apps : macros}
      {@const edit = category === "app" ? onEditApp : onEditMacro}
      {@const letter = category === "app" ? "A" : "M"}
      <div class="cards">
        {#each items as item}
          <div class="item lib">
            <button class="pick" onclick={() => (actionMode ? emitAction(item.action) : (draft = assignAction(draft, { library: { _0: item.id } })))}>
              <small>{letter}{item.slot}</small>{item.name}
              {#if category === "app"}<small class="sub">{(item.action as { application: { _0: { bundleIdentifier: string } } }).application._0.bundleIdentifier}</small>{/if}
            </button>
            {#if edit}<button class="edit" data-edit-app={category === "app" ? item.id : undefined} data-edit-macro={category === "macro" ? item.id : undefined} title={t("编辑")} onclick={() => edit?.(item.id)}>✎</button>{/if}
          </div>
        {/each}
        {#if edit}<button class="item new" data-role={category === "app" ? "new-app" : "new-macro"} onclick={() => edit?.(null)}>＋ {t(category === "app" ? "添加 APP" : "新建宏")}</button>{/if}
      </div>
```

props 加 `onEditApp?: (id: string | null) => void; onEditMacro?: (id: string | null) => void;`；`.item.lib` 复用 `.item.halo` 的样式（把选择器改成 `.item.halo, .item.lib`），`.sub { font-size: 10px; opacity: .7; }`。删掉那两行"在本机构建里还没有的功能"。`i18n`：`"编辑 APP": "Edit app"`, `"添加 APP": "Add app"`, `"显示名称": "Display name"`, `"可执行文件路径": "Executable path"`, `"从当前打开的窗口选": "Pick from open windows"`, `"没有找到窗口": "No windows found"`, `"名称不能超过 200 字": "Name must be 200 characters or fewer"`, `"WM_CLASS 和路径至少填一个": "Fill in WM_CLASS or the path"`, `"应用最多 16 个": "At most 16 apps"`, `"删除": "Delete"`, `"保存": "Save"`, `"编辑": "Edit"`, `"新建宏": "New macro"`。

- [ ] **Step 5: Keymap 接线**

```ts
  import AppEditor from "../components/AppEditor.svelte";
  let appEditing = $state<{ id: string | null } | null>(null);
  function closeApp(savedId: string | null) {
    if (appEditing && appEditing.id === null && savedId) draft = assignAction(draft, { library: { _0: savedId } });
    appEditing = null;
  }
```

模板：`<Palette … onEditApp={(id) => (appEditing = { id })} {onEditMacro} />`，并在 `</div>`（`.keymap`）前 `{#if appEditing}<AppEditor bind:draft itemId={appEditing.id} onClose={closeApp} />{/if}`。`Keymap` props 加 `onEditMacro?: (id: string | null) => void`。`assignAction` 已在 import 里？没有则加。

- [ ] **Step 6: 运行** `npm test && npm run check && npm run build`；`cargo build --release -p halokey-app` 后真机：APP 分类 → 添加 APP → 窗口列表里有当前打开的窗口 → 选一个保存 → 分配到键 → 按键切到该窗口。
- [ ] **Step 7: 提交** `git add -A ui && git commit -m "feat(ui): APP editor dialog with window picker; library cards in the palette"`。

---

### Task 6: 宏编辑器页面 + 面板"宏"分类 + 路由

**Files:**
- Create: `ui/src/pages/MacroEditor.svelte`, `ui/src/model/macro.ts`
- Modify: `ui/src/App.svelte`, `ui/src/lib/i18n.ts`
- Test: `ui/src/model/__tests__/macro.test.ts`, `ui/src/pages/__tests__/macro-editor.test.svelte.ts`

**Interfaces:**
- `macro.ts`：
  - `type MacroStepKind = "keyboard" | "application" | "delay"`
  - `stepLabel(step: MacroStep, apps: NamedHostAction[]): string`（`⌃⇧T` / `切换到 Chrome` / `等待 0.3 秒`）
  - `macroProblems(name: string, steps: MacroStep[]): string[]`（名称为空 / 超 80 字；0 步 / 超过 32 步；等待不在 0.05–10 秒；键盘步为空）
  - `moveStep(steps, i, delta)`、`removeStep(steps, i)`、`insertStep(steps, i, step)`（纯函数，返回新数组）
- `MacroEditor` props：`{ draft: Draft (bindable); itemId: string | null; onClose: (savedId: string | null) => void }`。布局：顶栏（返回 / 标题 / 名称 `data-role="macro-name"`）；左：步骤列表（`li[data-step]`，每行 序号 / 类型图标 / `stepLabel` / ↑↓✕，`data-role="step-up|step-down|step-remove"`）；右："添加步骤"三块：
  - 组合键：`<ComboRecorder assignLabel={t("添加为步骤")} onAssign={(e) => add({ keyboard: { _0: e } })} />`，上方一行提示 `t("录制时 halokey 窗口有焦点，设备注入的按键也会被录进来")`；
  - APP：当前功能库里的应用按钮（`data-add-app=<id>`，点一下插入内联 `application` 步骤——宏步骤不能引用功能库，复制内容）+ 从窗口列表临时挑（复用 `api.listWindows`，按钮 `data-add-window=<class>`，插入 `{ application: { _0: { bundleIdentifier: class, path: exe, name: title || class } } }`）；
  - 等待：数字输入 `data-role="delay-seconds"`（默认 0.3，步进 0.05）+ "添加等待"（`data-role="add-delay"`）。
  - 新步骤插在当前选中步骤之后（无选中则末尾）；"清空最后一步"按钮（`data-role="pop-step"`）。
  - 底栏：删除宏（编辑已有，被引用则提示）、取消、保存宏（`macroProblems` 为空 → `upsertLibraryItem` with `slot = existing?.slot ?? nextSlot(map, "macro")`，满了提示"宏最多 16 个"）。
- `App.svelte`：`page` 加 `"macro"`，`openMacro(id)` / `closeMacro(savedId)`（新建成功 → 分配 `{ library: { _0: savedId } }` 到 `returnTo`），`<Keymap … onEditMacro={openMacro} />`。

- [ ] **Step 1: 写失败的测试**

`macro.test.ts`：

```ts
import { describe, it, expect } from "vitest";
import { stepLabel, macroProblems, moveStep, removeStep, insertStep } from "../macro";
import type { MacroStep } from "../keymap";

const kb: MacroStep = { keyboard: { _0: [{ type: 2, code: 0xe0 }, { type: 2, code: 0x17 }] } };
const ap: MacroStep = { application: { _0: { bundleIdentifier: "google-chrome", path: "", name: "Chrome" } } };
const dl: MacroStep = { delay: { _0: 0.3 } };

describe("macro model", () => {
  it("labels steps", () => {
    expect(stepLabel(kb, [])).toBe("⌃T");
    expect(stepLabel(ap, [])).toBe("切换到 Chrome");
    expect(stepLabel(dl, [])).toBe("等待 0.3 秒");
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
```

`macro-editor.test.svelte.ts`：

```ts
import { describe, it, expect, afterEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import MacroEditor from "../MacroEditor.svelte";
import { initialDraft, loadSaved, type Draft } from "../../model/draft";
import type { HostKeymap } from "../../model/keymap";
import { setLang } from "../../lib/i18n";

const load = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/halo-menu.json"), "utf8"));
let app: ReturnType<typeof mount> | undefined;
afterEach(() => { if (app) unmount(app); app = undefined; document.body.innerHTML = ""; });
const buttons = () => [...document.querySelectorAll("button")] as HTMLButtonElement[];
const byText = (t: string) => buttons().find((b) => b.textContent?.trim() === t);
const tick = () => new Promise((r) => setTimeout(r, 0));
const setInput = (el: HTMLInputElement, v: string) => { el.value = v; el.dispatchEvent(new Event("input", { bubbles: true })); };
const key = (el: Element, code: string) => el.dispatchEvent(new KeyboardEvent("keydown", { code, bubbles: true, cancelable: true }));

describe("macro editor", () => {
  it("builds a macro from recorded keys, a window and a delay, then saves it", async () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    const closed: (string | null)[] = [];
    app = mount(MacroEditor, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, itemId: null, onClose: (id: string | null) => closed.push(id) } });
    await tick(); flushSync();
    expect(document.body.textContent).toContain("设备注入的按键也会被录进来");
    byText("保存宏")!.click(); flushSync();
    expect(document.body.textContent).toContain("名称不能为空");
    setInput(document.querySelector("input[data-role=macro-name]") as HTMLInputElement, "打开并输入");
    (document.querySelector("button[data-add-window='google-chrome']") as HTMLButtonElement).click(); flushSync();
    setInput(document.querySelector("input[data-role=delay-seconds]") as HTMLInputElement, "0.5");
    (document.querySelector("[data-role=add-delay]") as HTMLButtonElement).click(); flushSync();
    const rec = document.querySelector("[data-role=combo-rec]")!;
    key(rec, "ControlLeft"); key(rec, "KeyT"); flushSync();
    (document.querySelector("[data-role=combo-assign]") as HTMLButtonElement).click(); flushSync();
    expect(document.querySelectorAll("li[data-step]")).toHaveLength(3);
    (document.querySelector("[data-role=pop-step]") as HTMLButtonElement).click(); flushSync();
    expect(document.querySelectorAll("li[data-step]")).toHaveLength(2);
    key(rec, "KeyA"); flushSync();
    (document.querySelector("[data-role=combo-assign]") as HTMLButtonElement).click(); flushSync();
    byText("保存宏")!.click(); flushSync();
    expect(closed).toHaveLength(1);
    const item = draft.v.map!.actionLibrary![0]!;
    expect(item.name).toBe("打开并输入");
    expect(item.slot).toBe(0);
    expect(item.action).toEqual({ macro: { _0: [
      { application: { _0: { bundleIdentifier: "google-chrome", path: "/usr/bin/google-chrome", name: "Chrome" } } },
      { delay: { _0: 0.5 } },
      { keyboard: { _0: [{ type: 2, code: 0x04 }] } },
    ] } });
  });
  it("edits, reorders and refuses deleting a referenced macro", async () => {
    setLang("zh");
    let d = loadSaved(initialDraft(), load());
    const id = "55555555-5555-4555-8555-555555555555";
    d = { ...d, map: { ...d.map!, actionLibrary: [{ id, name: "宏一", slot: 0, action: { macro: { _0: [{ delay: { _0: 0.3 } }, { keyboard: { _0: [{ type: 2, code: 0x28 }] } }] } } }] } };
    d = { ...d, map: { ...d.map!, controls: d.map!.controls.map((c, i) => (i === 1 ? { ...c, longPressAction: { library: { _0: id } } } : c)) } };
    const draft = $state<{ v: Draft }>({ v: d });
    app = mount(MacroEditor, { target: document.body, props: { get draft() { return draft.v; }, set draft(dd: Draft) { draft.v = dd; }, itemId: id, onClose: () => {} } });
    await tick(); flushSync();
    expect((document.querySelector("input[data-role=macro-name]") as HTMLInputElement).value).toBe("宏一");
    ([...document.querySelectorAll("[data-role=step-down]")][0] as HTMLButtonElement).click(); flushSync();
    expect(document.querySelectorAll("li[data-step]")[0]!.textContent).toContain("Enter");
    byText("删除宏")!.click(); flushSync();
    expect(document.body.textContent).toContain("中间按键 › 长按");
  });
});
```

- [ ] **Step 2: 运行确认失败** → 两个文件找不到模块。

- [ ] **Step 3: `macro.ts`**

```ts
import type { MacroStep, NamedHostAction, ApplicationTarget, KeyEntry } from "./keymap";
import { entriesLabel } from "./labels";
import { t, lang } from "../lib/i18n";

export type MacroStepKind = "keyboard" | "application" | "delay";
export const stepKind = (s: MacroStep): MacroStepKind => Object.keys(s)[0] as MacroStepKind;

export function stepLabel(step: MacroStep, _apps: NamedHostAction[]): string {
  if ("keyboard" in step) return entriesLabel((step as { keyboard: { _0: KeyEntry[] } }).keyboard._0);
  if ("application" in step) return `${t("切换到")} ${(step as { application: { _0: ApplicationTarget } }).application._0.name}`;
  const s = (step as { delay: { _0: number } }).delay._0;
  return lang() === "en" ? `Wait ${s} s` : `等待 ${s} 秒`;
}

export function macroProblems(name: string, steps: MacroStep[]): string[] {
  const p: string[] = [];
  const n = name.trim();
  if (!n) p.push("名称不能为空"); else if ([...n].length > 80) p.push("名称不能超过 80 字");
  if (!steps.length) p.push("至少要有一步");
  if (steps.length > 32) { p.push("最多 32 步"); return p; }
  steps.forEach((s, i) => {
    if ("delay" in s) { const d = (s as { delay: { _0: number } }).delay._0; if (!Number.isFinite(d) || d < 0.05 || d > 10) p.push(`第 ${i + 1} 步等待时间应为 0.05–10 秒`); }
    if ("keyboard" in s && !(s as { keyboard: { _0: KeyEntry[] } }).keyboard._0.length) p.push(`第 ${i + 1} 步没有按键`);
  });
  return p;
}

export function moveStep(steps: MacroStep[], i: number, delta: -1 | 1): MacroStep[] {
  const j = i + delta;
  if (j < 0 || j >= steps.length) return steps;
  const out = steps.slice();
  [out[i], out[j]] = [out[j]!, out[i]!];
  return out;
}
export const removeStep = (steps: MacroStep[], i: number): MacroStep[] => steps.filter((_, k) => k !== i);
export const insertStep = (steps: MacroStep[], at: number, step: MacroStep): MacroStep[] => [...steps.slice(0, at), step, ...steps.slice(at)];
```

（`macroProblems` 的中文是数据字符串，与 `haloProblems` 一样留给第 5 期做结构化 i18n。）

- [ ] **Step 4: `MacroEditor.svelte`**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../lib/i18n";
  import { api, type WindowInfo } from "../lib/tauri";
  import ComboRecorder from "../components/ComboRecorder.svelte";
  import { stepLabel, stepKind, macroProblems, moveStep, removeStep, insertStep } from "../model/macro";
  import { newUuid, nextSlot, upsertLibraryItem, deleteLibraryItem, libraryItems, type Draft } from "../model/draft";
  import type { MacroStep, NamedHostAction, ApplicationTarget, KeyEntry } from "../model/keymap";

  let { draft = $bindable(), itemId, onClose }: { draft: Draft; itemId: string | null; onClose: (savedId: string | null) => void } = $props();
  // svelte-ignore state_referenced_locally
  const existing = itemId ? (draft.map?.actionLibrary ?? []).find((i) => i.id === itemId) ?? null : null;
  const initialSteps: MacroStep[] = existing && "macro" in existing.action ? JSON.parse(JSON.stringify((existing.action as { macro: { _0: MacroStep[] } }).macro._0)) : [];
  let name = $state(existing?.name ?? "");
  let steps = $state<MacroStep[]>(initialSteps);
  let selected = $state<number | null>(null);
  let delay = $state(0.3);
  let windows = $state<WindowInfo[]>([]);
  let listError = $state("");
  let problems = $state<string[]>([]);
  let blocked = $state<string[]>([]);
  const apps = $derived(draft.map ? libraryItems(draft.map, "application") : []);
  const ICON: Record<string, string> = { keyboard: "⌨", application: "▣", delay: "⏱" };
  onMount(async () => { try { const r = await api.listWindows(); windows = r.windows; listError = r.error ?? ""; } catch (e) { listError = String(e); } });

  function add(step: MacroStep) {
    const at = selected === null ? steps.length : selected + 1;
    steps = insertStep(steps, at, step);
    selected = at;
    problems = [];
  }
  const addApp = (a: NamedHostAction) => add({ application: { _0: JSON.parse(JSON.stringify((a.action as { application: { _0: ApplicationTarget } }).application._0)) } });
  const addWindow = (w: WindowInfo) => add({ application: { _0: { bundleIdentifier: w.class, path: w.exe, name: w.title || w.class } } });
  const addKeys = (entries: KeyEntry[]) => add({ keyboard: { _0: entries } });
  function pop() { if (!steps.length) return; steps = removeStep(steps, steps.length - 1); selected = steps.length ? Math.min(selected ?? 0, steps.length - 1) : null; }
  function save() {
    problems = macroProblems(name, steps);
    if (problems.length) return;
    const id = existing?.id ?? newUuid();
    const slot = existing?.slot ?? nextSlot(draft.map!, "macro");
    if (slot === null) { problems = [t("宏最多 16 个")]; return; }
    draft = upsertLibraryItem(draft, { id, name: name.trim(), slot, action: { macro: { _0: steps } } });
    onClose(id);
  }
  function remove() {
    if (!existing) return;
    const r = deleteLibraryItem(draft, existing.id);
    if (r.blockedBy.length) { blocked = r.blockedBy; return; }
    draft = r.draft; onClose(null);
  }
</script>

<div class="page">
  <header>
    <button class="back" onclick={() => onClose(null)} title={t("返回")}>←</button>
    <h2>{existing ? t("编辑宏") : t("新建宏")}</h2>
    <input data-role="macro-name" placeholder={t("宏名称")} bind:value={name} />
  </header>
  <section class="body">
    <div class="steps">
      <div class="muted">{t("步骤")}（{steps.length}/32）<button class="small" data-role="pop-step" disabled={!steps.length} onclick={pop}>{t("清空最后一步")}</button></div>
      <ol>
        {#each steps as s, i}
          <li data-step={i} class:on={i === selected}>
            <button class="row" onclick={() => (selected = i)}><span class="n">{i + 1}</span><span class="ic">{ICON[stepKind(s)]}</span><span class="lbl">{stepLabel(s, apps)}</span></button>
            <span class="ops">
              <button data-role="step-up" disabled={i === 0} onclick={() => { steps = moveStep(steps, i, -1); selected = i - 1; }}>↑</button>
              <button data-role="step-down" disabled={i === steps.length - 1} onclick={() => { steps = moveStep(steps, i, 1); selected = i + 1; }}>↓</button>
              <button data-role="step-remove" onclick={() => { steps = removeStep(steps, i); selected = null; }}>✕</button>
            </span>
          </li>
        {/each}
      </ol>
      {#if !steps.length}<p class="muted">{t("右边添加步骤；新步骤插在选中步骤之后")}</p>{/if}
    </div>
    <div class="adders">
      <h4>⌨ {t("组合键")}</h4>
      <p class="muted warn">{t("录制时 halokey 窗口有焦点，设备注入的按键也会被录进来")}</p>
      <ComboRecorder assignLabel={t("添加为步骤")} onAssign={addKeys} />
      <h4>▣ APP</h4>
      <div class="cards">
        {#each apps as a}<button data-add-app={a.id} onclick={() => addApp(a)}>A{a.slot} {a.name}</button>{/each}
        {#each windows as w}<button data-add-window={w.class} onclick={() => addWindow(w)}><b>{w.class}</b><small>{w.title}</small></button>{/each}
        {#if listError}<span class="muted">{listError}</span>{/if}
      </div>
      <h4>⏱ {t("等待")}</h4>
      <div class="delay"><input type="number" data-role="delay-seconds" min="0.05" max="10" step="0.05" bind:value={delay} /> {t("秒")} <button data-role="add-delay" onclick={() => add({ delay: { _0: Number(delay) } })}>{t("添加等待")}</button></div>
    </div>
  </section>
  <footer>
    {#if problems.length}<span class="bad">{problems.join("；")}</span>{/if}
    {#if blocked.length}<span class="bad">{t("被以下位置引用，无法删除")}：{blocked.join("、")}</span>{/if}
    {#if existing}<button onclick={remove}>{t("删除宏")}</button>{/if}
    <button onclick={() => onClose(null)}>{t("取消")}</button>
    <button class="primary" onclick={save}>{t("保存宏")}</button>
  </footer>
</div>

<style>
  .page { display: flex; flex-direction: column; min-height: 100%; }
  header { display: flex; align-items: center; gap: 14px; padding: 12px 22px; border-bottom: 1px solid var(--line); }
  header h2 { margin: 0; font-size: 16px; } header input { flex: 0 0 260px; }
  .back { background: none; font-size: 18px; padding: 2px 8px; }
  .body { display: grid; grid-template-columns: 1fr 1fr; gap: 24px; padding: 16px 22px; align-items: start; }
  ol { list-style: none; margin: 8px 0 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  li { display: flex; align-items: center; gap: 6px; background: var(--card); border-radius: 10px; padding: 4px 8px 4px 4px; border: 1px solid transparent; }
  li.on { border-color: var(--accent); }
  .row { flex: 1; display: flex; align-items: center; gap: 10px; background: none; text-align: left; }
  .n { color: var(--muted); font-family: var(--mono); width: 22px; } .ic { width: 20px; text-align: center; color: var(--muted); }
  .lbl { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ops { display: flex; gap: 4px; } .ops button { padding: 2px 8px; }
  .adders h4 { margin: 14px 0 6px; font-size: 13px; color: var(--muted); }
  .cards { display: flex; flex-wrap: wrap; gap: 8px; }
  .cards button { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; }
  .cards small { color: var(--muted); font-size: 11px; }
  .delay { display: flex; align-items: center; gap: 8px; } .delay input { width: 90px; }
  .small { padding: 2px 8px; font-size: 12px; margin-left: 8px; }
  .muted { color: var(--muted); font-size: 13px; } .warn { margin: 0 0 8px; }
  footer { display: flex; justify-content: flex-end; align-items: center; gap: 10px; padding: 12px 22px; border-top: 1px solid var(--line); background: var(--bg); }
  .bad { color: var(--bad); margin-right: auto; font-size: 13px; }
</style>
```

`i18n` 补：`"编辑宏": "Edit macro"`, `"宏名称": "Macro name"`, `"步骤": "Steps"`, `"清空最后一步": "Remove last step"`, `"右边添加步骤；新步骤插在选中步骤之后": "Add steps on the right; new steps go after the selected one"`, `"录制时 halokey 窗口有焦点，设备注入的按键也会被录进来": "While recording, keys injected by the device into this focused window are captured too"`, `"添加为步骤": "Add as step"`, `"等待": "Wait"`, `"添加等待": "Add wait"`, `"宏最多 16 个": "At most 16 macros"`, `"删除宏": "Delete macro"`, `"保存宏": "Save macro"`。

- [ ] **Step 5: `App.svelte` 路由**

`page` 类型加 `"macro"`；复用光环的 `editing` 结构：

```ts
  let editing = $state<{ kind: "halo" | "macro"; id: string | null; wasNew: boolean; returnTo: Selection } | null>(null);
  function openHalo(id: string | null) { editing = { kind: "halo", id, wasNew: id === null, returnTo: draft.selection }; page = "halo"; }
  function openMacro(id: string | null) { editing = { kind: "macro", id, wasNew: id === null, returnTo: draft.selection }; page = "macro"; }
  function closeEditor(savedId: string | null) {
    if (editing?.wasNew && savedId) {
      const action = editing.kind === "halo" ? { halo: { _0: savedId } } : { library: { _0: savedId } };
      draft = assignAction(select(draft, editing.returnTo), action);
    }
    editing = null; page = "keymap";
  }
```

模板：`{:else if page === "macro"}<MacroEditor bind:draft itemId={editing?.id ?? null} onClose={closeEditor} />`；光环分支改用 `closeEditor`；`<Keymap … onEditHalo={openHalo} onEditMacro={openMacro} />`；`TopBar page={page === "keymap" || page === "halo" || page === "macro" ? "keymap" : page}`。

- [ ] **Step 6: 运行** `npm test && npm run check && npm run build`；浏览器里走一遍新建宏。
- [ ] **Step 7: 提交** `git add -A ui && git commit -m "feat(ui): macro editor page with recorded keys, apps and waits"`。

---

### Task 7: 文档 + 真机清单

**Files:**
- Modify: `docs/03-config-format.md`, `docs/03-config-format.zh.md`, `docs/01-architecture.md`, `docs/01-architecture.zh.md`

- [ ] **Step 1: 文档**

`docs/03-config-format.md` §3（Entries and actions）末尾加一段：

```markdown
The settings window edits `actionLibrary` through the APP and Macro editors (the APP editor can pick WM_CLASS / executable from the current X11 windows via `list_windows`), and records key combinations from the real keyboard (modifiers `0xE0–0xE7` first, then the main key). Macro steps copy the application inline; they never reference the library.
```

中文版对应：

```markdown
设置窗口通过 APP 编辑器与宏编辑器维护 `actionLibrary`（APP 编辑器可借 `list_windows` 从当前 X11 窗口里挑 WM_CLASS / 可执行文件），组合键从真实键盘录制（修饰键 `0xE0–0xE7` 在前、主键在后）。宏步骤内联复制应用，不引用功能库。
```

`docs/01-architecture.md` 第 4 条里程碑（Halo overlay）末尾追加 "**Combo recording, APP and macro editors 2026-10-18** (phase 4)."，中文版 "**组合键录制、APP 与宏编辑器 2026-10-18**（第 4 期）。"。

- [ ] **Step 2: 全量检查** `python3 tools/check_docs.py && cd ui && npm test && npm run check && npm run build && cd .. && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo build --release -p halokey-app`。

- [ ] **Step 3: 真机清单（本机 GNOME/X11，逐项记录到提交信息）**

1. 组合键：选"顶部按键 › 单击"→ 组合键 → 点录制框按 `Ctrl+Shift+T` → 卡片显示 `⌃⇧T` → 保存 → 在 Chrome 里按键 1 重新打开关闭的标签页。
2. 组合键手动：勾"左 Super"+ 主键 `Space`（或任意）→ 分配 → 卡片 `⌘Space`。
3. 不支持的键：录制框里按媒体键（音量键）→ 提示"不支持这个键"，状态不变。
4. APP：APP 分类 → 添加 APP → 窗口列表里能看到当前窗口（含 Chrome / 终端）→ 选 Chrome 保存 → 自动分配到当前手势 → 保存到本机 → 按键切到 Chrome；再按一次（Chrome 已在前台）看行为与第 2 期一致。
5. APP 手填：只填名称与路径 `/usr/bin/gnome-text-editor` → 保存 → 按键能启动 / 切换。
6. 宏：新建宏"打开并输入"：窗口列表选文本编辑器 → 等待 0.5 → 录 `Ctrl+N` → 等待 0.3 → 录 `h`、`i`（两步）→ 保存 → 分配到旋钮按下 → 保存到本机 → 按旋钮：编辑器打开新文档并输入 hi。
7. 编辑宏：把第一步下移、删一步、保存；再触发看顺序变化。
8. 删除保护：删除被引用的 APP / 宏 → 提示引用位置；清掉引用后可删。
9. 16 个上限：连续添加到第 17 个 APP → 提示"应用最多 16 个"。
10. 英文界面：三个分类、两个编辑器、录制框文案全英文（校验信息除外，第 5 期）。

- [ ] **Step 4: 提交并推送**

```bash
git add -A && git commit -m "docs: action-library editors and combo recording; phase 4 complete

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>" && git push origin main
```

---

## 自检记录

- 规格覆盖：§4.4 组合键（录制 4 修饰 + 1 主键 + "分配组合键"，手动勾选 + 主键）→ Task 1、3；APP 卡片 `A0 A1…`（名称 + WM_CLASS）、"添加 APP" 编辑器（名称、WM_CLASS、路径、从窗口列表挑）→ Task 4、5；宏卡片 `M0…`、"新建宏" 编辑器（APP / 等待 / 组合键，上下移、删除，录制模式）→ Task 6；§8 `list_windows` → Task 4；§12 第 4 期范围。
- 类型一致性：`usageFromCode / comboEntries / isModifier / MODIFIER_USAGES`（Task 1）在 Task 3 同名；`newUuid / nextSlot / upsertLibraryItem / deleteLibraryItem / libraryReferences / libraryItems`（Task 2）在 Task 5、6 同名；`WindowInfo { class, instance, title, exe }` 在 Rust / TS 同形（camelCase 下字段名本来就是小写单词）；`api.listWindows()` 返回 `{ windows, error }` 在 Task 5、6 同用；`ComboRecorder` 的 `onAssign / assignLabel`（Task 3）在 Task 6 同名；`Palette` 的 `onEditApp / onEditMacro`（Task 5）在 Keymap / App 同名。
- Review Focus 五项：1 → Task 1 `Unidentified / MediaPlayPause → null` + Task 3 录制框测试；2 → Task 2 `deleteLibraryItem` + Task 5 / 6 删除测试；3 → Task 2 `nextSlot → null` 测试 + Task 5 / 6 提示（真机清单 9）；4 → Task 4 `error` 字段 + Task 5 mock 无窗口时仍可保存（第二个用例手填）；5 → Task 6 文案测试 + "清空最后一步"。
- 无自动化测试的部分：X11 `list_windows` 本体（需要 X 连接）——真机清单 4；执行时按 Ruling 记录。
