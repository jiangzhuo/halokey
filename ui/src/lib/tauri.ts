import type { HostKeymap, Halo, LayerLighting } from "../model/keymap";
import example from "../../../docs/examples/halo-menu.json";

export type UiSnapshot = { connected: boolean; online: boolean | null; battery: [number | null, number, boolean | null] | null; heartbeats: number; echoes: number; injected: number; lastError: string | null; configPath: string; paused: boolean; standby: boolean | null; link: boolean | null; activeLayers: number[]; effectiveLighting: LayerLighting | null };
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
export type AppSettings = { idleMinutes: number; halo: HaloSettings };
let mockSettings: AppSettings = { idleMinutes: 5, halo: { position: "pointer", timeoutSeconds: 4, size: "medium" } };

export const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

// 浏览器里（npm run dev / vitest）没有 Tauri，用内存数据让界面能跑。
let memory: HostKeymap = JSON.parse(JSON.stringify(example));
let mockActive: number[] = [];
const mockSnapshot = (): UiSnapshot => ({ connected: true, online: true, battery: [60, 3791, false], heartbeats: 1024, echoes: 1024, injected: 58, lastError: null, configPath: "~/.config/halokey/keymap.json", paused: false, standby: false, link: true, activeLayers: mockActive, effectiveLighting: null });
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
  deviceInfo: (): Promise<UiDeviceInfo> => isTauri() ? invoke("device_info") : Promise.resolve({ firmware: "4.4.2", dongleFirmware: "4.4.2", noiseReduction: [1, 64, 6, 232], lighting: { mode: 2, brightness: 14, lights: [] }, errors: [] }),
  powerInfo: (): Promise<UiPower> => isTauri() ? invoke("power_info") : Promise.resolve({ standby: false, standbyDelay: 300, sleepDelay: 3600, micEnabled: true, noiseReduction: 1, errors: [] }),
  getSettings: (): Promise<AppSettings> => isTauri() ? invoke("get_settings") : Promise.resolve(JSON.parse(JSON.stringify(mockSettings))),
  setSettings: (patch: { idleMinutes?: number; halo?: Partial<HaloSettings> }): Promise<AppSettings> => {
    if (isTauri()) return invoke("set_settings", { patch });
    mockSettings = { ...mockSettings, ...patch, halo: { ...mockSettings.halo, ...(patch.halo ?? {}) } };
    return Promise.resolve(JSON.parse(JSON.stringify(mockSettings)));
  },
  setIdleMinutes: (minutes: number): Promise<void> => isTauri() ? invoke("set_idle_minutes", { minutes }) : Promise.resolve(),
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
