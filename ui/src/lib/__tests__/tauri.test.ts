import { describe, it, expect } from "vitest";
import { api, isTauri } from "../tauri";

describe("tauri bridge (mock mode in jsdom)", () => {
  it("reports no tauri and serves a keymap", async () => {
    expect(isTauri()).toBe(false);
    const map = await api.getKeymap();
    expect(map.controls).toHaveLength(6);
    const snap = await api.snapshot();
    expect(snap.connected).toBe(true);
    expect(Array.isArray(snap.activeLayers)).toBe(true);
  });
  it("validates and saves in memory", async () => {
    const map = await api.getKeymap();
    await expect(api.validateKeymap({ ...map, controls: map.controls.slice(0, 5) })).rejects.toMatch(/六个控件/);
    map.doublePressWindow = 0.3;
    await api.saveKeymap(map);
    expect((await api.getKeymap()).doublePressWindow).toBe(0.3);
  });
  it("mock snapshot carries effectiveLighting", async () => {
    const snap = await api.snapshot();
    expect("effectiveLighting" in snap).toBe(true);
  });
  it("mock validation rejects bad lighting like Rust does", async () => {
    const map = await api.getKeymap();
    await expect(api.validateKeymap({ ...map, lighting: { lights: [{ mode: "solid", brightness: 21 }, { mode: "off", brightness: 0 }, { mode: "off", brightness: 0 }, { mode: "off", brightness: 0 }] } })).rejects.toMatch(/0–20/);
    await expect(api.validateKeymap({ ...map, lighting: { lights: [{ mode: "solid", brightness: 1 }] } })).rejects.toMatch(/四盏/);
  });
  it("mock window list has classes", async () => {
    const r = await api.listWindows();
    expect(r.error).toBeNull();
    expect(r.windows.map((w) => w.class)).toContain("google-chrome");
  });
  it("mock device writes round-trip, and the mic mirrors the hardware quirk", async () => {
    expect(await api.setSleepDelay(7200)).toBe(7200);
    expect(await api.setStandbyDelay(120)).toBe(120);
    expect(await api.setNoiseReduction(0)).toBe(0);
    const info = await api.deviceInfo();
    const lit = await api.setLighting({ ...info.lighting!, mode: 1, brightness: 7 });
    expect(lit.mode).toBe(1);
    expect((await api.deviceInfo()).lighting!.brightness).toBe(7);
    expect((await api.readDeviceKeys()).map(([i]) => i)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(await api.setDefaultAudioInput()).toContain("AU05");
    expect((await api.snapshot()).version).toBe("0.1.0");
  });
  it("mock keymap file actions replace the stored keymap", async () => {
    const def = await api.loadDefaultKeymap();
    expect(def.controls.map((c) => c.press[0]!.code)).toEqual([0x00, 0x28, 0x29, 0x46, 0x4f, 0x2a]); // 键 1 = 无动作，不是固件的无效码
    expect((await api.getKeymap()).halos).toBeUndefined();
    const fromDev = await api.keymapFromDevice();
    expect(fromDev.controls).toHaveLength(6);
    expect(await api.exportKeymap()).toBe("/tmp/keymap.json");
    expect((await api.importKeymap())!.controls).toHaveLength(6);
    const s = await api.setSettings({ closeToTray: false });
    expect(s.closeToTray).toBe(false);
  });
});
