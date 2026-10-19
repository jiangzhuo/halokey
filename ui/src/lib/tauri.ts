import type { HostKeymap, Halo, LayerLighting } from "../model/keymap";
import example from "../../../docs/examples/halo-menu.json";

export type UiSnapshot = { connected: boolean; online: boolean | null; battery: [number | null, number, boolean | null] | null; heartbeats: number; echoes: number; injected: number; lastError: string | null; configPath: string; paused: boolean; standby: boolean | null; link: boolean | null; activeLayers: number[]; effectiveLighting: LayerLighting | null; version: string };
export type UiEvent =
  | { kind: "connected" } | { kind: "disconnected" }
  | { kind: "key"; control: number; name: string; pressed: boolean; at: number }
  | { kind: "battery"; percent: number | null; millivolts: number; charging: boolean | null }
  | { kind: "haloShow"; halo: Halo; selected: number; size?: number } | { kind: "haloSelect"; selected: number } | { kind: "haloHide" }
  | { kind: "error"; text: string } | { kind: "paused" } | { kind: "resumed" } | { kind: "stopped" }
  | { kind: "layers"; active: number[] };
export type UiLight = { kind: number; workTime: number; breatheLevel: number; breatheBrightness: number; alwaysOnBrightness: number };
export type UiLighting = { mode: number; brightness: number; lights: UiLight[] };
export type UiDeviceInfo = { firmware: string | null; dongleFirmware: string | null; noiseReduction: number[] | null; lighting: UiLighting | null; errors: string[] };
export type UiPower = { standby: boolean | null; standbyDelay: number | null; sleepDelay: number | null; micEnabled: boolean | null; noiseReduction: number | null; errors: string[] };
export type HaloSettings = { position: "pointer" | "center"; timeoutSeconds: number; size: "small" | "medium" | "large" };
export type AppSettings = { idleMinutes: number; halo: HaloSettings; closeToTray: boolean };
const defaultSettings = (): AppSettings => ({ idleMinutes: 5, halo: { position: "pointer", timeoutSeconds: 4, size: "medium" }, closeToTray: true });
let mockSettings: AppSettings = defaultSettings();
/** 设备六格表：`[index, [[type, code], …]]`。 */
export type DeviceKeys = [number, [number, number][]][];
const defaultInfo = (): UiDeviceInfo => ({ firmware: "4.4.2", dongleFirmware: "4.4.2", noiseReduction: [1, 64, 6, 232], lighting: { mode: 2, brightness: 14, lights: Array.from({ length: 4 }, () => ({ kind: 0, workTime: 10, breatheLevel: 3, breatheBrightness: 14, alwaysOnBrightness: 14 })) }, errors: [] });
let mockInfo: UiDeviceInfo = defaultInfo();
// 收音开关的 mock 照抄硬件：写入应答但读回仍为关（2026-10-18 实测）。
const defaultPower = (): UiPower => ({ standby: false, standbyDelay: 300, sleepDelay: 3600, micEnabled: false, noiseReduction: 1, errors: [] });
let mockPower: UiPower = defaultPower();
const FACTORY: DeviceKeys = [[0, [[2, 0x01]]], [1, [[2, 0x28]]], [2, [[2, 0x29]]], [3, [[2, 0x46]]], [4, [[2, 0x4f]]], [5, [[2, 0x2a]]]];
const factoryKeymap = (): HostKeymap => ({ version: 5, controls: FACTORY.map(([index, entries]) => ({ index, press: entries.map(([type, code]) => ({ type, code })) })), doublePressWindow: 0.3, longPressThreshold: 0.5 });

export const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

// 浏览器里（npm run dev / vitest）没有 Tauri，用内存数据让界面能跑。
let memory: HostKeymap = JSON.parse(JSON.stringify(example));
/** 测试用：把内存 mock 全部复位（模块单例，不然用例之间互相污染）。 */
export function resetMocks() { memory = JSON.parse(JSON.stringify(example)); mockActive = []; mockSettings = defaultSettings(); mockInfo = defaultInfo(); mockPower = defaultPower(); }
let mockActive: number[] = [];
const mockSnapshot = (): UiSnapshot => ({ connected: true, online: true, battery: [60, 3791, false], heartbeats: 1024, echoes: 1024, injected: 58, lastError: null, configPath: "~/.config/halokey/keymap.json", paused: false, standby: false, link: true, activeLayers: mockActive, effectiveLighting: null, version: "0.1.0" });
function mockValidate(map: HostKeymap): void {
  const idx = new Set(map.controls.map((c) => c.index));
  if (map.controls.length !== 6 || idx.size !== 6) throw "配置必须包含且仅包含六个控件，每个控件只能出现一次。";
  for (const rec of [map.lighting, ...(map.layers ?? []).map((l) => l.lighting)]) {
    if (!rec) continue;
    if (rec.lights.length !== 4) throw "背光记录必须有四盏灯。";
    if (rec.lights.some((l) => l.brightness < 0 || l.brightness > 20)) throw "背光亮度必须在 0–20 之间。";
  }
}

/** `list_windows` 的一项：当前打开的顶层窗口（Linux X11）。 */
export type WindowInfo = { class: string; instance: string; title: string; exe: string };
export type WindowList = { windows: WindowInfo[]; error: string | null };
const MOCK_WINDOWS: WindowInfo[] = [
  { class: "google-chrome", instance: "google-chrome", title: "Chrome", exe: "/usr/bin/google-chrome" },
  { class: "org.gnome.TextEditor", instance: "gnome-text-editor", title: "Text Editor", exe: "/usr/bin/gnome-text-editor" },
];

export const api = {
  snapshot: (): Promise<UiSnapshot> => isTauri() ? invoke("snapshot") : Promise.resolve(mockSnapshot()),
  getKeymap: (): Promise<HostKeymap> => isTauri() ? invoke("get_keymap") : Promise.resolve(JSON.parse(JSON.stringify(memory))),
  validateKeymap: async (keymap: HostKeymap): Promise<void> => { if (isTauri()) return invoke("validate_keymap", { keymap }); mockValidate(keymap); },
  saveKeymap: async (keymap: HostKeymap): Promise<void> => { if (isTauri()) return invoke("save_keymap", { keymap }); mockValidate(keymap); memory = JSON.parse(JSON.stringify(keymap)); },
  deviceInfo: (): Promise<UiDeviceInfo> => isTauri() ? invoke("device_info") : Promise.resolve(JSON.parse(JSON.stringify(mockInfo))),
  powerInfo: (): Promise<UiPower> => isTauri() ? invoke("power_info") : Promise.resolve(JSON.parse(JSON.stringify(mockPower))),
  setLighting: (lighting: UiLighting): Promise<UiLighting> => { if (isTauri()) return invoke("set_lighting", { lighting }); mockInfo = { ...mockInfo, lighting: JSON.parse(JSON.stringify(lighting)) }; return Promise.resolve(lighting); },
  setSleepDelay: (seconds: number): Promise<number> => { if (isTauri()) return invoke("set_sleep_delay", { seconds }); mockPower.sleepDelay = seconds; return Promise.resolve(seconds); },
  setStandbyDelay: (seconds: number): Promise<number> => { if (isTauri()) return invoke("set_standby_delay", { seconds }); mockPower.standbyDelay = seconds; return Promise.resolve(seconds); },
  setMicEnabled: (enabled: boolean): Promise<boolean> => { if (isTauri()) return invoke("set_mic_enabled", { enabled }); mockPower.micEnabled = false; return Promise.resolve(false); },
  setNoiseReduction: (level: 0 | 1): Promise<number> => { if (isTauri()) return invoke("set_noise_reduction", { level }); mockPower.noiseReduction = level; return Promise.resolve(level); },
  setDefaultAudioInput: (): Promise<string> => isTauri() ? invoke("set_default_audio_input") : Promise.resolve("alsa_input.usb-AU05_mock-00.analog-stereo"),
  readDeviceKeys: (): Promise<DeviceKeys> => isTauri() ? invoke("read_device_keys") : Promise.resolve(JSON.parse(JSON.stringify(FACTORY))),
  writeDeviceKeys: (): Promise<string[]> => isTauri() ? invoke("write_device_keys") : Promise.resolve(["控件 0：已写入", "控件 1：已写入"]),
  importKeymap: (): Promise<HostKeymap | null> => isTauri() ? invoke("import_keymap") : Promise.resolve(JSON.parse(JSON.stringify(memory))),
  exportKeymap: (): Promise<string | null> => isTauri() ? invoke("export_keymap") : Promise.resolve("/tmp/keymap.json"),
  loadDefaultKeymap: (): Promise<HostKeymap> => { if (isTauri()) return invoke("load_default_keymap"); memory = factoryKeymap(); return Promise.resolve(JSON.parse(JSON.stringify(memory))); },
  keymapFromDevice: (): Promise<HostKeymap> => { if (isTauri()) return invoke("keymap_from_device"); memory = factoryKeymap(); return Promise.resolve(JSON.parse(JSON.stringify(memory))); },
  getSettings: (): Promise<AppSettings> => isTauri() ? invoke("get_settings") : Promise.resolve(JSON.parse(JSON.stringify(mockSettings))),
  setSettings: (patch: { idleMinutes?: number; closeToTray?: boolean; halo?: Partial<HaloSettings> }): Promise<AppSettings> => {
    if (isTauri()) return invoke("set_settings", { patch });
    mockSettings = { ...mockSettings, ...patch, halo: { ...mockSettings.halo, ...(patch.halo ?? {}) } };
    return Promise.resolve(JSON.parse(JSON.stringify(mockSettings)));
  },
  listWindows: (): Promise<WindowList> => isTauri() ? invoke("list_windows") : Promise.resolve({ windows: JSON.parse(JSON.stringify(MOCK_WINDOWS)), error: null }),
  getAutostart: (): Promise<boolean> => isTauri() ? invoke("get_autostart") : Promise.resolve(false),
  setAutostart: (enabled: boolean): Promise<boolean> => isTauri() ? invoke("set_autostart", { enabled }) : Promise.resolve(enabled),
};

export async function subscribe(handler: (e: UiEvent) => void): Promise<() => void> {
  if (!isTauri()) {
    // 假事件：每 3 秒一下右旋；每 6 秒按住 / 松开 Layer 1，让 Layer 条的绿点和灯圈预览能在浏览器里看到。
    let tick = 0;
    const timer = setInterval(() => {
      tick += 1;
      handler({ kind: "key", control: 4, name: "右旋", pressed: true, at: performance.now() / 1000 });
      mockActive = tick % 2 === 1 ? [1] : [];
      handler({ kind: "layers", active: mockActive });
      // 转盘：显示 6 秒（选中项会变）、隐藏 3 秒，让 halo.html 在浏览器里能看。
      if (tick % 3 !== 0 && example.halos?.[0]) handler({ kind: "haloShow", halo: example.halos[0] as Halo, selected: tick % 7, size: 360 });
      else handler({ kind: "haloHide" });
    }, 3000);
    return () => clearInterval(timer);
  }
  const { listen } = await import("@tauri-apps/api/event");
  return listen<UiEvent>("halokey", (ev) => handler(ev.payload));
}
