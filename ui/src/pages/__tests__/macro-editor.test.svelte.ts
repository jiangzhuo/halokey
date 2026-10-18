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
  it("builds a macro from a window, a delay and recorded keys, then saves it", async () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    const closed: (string | null)[] = [];
    app = mount(MacroEditor, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, itemId: null, onClose: (id: string | null) => closed.push(id) } });
    await tick(); flushSync();
    expect(document.body.textContent).toContain("设备注入的按键也会被录进来");
    byText("完成")!.click(); flushSync();
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
    byText("完成")!.click(); flushSync();
    expect(closed).toHaveLength(1);
    const item = draft.v.map!.actionLibrary![0]!;
    expect(closed[0]).toBe(item.id);
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
    expect(document.querySelectorAll("li[data-step]")).toHaveLength(2);
    ([...document.querySelectorAll("[data-role=step-down]")][0] as HTMLButtonElement).click(); flushSync();
    expect(document.querySelectorAll("li[data-step]")[0]!.textContent).toContain("Enter");
    byText("删除宏")!.click(); flushSync();
    expect(document.body.textContent).toContain("中间按键 › 长按");
    expect(draft.v.map!.actionLibrary).toHaveLength(1);
  });
});
