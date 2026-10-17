# 光环编辑器 + 扇形转盘 · 第 3 期实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在键位页里新建 / 编辑 / 删除光环菜单（整页编辑器：环形预览 + 槽位列表 + 复用键码面板），屏幕上的转盘换成 Surface Dial 式扇形环，并能设置出现位置、大小与超时。

**Architecture:** 光环只是键位文件里的 `halos[]`，编辑全部走内存草稿（新增几个纯 reducer），保存仍经 `validate_keymap` → `save_keymap`。转盘窗口的内容由现有运行时事件驱动（`haloShow / haloSelect / haloHide`），本期只换渲染组件并让 Rust 按设置调整窗口大小与位置；超时经新的 `Command::SetHaloTimeout` 下发。应用偏好进 `settings.json`（新增 `halo` 段），通过通用的 `set_settings(patch)` 命令写入。

**Tech Stack:** Svelte 5 + TypeScript + vitest（`ui/`），Rust（`crates/runtime`、`apps/halokey`），Tauri 2 窗口 API（`set_size` / `set_position` / `center`）。

**Spec:** `docs/superpowers/specs/2026-10-17-settings-ui-redesign-design.md` §4.4 光环分类、§5.1 编辑器、§5.2 转盘悬浮窗、§6 "光环转盘"卡、§7 `settings.json`、§8 `get_settings / set_settings`、§12 第 3 期。

## Global Constraints

- 光环校验规则不变（Rust `Halo::validate`）：标识与名称非空且 ≤ 80 字，1–8 个选项，每个选项标签非空，动作不能是 `library` 或 `momentaryLayer`；引用的光环必须存在。前端编辑器在保存光环时做同样的检查并给中文提示，Rust 仍是最后一道门。
- 转盘尺寸：小 / 中 / 大 = 外径 280 / 360 / 440 px，内径约 60%；N 个等分扇形从 12 点钟方向顺时针排；高亮扇形用强调色填充；中心圆显示高亮项名称；底部提示"拧旋钮选择 · 按下确认 · 按任意键取消"。
- 出现位置：鼠标指针周围（默认）或屏幕中央；超时默认 4 秒，可选 2 / 4 / 6 / 10。
- `settings.json` 形状（§7）：`{ "idleMinutes": 5, "halo": { "position": "pointer", "timeoutSeconds": 4, "size": "medium" } }`；`language / theme / closeToTray` 留第 5 期；缺省字段用默认值，旧文件照常读。
- 配置里 `options` 只含有效项，顺序即槽位顺序；空槽位不写进文件。
- 界面文字中文经 `t()`，英文表同步；每个任务结束 `npm test`、`npm run check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`、`python3 tools/check_docs.py` 全绿；提交英文，末尾 `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`。
- 不新增依赖。
- 调试：`cd ui && npm run dev` + `cargo run -p halokey-app`；发布：`npm run build` 后 `cargo build --release -p halokey-app`（`build.rs` 跟踪 `ui/dist`）。转盘窗口在浏览器里可用 `http://localhost:5177/halo.html` 看（mock 每 6 秒推一次假的 `haloShow`）。

## Review Focus

1. **删除一个仍被手势或其它光环引用的光环**：不能让文件变成 Rust 会拒绝的状态（`MissingHalo`）。→ Task 1：`deleteHalo` 拒绝并返回引用位置；编辑器显示"被 X 引用，无法删除"。
2. **编辑器里把自己选成某个槽位的动作（光环套自己）**：会死循环。→ Task 1 `haloActionAllowed` + Task 3 面板在光环分类里隐藏正在编辑的那个。
3. **8 个槽位全空就点"保存光环"**：Rust 会拒绝（至少 1 项）；编辑器要先拦住并提示。→ Task 2 `haloProblems` 测试。
4. **转盘只有 1 个或 2 个选项**：扇形仍要画对（1 个 = 整圆），选择 / 确认不崩。→ Task 4 `wedgePath` 测试覆盖 n=1、2、8。
5. **旧 `settings.json` 没有 `halo` 段**：读出默认值，写回时不丢 `idleMinutes`。→ Task 5 Rust 单测 `settings_merge_keeps_unknown_and_defaults`。

---

## 文件结构

```
ui/src/model/draft.ts                 upsertHalo / deleteHalo / haloReferences / haloActionAllowed / newHaloId
ui/src/model/halo.ts                  编辑器用的槽位模型：Slot、slotsFrom(halo)、haloFrom(slots)、haloProblems
ui/src/model/__tests__/halo.test.ts   + draft.test.ts 追加
ui/src/components/Palette.svelte      新增 mode="action" + onAction + excludeHalo + onEditHalo（新建 / 编辑入口）
ui/src/components/RingPreview.svelte  SVG 8 槽环形预览（编辑器）
ui/src/components/HaloWheel.svelte    转盘悬浮窗渲染（替换 HaloBubbles）
ui/src/lib/wedge.ts                   wedgePath(n, i, outer, inner) 纯函数（预览与转盘共用）
ui/src/pages/HaloEditor.svelte        整页编辑器
ui/src/App.svelte                     页面路由加 "halo"（editing: id | null, returnTo）
ui/src/pages/Keymap.svelte            把 onEditHalo 往上抛
ui/src/pages/Settings.svelte          "光环转盘"卡（位置 / 超时 / 大小）
ui/src/lib/tauri.ts                   AppSettings.halo、api.setSettings、haloShow 事件带 size
ui/src/halo.ts                        挂 HaloWheel
crates/runtime/src/lib.rs             Config.halo_timeout、Command::SetHaloTimeout
apps/halokey/src/main.rs              AppSettings.halo、set_settings(patch)、HaloShow 按设置调窗口
docs/03-config-format.md (+zh)        §4 光环：补"编辑器只写有效选项"一句
```

---

### Task 1: 草稿 reducer — 光环的增删改与引用检查

**Files:**
- Modify: `ui/src/model/draft.ts`
- Test: `ui/src/model/__tests__/draft.test.ts`

**Interfaces:**
- Produces:
  - `newHaloId(map: HostKeymap): string`（`halo-1`、`halo-2`… 取第一个未用的）
  - `upsertHalo(d: Draft, halo: Halo): Draft`（按 `id` 替换或追加；标记 dirty）
  - `haloReferences(map: HostKeymap, id: string): string[]`（中文描述列表：`"顶部按键 › 长按"`、`"Layer 1 · 中间按键 › 单击"`、`"光环「主菜单」槽位 7"`）
  - `deleteHalo(d: Draft, id: string): { draft: Draft; blockedBy: string[] }`（有引用时不改草稿）
  - `haloActionAllowed(action: HostAction, editingId: string | null): boolean`（禁 `library`、`momentaryLayer`、以及 `halo` 指向自己）

- [ ] **Step 1: 写失败的测试（追加到 `draft.test.ts`）**

```ts
import { newHaloId, upsertHalo, deleteHalo, haloReferences, haloActionAllowed } from "../draft";
// …放进文件顶部的 import 列表

describe("draft halos", () => {
  const loadHalo = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/halo-menu.json"), "utf8"));
  it("allocates ids and upserts", () => {
    let d = loadSaved(initialDraft(), loadHalo());
    expect(newHaloId(d.map!)).toBe("halo-1");
    d = upsertHalo(d, { id: "halo-1", name: "新建", options: [{ label: "A", action: { keyboard: { _0: [{ type: 2, code: 4 }] } } }] });
    expect(d.dirty).toBe(true);
    expect(d.map!.halos!.map((h) => h.id)).toEqual(["main", "more", "halo-1"]);
    expect(newHaloId(d.map!)).toBe("halo-2");
    d = upsertHalo(d, { id: "main", name: "改名", options: d.map!.halos![0]!.options });
    expect(d.map!.halos![0]!.name).toBe("改名");
    expect(d.map!.halos).toHaveLength(3);
  });
  it("lists references and refuses to delete a referenced halo", () => {
    const m = loadHalo();
    expect(haloReferences(m, "main")).toEqual(["旋钮按下 › 单击"]);
    expect(haloReferences(m, "more")).toEqual(["光环「主菜单」槽位 7"]);
    let d = loadSaved(initialDraft(), m);
    const r = deleteHalo(d, "more");
    expect(r.blockedBy).toEqual(["光环「主菜单」槽位 7"]);
    expect(r.draft.map!.halos).toHaveLength(2);
    expect(r.draft.dirty).toBe(false);
    d = upsertHalo(d, { id: "halo-1", name: "孤儿", options: [{ label: "x", action: { keyboard: { _0: [{ type: 2, code: 4 }] } } }] });
    const r2 = deleteHalo(d, "halo-1");
    expect(r2.blockedBy).toEqual([]);
    expect(r2.draft.map!.halos!.map((h) => h.id)).toEqual(["main", "more"]);
  });
  it("finds references inside layers", () => {
    let d = loadSaved(initialDraft(), loadHalo());
    d = assignAction(select(d, { layer: 2, index: 1, gesture: "long" }), { halo: { _0: "more" } });
    expect(haloReferences(d.map!, "more")).toEqual(["光环「主菜单」槽位 7", "Layer 2 · 中间按键 › 长按"]);
  });
  it("blocks self-reference, library and layer actions in halo options", () => {
    expect(haloActionAllowed({ halo: { _0: "x" } }, "x")).toBe(false);
    expect(haloActionAllowed({ halo: { _0: "y" } }, "x")).toBe(true);
    expect(haloActionAllowed({ library: { _0: "a" } }, null)).toBe(false);
    expect(haloActionAllowed({ momentaryLayer: { _0: 1 } }, null)).toBe(false);
    expect(haloActionAllowed({ pointerMove: { _0: { dx: 1, dy: 0 } } }, null)).toBe(true);
  });
});
```

- [ ] **Step 2: 运行确认失败** — `cd ui && npx vitest run src/model/__tests__/draft.test.ts` → 新用例失败（函数不存在）。

- [ ] **Step 3: 实现（追加到 `draft.ts`）**

```ts
import type { Halo } from "./keymap";
import { CONTROL_NAMES } from "./labels";

// ---- 光环 ----

export function newHaloId(map: HostKeymap): string {
  const used = new Set((map.halos ?? []).map((h) => h.id));
  for (let i = 1; ; i++) if (!used.has(`halo-${i}`)) return `halo-${i}`;
}

export function upsertHalo(d: Draft, halo: Halo): Draft {
  if (!d.map) return d;
  const map = clone(d.map);
  map.halos ??= [];
  const i = map.halos.findIndex((h) => h.id === halo.id);
  if (i >= 0) map.halos[i] = clone(halo); else map.halos.push(clone(halo));
  return { ...d, map, dirty: true };
}

const GESTURE_NAME = { press: "单击", double: "双击", long: "长按" } as const;

/** 谁在用这个光环：基础层 / 各层的手势动作，以及其它光环的槽位。 */
export function haloReferences(map: HostKeymap, id: string): string[] {
  const refs: string[] = [];
  const isRef = (a: HostAction | null | undefined) => !!a && "halo" in a && (a as { halo: { _0: string } }).halo._0 === id;
  const scan = (controls: ControlActionMap[], prefix: string) => {
    for (const c of controls) {
      const name = CONTROL_NAMES[c.index] ?? `控件 ${c.index}`;
      if (isRef(c.pressAction)) refs.push(`${prefix}${name} › ${GESTURE_NAME.press}`);
      if (isRef(c.doublePressAction)) refs.push(`${prefix}${name} › ${GESTURE_NAME.double}`);
      if (isRef(c.longPressAction)) refs.push(`${prefix}${name} › ${GESTURE_NAME.long}`);
    }
  };
  scan(map.controls, "");
  for (const h of map.halos ?? []) h.options.forEach((o, i) => { if (isRef(o.action)) refs.push(`光环「${h.name}」槽位 ${i + 1}`); });
  for (const l of map.layers ?? []) scan(l.controls, `Layer ${l.id} · `);
  return refs;
}

export function deleteHalo(d: Draft, id: string): { draft: Draft; blockedBy: string[] } {
  if (!d.map) return { draft: d, blockedBy: [] };
  const blockedBy = haloReferences(d.map, id);
  if (blockedBy.length) return { draft: d, blockedBy };
  const map = clone(d.map);
  map.halos = (map.halos ?? []).filter((h) => h.id !== id);
  if (!map.halos.length) delete map.halos;
  return { draft: { ...d, map, dirty: true }, blockedBy: [] };
}

/** 光环选项允许的动作：不能引用功能库、不能切层、不能套自己（与 Rust Halo::validate 一致）。 */
export function haloActionAllowed(action: HostAction, editingId: string | null): boolean {
  if ("library" in action || "momentaryLayer" in action) return false;
  if ("halo" in action && (action as { halo: { _0: string } }).halo._0 === editingId) return false;
  return true;
}
```

`haloReferences` 的顺序：基础层 → 其它光环 → 各层（测试按此顺序断言）。

- [ ] **Step 4: 运行测试** → `npx vitest run` 全绿，`npm run check` 0 错。
- [ ] **Step 5: 提交** `git add ui && git commit -m "feat(ui): halo upsert, delete with reference check, and option-action rules"`（带 Co-Authored-By）。

---

### Task 2: 编辑器槽位模型 + 扇形几何（纯函数）

**Files:**
- Create: `ui/src/model/halo.ts`, `ui/src/lib/wedge.ts`
- Test: `ui/src/model/__tests__/halo.test.ts`, `ui/src/lib/__tests__/wedge.test.ts`

**Interfaces:**
- `halo.ts`：
  - `type Slot = { label: string; action: HostAction | null }`
  - `slotsFrom(halo: Halo | null): Slot[]`（固定 8 个；`null` = 全空）
  - `haloFrom(id: string, name: string, slots: Slot[]): Halo`（只收 `action` 非空的槽位，保持顺序）
  - `haloProblems(name: string, slots: Slot[]): string[]`（中文：名称为空 / 超过 80 字、没有任何选项、第 N 槽有动作但没名字 / 名字超过 80 字）
  - `moveSlot(slots, from, delta: -1 | 1): Slot[]`、`clearSlot(slots, i): Slot[]`
- `wedge.ts`：`wedgePath(n: number, i: number, outer: number, inner: number, cx = 0, cy = 0): string`（SVG path；n=1 画整环用两段弧）、`wedgeCenter(n, i, r, cx = 0, cy = 0): { x: number; y: number }`（标签位置，半径 r）。

- [ ] **Step 1: 写失败的测试**

`ui/src/model/__tests__/halo.test.ts`：

```ts
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
```

`ui/src/lib/__tests__/wedge.test.ts`：

```ts
import { describe, it, expect } from "vitest";
import { wedgePath, wedgeCenter } from "../wedge";

describe("wedge geometry", () => {
  it("places the first wedge centred on 12 o'clock and goes clockwise", () => {
    const c0 = wedgeCenter(8, 0, 100);
    expect(Math.abs(c0.x)).toBeLessThan(1e-9);
    expect(c0.y).toBeCloseTo(-100);
    const c2 = wedgeCenter(8, 2, 100);
    expect(c2.x).toBeCloseTo(100);
    expect(Math.abs(c2.y)).toBeLessThan(1e-9);
  });
  it("draws a full ring for a single option and valid arcs for two", () => {
    const one = wedgePath(1, 0, 100, 60);
    expect(one.startsWith("M")).toBe(true);
    expect((one.match(/A/g) ?? []).length).toBe(4);
    const two = wedgePath(2, 1, 100, 60);
    expect(two).toContain("A 100 100 0 0 1");
    expect(two).toContain("A 60 60 0 0 0");
    expect(two.endsWith("Z")).toBe(true);
  });
});
```

- [ ] **Step 2: 运行确认失败** → 两个文件都找不到模块。

- [ ] **Step 3: 实现**

`ui/src/model/halo.ts`：

```ts
import type { Halo, HostAction } from "./keymap";

export type Slot = { label: string; action: HostAction | null };
export const SLOT_COUNT = 8;
const empty = (): Slot => ({ label: "", action: null });

export function slotsFrom(halo: Halo | null): Slot[] {
  const slots = Array.from({ length: SLOT_COUNT }, empty);
  halo?.options.slice(0, SLOT_COUNT).forEach((o, i) => { slots[i] = { label: o.label, action: JSON.parse(JSON.stringify(o.action)) }; });
  return slots;
}

export function haloFrom(id: string, name: string, slots: Slot[]): Halo {
  return { id, name: name.trim(), options: slots.filter((s) => s.action).map((s) => ({ label: s.label.trim(), action: s.action! })) };
}

export function haloProblems(name: string, slots: Slot[]): string[] {
  const problems: string[] = [];
  const n = name.trim();
  if (!n) problems.push("名称不能为空");
  else if ([...n].length > 80) problems.push("名称不能超过 80 字");
  if (!slots.some((s) => s.action)) problems.push("至少要有一个选项");
  slots.forEach((s, i) => {
    if (!s.action) return;
    if (!s.label.trim()) problems.push(`槽位 ${i + 1} 有动作但没有名字`);
    else if ([...s.label.trim()].length > 80) problems.push(`槽位 ${i + 1} 的名字不能超过 80 字`);
  });
  return problems;
}

export function moveSlot(slots: Slot[], from: number, delta: -1 | 1): Slot[] {
  const to = from + delta;
  if (to < 0 || to >= slots.length) return slots;
  const out = slots.map((s) => ({ ...s }));
  [out[from], out[to]] = [out[to]!, out[from]!];
  return out;
}

export function clearSlot(slots: Slot[], i: number): Slot[] {
  const out = slots.map((s) => ({ ...s }));
  out[i] = empty();
  return out;
}
```

`ui/src/lib/wedge.ts`：

```ts
// 扇形环几何：N 个等分扇形，第 0 个以 12 点钟为中心，顺时针。SVG 坐标 y 向下。
const polar = (r: number, angle: number, cx: number, cy: number) => ({ x: cx + r * Math.sin(angle), y: cy - r * Math.cos(angle) });
const fmt = (v: number) => (Math.abs(v) < 1e-9 ? 0 : Math.round(v * 1000) / 1000);

export function wedgeCenter(n: number, i: number, r: number, cx = 0, cy = 0) {
  const a = (i * 2 * Math.PI) / Math.max(n, 1);
  const p = polar(r, a, cx, cy);
  return { x: fmt(p.x), y: fmt(p.y) };
}

export function wedgePath(n: number, i: number, outer: number, inner: number, cx = 0, cy = 0): string {
  if (n <= 1) {
    // 整环：两段半圆拼起来（外圈顺时针，内圈逆时针）。
    const o1 = polar(outer, 0, cx, cy), o2 = polar(outer, Math.PI, cx, cy);
    const i1 = polar(inner, 0, cx, cy), i2 = polar(inner, Math.PI, cx, cy);
    return `M ${fmt(o1.x)} ${fmt(o1.y)} A ${outer} ${outer} 0 1 1 ${fmt(o2.x)} ${fmt(o2.y)} A ${outer} ${outer} 0 1 1 ${fmt(o1.x)} ${fmt(o1.y)} ` +
      `M ${fmt(i1.x)} ${fmt(i1.y)} A ${inner} ${inner} 0 1 0 ${fmt(i2.x)} ${fmt(i2.y)} A ${inner} ${inner} 0 1 0 ${fmt(i1.x)} ${fmt(i1.y)} Z`;
  }
  const step = (2 * Math.PI) / n;
  const start = i * step - step / 2, end = start + step;
  const large = step > Math.PI ? 1 : 0;
  const o1 = polar(outer, start, cx, cy), o2 = polar(outer, end, cx, cy);
  const i1 = polar(inner, end, cx, cy), i2 = polar(inner, start, cx, cy);
  return `M ${fmt(o1.x)} ${fmt(o1.y)} A ${outer} ${outer} 0 ${large} 1 ${fmt(o2.x)} ${fmt(o2.y)} L ${fmt(i1.x)} ${fmt(i1.y)} A ${inner} ${inner} 0 ${large} 0 ${fmt(i2.x)} ${fmt(i2.y)} Z`;
}
```

- [ ] **Step 4: 运行测试** → 全绿。
- [ ] **Step 5: 提交** `git add ui && git commit -m "feat(ui): halo slot model and wedge geometry"`。

---

### Task 3: 键码面板的"动作模式"与光环分类入口

**Files:**
- Modify: `ui/src/components/Palette.svelte`, `ui/src/pages/Keymap.svelte`, `ui/src/lib/i18n.ts`
- Test: `ui/src/pages/__tests__/palette.test.svelte.ts`（追加）

**Interfaces:**
- `Palette` 新 props（都可选，默认沿用现有手势模式）：
  - `mode?: "gesture" | "action"`（`action` 模式下不读 `draft.selection`，所有选择都通过 `onAction` 回调给出一个 `HostAction`；不显示输出方式分段、不显示 ▽ / ⊠；Layer 分类隐藏）
  - `onAction?: (a: HostAction) => void`
  - `excludeHalo?: string | null`（光环分类里隐藏这个 id——正在编辑的那个）
  - `onEditHalo?: (id: string | null) => void`（光环卡片上的"编辑"按钮与"新建光环"卡片；`null` = 新建）
  - `crumb?: string`（`action` 模式下面包屑文字，如"槽位 3"）
- `Keymap.svelte` 新 prop：`onEditHalo: (id: string | null) => void`，透传给 Palette。
- APP / 宏卡片在 `action` 模式下产出**内联**动作（`application` / `macro`，从 `actionLibrary` 项复制），因为光环选项不能引用功能库。

- [ ] **Step 1: 写失败的测试（追加到 `palette.test.svelte.ts`）**

```ts
import Palette from "../../components/Palette.svelte";
import type { HostAction } from "../../model/keymap";
// …import 列表

describe("Palette action mode", () => {
  it("emits actions instead of writing the draft, hides Layer and the halo being edited", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    const got: HostAction[] = [];
    app = mount(Palette, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, halos: draft.v.map!.halos!, apps: [], macros: [], mode: "action", onAction: (a: HostAction) => got.push(a), excludeHalo: "main", crumb: "槽位 1" } });
    flushSync();
    expect(byText("Layer")).toBeUndefined();
    expect(byText("保持按住")).toBeUndefined();
    expect(document.body.textContent).toContain("槽位 1");
    byText("Enter")!.click();
    flushSync();
    expect(got).toEqual([{ keyboard: { _0: [{ type: 2, code: 0x28 }] } }]);
    expect(draft.v.dirty).toBe(false);
    byText("光环")!.click();
    flushSync();
    expect(document.body.textContent).not.toContain("主菜单");
    expect(document.body.textContent).toContain("更多");
    byText("指针")!.click();
    flushSync();
    byText("↑")!.click();
    flushSync();
    expect(got[1]).toEqual({ pointerMove: { _0: { dx: 0, dy: -20 } } });
  });
  it("offers new / edit halo entries in gesture mode", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    const edits: (string | null)[] = [];
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null, onEditHalo: (id: string | null) => edits.push(id) } });
    flushSync();
    byText("光环")!.click();
    flushSync();
    byText("新建光环")!.click();
    flushSync();
    expect(edits).toEqual([null]);
    ([...document.querySelectorAll("button[data-edit-halo]")][0] as HTMLButtonElement).click();
    flushSync();
    expect(edits).toEqual([null, "main"]);
  });
});
```

- [ ] **Step 2: 运行确认失败** → 两个用例失败。

- [ ] **Step 3: 实现**

`Palette.svelte` 的 `<script>`：

```ts
  let { draft = $bindable(), halos, apps, macros, mode = "gesture", onAction, excludeHalo = null, onEditHalo, crumb }: {
    draft: Draft; halos: Halo[]; apps: NamedHostAction[]; macros: NamedHostAction[];
    mode?: "gesture" | "action"; onAction?: (a: HostAction) => void; excludeHalo?: string | null; onEditHalo?: (id: string | null) => void; crumb?: string;
  } = $props();
  const actionMode = $derived(mode === "action");
  /** 手势模式写草稿；动作模式只回调。 */
  const emitEntries = (entries: KeyEntry[]) => { if (actionMode) onAction?.({ keyboard: { _0: entries } }); else draft = assignEntries(draft, entries); };
  const emitAction = (a: HostAction) => { if (actionMode) onAction?.(a); else draft = assignAction(draft, a); };
  const visibleHalos = $derived(halos.filter((h) => h.id !== excludeHalo));
  const layerAllowed = $derived(!actionMode && !rotation && sel.gesture !== "double");
```

模板改动：
- 分类按钮：`{#if c.id === "layer" && actionMode}{:else}<button …>{/if}`（动作模式不渲染 Layer）。
- 面包屑：`{actionMode ? crumb ?? "" : `${t(CONTROL_NAMES[sel.index] ?? "")} › ${t(gestureName[sel.gesture] ?? "")}`}`；输出方式分段只在 `!actionMode` 时渲染。
- `KeyGrid`：`showTransparent={!actionMode && sel.layer !== 0}`，`onPick={(e) => emitEntries(e)}`，`onClear` 在动作模式下 `onAction?.({ keyboard: { _0: [{ type: 2, code: 0 }] } })`，`onTransparent` 不变。
- 指针：`pointer()` 改为 `emitAction({ pointerMove: … })`。
- 光环分类：

```svelte
      <div class="cards">
        {#each visibleHalos as h, i}
          <div class="item halo">
            <button class="pick" onclick={() => emitAction({ halo: { _0: h.id } })}><small>◎ H{i}</small>{h.name}</button>
            {#if onEditHalo}<button class="edit" data-edit-halo={h.id} title={t("编辑光环")} onclick={() => onEditHalo?.(h.id)}>✎</button>{/if}
          </div>
        {/each}
        {#if onEditHalo}<button class="item new" onclick={() => onEditHalo?.(null)}>＋ {t("新建光环")}</button>{/if}
      </div>
```

- APP / 宏分类：动作模式下 `emitAction(item.action)`（内联复制），手势模式下仍 `assignAction(draft, { library: { _0: item.id } })`：

```svelte
        {#each (category === "app" ? apps : macros) as item}<button class="item" onclick={() => (actionMode ? emitAction(item.action) : (draft = assignAction(draft, { library: { _0: item.id } })))}>…</button>{/each}
```

样式加：`.item.halo { display: flex; align-items: stretch; width: 150px; } .item.halo .pick { flex: 1; display: flex; flex-direction: column; align-items: center; gap: 2px; border-radius: 8px 0 0 8px; } .item.halo .edit { width: 32px; border-radius: 0 8px 8px 0; background: var(--card-on); } .item.new { border: 1px dashed var(--muted); background: transparent; color: var(--muted); }`。

`Keymap.svelte`：props 加 `onEditHalo?: (id: string | null) => void`，`<Palette … {onEditHalo} />`。

`i18n.ts` 补：`"编辑光环": "Edit halo"`, `"新建光环": "New halo"`。

- [ ] **Step 4: 运行** `npm test && npm run check` 全绿（原有手势模式测试不变）。
- [ ] **Step 5: 提交** `git add ui && git commit -m "feat(ui): palette action mode and halo new/edit entries"`。

---

### Task 4: 转盘悬浮窗换成扇形环

**Files:**
- Create: `ui/src/components/HaloWheel.svelte`
- Modify: `ui/src/halo.ts`, `ui/src/lib/tauri.ts`（`haloShow` 事件带 `size`；mock 推假事件）
- Delete: `ui/src/components/HaloBubbles.svelte`
- Test: `ui/src/components/__tests__/wheel.test.svelte.ts`

**Interfaces:**
- `UiEvent` 的 `haloShow` 变为 `{ kind: "haloShow"; halo: Halo; selected: number; size: number }`（`size` = 外径像素，Rust 按设置给 280 / 360 / 440；Task 5 实现，本任务前端先兼容缺省：`size ?? 360`）。
- `HaloWheel` props：`{ halo: Halo | null; selected: number; size: number }`；根元素 `.wheel`，每个扇形 `path.wedge[data-index]`，高亮加 `class="sel"`；中心 `.center` 显示高亮项名称；`.hint` 提示。窗口由 Rust 设为 `size × (size + 28)`（底部提示一行）。

- [ ] **Step 1: 写失败的测试**

```ts
import { describe, it, expect, afterEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import HaloWheel from "../HaloWheel.svelte";
import { setLang } from "../../lib/i18n";

let app: ReturnType<typeof mount> | undefined;
afterEach(() => { if (app) unmount(app); app = undefined; document.body.innerHTML = ""; });
const kb = (c: number) => ({ keyboard: { _0: [{ type: 2, code: c }] } });

describe("halo wheel", () => {
  it("draws one wedge per option, highlights the selection and names it in the centre", () => {
    setLang("zh");
    const halo = { id: "m", name: "主菜单", options: [{ label: "甲", action: kb(4) }, { label: "乙", action: kb(5) }, { label: "丙", action: kb(6) }] };
    app = mount(HaloWheel, { target: document.body, props: { halo, selected: 1, size: 360 } });
    flushSync();
    expect(document.querySelectorAll("path.wedge")).toHaveLength(3);
    expect(document.querySelector("path.wedge.sel")!.getAttribute("data-index")).toBe("1");
    expect(document.querySelector(".center")!.textContent).toContain("乙");
    expect(document.body.textContent).toContain("拧旋钮选择");
    const svg = document.querySelector("svg")!;
    expect(svg.getAttribute("width")).toBe("360");
  });
  it("renders nothing without a halo", () => {
    app = mount(HaloWheel, { target: document.body, props: { halo: null, selected: 0, size: 280 } });
    flushSync();
    expect(document.querySelectorAll("path.wedge")).toHaveLength(0);
  });
});
```

- [ ] **Step 2: 运行确认失败** → 找不到模块。

- [ ] **Step 3: 实现 `HaloWheel.svelte`**

```svelte
<script lang="ts">
  import { t } from "../lib/i18n";
  import { wedgePath, wedgeCenter } from "../lib/wedge";
  import type { Halo } from "../model/keymap";
  let { halo, selected, size }: { halo: Halo | null; selected: number; size: number } = $props();
  const outer = $derived(size / 2 - 4);
  const inner = $derived(outer * 0.6);
  const c = $derived(size / 2);
  const n = $derived(halo?.options.length ?? 0);
  const labelR = $derived((outer + inner) / 2);
  const fontSize = $derived(n > 6 ? 12 : 13);
</script>

{#if halo}
  <div class="wheel" style:width={`${size}px`}>
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
      {#each halo.options as o, i}
        <path class="wedge" class:sel={i === selected} data-index={i} d={wedgePath(n, i, outer, inner, c, c)} />
      {/each}
      {#each halo.options as o, i}
        {@const p = wedgeCenter(n, i, labelR, c, c)}
        <text class="label" class:sel={i === selected} x={p.x} y={p.y} font-size={fontSize} text-anchor="middle" dominant-baseline="middle">{o.label}</text>
      {/each}
    </svg>
    <div class="center" style:top={`${c}px`} style:width={`${inner * 1.7}px`} style:height={`${inner * 1.7}px`}>{halo.options[selected]?.label ?? halo.name}</div>
    <div class="hint">{t("拧旋钮选择 · 按下确认 · 按任意键取消")}</div>
  </div>
{/if}

<style>
  :global(html, body) { margin: 0; background: transparent; overflow: hidden; }
  .wheel { position: relative; font: 13px/1.3 system-ui, sans-serif; color: #e6e9f0; }
  .wedge { fill: rgba(27, 31, 42, .9); stroke: rgba(230, 233, 240, .18); stroke-width: 1.5; transition: fill 90ms; }
  .wedge.sel { fill: #f2c9b5; }
  .label { fill: #e6e9f0; pointer-events: none; }
  .label.sel { fill: #0b1220; font-weight: 700; }
  .center { position: absolute; left: 50%; transform: translate(-50%, -50%); border-radius: 50%; background: rgba(17, 20, 28, .94); border: 2px solid rgba(242, 201, 181, .6); display: flex; align-items: center; justify-content: center; text-align: center; font-weight: 600; padding: 8px; box-sizing: border-box; }
  .hint { text-align: center; color: rgba(230, 233, 240, .6); font-size: 11px; height: 28px; line-height: 28px; }
</style>
```

（`.center` 的 `top` 用 `style:top={c}px` 以 SVG 为基准，因为 `.wheel` 还包含 28 px 的提示行。）

`ui/src/halo.ts`：

```ts
import { mount } from "svelte";
import HaloWheel from "./components/HaloWheel.svelte";
import { subscribe, type UiEvent } from "./lib/tauri";
import type { Halo } from "./model/keymap";

const state = $state({ halo: null as Halo | null, selected: 0, size: 360 });
mount(HaloWheel, { target: document.body, props: state });
subscribe((e: UiEvent) => {
  if (e.kind === "haloShow") { state.halo = e.halo; state.selected = e.selected; state.size = e.size ?? 360; }
  else if (e.kind === "haloSelect") state.selected = e.selected;
  else if (e.kind === "haloHide") state.halo = null;
});
```

`.ts` 文件里不能用 `$state`：把 `halo.ts` 改名为 `halo.svelte.ts`（Svelte 5 允许在 `.svelte.ts` 里用 runes），`halo.html` 的 `src="/src/halo.svelte.ts"`，`csp.test.ts` 不受影响。`tauri.ts`：`haloShow` 类型加 `size?: number`；mock `subscribe` 里每 6 秒推一次 `haloShow`（用 `example.halos[0]`，`size: 360`）、3 秒后 `haloHide`，让 `halo.html` 在浏览器里能看。

删除 `HaloBubbles.svelte`。

- [ ] **Step 4: 运行** `npm test && npm run check && npm run build`；浏览器打开 `http://localhost:5177/halo.html` 看扇形环（mock 每 6 秒出现一次）。
- [ ] **Step 5: 提交** `git add -A ui && git commit -m "feat(ui): Surface-Dial style halo wheel"`。

---

### Task 5: Rust — 转盘设置（位置 / 超时 / 大小）与 `set_settings`

**Files:**
- Modify: `apps/halokey/src/main.rs`, `crates/runtime/src/lib.rs`
- Test: `apps/halokey/src/main.rs` 的 `mod tests`（`settings_merge_keeps_unknown_and_defaults`）

**Interfaces:**
- `AppSettings { idle_minutes: u64, halo: HaloSettings }`，`HaloSettings { position: "pointer"|"center", timeout_seconds: u64, size: "small"|"medium"|"large" }`，全部 `#[serde(default)]`，camelCase；`HaloSettings::size_px() -> u32`（280 / 360 / 440）。
- 命令 `set_settings(patch: serde_json::Value) -> Result<AppSettings, String>`：读现有 → JSON 深合并（只覆盖 patch 里出现的键）→ 校验（`timeoutSeconds` 1–60，`idleMinutes` ≤ 1440）→ 写文件 → 把 `idleMinutes` 与 `halo.timeoutSeconds` 下发运行时 → 返回合并后的设置。`set_idle_minutes` 保留（内部改调 `set_settings`）。
- 运行时：`Config.halo_timeout: Duration`，`Command::SetHaloTimeout(Duration)`，`Worker.halo_timeout` 取代常量 `HALO_TIMEOUT`（常量保留作默认值）。
- `UiEvent::HaloShow { halo, selected, size: u32 }`；显示时窗口 `set_size(size, size + 28)`，位置按 `position`。

- [ ] **Step 1: 写失败的测试（追加到 `main.rs` 的 `mod tests`）**

```rust
    #[test]
    fn settings_merge_keeps_unknown_and_defaults() {
        let old: serde_json::Value = serde_json::json!({ "idleMinutes": 10, "future": true });
        let patch = serde_json::json!({ "halo": { "size": "large" } });
        let merged = merge_settings(old, &patch).unwrap();
        assert_eq!(merged.idle_minutes, 10);
        assert_eq!(merged.halo.size, HaloSize::Large);
        assert_eq!(merged.halo.timeout_seconds, 4, "缺省段用默认值");
        assert_eq!(merged.halo.position, HaloPosition::Pointer);
        let json = serde_json::to_value(&merged).unwrap();
        assert_eq!(json["halo"]["size"], "large");
        assert_eq!(json["halo"]["timeoutSeconds"], 4);
        assert!(merge_settings(serde_json::json!({}), &serde_json::json!({ "halo": { "timeoutSeconds": 0 } })).is_err());
        assert_eq!(HaloSettings::default().size_px(), 360);
    }
```

Run: `cargo test -p halokey-app 2>&1 | grep -E "^error|cannot find"` → 编译错误。

- [ ] **Step 2: 实现设置类型与合并**

```rust
#[derive(Serialize, serde::Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
enum HaloPosition {
    Pointer,
    Center,
}
#[derive(Serialize, serde::Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
enum HaloSize {
    Small,
    Medium,
    Large,
}
#[derive(Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
struct HaloSettings {
    position: HaloPosition,
    timeout_seconds: u64,
    size: HaloSize,
}
impl Default for HaloSettings {
    fn default() -> Self {
        HaloSettings { position: HaloPosition::Pointer, timeout_seconds: 4, size: HaloSize::Medium }
    }
}
impl HaloSettings {
    fn size_px(&self) -> u32 {
        match self.size {
            HaloSize::Small => 280,
            HaloSize::Medium => 360,
            HaloSize::Large => 440,
        }
    }
}
#[derive(Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
struct AppSettings {
    idle_minutes: u64,
    halo: HaloSettings,
}
impl Default for AppSettings {
    fn default() -> Self {
        AppSettings { idle_minutes: 5, halo: HaloSettings::default() }
    }
}

/// 深合并：patch 里出现的键覆盖，其余保留（包括本版本不认识的键）。
fn deep_merge(base: &mut serde_json::Value, patch: &serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(b), serde_json::Value::Object(p)) => {
            for (k, v) in p {
                deep_merge(b.entry(k.clone()).or_insert(serde_json::Value::Null), v);
            }
        }
        (b, p) => *b = p.clone(),
    }
}

fn merge_settings(mut current: serde_json::Value, patch: &serde_json::Value) -> Result<AppSettings, String> {
    if !current.is_object() {
        current = serde_json::json!({});
    }
    deep_merge(&mut current, patch);
    let settings: AppSettings = serde_json::from_value(current).map_err(|e| format!("设置格式无效：{e}"))?;
    if !(1..=60).contains(&settings.halo.timeout_seconds) {
        return Err("转盘自动关闭时间应为 1–60 秒".into());
    }
    if settings.idle_minutes > 1440 {
        return Err("空闲时间最多 1440 分钟".into());
    }
    Ok(settings)
}
```

`load_settings` 改成先读原始 `Value`（保留未知键）：新增 `fn load_settings_value(config_path) -> serde_json::Value`（读不到 → `{}`），`load_settings` = `merge_settings(load_settings_value(..), &json!({})).unwrap_or_default()`。`save_settings` 写合并后的 `AppSettings`（未知键会丢——可接受，测试只要求合并阶段保留；在 `save_settings` 前把 `settings` 序列化后再 `deep_merge` 进原始 Value 即可两全：`let mut raw = load_settings_value(..); deep_merge(&mut raw, &serde_json::to_value(settings)?); write(raw)`，就按这个写）。

命令：

```rust
#[tauri::command]
fn set_settings(state: State<'_, App>, patch: serde_json::Value) -> Result<AppSettings, String> {
    let settings = merge_settings(load_settings_value(&state.config_path), &patch)?;
    save_settings(&state.config_path, &settings)?;
    let guard = state.service.lock().map_err(|e| e.to_string())?;
    let service = guard.as_ref().ok_or("服务未运行")?;
    let idle = (settings.idle_minutes > 0).then(|| std::time::Duration::from_secs(settings.idle_minutes * 60));
    if !service.send(Command::SetIdleTimeout(idle))
        || !service.send(Command::SetHaloTimeout(std::time::Duration::from_secs(settings.halo.timeout_seconds)))
    {
        return Err("设备线程已退出".into());
    }
    Ok(settings)
}
```

`set_idle_minutes` 改为 `set_settings(state, json!({ "idleMinutes": minutes })).map(|_| ())`。注册 `set_settings` 到 `generate_handler!`。启动时 `Config { halo_timeout: Duration::from_secs(settings.halo.timeout_seconds), .. }`。

- [ ] **Step 3: 运行时超时可配置**

`crates/runtime/src/lib.rs`：`Config` 加 `pub halo_timeout: Duration`（CLI 的 `Config { … }` 构造处补 `halo_timeout: HALO_TIMEOUT`，`grep -rn "Config {" crates apps` 找齐）；`Command` 加 `SetHaloTimeout(Duration)`；`Worker` 加 `halo_timeout: Duration`（`new()` 里从 config 取）；两处命令循环里加 `Ok(Command::SetHaloTimeout(t)) => self.halo_timeout = t,`（等待循环与连接循环，照 `SetIdleTimeout` 的位置）；超时判断 `h.last_input.elapsed() > HALO_TIMEOUT` 改为 `> self.halo_timeout`。

- [ ] **Step 4: 显示时按设置调窗口**

`forward()` 的 `Event::HaloShow` 分支：

```rust
        Event::HaloShow(halo, selected) => {
            let settings = load_settings(&app.state::<App>().config_path);
            let size = settings.halo.size_px();
            if let Some(w) = app.get_webview_window("halo") {
                let _ = w.set_size(tauri::PhysicalSize::new(size, size + 28));
                let placed = settings.halo.position == HaloPosition::Pointer
                    && match app.cursor_position() {
                        Ok(pos) => w
                            .set_position(tauri::PhysicalPosition::new(
                                (pos.x - f64::from(size) / 2.0).max(0.0),
                                (pos.y - f64::from(size) / 2.0).max(0.0),
                            ))
                            .is_ok(),
                        Err(_) => false,
                    };
                if !placed {
                    let _ = w.center();
                }
                let _ = w.show();
            }
            UiEvent::HaloShow { halo, selected, size }
        }
```

`UiEvent::HaloShow` 加字段 `size: u32`。`tauri.conf.json` 的 halo 窗口 `width/height` 改为 360 / 388（默认中号）。

- [ ] **Step 5: 验证** `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`；`cargo build --release -p halokey-app` 后重启，按旋钮（配置里旋钮按下 = 光环）应出现 360 px 的扇形环。
- [ ] **Step 6: 提交** `git add -A && git commit -m "feat(app,runtime): halo wheel settings (position, timeout, size) via set_settings"`。

---

### Task 6: 光环编辑器页面 + 路由

**Files:**
- Create: `ui/src/pages/HaloEditor.svelte`, `ui/src/components/RingPreview.svelte`
- Modify: `ui/src/App.svelte`, `ui/src/pages/Keymap.svelte`（透传 `onEditHalo`，已在 Task 3）, `ui/src/lib/i18n.ts`
- Test: `ui/src/pages/__tests__/halo-editor.test.svelte.ts`

**Interfaces:**
- `App` 页面状态：`page: "keymap" | "settings" | "halo"`，`editing: { id: string | null }`；进入编辑器时记下 `returnTo = draft.selection`。
- `HaloEditor` props：`{ draft: Draft (bindable); haloId: string | null; onClose: (savedId: string | null) => void }`。保存：`haloProblems` 为空 → `upsertHalo` → `onClose(id)`；取消 → `onClose(null)`；删除（仅编辑已有）→ `deleteHalo`，有引用则显示"被 … 引用，无法删除"。
- `App` 收到 `onClose(savedId)`：如果是**新建**且 `savedId` 非空，就 `assignAction(draft, { halo: { _0: savedId } })` 到 `returnTo` 的手势（先 `select` 回去）；然后 `page = "keymap"`。
- `RingPreview` props：`{ slots: Slot[]; selected: number; onPick: (i: number) => void }`：8 个扇形（`path.wedge[data-index]`），有动作的槽显示名称 + 动作简名，空槽显示 ＋；选中加 `sel`。

- [ ] **Step 1: 写失败的测试**

```ts
import { describe, it, expect, afterEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import HaloEditor from "../HaloEditor.svelte";
import { initialDraft, loadSaved, type Draft } from "../../model/draft";
import type { HostKeymap } from "../../model/keymap";
import { setLang } from "../../lib/i18n";

const load = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/halo-menu.json"), "utf8"));
let app: ReturnType<typeof mount> | undefined;
afterEach(() => { if (app) unmount(app); app = undefined; document.body.innerHTML = ""; });
const buttons = () => [...document.querySelectorAll("button")] as HTMLButtonElement[];
const byText = (t: string) => buttons().find((b) => b.textContent?.trim() === t);
const setInput = (el: HTMLInputElement, v: string) => { el.value = v; el.dispatchEvent(new Event("input", { bubbles: true })); };

describe("halo editor", () => {
  it("creates a halo from the palette and reports problems before saving", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    const closed: (string | null)[] = [];
    app = mount(HaloEditor, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, haloId: null, onClose: (id: string | null) => closed.push(id) } });
    flushSync();
    expect(document.querySelectorAll("path.wedge")).toHaveLength(8);
    byText("保存光环")!.click();
    flushSync();
    expect(document.body.textContent).toContain("名称不能为空");
    expect(document.body.textContent).toContain("至少要有一个选项");
    expect(closed).toEqual([]);
    setInput(document.querySelector("input[data-role=halo-name]") as HTMLInputElement, "工具");
    (document.querySelectorAll("path.wedge")[2] as SVGElement).dispatchEvent(new MouseEvent("click", { bubbles: true }));
    flushSync();
    expect(document.body.textContent).toContain("槽位 3");
    byText("Enter")!.click();
    flushSync();
    setInput(document.querySelectorAll("input[data-role=slot-label]")[2] as HTMLInputElement, "回车");
    byText("保存光环")!.click();
    flushSync();
    expect(closed).toEqual(["halo-1"]);
    const h = draft.v.map!.halos!.find((x) => x.id === "halo-1")!;
    expect(h.name).toBe("工具");
    expect(h.options).toEqual([{ label: "回车", action: { keyboard: { _0: [{ type: 2, code: 0x28 }] } } }]);
  });
  it("edits an existing halo, moves slots, and refuses to delete a referenced one", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    const closed: (string | null)[] = [];
    app = mount(HaloEditor, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, haloId: "main", onClose: (id: string | null) => closed.push(id) } });
    flushSync();
    expect((document.querySelector("input[data-role=halo-name]") as HTMLInputElement).value).toBe("主菜单");
    ([...document.querySelectorAll("button[data-role=slot-down]")][0] as HTMLButtonElement).click();
    flushSync();
    byText("保存光环")!.click();
    flushSync();
    expect(draft.v.map!.halos![0]!.options[1]!.label).toBe("文本编辑器");
    expect(closed).toEqual(["main"]);
  });
  it("blocks deleting a referenced halo", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    app = mount(HaloEditor, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, haloId: "more", onClose: () => {} } });
    flushSync();
    byText("删除光环")!.click();
    flushSync();
    expect(document.body.textContent).toContain("光环「主菜单」槽位 7");
    expect(draft.v.map!.halos).toHaveLength(2);
  });
});
```

- [ ] **Step 2: 运行确认失败** → 找不到模块。

- [ ] **Step 3: `RingPreview.svelte`**

```svelte
<script lang="ts">
  import { wedgePath, wedgeCenter } from "../lib/wedge";
  import { actionLabel } from "../model/labels";
  import type { Slot } from "../model/halo";
  import type { Halo } from "../model/keymap";
  let { slots, selected, onPick, halos = [] }: { slots: Slot[]; selected: number; onPick: (i: number) => void; halos?: Halo[] } = $props();
  const SIZE = 400, C = 200, OUTER = 190, INNER = 110;
</script>

<svg class="ring" width={SIZE} height={SIZE} viewBox={`0 0 ${SIZE} ${SIZE}`}>
  {#each slots as s, i}
    <path class="wedge" class:sel={i === selected} class:filled={!!s.action} data-index={i} d={wedgePath(slots.length, i, OUTER, INNER, C, C)} role="button" tabindex="0" onclick={() => onPick(i)} onkeydown={(e) => e.key === "Enter" && onPick(i)} />
  {/each}
  {#each slots as s, i}
    {@const p = wedgeCenter(slots.length, i, (OUTER + INNER) / 2, C, C)}
    <text class="name" x={p.x} y={p.y - 7} text-anchor="middle" font-size="12">{s.action ? s.label || "（未命名）" : "＋"}</text>
    {#if s.action}<text class="act" x={p.x} y={p.y + 9} text-anchor="middle" font-size="10">{actionLabel(s.action, null, halos)}</text>{/if}
  {/each}
  <circle cx={C} cy={C} r="58" class="hub" />
  <text x={C} y={C + 4} text-anchor="middle" font-size="13" class="hubtext">{selected + 1} / 8</text>
</svg>

<style>
  .wedge { fill: var(--card); stroke: var(--bg); stroke-width: 2; cursor: pointer; }
  .wedge.filled { fill: var(--card-on); }
  .wedge.sel { fill: var(--accent); }
  .name { fill: var(--text); pointer-events: none; }
  .act { fill: var(--muted); pointer-events: none; }
  .hub { fill: var(--bg); stroke: var(--line); stroke-width: 2; }
  .hubtext { fill: var(--muted); }
</style>
```

- [ ] **Step 4: `HaloEditor.svelte`**

```svelte
<script lang="ts">
  import { t } from "../lib/i18n";
  import RingPreview from "../components/RingPreview.svelte";
  import Palette from "../components/Palette.svelte";
  import { actionLabel } from "../model/labels";
  import { slotsFrom, haloFrom, haloProblems, moveSlot, clearSlot, type Slot } from "../model/halo";
  import { newHaloId, upsertHalo, deleteHalo, haloActionAllowed, type Draft } from "../model/draft";
  import type { HostAction } from "../model/keymap";

  let { draft = $bindable(), haloId, onClose }: { draft: Draft; haloId: string | null; onClose: (savedId: string | null) => void } = $props();
  const existing = $derived(haloId ? (draft.map?.halos ?? []).find((h) => h.id === haloId) ?? null : null);
  const id = haloId ?? newHaloId(draft.map!);
  let name = $state(existing?.name ?? "");
  let slots = $state<Slot[]>(slotsFrom(existing));
  let selected = $state(0);
  let problems = $state<string[]>([]);
  let blocked = $state<string[]>([]);
  const halos = $derived(draft.map?.halos ?? []);

  function pickAction(a: HostAction) {
    if (!haloActionAllowed(a, id)) { problems = [t("光环选项不能引用功能库、切层或自己")]; return; }
    const s = slots.map((x) => ({ ...x }));
    s[selected] = { label: s[selected]!.label, action: a };
    slots = s; problems = [];
  }
  function save() {
    problems = haloProblems(name, slots);
    if (problems.length) return;
    draft = upsertHalo(draft, haloFrom(id, name, slots));
    onClose(id);
  }
  function remove() {
    const r = deleteHalo(draft, id);
    if (r.blockedBy.length) { blocked = r.blockedBy; return; }
    draft = r.draft; onClose(null);
  }
</script>

<div class="page">
  <header>
    <button class="back" onclick={() => onClose(null)} title={t("返回")}>←</button>
    <h2>{existing ? t("编辑光环") : t("新建光环")}</h2>
    <input data-role="halo-name" placeholder={t("光环名称")} bind:value={name} />
    <span class="id mono">{id}</span>
  </header>
  <section class="body">
    <RingPreview {slots} {selected} {halos} onPick={(i) => (selected = i)} />
    <div class="slots">
      {#each slots as s, i}
        <div class="slot" class:on={i === selected} role="button" tabindex="0" onclick={() => (selected = i)} onkeydown={(e) => e.key === "Enter" && (selected = i)}>
          <span class="n mono">{i + 1}</span>
          <input data-role="slot-label" placeholder={t("名称")} value={s.label} oninput={(e) => { const v = (e.target as HTMLInputElement).value; const c = slots.map((x) => ({ ...x })); c[i] = { ...c[i]!, label: v }; slots = c; }} />
          <span class="act">{s.action ? actionLabel(s.action, null, halos) : t("（空）")}</span>
          <span class="ops">
            <button data-role="slot-up" disabled={i === 0} onclick={(e) => { e.stopPropagation(); slots = moveSlot(slots, i, -1); selected = Math.max(0, i - 1); }}>↑</button>
            <button data-role="slot-down" disabled={i === slots.length - 1} onclick={(e) => { e.stopPropagation(); slots = moveSlot(slots, i, 1); selected = Math.min(slots.length - 1, i + 1); }}>↓</button>
            <button data-role="slot-clear" disabled={!s.action && !s.label} onclick={(e) => { e.stopPropagation(); slots = clearSlot(slots, i); }}>✕</button>
          </span>
        </div>
      {/each}
    </div>
  </section>
  <Palette bind:draft {halos} apps={(draft.map?.actionLibrary ?? []).filter((a) => "application" in a.action)} macros={(draft.map?.actionLibrary ?? []).filter((a) => "macro" in a.action)} mode="action" onAction={pickAction} excludeHalo={id} crumb={`${t("槽位")} ${selected + 1}`} />
  <footer>
    {#if problems.length}<span class="bad">{problems.join("；")}</span>{/if}
    {#if blocked.length}<span class="bad">{t("被以下位置引用，无法删除")}：{blocked.join("、")}</span>{/if}
    {#if existing}<button onclick={remove}>{t("删除光环")}</button>{/if}
    <button onclick={() => onClose(null)}>{t("取消")}</button>
    <button class="primary" onclick={save}>{t("保存光环")}</button>
  </footer>
</div>

<style>
  .page { display: flex; flex-direction: column; min-height: 100%; }
  header { display: flex; align-items: center; gap: 14px; padding: 12px 22px; border-bottom: 1px solid var(--line); }
  header h2 { margin: 0; font-size: 16px; }
  header input { flex: 0 0 260px; }
  .back { background: none; font-size: 18px; padding: 2px 8px; }
  .id { color: var(--muted); font-size: 12px; }
  .mono { font-family: var(--mono); }
  .body { display: grid; grid-template-columns: 420px 1fr; gap: 24px; padding: 16px 22px; align-items: center; }
  .slots { display: flex; flex-direction: column; gap: 6px; }
  .slot { display: grid; grid-template-columns: 24px 1fr 1.2fr auto; gap: 10px; align-items: center; background: var(--card); border-radius: 10px; padding: 6px 12px; border: 1px solid transparent; cursor: pointer; }
  .slot.on { border-color: var(--accent); }
  .slot .n { color: var(--muted); }
  .slot .act { color: var(--accent); font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .slot .ops { display: flex; gap: 4px; } .slot .ops button { padding: 2px 8px; }
  footer { display: flex; justify-content: flex-end; align-items: center; gap: 10px; padding: 12px 22px; border-top: 1px solid var(--line); position: sticky; bottom: 0; background: var(--bg); }
  .bad { color: var(--bad); margin-right: auto; font-size: 13px; }
</style>
```

`i18n.ts` 补：`"光环名称": "Halo name"`, `"返回": "Back"`, `"名称": "Name"`, `"（空）": "(empty)"`, `"槽位": "Slot"`, `"删除光环": "Delete halo"`, `"取消": "Cancel"`, `"保存光环": "Save halo"`, `"被以下位置引用，无法删除": "Cannot delete, referenced by"`, `"光环选项不能引用功能库、切层或自己": "A halo option cannot use the library, a layer switch or this halo itself"`, `"（未命名）": "(unnamed)"`。`RingPreview` 里的 `"（未命名）"` / `"＋"` 要经 `t()`（i18n 扫描测试会拦）。

- [ ] **Step 5: `App.svelte` 路由**

```ts
  let page = $state<"keymap" | "settings" | "halo">("keymap");
  let editing = $state<{ id: string | null; wasNew: boolean; returnTo: Selection } | null>(null);
  function openHalo(id: string | null) {
    editing = { id, wasNew: id === null, returnTo: draft.selection };
    page = "halo";
  }
  function closeHalo(savedId: string | null) {
    if (editing?.wasNew && savedId) {
      draft = assignAction(select(draft, editing.returnTo), { halo: { _0: savedId } });
    }
    editing = null;
    page = "keymap";
  }
```

模板：`{:else if page === "halo"}<HaloEditor bind:draft haloId={editing?.id ?? null} onClose={closeHalo} />{/if}`；`<Keymap … onEditHalo={openHalo} />`；`TopBar` 的 `page` 在编辑器时传 `"keymap"`（页签仍高亮键位）。`import { assignAction, select, type Selection } from "./model/draft"`。

- [ ] **Step 6: 运行** `npm test && npm run check && npm run build`；浏览器：键位页 → 光环 → 新建光环 → 编辑器能用。
- [ ] **Step 7: 提交** `git add -A ui && git commit -m "feat(ui): full-page halo editor with ring preview and slot list"`。

---

### Task 7: 设置页"光环转盘"卡 + 文档 + 真机清单

**Files:**
- Modify: `ui/src/pages/Settings.svelte`, `ui/src/lib/tauri.ts`（`AppSettings.halo`、`api.setSettings`），`ui/src/lib/i18n.ts`, `docs/03-config-format.md` (+zh), `docs/01-architecture.md` (+zh)
- Test: `ui/src/pages/__tests__/settings.test.svelte.ts`（追加）

**Interfaces:**
- `AppSettings = { idleMinutes: number; halo: { position: "pointer" | "center"; timeoutSeconds: number; size: "small" | "medium" | "large" } }`；`api.setSettings(patch: Partial<…深>): Promise<AppSettings>`（mock：深合并到内存）。
- 设置页新卡"光环转盘"：出现位置分段 [指针周围][屏幕中央]、自动关闭下拉 2 / 4 / 6 / 10 秒、大小分段 [小][中][大]；每项改动立即 `setSettings`，失败行内红字。
- 第 1 期的"第 1 期：设置页只读"提示改成"写入项将在后续计划中补全"。

- [ ] **Step 1: 写失败的测试（追加到 `settings.test.svelte.ts`）**

```ts
import { api } from "../../lib/tauri";
  it("writes halo settings through the bridge", async () => {
    setLang("zh");
    app = mount(Settings, { target: document.body, props: { snapshot } });
    await tick(); await tick();
    flushSync();
    expect(document.body.textContent).toContain("光环转盘");
    ([...document.querySelectorAll("button")].find((b) => b.textContent?.trim() === "大") as HTMLButtonElement).click();
    await tick(); await tick();
    expect((await api.getSettings()).halo.size).toBe("large");
    const sel = document.querySelector("select[data-role=halo-timeout]") as HTMLSelectElement;
    sel.value = "10"; sel.dispatchEvent(new Event("change", { bubbles: true }));
    await tick(); await tick();
    expect((await api.getSettings()).halo.timeoutSeconds).toBe(10);
  });
```

- [ ] **Step 2: 运行确认失败**。

- [ ] **Step 3: 实现**

`tauri.ts`：

```ts
export type HaloSettings = { position: "pointer" | "center"; timeoutSeconds: number; size: "small" | "medium" | "large" };
export type AppSettings = { idleMinutes: number; halo: HaloSettings };
let mockSettings: AppSettings = { idleMinutes: 5, halo: { position: "pointer", timeoutSeconds: 4, size: "medium" } };
// api 里：
  getSettings: (): Promise<AppSettings> => isTauri() ? invoke("get_settings") : Promise.resolve(JSON.parse(JSON.stringify(mockSettings))),
  setSettings: (patch: { idleMinutes?: number; halo?: Partial<HaloSettings> }): Promise<AppSettings> => isTauri() ? invoke("set_settings", { patch }) : Promise.resolve((mockSettings = { ...mockSettings, ...patch, halo: { ...mockSettings.halo, ...(patch.halo ?? {}) } }, JSON.parse(JSON.stringify(mockSettings)))),
```

`Settings.svelte`：`let halo = $state<HaloSettings>({ position: "pointer", timeoutSeconds: 4, size: "medium" })`，`load()` 里 `halo = (await api.getSettings()).halo`；新卡放在"电源"旁（`.two` 网格里）：

```svelte
    <section class="card">
      <h3>{t("光环转盘")}</h3>
      <div class="row"><span class="k">{t("出现位置")}</span><span class="v seg">
        <button class:on={halo.position === "pointer"} onclick={() => setHalo({ position: "pointer" })}>{t("指针周围")}</button>
        <button class:on={halo.position === "center"} onclick={() => setHalo({ position: "center" })}>{t("屏幕中央")}</button></span></div>
      <div class="row"><span class="k">{t("自动关闭")}</span><span class="v"><select data-role="halo-timeout" value={String(halo.timeoutSeconds)} onchange={(e) => setHalo({ timeoutSeconds: Number((e.target as HTMLSelectElement).value) })}>{#each [2, 4, 6, 10] as s}<option value={String(s)}>{s} {t("秒")}</option>{/each}</select></span></div>
      <div class="row"><span class="k">{t("大小")}</span><span class="v seg">
        {#each [["small", "小"], ["medium", "中"], ["large", "大"]] as [v, name]}<button class:on={halo.size === v} onclick={() => setHalo({ size: v as HaloSettings["size"] })}>{t(name)}</button>{/each}</span></div>
      {#if haloError}<p class="bad">{haloError}</p>{/if}
    </section>
```

```ts
  let haloError = $state("");
  async function setHalo(patch: Partial<HaloSettings>) {
    try { halo = (await api.setSettings({ halo: patch })).halo; haloError = ""; } catch (e) { haloError = String(e); }
  }
```

分段样式复用 `.seg` 与 `.on`（参照 `OutputModeSegment`）。`i18n` 补：`"光环转盘"`（已有）, `"出现位置": "Position"`, `"指针周围": "Around the pointer"`, `"屏幕中央": "Screen centre"`, `"自动关闭": "Auto close"`, `"秒": "s"`, `"大小": "Size"`, `"小": "Small"`, `"中": "Medium"`, `"大": "Large"`, 把 `"第 1 期：设置页只读；写入项在后续计划中"` 的文案与英文改为 `"其余写入项在后续计划中补全"` / `"Remaining settings become editable in later phases"`（旧 key 删除）。

文档：`docs/03-config-format.md` §4 光环末尾加一句 "The settings window's halo editor writes only slots that have an action, in slot order; the on-screen wheel's position, timeout and size live in `settings.json` (`halo` section), not in the keymap."（中文版对应）。`docs/01-architecture.md` 第 4 条里程碑追加 "**Wheel redesign + halo editor 2026-10-18** (phase 3): Surface-Dial style wedge wheel with size / position / timeout settings; full-page halo editor."（中文版对应）。

- [ ] **Step 4: 全量检查** `python3 tools/check_docs.py && cd ui && npm test && npm run check && npm run build && cd .. && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo build --release -p halokey-app`。

- [ ] **Step 5: 真机清单（本机，逐项记录到提交信息）**

1. 旋钮按下已分配光环时按下旋钮：扇形环在指针周围出现，中号 360 px，第一个扇形高亮、中心显示它的名字；拧旋钮高亮顺时针走；按下执行并关闭；按任意键取消；4 秒不动自动关闭。
2. 设置页把大小改"大"、位置改"屏幕中央"、超时改 10 秒：再按旋钮，环 440 px 居中，10 秒后关闭；改回。
3. 键位页 → 光环 → 新建光环：名字"工具"，槽 1 Enter、槽 3 指针 →20、槽 5 另一个光环"更多"；保存后当前手势自动分配了"光环「工具」"；保存到本机；按下该手势 → 转盘出现 3 个扇形（1 = 整环？不，3 个），选第 3 个按下 → 打开"更多"的转盘（嵌套）。
4. 编辑"主菜单"：把槽 1 下移、改名、保存；转盘顺序随之变化。
5. 删除"更多"：提示被"主菜单 槽位 7"引用；把主菜单槽 7 清空保存后再删 → 成功；保存到本机后 Rust 不报错。
6. 只有 1 个选项的光环：转盘是整环，拧动不崩，按下执行。
7. 英文界面：编辑器、转盘提示、设置卡全英文。
8. 重启应用后 `settings.json` 的 `halo` 段仍在，`idleMinutes` 没丢。

- [ ] **Step 6: 提交并推送**

```bash
git add -A && git commit -m "feat(ui,docs): halo wheel settings card; phase 3 complete

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>" && git push origin main
```

---

## 自检记录

- 规格覆盖：§4.4 光环分类的卡片 + 新建 / 编辑入口（Task 3）；§5.1 编辑器：整页、环形预览 8 槽、槽位列表（名称 / 动作简名 / 上下移 / 删除）、复用键码面板去掉 Layer、动作可以是另一个光环、底栏取消 / 保存、空槽不写进配置（Task 2、6）；§5.2 扇形环尺寸 280 / 360 / 440、内径 60%、12 点钟顺时针、强调色高亮、中心名称、提示行、位置 / 超时可设（Task 4、5）；§6 光环转盘卡（Task 7）；§7 `settings.json` 的 `halo` 段（Task 5）；§8 `set_settings(partial)`（Task 5；`get_autostart / set_autostart` 的合并留第 5 期）。Wayland 退化属已知限制，本期不碰。
- 类型一致性：`Slot` / `slotsFrom` / `haloFrom` / `haloProblems` / `moveSlot` / `clearSlot`（Task 2）在 Task 6 同名；`wedgePath(n, i, outer, inner, cx, cy)` / `wedgeCenter(n, i, r, cx, cy)` 在 Task 4、6 同参；`upsertHalo` / `deleteHalo` / `haloReferences` / `haloActionAllowed` / `newHaloId`（Task 1）在 Task 6 同名；`Palette` 的 `mode / onAction / excludeHalo / onEditHalo / crumb`（Task 3）在 Task 6 同名；`AppSettings.halo` 与 Rust `HaloSettings` 字段 camelCase 一致（Task 5、7）；`haloShow.size` 在 Task 4、5 一致。
- Review Focus 五项：1 → Task 1 `deleteHalo` 测试 + Task 6 "blocks deleting"；2 → Task 1 `haloActionAllowed` + Task 3 `excludeHalo` 测试；3 → Task 2 `haloProblems` + Task 6 保存前提示测试；4 → Task 2 `wedgePath` n=1、2 测试 + 真机清单 6；5 → Task 5 Rust 合并测试。
- 无自动化测试的部分：运行时 `SetHaloTimeout`（Worker 需假后端）、Tauri 窗口尺寸 / 位置——由真机清单 1、2 覆盖；执行时按 Ruling 记录。
