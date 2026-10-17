import { describe, it, expect, afterEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import Keymap from "../Keymap.svelte";
import { initialDraft, loadSaved, controlAt, type Draft } from "../../model/draft";
import type { HostKeymap } from "../../model/keymap";
import { setLang } from "../../lib/i18n";
import { api } from "../../lib/tauri";

const load = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/halo-menu.json"), "utf8"));
let app: ReturnType<typeof mount> | undefined;
afterEach(() => { if (app) unmount(app); app = undefined; document.body.innerHTML = ""; });
const buttons = () => [...document.querySelectorAll("button")] as HTMLButtonElement[];
const byText = (text: string) => buttons().find((b) => b.textContent?.trim() === text);
const tick = () => new Promise((r) => setTimeout(r, 0));

describe("Keymap save and revert", () => {
  it("revert restores the saved map and save persists through the bridge", async () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null } });
    flushSync();
    expect(byText("保存到本机")!.disabled).toBe(true);
    byText("Esc")!.click();
    flushSync();
    expect(byText("保存到本机")!.disabled).toBe(false);
    byText("撤销更改")!.click();
    flushSync();
    expect(draft.v.dirty).toBe(false);
    expect(controlAt(draft.v.map!, 0, 0)!.press).toEqual(load().controls[0]!.press);
    byText("Esc")!.click();
    flushSync();
    byText("保存到本机")!.click();
    await tick(); await tick();
    flushSync();
    expect(draft.v.dirty).toBe(false);
    expect(document.body.textContent).toContain("已保存");
    expect(controlAt(await api.getKeymap(), 0, 0)!.press).toEqual([{ type: 2, code: 0x29 }]);
  });
});
