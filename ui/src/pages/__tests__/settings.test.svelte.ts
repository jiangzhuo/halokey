import { describe, it, expect, afterEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import Settings from "../Settings.svelte";
import { setLang } from "../../lib/i18n";
import type { UiSnapshot } from "../../lib/tauri";
import { api } from "../../lib/tauri";

let app: ReturnType<typeof mount> | undefined;
afterEach(() => { if (app) unmount(app); app = undefined; document.body.innerHTML = ""; });
const tick = () => new Promise((r) => setTimeout(r, 0));
const snapshot: UiSnapshot = { connected: true, online: true, battery: [60, 3791, false], heartbeats: 12, echoes: 11, injected: 0, lastError: null, configPath: "/tmp/keymap.json", paused: false, standby: false, link: true, activeLayers: [], effectiveLighting: null };

describe("Settings page", () => {
  it("shows firmware, lighting, power and general cards from the bridge", async () => {
    setLang("zh");
    app = mount(Settings, { target: document.body, props: { snapshot } });
    await tick(); await tick();
    flushSync();
    const text = document.body.textContent ?? "";
    expect(text).toContain("4.4.2 / 4.4.2");
    expect(text).toContain("本体在线");
    expect(text).toContain("工作");
    expect(text).toContain("14 / 20");
    expect(text).toContain("1 h");
    expect(text).toContain("300 s");
    expect(text).toContain("/tmp/keymap.json");
    const idle = document.querySelector("select[data-role=idle]") as HTMLSelectElement;
    expect(idle.value).toBe("5");
  });
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
});
