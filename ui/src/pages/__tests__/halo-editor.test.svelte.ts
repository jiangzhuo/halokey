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
  it("edits an existing halo, moves slots, and saves", () => {
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
