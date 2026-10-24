import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import UpdateCard from "../UpdateCard.svelte";
import { api, resetMocks, setMockUpdate, emitMockUpdate } from "../../lib/tauri";
import { setLang } from "../../lib/i18n";

let app: ReturnType<typeof mount> | undefined;
beforeEach(() => { setLang("en"); resetMocks(); });
afterEach(() => { vi.restoreAllMocks(); if (app) unmount(app); app = undefined; document.body.innerHTML = ""; });

/** 挂载并等异步加载（状态、设置、版本）落定。 */
async function render() {
  app = mount(UpdateCard, { target: document.body });
  await new Promise((r) => setTimeout(r, 0));
  flushSync();
}
const button = (name: string) => [...document.querySelectorAll("button")].find((b) => b.textContent?.trim() === name) as HTMLButtonElement | undefined;
const click = (el: Element) => { (el as HTMLElement).click(); flushSync(); };
const text = () => document.body.textContent ?? "";

describe("update card", () => {
  it("shows the version and installs on click", async () => {
    setMockUpdate({ state: "available", version: "0.2.0" });
    const install = vi.spyOn(api, "installUpdate");
    await render();
    expect(text()).toMatch(/0\.2\.0/);
    click(button("Download and install")!);
    expect(install).toHaveBeenCalledOnce();
  });
  it("disables install while downloading and shows progress", async () => {
    setMockUpdate({ state: "downloading", downloaded: 1048576, total: 4194304 });
    await render();
    expect(button("Download and install")).toBeUndefined();
    expect(button("Check for updates")!.disabled).toBe(true);
    expect(text()).toMatch(/1\.0 \/ 4\.0 MB/);
  });
  it("explains source runs and cancelled authorization", async () => {
    setMockUpdate({ state: "unsupported", reason: "fromSource" });
    await render();
    expect(text()).toContain("Running from source; updates are not automatic");
  });
  it("explains unsupported bundles", async () => {
    setMockUpdate({ state: "unsupported", reason: "bundle" });
    await render();
    expect(text()).toContain("Automatic updates are not supported for this install type");
  });
  it("auto-check toggle writes settings", async () => {
    const spy = vi.spyOn(api, "setSettings");
    await render();
    const box = document.querySelector('input[type=checkbox][aria-label="Check for updates automatically"]') as HTMLInputElement;
    expect(box.checked).toBe(true);
    click(box);
    expect(spy).toHaveBeenCalledWith({ updates: { autoCheck: false } });
  });
  it("release notes link opens the release page", async () => {
    setMockUpdate({ state: "available", version: "0.2.0" });
    const open = vi.spyOn(api, "openReleaseNotes");
    await render();
    click(button("Release notes")!);
    expect(open).toHaveBeenCalledWith("0.2.0");
  });
  it("progress without a total shows only the downloaded size", async () => {
    setMockUpdate({ state: "downloading", downloaded: 2097152, total: null });
    await render();
    expect(text()).toMatch(/2\.0 MB/);
    expect(text()).not.toContain("NaN");
    expect(text()).not.toContain(" / ");
  });
  it("renders the remaining states", async () => {
    const cases: [Parameters<typeof setMockUpdate>[0], string][] = [
      [{ state: "cancelled" }, "Authorization cancelled; you can try again later"],
      [{ state: "error", message: "下载失败：网络不通" }, "下载失败：网络不通"],
      [{ state: "upToDate" }, "You're up to date"],
      [{ state: "checking" }, "Checking…"],
      [{ state: "installing" }, "Installing…"],
    ];
    for (const [s, expected] of cases) {
      setMockUpdate(s);
      await render();
      expect(text()).toContain(expected);
      if (s.state === "checking" || s.state === "installing") expect(button("Check for updates")!.disabled).toBe(true);
      else expect(button("Check for updates")!.disabled).toBe(false);
      unmount(app!); app = undefined; document.body.innerHTML = "";
    }
  });
  it("check button asks Rust and shows the result", async () => {
    const check = vi.spyOn(api, "checkUpdate");
    await render();
    click(button("Check for updates")!);
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    expect(check).toHaveBeenCalledOnce();
    expect(text()).toContain("You're up to date");
  });
  it("auto-check toggle rolls the checkbox back and shows the error when saving fails", async () => {
    vi.spyOn(api, "setSettings").mockRejectedValueOnce("boom");
    await render();
    const box = document.querySelector('input[type=checkbox][aria-label="Check for updates automatically"]') as HTMLInputElement;
    click(box);
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    expect(box.checked).toBe(true);
    expect(text()).toContain("boom");
  });
  it("follows pushed states (progress, then installing)", async () => {
    await render();
    emitMockUpdate({ state: "downloading", downloaded: 1048576, total: 4194304 });
    flushSync();
    expect(text()).toMatch(/1\.0 \/ 4\.0 MB/);
    emitMockUpdate({ state: "installing" });
    flushSync();
    expect(text()).toContain("Installing…");
    expect(text()).not.toMatch(/4\.0 MB/);
  });
  it("a push arriving before updateStatus resolves is not overwritten by it", async () => {
    let resolve!: (s: Awaited<ReturnType<typeof api.updateStatus>>) => void;
    vi.spyOn(api, "updateStatus").mockReturnValueOnce(new Promise((r) => { resolve = r; }));
    app = mount(UpdateCard, { target: document.body });
    await new Promise((r) => setTimeout(r, 0)); // 等订阅注册
    emitMockUpdate({ state: "installing" });
    resolve({ state: "idle" });
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    expect(text()).toContain("Installing…");
  });
  it("stops listening after unmount", async () => {
    await render();
    unmount(app!); app = undefined;
    expect(() => emitMockUpdate({ state: "upToDate" })).not.toThrow();
    expect(text()).not.toContain("You're up to date");
  });
  it("shows the error row when checkUpdate rejects", async () => {
    vi.spyOn(api, "checkUpdate").mockRejectedValueOnce("network down");
    await render();
    click(button("Check for updates")!);
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    expect(text()).toContain("network down");
  });
});
