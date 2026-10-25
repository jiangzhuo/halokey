import { describe, it, expect, afterEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import type { UiEvent, UiSnapshot } from "../lib/tauri";

// 拿到 App 订阅的事件回调，测试里手动发错误事件。
let emit: ((e: UiEvent) => void) | undefined;
vi.mock("../lib/tauri", async (importOriginal) => {
  const mod = await importOriginal<typeof import("../lib/tauri")>();
  return { ...mod, subscribe: async (h: (e: UiEvent) => void) => { emit = h; return () => {}; } };
});
const { api } = await import("../lib/tauri");
const { default: App } = await import("../App.svelte");
const { setLang } = await import("../lib/i18n");

let app: ReturnType<typeof mount> | undefined;
afterEach(() => { vi.useRealTimers(); if (app) unmount(app); app = undefined; emit = undefined; document.body.innerHTML = ""; vi.restoreAllMocks(); });
const tick = () => new Promise((r) => setTimeout(r, 0));
const snap = (lastError: string | null): UiSnapshot => ({ connected: false, online: null, battery: null, heartbeats: 0, echoes: 0, injected: 0, lastError, configPath: "x", paused: false, standby: null, link: null, activeLayers: [], effectiveLighting: null, version: "0.1.0" });
const bar = () => document.querySelector('[data-role="backend-error"]')?.textContent ?? null;

describe("App backend error bar", () => {
  it("keeps an error event that arrives while an older snapshot request is in flight", async () => {
    setLang("zh");
    let resolve: ((s: UiSnapshot) => void) | undefined;
    vi.spyOn(api, "snapshot").mockImplementation(() => new Promise((r) => { resolve = r; }));
    app = mount(App, { target: document.body });
    await tick();
    // 第一份快照还没回来就来了错误事件：马上显示。
    emit?.({ kind: "error", text: "新错误" });
    flushSync();
    expect(bar()).toContain("新错误");
    // 事件之前发出的请求晚回来，带的是旧状态：不能盖掉事件。
    resolve?.(snap("旧错误"));
    await tick(); flushSync();
    expect(bar()).toContain("新错误");
  });

  it("lets a snapshot requested after the event clear the error", async () => {
    setLang("zh");
    const spy = vi.spyOn(api, "snapshot").mockResolvedValue(snap(null));
    // 只假 setInterval（2 秒一次的轮询），tick 用的 setTimeout 照常。
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    app = mount(App, { target: document.body });
    await tick(); await tick();
    emit?.({ kind: "error", text: "注入器" });
    flushSync();
    expect(bar()).toContain("注入器");
    // 后端恢复后，下一份快照没有错误：提示消失。
    vi.advanceTimersByTime(2000);
    await tick(); flushSync();
    expect(spy).toHaveBeenCalledTimes(2);
    expect(bar()).toBeNull();
  });

  it("ignores an older snapshot that resolves after a newer one", async () => {
    setLang("zh");
    const pending: ((s: UiSnapshot) => void)[] = [];
    vi.spyOn(api, "snapshot").mockImplementation(() => new Promise((r) => { pending.push(r); }));
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    app = mount(App, { target: document.body });
    await tick();
    vi.advanceTimersByTime(2000);
    await tick();
    expect(pending.length).toBe(2);
    // 新请求先回来：后端已经没有错误。
    pending[1]!(snap(null));
    await tick(); flushSync();
    expect(bar()).toBeNull();
    // 旧请求晚回来，带着旧错误：丢掉。
    pending[0]!(snap("旧错误"));
    await tick(); flushSync();
    expect(bar()).toBeNull();
  });
});
