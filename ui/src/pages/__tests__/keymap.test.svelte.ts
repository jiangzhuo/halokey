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

describe("Keymap page stage", () => {
  it("renders every control's effective action and selects on click", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null } });
    flushSync();
    const text = document.body.textContent ?? "";
    expect(text).toContain("Enter");
    expect(text).toContain("光环「主菜单」");
    expect(text).toContain("滚轮↓");
    expect(text).toContain("滚轮↑");
    const longOfKey3 = [...document.querySelectorAll("button.g")].find((b) => b.textContent?.includes("长按") && b.closest(".side.right"));
    expect(longOfKey3).toBeTruthy();
    (longOfKey3 as HTMLButtonElement).click();
    flushSync();
    expect(draft.v.selection.gesture).toBe("long");
    expect(draft.v.selection.index).toBe(0);
    const layer2 = [...document.querySelectorAll(".seg button")].find((b) => b.textContent?.trim() === "2") as HTMLButtonElement;
    layer2.click();
    flushSync();
    expect(draft.v.selection.layer).toBe(2);
    expect(document.body.textContent).toContain("▽");
  });
});
