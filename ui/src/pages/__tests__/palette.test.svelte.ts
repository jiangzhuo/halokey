import { describe, it, expect, afterEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import Keymap from "../Keymap.svelte";
import { initialDraft, loadSaved, controlAt, type Draft } from "../../model/draft";
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
    expect(byText("Layer")!.disabled).toBe(true);
    byText("指针")!.click();
    flushSync();
    byText("→")!.click();
    flushSync();
    expect(controlAt(draft.v.map!, 0, 4)!.pressAction).toEqual({ pointerMove: { _0: { dx: 20, dy: 0 } } });
  });
});
