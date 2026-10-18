import { describe, it, expect, afterEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import Keymap from "../Keymap.svelte";
import { initialDraft, loadSaved, type Draft } from "../../model/draft";
import type { HostKeymap } from "../../model/keymap";
import { setLang } from "../../lib/i18n";

const load = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/layer-backlight.json"), "utf8"));
let app: ReturnType<typeof mount> | undefined;
afterEach(() => { if (app) unmount(app); app = undefined; document.body.innerHTML = ""; });
const buttons = () => [...document.querySelectorAll("button")] as HTMLButtonElement[];
const byText = (t: string) => buttons().find((b) => b.textContent?.trim() === t);

describe("backlight strip", () => {
  it("shows the effective lighting, expands an editor and writes the draft", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null } });
    flushSync();
    expect(document.body.textContent).toContain("本层背光");
    // 键 1 在 AU05 上没有可见的灯：状态条和渲染图都不显示它，编辑器里那一行不画控件、只留说明。
    expect(document.querySelectorAll(".strip .dot[data-mode=solid]")).toHaveLength(2);
    expect(document.querySelectorAll(".device .spot[data-mode=solid]")).toHaveLength(2);
    (document.querySelector(".strip .toggle") as HTMLButtonElement).click();
    flushSync();
    const rows = document.querySelectorAll(".editor .light");
    expect(rows).toHaveLength(4);
    expect(rows[0]!.textContent).toContain("无可见灯");
    expect(rows[0]!.querySelector("button[data-mode]")).toBeNull(); // 键 1 无可见灯：不画控件，只留说明
    expect(rows[0]!.textContent).toContain("此设备无可见灯");
    (rows[3]!.querySelector("button[data-mode=breathe]") as HTMLButtonElement).click();
    flushSync();
    expect(draft.v.map!.lighting!.lights[3]!.mode).toBe("breathe");
    expect(draft.v.dirty).toBe(true);
    expect(document.querySelectorAll(".device .spot[data-mode=breathe]")).toHaveLength(1);
  });
  it("layer 1 follows layer 0 when its record is removed", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null } });
    flushSync();
    ([...document.querySelectorAll(".seg button")].find((b) => b.textContent?.trim() === "1") as HTMLButtonElement).click();
    flushSync();
    expect(document.querySelectorAll(".strip .dot[data-mode=breathe]")).toHaveLength(2);
    (document.querySelector(".strip .toggle") as HTMLButtonElement).click();
    flushSync();
    const follow = document.querySelector(".editor input[data-role=follow]") as HTMLInputElement;
    expect(follow.checked).toBe(false);
    follow.click();
    flushSync();
    expect(draft.v.map!.layers!.find((l) => l.id === 1)!.lighting).toBeUndefined();
    expect(document.querySelectorAll(".strip .dot[data-mode=solid]")).toHaveLength(2);
  });
});
