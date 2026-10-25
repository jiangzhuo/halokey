import { describe, it, expect, afterEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import App from "../App.svelte";
import { api } from "../lib/tauri";
import { setLang } from "../lib/i18n";

let app: ReturnType<typeof mount> | undefined;
afterEach(() => { if (app) unmount(app); app = undefined; document.body.innerHTML = ""; vi.restoreAllMocks(); });
const tick = () => new Promise((r) => setTimeout(r, 0));

describe("App", () => {
  it("loads the keymap for editing even when the receiver is not connected", async () => {
    setLang("zh");
    vi.spyOn(api, "snapshot").mockResolvedValue({ connected: false, online: null, battery: null, heartbeats: 0, echoes: 0, injected: 0, lastError: null, configPath: "x", paused: false, standby: null, link: null, activeLayers: [], effectiveLighting: null, version: "0.1.0" });
    app = mount(App, { target: document.body });
    await tick(); await tick(); await tick();
    flushSync();
    expect(document.body.textContent).toContain("Enter");
    expect(document.body.textContent).toContain("保存到本机");
  });

  it("shows the backend's last error and lets the user dismiss it", async () => {
    setLang("zh");
    vi.spyOn(api, "snapshot").mockResolvedValue({ connected: false, online: null, battery: null, heartbeats: 0, echoes: 0, injected: 0, lastError: "HID 初始化失败：测试", configPath: "x", paused: false, standby: null, link: null, activeLayers: [], effectiveLighting: null, version: "0.1.0" });
    app = mount(App, { target: document.body });
    await tick(); await tick(); await tick();
    flushSync();
    const bar = document.querySelector('[data-role="backend-error"]');
    expect(bar?.textContent).toContain("HID 初始化失败：测试");
    (document.querySelector('[data-role="dismiss-error"]') as HTMLButtonElement).click();
    flushSync();
    expect(document.querySelector('[data-role="backend-error"]')).toBeNull();
  });
});
