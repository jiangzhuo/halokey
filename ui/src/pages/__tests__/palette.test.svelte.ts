import { describe, it, expect, afterEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import Keymap from "../Keymap.svelte";
import Palette from "../../components/Palette.svelte";
import type { HostAction } from "../../model/keymap";
import { initialDraft, loadSaved, controlAt, select, type Draft } from "../../model/draft";
import type { HostKeymap } from "../../model/keymap";
import { setLang } from "../../lib/i18n";

const load = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/halo-menu.json"), "utf8"));
let app: ReturnType<typeof mount> | undefined;
afterEach(() => { if (app) unmount(app); app = undefined; document.body.innerHTML = ""; });
const buttons = () => [...document.querySelectorAll("button")] as HTMLButtonElement[];
const byText = (text: string) => buttons().find((b) => b.textContent?.trim() === text);

describe("Keymap palette", () => {
  it("assigns a keycap to the selected gesture and marks the draft dirty", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null } });
    flushSync();
    expect(document.body.textContent).toContain("顶部按键 › 单击");
    byText("Enter")!.click();
    flushSync();
    expect(draft.v.dirty).toBe(true);
    expect(controlAt(draft.v.map!, 0, 0)!.press).toEqual([{ type: 2, code: 0x28 }]);
    expect(byText("保持按住")).toBeTruthy();
  });
  it("hides hold for rotation, disables Layer for rotation, and assigns pointer moves", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null } });
    flushSync();
    buttons().find((b) => b.textContent?.includes("右旋"))!.click();
    flushSync();
    expect(byText("保持按住")).toBeUndefined();
    expect(byText("短按一次")!.classList.contains("on")).toBe(true);
    expect(byText("Layer")!.disabled).toBe(true);
    byText("指针")!.click();
    flushSync();
    byText("→")!.click();
    flushSync();
    expect(controlAt(draft.v.map!, 0, 4)!.pressAction).toEqual({ pointerMove: { _0: { dx: 20, dy: 0 } } });
  });
  it("keeps the Layer panel on double press with MO disabled and assigns TG / TO / OSL", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: select(loadSaved(initialDraft(), load()), { layer: 0, index: 0, gesture: "double" }) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null } });
    flushSync();
    byText("Layer")!.click();
    flushSync();
    expect(byText("MO(1)")!.disabled).toBe(true);
    expect(byText("TO(0)")).toBeTruthy();
    expect(byText("TG(0)")).toBeUndefined();
    expect(byText("OSL(0)")).toBeUndefined();
    byText("TG(2)")!.click();
    flushSync();
    expect(controlAt(draft.v.map!, 0, 0)!.doublePressAction).toEqual({ toggleLayer: { _0: 2 } });
    byText("OSL(3)")!.click();
    flushSync();
    expect(controlAt(draft.v.map!, 0, 0)!.doublePressAction).toEqual({ oneShotLayer: { _0: 3 } });
    expect(document.body.textContent).toContain("按一下开、再按一下关");
  });
  it("assigns a fast scroll with the chosen notches from the Mouse panel", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: select(loadSaved(initialDraft(), load()), { layer: 0, index: 4, gesture: "press" }) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null } });
    flushSync();
    byText("鼠标")!.click();
    flushSync();
    expect(document.body.textContent).toContain("快速滚动");
    const input = document.querySelector<HTMLInputElement>("input[data-role=scroll-notches]")!;
    input.value = "5";
    input.dispatchEvent(new Event("input"));
    flushSync();
    byText("↓")!.click();
    flushSync();
    expect(controlAt(draft.v.map!, 0, 4)!.pressAction).toEqual({ scroll: { _0: { dx: 0, dy: 5 } } });
    // 超出 1–10 的输入按边界发出，不会写出 Rust 会拒绝的值。
    input.value = "40";
    input.dispatchEvent(new Event("input"));
    flushSync();
    byText("←")!.click();
    flushSync();
    expect(controlAt(draft.v.map!, 0, 4)!.pressAction).toEqual({ scroll: { _0: { dx: -10, dy: 0 } } });
  });
  it("leaves the Layer panel when a rotation gets selected", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null } });
    flushSync();
    byText("Layer")!.click();
    flushSync();
    expect(byText("MO(1)")).toBeTruthy();
    buttons().find((b) => b.textContent?.includes("右旋"))!.click();
    flushSync();
    expect(byText("MO(1)")).toBeUndefined();
    expect(byText("Enter")).toBeTruthy();
  });
});

describe("Palette action mode", () => {
  it("emits actions instead of writing the draft, hides Layer and the halo being edited", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    const got: HostAction[] = [];
    app = mount(Palette, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, halos: draft.v.map!.halos!, apps: [], macros: [], mode: "action", onAction: (a: HostAction) => got.push(a), excludeHalo: "main", crumb: "槽位 1" } });
    flushSync();
    expect(byText("Layer")).toBeUndefined();
    expect(byText("保持按住")).toBeUndefined();
    expect(byText("⊠")).toBeUndefined(); // 动作模式没有"不执行"键帽，清空走槽位的 ✕
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
    byText("＋ 新建光环")!.click();
    flushSync();
    expect(edits).toEqual([null]);
    ([...document.querySelectorAll("button[data-edit-halo]")][0] as HTMLButtonElement).click();
    flushSync();
    expect(edits).toEqual([null, "main"]);
  });
  it("clears or inherits the selected gesture from the crumb-row quick buttons", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null } });
    flushSync();
    expect(byText("▽")).toBeUndefined(); // 基础层没有"透明继承"
    byText("⊠")!.click();
    flushSync();
    expect(controlAt(draft.v.map!, 0, 0)!.press).toEqual([{ type: 2, code: 0 }]);
    ([...document.querySelectorAll(".seg button")].find((b) => b.textContent?.trim() === "1") as HTMLButtonElement).click();
    flushSync();
    byText("▽")!.click();
    flushSync();
    expect(controlAt(draft.v.map!, 1, 0)!.inheritedGestures).toContain("press");
    expect(document.querySelectorAll(".grid .special")).toHaveLength(0); // 键帽网格里不再有清除 / 透明
  });
});
