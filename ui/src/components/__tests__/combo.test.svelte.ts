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
    expect(document.body.textContent).toContain("⇧⌘A");
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
    expect((document.querySelector("[data-role=combo-assign]") as HTMLButtonElement).disabled).toBe(false);
    key(rec, "Escape"); flushSync();
    expect((document.querySelector("[data-role=combo-assign]") as HTMLButtonElement).disabled).toBe(true);
  });
});
