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
});
