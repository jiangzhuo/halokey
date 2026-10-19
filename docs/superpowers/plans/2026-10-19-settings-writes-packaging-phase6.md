# 设置写入、键位导入导出、文档截图与 .deb · 第 6 期实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 设置页的每一项都能写（灯光模式 / 亮度、休眠、待机、收音、降噪、系统默认输入、关闭窗口行为、设备键位读写），键位文件能导入 / 导出 / 载入默认 / 从设备生成；README 换上新界面截图；版本 0.1.0 打出 .deb 并在本机装上。

**Architecture:** Rust 端已有 `set_lighting / set_sleep_delay / set_standby_delay / set_mic_enabled / set_noise_reduction / read_device_keys / write_device_keys`，只补五条新命令（`import_keymap / export_keymap / load_default_keymap / keymap_from_device / set_default_audio_input`）和一个设置项 `closeToTray`；文件对话框用 `tauri-plugin-dialog` 的 Rust API（阻塞式，在 `spawn_blocking` 里跑），前端不引新包。前端只扩 `api` 桥与 `Settings.svelte`；设置页通过 props 拿到草稿是否有未保存改动与键位是否带背光，决定三类"替换键位文件"操作是否可用、灯光模式是否只读。

**Tech Stack:** Rust（`apps/halokey`，Tauri 2.12 + `tauri-plugin-dialog 2`），Svelte 5 + vitest（`ui/`），Playwright（截图），`cargo tauri build --bundles deb`。

**Spec:** `docs/superpowers/specs/2026-10-17-settings-ui-redesign-design.md` §6（设置页七张卡）、§7（第 5 期待决：灯光模式与分层背光共存）、§8（`set_default_audio_input / import_keymap / export_keymap`）、§9（设备写入失败行内红字，不弹窗）、§11、§12 第 5–6 期。

## Global Constraints

- 设置页"每项改动立即写入设备并读回核对，失败在该行右侧红字提示；没有保存按钮"（§6）；"设备写入失败：行内红字 + 事件日志；不弹窗"（§9）。
- 灯光：`set_lighting` 要四盏灯的完整记录（`Lighting::try_from` 拒绝非 4 项），全局改动把 `device_info().lighting` 原样回传、只改 `mode / brightness`；运行时在 `SetLighting` 后作废自己的缓存，下次切层重写。键位文件带任何背光记录时（`lighting` 或任一 `layers[].lighting`），灯光模式与全亮亮度显示为只读并提示"由分层背光接管"（§7 待决项的结论）。
- 收音开关：硬件对 `set_mic_enabled(true)` 应答但读回仍为 false（2026-10-18 实测）；界面必须按读回值显示，并在不一致时红字"设备未接受"。
- 替换键位文件（导入 / 载入默认 / 从设备生成）会同时写文件并下发运行时；键位页有未保存改动时这三个按钮禁用并提示，避免丢改动。
- 设备序列号不出现在任何文档、测试样例或日志示例里（AGENTS.md）：`pactl` 源名里的序列号写成 `202609xxxxxx`。
- 新字符串全部经 `t()`，EN / JA 表键集合一致（测试强制）；`.svelte` 模板里不留中文字面量。
- 每个任务结束：`cd ui && npm test && npm run check && npm run build`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`、`python3 tools/check_docs.py` 全绿；提交英文，末尾 `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`。
- `cargo` 在 `~/.cargo/bin`；发布版 `cargo build --release -p halokey-app` 会嵌入 `ui/dist`，所以先 `npm run build`。

## Review Focus

1. **导入一个旧版本 / 坏掉的 keymap.json**（版本 5、缺控件、UUID 不合法）：必须被 Rust 校验拒绝、原文件不动、错误显示在"键位配置"卡片里。→ Task 1 `import_keymap` 在写文件前 `HostKeymap::from_json`；Task 4 测试 mock 返回错误时卡片显示红字且 `onKeymapReplaced` 不被调用。
2. **收音开关写入后读回不一致**：界面不能显示"已开启"假象。→ Task 3 测试 mock `setMicEnabled` 返回 `false`，断言开关回到关且出现"设备未接受"。
3. **键位文件带背光时改灯光模式**：下一次切层会被运行时覆盖，用户会以为坏了。→ Task 3 `hasBacklight = true` 时灯光行只读 + 提示，测试断言没有可点的模式按钮。
4. **没有 Vibe Key 麦克风（设备没开、pactl 不存在、Wayland 用 PipeWire 但 pactl 缺失）**："设为系统默认输入"要给出原因而不是静默。→ Task 1 `pick_vibe_source` 返回 `None` 的测试 + 命令错误文案；Task 3 测试 mock 拒绝时行内红字。
5. **导出时用户在对话框点取消**：不能报错、不能写空文件。→ Task 1 `export_keymap` 取消返回 `Ok(None)`；Task 4 测试 mock 返回 `null` 时不显示任何提示。

---

## 文件结构

```
apps/halokey/Cargo.toml              + tauri-plugin-dialog = "2"
apps/halokey/src/main.rs             AppSettings.close_to_tray；UiSnapshot.version；install_keymap()；
                                     import_keymap / export_keymap / load_default_keymap / keymap_from_device /
                                     set_default_audio_input；pick_vibe_source()；on_window_event 读 closeToTray；删 set_idle_minutes
apps/halokey/tauri.conf.json         version 0.1.0
Cargo.toml                           workspace version 0.1.0
ui/src/lib/tauri.ts                  api 新增 12 个函数 + mock；AppSettings.closeToTray；UiSnapshot.version
ui/src/lib/__tests__/tauri.test.ts   mock 行为
ui/src/pages/Settings.svelte         七张卡全部可写；props dirty / hasBacklight / onKeymapReplaced
ui/src/pages/__tests__/settings.test.svelte.ts
ui/src/App.svelte                    传 dirty / hasBacklight / onKeymapReplaced
ui/src/lib/i18n.ts                   新字符串（EN / JA）
docs/03-config-format.md (+zh)       新增 "settings.json" 小节
docs/screenshots/keymap-en.png, keymap-zh.png, settings-en.png
README.md / README.zh.md             截图 + 设置页功能一句话
```

---

### Task 1: Rust — 键位文件命令、系统默认输入、关闭窗口行为、版本号

**Files:**
- Modify: `apps/halokey/Cargo.toml`, `apps/halokey/src/main.rs`
- Test: `apps/halokey/src/main.rs` 的 `#[cfg(test)] mod tests`（已有，追加）

**Interfaces（Produces）:**
- `AppSettings { idle_minutes, halo, close_to_tray: bool /* 默认 true */ }`（serde camelCase → `closeToTray`）
- `UiSnapshot.version: String`（`env!("CARGO_PKG_VERSION")`）
- 命令：
  - `import_keymap(app, state) -> Result<Option<serde_json::Value>, String>`：弹开文件对话框（JSON 过滤），取消 → `Ok(None)`；否则 `HostKeymap::from_json` 校验 → `install_keymap` → 返回归一化 JSON。
  - `export_keymap(app, state) -> Result<Option<String>, String>`：保存对话框（默认名 `keymap.json`），取消 → `Ok(None)`；否则把配置文件原样拷到目标路径，返回路径。
  - `load_default_keymap(state) -> Result<serde_json::Value, String>`：`HostKeymap::default_keymap()` → `install_keymap`。
  - `keymap_from_device(state) -> Result<serde_json::Value, String>`：`Command::ReadKeys` → `HostKeymap::from_device_bindings` → `install_keymap`。
  - `set_default_audio_input() -> Result<String, String>`：`pactl list sources short` 里挑含 `AU05` 且非 `.monitor` 的源，`pactl set-default-source`，返回源名。
- 纯函数：`fn pick_vibe_source(list: &str) -> Option<String>`；`fn install_keymap(state: &App, map: HostKeymap) -> Result<serde_json::Value, String>`（写归一化 JSON 到 `config_path` + `SetKeymap`；`save_keymap` 改为调用它）。
- 删除：`set_idle_minutes`（前端改走 `set_settings`）。

- [ ] **Step 1: 写失败的测试（追加到 main.rs 的 tests 模块）**

```rust
    #[test]
    fn picks_the_vibe_key_capture_source_and_skips_monitors() {
        let list = "94\talsa_output.platform-hdmi.monitor\tPipeWire\ts32le 2ch 48000Hz\tRUNNING\n\
                    4779424\talsa_input.usb-AU05_AU05_202609xxxxxx-00.analog-stereo\tPipeWire\ts24le 2ch 48000Hz\tSUSPENDED\n\
                    4779425\talsa_input.usb-AU05_AU05_202609xxxxxx-00.analog-stereo.monitor\tPipeWire\ts24le 2ch 48000Hz\tSUSPENDED\n";
        assert_eq!(
            pick_vibe_source(list).as_deref(),
            Some("alsa_input.usb-AU05_AU05_202609xxxxxx-00.analog-stereo")
        );
        assert_eq!(pick_vibe_source("1\talsa_input.usb-Other-00.analog-stereo\tPipeWire\n"), None);
        assert_eq!(pick_vibe_source(""), None);
    }

    #[test]
    fn settings_default_close_to_tray_and_merge_keeps_it() {
        let s = AppSettings::default();
        assert!(s.close_to_tray);
        let merged: AppSettings = serde_json::from_value(serde_json::json!({ "idleMinutes": 3 })).unwrap();
        assert!(merged.close_to_tray, "缺省字段用默认值");
        let off: AppSettings = serde_json::from_value(serde_json::json!({ "closeToTray": false })).unwrap();
        assert!(!off.close_to_tray);
    }
```

- [ ] **Step 2: 运行确认失败** — `cargo test -p halokey-app 2>&1 | grep -E "^error|cannot find"` → `pick_vibe_source` 未定义、`close_to_tray` 不存在。

- [ ] **Step 3: 实现**

`apps/halokey/Cargo.toml` `[dependencies]` 加 `tauri-plugin-dialog = "2"`。

`main.rs`：

```rust
// AppSettings 加字段（保持 #[serde(default)] 行为：结构体上有 default 就行；否则给字段 #[serde(default = "default_true")]）
#[derive(Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
struct AppSettings {
    idle_minutes: u64,
    halo: HaloSettings,
    /// 关闭设置窗口时留在托盘（true）还是退出（false）。
    close_to_tray: bool,
}
impl Default for AppSettings {
    fn default() -> Self {
        AppSettings { idle_minutes: 5, halo: HaloSettings::default(), close_to_tray: true }
    }
}

// UiSnapshot 加 version: String，from() 里 version: env!("CARGO_PKG_VERSION").to_string()

/// 把一份已校验的键位写到配置文件（归一化 JSON）并下发运行时；save_keymap / 导入 / 默认 / 从设备都走这里。
fn install_keymap(state: &App, map: HostKeymap) -> Result<serde_json::Value, String> {
    let normalized: serde_json::Value =
        serde_json::from_slice(&map.to_json().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&normalized).map_err(|e| e.to_string())?;
    std::fs::write(&state.config_path, text).map_err(|e| format!("写入 {} 失败：{e}", state.config_path.display()))?;
    let guard = state.service.lock().map_err(|e| e.to_string())?;
    let service = guard.as_ref().ok_or("服务未运行")?;
    if !service.send(Command::SetKeymap(map)) {
        return Err("设备线程已退出".into());
    }
    Ok(normalized)
}

#[tauri::command]
async fn import_keymap(app: AppHandle, state: State<'_, App>) -> Result<Option<serde_json::Value>, String> {
    use tauri_plugin_dialog::DialogExt;
    let handle = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        handle.dialog().file().add_filter("JSON", &["json"]).blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(file) = picked else { return Ok(None) };
    let path = file.into_path().map_err(|e| e.to_string())?;
    let bytes = std::fs::read(&path).map_err(|e| format!("读取 {} 失败：{e}", path.display()))?;
    let map = HostKeymap::from_json(&bytes).map_err(|e| e.to_string())?;
    install_keymap(&state, map).map(Some)
}

#[tauri::command]
async fn export_keymap(app: AppHandle, state: State<'_, App>) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let handle = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        handle.dialog().file().add_filter("JSON", &["json"]).set_file_name("keymap.json").blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(file) = picked else { return Ok(None) };
    let target = file.into_path().map_err(|e| e.to_string())?;
    std::fs::copy(&state.config_path, &target).map_err(|e| format!("写入 {} 失败：{e}", target.display()))?;
    Ok(Some(target.display().to_string()))
}

#[tauri::command]
fn load_default_keymap(state: State<'_, App>) -> Result<serde_json::Value, String> {
    install_keymap(&state, HostKeymap::default_keymap())
}

#[tauri::command]
async fn keymap_from_device(state: State<'_, App>) -> Result<serde_json::Value, String> {
    let replies = ask(&state, Command::ReadKeys, 12)??;
    let bindings: Vec<(u8, Vec<halokey_host::KeyEntry>)> = replies
        .into_iter()
        .map(|r| (r.index, r.entries.iter().map(|e| halokey_host::KeyEntry { kind: e.kind.byte(), code: e.code }).collect()))
        .collect();
    let map = HostKeymap::from_device_bindings(&bindings).map_err(|e| e.to_string())?;
    install_keymap(&state, map)
}

/// 从 `pactl list sources short` 的输出里挑 Vibe Key 的输入源（名字含 AU05，排除 .monitor）。
fn pick_vibe_source(list: &str) -> Option<String> {
    list.lines()
        .filter_map(|l| l.split('\t').nth(1))
        .find(|n| n.contains("AU05") && !n.ends_with(".monitor"))
        .map(str::to_string)
}

/// Linux：把 Vibe Key 设为系统默认输入（PipeWire / PulseAudio 都认 pactl）。
#[tauri::command]
async fn set_default_audio_input() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let out = std::process::Command::new("pactl")
            .args(["list", "sources", "short"])
            .output()
            .map_err(|e| format!("找不到 pactl：{e}"))?;
        let name = pick_vibe_source(&String::from_utf8_lossy(&out.stdout))
            .ok_or("没有找到 Vibe Key 的麦克风（本体开着、收音打开了吗？）")?;
        let status = std::process::Command::new("pactl")
            .args(["set-default-source", &name])
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("pactl set-default-source 失败".to_string());
        }
        Ok(name)
    })
    .await
    .map_err(|e| e.to_string())?
}
```

`save_keymap` 的"落盘 + SetKeymap"段落替换为 `install_keymap(&state, map).map(|_| ())`（保留那段 eprintln）。`halokey_host::KeyEntry` 是 `{ kind: u8, code: u8 }` 的普通结构体（`action.rs:11`），直接字面量构造；`from_device_bindings(&[(u8, Vec<KeyEntry>)])` 见 `keymap.rs:203`。

`on_window_event`：

```rust
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let close_to_tray = load_settings(&window.state::<App>().config_path).close_to_tray;
                    if close_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        window.app_handle().exit(0);
                    }
                }
            }
```

`generate_handler!` 去掉 `set_idle_minutes`，加 `import_keymap, export_keymap, load_default_keymap, keymap_from_device, set_default_audio_input`；`.plugin(tauri_plugin_dialog::init())` 加在 `.invoke_handler` 之前；删掉 `set_idle_minutes` 函数。

- [ ] **Step 4: 验证** `cargo test -p halokey-app`（2 个新测试通过）、`cargo clippy --workspace --all-targets -- -D warnings`、`cd ui && npm run build && cd .. && cargo build --release -p halokey-app`。
- [ ] **Step 5: 提交** `git add -A apps Cargo.lock && git commit -m "feat(app): keymap import/export/default/from-device, default audio input, close-to-tray setting"`。

---

### Task 2: 前端桥 — 新命令、mock 与类型

**Files:**
- Modify: `ui/src/lib/tauri.ts`
- Test: `ui/src/lib/__tests__/tauri.test.ts`

**Interfaces（Produces）:**
```ts
export type AppSettings = { idleMinutes: number; halo: HaloSettings; closeToTray: boolean };
UiSnapshot.version: string
api.setLighting(l: UiLighting): Promise<UiLighting>
api.setSleepDelay(seconds: number): Promise<number>
api.setStandbyDelay(seconds: number): Promise<number>
api.setMicEnabled(enabled: boolean): Promise<boolean>        // 返回读回值
api.setNoiseReduction(level: 0 | 1): Promise<number>
api.setDefaultAudioInput(): Promise<string>
api.readDeviceKeys(): Promise<[number, [number, number][]][]>
api.writeDeviceKeys(): Promise<string[]>
api.importKeymap(): Promise<HostKeymap | null>
api.exportKeymap(): Promise<string | null>
api.loadDefaultKeymap(): Promise<HostKeymap>
api.keymapFromDevice(): Promise<HostKeymap>
```
mock：`setLighting` 回传并记到 `mockInfo.lighting`；`setMicEnabled(true)` 返回 **false**（模拟硬件）；`setNoiseReduction` 回传；`readDeviceKeys` 返回出厂六格 `[[0,[[2,1]]],[1,[[2,0x28]]],[2,[[2,0x29]]],[3,[[2,0x46]]],[4,[[2,0x4f]]],[5,[[2,0x2a]]]]`；`writeDeviceKeys` 返回 `["控件 0：已写入"]`；`importKeymap` 返回 `memory` 的拷贝；`exportKeymap` 返回 `"/tmp/keymap.json"`；`loadDefaultKeymap` 返回出厂六键的 `HostKeymap`（`controls` 六项 press 分别为 0x01/0x28/0x29/0x46/0x4f/0x2a，`version` 由 Rust 决定，mock 写 5）并写进 `memory`；`keymapFromDevice` 同上；`setDefaultAudioInput` 返回 `"alsa_input.usb-AU05_mock"`。`setSettings` 的 patch 类型加 `closeToTray?: boolean`。`api.setIdleMinutes` 删除（Settings 改用 `setSettings({ idleMinutes })`）。

- [ ] **Step 1: 写失败的测试（追加）**

```ts
  it("mock device writes round-trip, and the mic mirrors the hardware quirk", async () => {
    expect(await api.setSleepDelay(7200)).toBe(7200);
    expect(await api.setStandbyDelay(120)).toBe(120);
    expect(await api.setNoiseReduction(0)).toBe(0);
    expect(await api.setMicEnabled(true)).toBe(false); // 硬件应答但读回仍为关
    const info = await api.deviceInfo();
    const lit = await api.setLighting({ ...info.lighting!, mode: 1, brightness: 7 });
    expect(lit.mode).toBe(1);
    expect((await api.deviceInfo()).lighting!.brightness).toBe(7);
    expect((await api.readDeviceKeys()).map(([i]) => i)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(await api.setDefaultAudioInput()).toContain("AU05");
  });
  it("mock keymap file actions replace the stored keymap", async () => {
    const def = await api.loadDefaultKeymap();
    expect(def.controls.map((c) => c.press[0]!.code)).toEqual([0x01, 0x28, 0x29, 0x46, 0x4f, 0x2a]);
    expect((await api.getKeymap()).halos).toBeUndefined();
    const fromDev = await api.keymapFromDevice();
    expect(fromDev.controls).toHaveLength(6);
    expect(await api.exportKeymap()).toBe("/tmp/keymap.json");
    expect((await api.importKeymap())!.controls).toHaveLength(6);
    const s = await api.setSettings({ closeToTray: false });
    expect(s.closeToTray).toBe(false);
  });
```

- [ ] **Step 2: 运行确认失败** `npx vitest run src/lib/__tests__/tauri.test.ts` → `api.setSleepDelay is not a function`。

- [ ] **Step 3: 实现（tauri.ts）**

```ts
export type AppSettings = { idleMinutes: number; halo: HaloSettings; closeToTray: boolean };
let mockSettings: AppSettings = { idleMinutes: 5, halo: { position: "pointer", timeoutSeconds: 4, size: "medium" }, closeToTray: true };
let mockInfo: UiDeviceInfo = { firmware: "4.4.2", dongleFirmware: "4.4.2", noiseReduction: [1, 64, 6, 232], lighting: { mode: 2, brightness: 14, lights: Array.from({ length: 4 }, () => ({ kind: 0, workTime: 10, breatheLevel: 3, breatheBrightness: 14, alwaysOnBrightness: 14 })) }, errors: [] };
let mockPower: UiPower = { standby: false, standbyDelay: 300, sleepDelay: 3600, micEnabled: false, noiseReduction: 1, errors: [] };
const FACTORY: [number, [number, number][]][] = [[0, [[2, 0x01]]], [1, [[2, 0x28]]], [2, [[2, 0x29]]], [3, [[2, 0x46]]], [4, [[2, 0x4f]]], [5, [[2, 0x2a]]]];
const factoryKeymap = (): HostKeymap => ({ version: 5, controls: FACTORY.map(([index, entries]) => ({ index, press: entries.map(([type, code]) => ({ type, code })) })) } as HostKeymap);
```

`api` 里（`UiSnapshot` 的 mock 加 `version: "0.1.0"`）：

```ts
  deviceInfo: (): Promise<UiDeviceInfo> => isTauri() ? invoke("device_info") : Promise.resolve(JSON.parse(JSON.stringify(mockInfo))),
  powerInfo: (): Promise<UiPower> => isTauri() ? invoke("power_info") : Promise.resolve(JSON.parse(JSON.stringify(mockPower))),
  setLighting: (lighting: UiLighting): Promise<UiLighting> => { if (isTauri()) return invoke("set_lighting", { lighting }); mockInfo = { ...mockInfo, lighting: JSON.parse(JSON.stringify(lighting)) }; return Promise.resolve(lighting); },
  setSleepDelay: (seconds: number): Promise<number> => { if (isTauri()) return invoke("set_sleep_delay", { seconds }); mockPower.sleepDelay = seconds; return Promise.resolve(seconds); },
  setStandbyDelay: (seconds: number): Promise<number> => { if (isTauri()) return invoke("set_standby_delay", { seconds }); mockPower.standbyDelay = seconds; return Promise.resolve(seconds); },
  setMicEnabled: (enabled: boolean): Promise<boolean> => { if (isTauri()) return invoke("set_mic_enabled", { enabled }); mockPower.micEnabled = false; return Promise.resolve(false); },
  setNoiseReduction: (level: 0 | 1): Promise<number> => { if (isTauri()) return invoke("set_noise_reduction", { level }); mockPower.noiseReduction = level; return Promise.resolve(level); },
  setDefaultAudioInput: (): Promise<string> => isTauri() ? invoke("set_default_audio_input") : Promise.resolve("alsa_input.usb-AU05_mock-00.analog-stereo"),
  readDeviceKeys: (): Promise<[number, [number, number][]][]> => isTauri() ? invoke("read_device_keys") : Promise.resolve(JSON.parse(JSON.stringify(FACTORY))),
  writeDeviceKeys: (): Promise<string[]> => isTauri() ? invoke("write_device_keys") : Promise.resolve(["控件 0：已写入", "控件 1：已写入"]),
  importKeymap: (): Promise<HostKeymap | null> => isTauri() ? invoke("import_keymap") : Promise.resolve(JSON.parse(JSON.stringify(memory))),
  exportKeymap: (): Promise<string | null> => isTauri() ? invoke("export_keymap") : Promise.resolve("/tmp/keymap.json"),
  loadDefaultKeymap: (): Promise<HostKeymap> => { if (isTauri()) return invoke("load_default_keymap"); memory = factoryKeymap(); return Promise.resolve(JSON.parse(JSON.stringify(memory))); },
  keymapFromDevice: (): Promise<HostKeymap> => { if (isTauri()) return invoke("keymap_from_device"); memory = factoryKeymap(); return Promise.resolve(JSON.parse(JSON.stringify(memory))); },
```

`setSettings` 的签名：`(patch: { idleMinutes?: number; closeToTray?: boolean; halo?: Partial<HaloSettings> })`。删除 `setIdleMinutes`；`Settings.svelte` 的 `changeIdle` 暂时改为 `await api.setSettings({ idleMinutes: idle })`（Task 3 会重写该文件，这里只保证编译）。`HostKeymap` 类型若要求 `version` 以外的必填字段，按 `ui/src/model/keymap.ts` 的类型补全（`controls` 的 `ControlActionMap` 只需 `index` 与 `press`）。

- [ ] **Step 4: 验证** `npm test && npm run check`。
- [ ] **Step 5: 提交** `git add ui && git commit -m "feat(ui): bridge for device writes and keymap file actions, with mocks"`。

---

### Task 3: 设置页 — 设备写入项（灯光、电源、麦克风、设备键位、连接帮助、通用）

**Files:**
- Modify: `ui/src/pages/Settings.svelte`, `ui/src/App.svelte`, `ui/src/lib/i18n.ts`
- Test: `ui/src/pages/__tests__/settings.test.svelte.ts`

**Interfaces:**
- `Settings` props：`{ snapshot: UiSnapshot | null; dirty?: boolean; hasBacklight?: boolean; onKeymapReplaced?: (map: HostKeymap) => void }`（后两个在 Task 4 用到，这里先声明）。
- 行内错误：每张卡一个 `rowError: Record<string, string>`，键为行名（`"lighting" | "sleep" | "standby" | "mic" | "noise" | "audio" | "deviceKeys"`），错误显示在该行右侧 `<span class="h bad">`。
- 灯光卡：`hasBacklight` 为真 → 模式与亮度显示为文字 + 提示 `t("由分层背光接管：各灯在键位页按 Layer 设置")`；否则分段 `[全灭][全亮][工作]`（`data-light-mode={0|1|2}`）+ `<input type="range" data-role="all-on" min=0 max=20>`，两者改动都调用 `api.setLighting({ ...info.lighting, mode, brightness })`，成功后 `info.lighting = 返回值`。
- 电源卡：休眠 `<select data-role="sleep">` 选项 `[0 关闭, 3600, 7200, 10800, 14400]`，显示 `永不 / 1 小时 … 4 小时`；待机 `<select data-role="standby">` 选项 `[60, 120, 300, 600, 1800]` 秒。
- 麦克风卡：收音 `<input class="switch" data-role="mic">`、硬件降噪 `<input class="switch" data-role="noise">`；写后用返回值回填，若与请求值不同 → `rowError.mic = t("设备未接受")`；"设为系统默认输入"按钮（`data-role="default-input"`）成功后行右侧显示源名（muted），失败红字。
- Vibe Key 卡：新增两行 —— "设备自身键位"：按钮 `写入设备`（`data-role="write-device"`，结果列表显示在行下方）与 `从设备读取`（`data-role="read-device"`，显示六行 `控件名 → entriesLabel`）；"连接帮助"：`<details>` 里三条（udev 规则路径 `/etc/udev/rules.d/70-halokey.rules`、插拔接收器、待机后按任意键唤醒）。`refresh` 按钮保留。
- 通用卡：新增 "关闭窗口时" 分段 `[留在托盘][退出]`（`data-close={"tray"|"quit"}`）→ `api.setSettings({ closeToTray })`；关于行改为 `halokey {snapshot?.version ?? "—"} · {configPath}`。
- 空闲时间改走 `api.setSettings({ idleMinutes })`。

- [ ] **Step 1: 写失败的测试（替换 settings.test.svelte.ts 全文）**

```ts
import { describe, it, expect, afterEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import Settings from "../Settings.svelte";
import { setLang } from "../../lib/i18n";
import type { UiSnapshot } from "../../lib/tauri";
import { api } from "../../lib/tauri";

let app: ReturnType<typeof mount> | undefined;
afterEach(() => { if (app) unmount(app); app = undefined; document.body.innerHTML = ""; vi.restoreAllMocks(); });
const tick = () => new Promise((r) => setTimeout(r, 0));
const settle = async () => { await tick(); await tick(); flushSync(); };
const snapshot: UiSnapshot = { connected: true, online: true, battery: [60, 3791, false], heartbeats: 12, echoes: 11, injected: 0, lastError: null, configPath: "/tmp/keymap.json", paused: false, standby: false, link: true, activeLayers: [], effectiveLighting: null, version: "0.1.0" };
const q = <T extends Element>(sel: string) => document.querySelector(sel) as T;
const change = (el: HTMLSelectElement, v: string) => { el.value = v; el.dispatchEvent(new Event("change", { bubbles: true })); };

describe("Settings page", () => {
  it("shows firmware, lighting, power and general cards from the bridge", async () => {
    setLang("zh");
    app = mount(Settings, { target: document.body, props: { snapshot } });
    await settle();
    const text = document.body.textContent ?? "";
    expect(text).toContain("4.4.2 / 4.4.2");
    expect(text).toContain("本体在线");
    expect(text).toContain("/tmp/keymap.json");
    expect(text).toContain("halokey 0.1.0");
    expect(q<HTMLSelectElement>("select[data-role=idle]").value).toBe("5");
    expect(q<HTMLSelectElement>("select[data-role=sleep]").value).toBe("3600");
    expect(q<HTMLSelectElement>("select[data-role=standby]").value).toBe("300");
  });
  it("writes global lighting unless layer backlight owns the lights", async () => {
    setLang("zh");
    const spy = vi.spyOn(api, "setLighting");
    app = mount(Settings, { target: document.body, props: { snapshot } });
    await settle();
    q<HTMLButtonElement>("button[data-light-mode='1']").click();
    await settle();
    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy.mock.calls[0]![0].mode).toBe(1);
    expect(spy.mock.calls[0]![0].lights).toHaveLength(4);
    const range = q<HTMLInputElement>("input[data-role=all-on]");
    range.value = "7"; range.dispatchEvent(new Event("change", { bubbles: true }));
    await settle();
    expect(spy.mock.calls[1]![0].brightness).toBe(7);
    unmount(app); app = undefined; document.body.innerHTML = "";
    app = mount(Settings, { target: document.body, props: { snapshot, hasBacklight: true } });
    await settle();
    expect(document.querySelector("button[data-light-mode]")).toBeNull();
    expect(document.body.textContent).toContain("由分层背光接管");
  });
  it("writes power and mic settings, and reports a rejected mic write inline", async () => {
    setLang("zh");
    const sleep = vi.spyOn(api, "setSleepDelay");
    const standby = vi.spyOn(api, "setStandbyDelay");
    const noise = vi.spyOn(api, "setNoiseReduction");
    app = mount(Settings, { target: document.body, props: { snapshot } });
    await settle();
    change(q<HTMLSelectElement>("select[data-role=sleep]"), "7200");
    change(q<HTMLSelectElement>("select[data-role=standby]"), "120");
    await settle();
    expect(sleep).toHaveBeenCalledWith(7200);
    expect(standby).toHaveBeenCalledWith(120);
    const noiseSw = q<HTMLInputElement>("input[data-role=noise]");
    noiseSw.click(); await settle();
    expect(noise).toHaveBeenCalledWith(0);
    const mic = q<HTMLInputElement>("input[data-role=mic]");
    expect(mic.checked).toBe(false);
    mic.click(); await settle();
    expect(mic.checked).toBe(false); // 读回仍为关
    expect(document.body.textContent).toContain("设备未接受");
  });
  it("sets the system default input and shows the reason when it fails", async () => {
    setLang("zh");
    app = mount(Settings, { target: document.body, props: { snapshot } });
    await settle();
    q<HTMLButtonElement>("button[data-role=default-input]").click();
    await settle();
    expect(document.body.textContent).toContain("AU05");
    vi.spyOn(api, "setDefaultAudioInput").mockRejectedValue("没有找到 Vibe Key 的麦克风");
    q<HTMLButtonElement>("button[data-role=default-input]").click();
    await settle();
    expect(document.body.textContent).toContain("没有找到 Vibe Key 的麦克风");
  });
  it("reads and writes the device's own key table", async () => {
    setLang("zh");
    app = mount(Settings, { target: document.body, props: { snapshot } });
    await settle();
    q<HTMLButtonElement>("button[data-role=read-device]").click();
    await settle();
    expect(document.body.textContent).toContain("顶部按键");
    expect(document.body.textContent).toContain("Enter");
    q<HTMLButtonElement>("button[data-role=write-device]").click();
    await settle();
    expect(document.body.textContent).toContain("控件 0：已写入");
  });
  it("writes halo and close-to-tray settings through the bridge", async () => {
    setLang("zh");
    app = mount(Settings, { target: document.body, props: { snapshot } });
    await settle();
    ([...document.querySelectorAll("button")].find((b) => b.textContent?.trim() === "大") as HTMLButtonElement).click();
    await settle();
    expect((await api.getSettings()).halo.size).toBe("large");
    change(q<HTMLSelectElement>("select[data-role=halo-timeout]"), "10");
    await settle();
    expect((await api.getSettings()).halo.timeoutSeconds).toBe(10);
    q<HTMLButtonElement>("button[data-close='quit']").click();
    await settle();
    expect((await api.getSettings()).closeToTray).toBe(false);
    q<HTMLButtonElement>("button[data-close='tray']").click();
    await settle();
    expect((await api.getSettings()).closeToTray).toBe(true);
  });
});
```

- [ ] **Step 2: 运行确认失败** → `select[data-role=sleep]` 为 null 等。

- [ ] **Step 3: 实现 Settings.svelte（替换 script 与模板；样式在现有基础上加）**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { t, lang, setLang, colon, type Lang } from "../lib/i18n";
  import { api, type UiSnapshot, type UiDeviceInfo, type UiPower, type HaloSettings, type UiLighting } from "../lib/tauri";
  import Icon from "../components/Icon.svelte";
  import { getTheme, setTheme, type Theme } from "../lib/theme";
  import { entriesLabel, CONTROL_NAMES } from "../model/labels";
  import type { HostKeymap } from "../model/keymap";

  let { snapshot, dirty = false, hasBacklight = false, onKeymapReplaced }: { snapshot: UiSnapshot | null; dirty?: boolean; hasBacklight?: boolean; onKeymapReplaced?: (map: HostKeymap) => void } = $props();
  let info = $state<UiDeviceInfo | null>(null);
  let power = $state<UiPower | null>(null);
  let idle = $state(5);
  let autostart = $state(false);
  let closeToTray = $state(true);
  let note = $state("");
  let halo = $state<HaloSettings>({ position: "pointer", timeoutSeconds: 4, size: "medium" });
  let theme = $state<Theme>(getTheme());
  /** 每行一条错误（§9：行内红字，不弹窗）。 */
  let rowError = $state<Record<string, string>>({});
  let audioSource = $state("");
  let deviceKeys = $state<[number, [number, number][]][] | null>(null);
  let writeReport = $state<string[]>([]);
  const THEMES: [Theme, string][] = [["system", "跟随系统"], ["dark", "深色"], ["light", "浅色"]];
  const SIZES: [HaloSettings["size"], string][] = [["small", "小"], ["medium", "中"], ["large", "大"]];
  const IDLE = [0, 1, 5, 10, 15, 30, 60];
  const SLEEP = [0, 3600, 7200, 10800, 14400];
  const STANDBY = [60, 120, 300, 600, 1800];
  const modeName = ["全灭", "全亮", "工作"];
  const onOff = (v: boolean | number | null | undefined) => (v == null ? "—" : v ? `● ${t("已开启")}` : `○ ${t("已关闭")}`);
  const sleepLabel = (s: number) => (s === 0 ? t("永不") : `${s / 3600} ${t("小时")}`);

  async function load() {
    try { [info, power] = await Promise.all([api.deviceInfo(), api.powerInfo()]); } catch (e) { note = String(e); }
    try { const s = await api.getSettings(); idle = s.idleMinutes; halo = s.halo; closeToTray = s.closeToTray; autostart = await api.getAutostart(); } catch { /* mock */ }
  }
  onMount(load);

  /** 跑一次设备写入：成功清掉该行错误并把读回值交给 apply，失败把错误放到该行。 */
  async function write<T>(row: string, run: () => Promise<T>, apply: (v: T) => void) {
    try { apply(await run()); rowError = { ...rowError, [row]: "" }; }
    catch (e) { rowError = { ...rowError, [row]: String(e) }; }
  }
  const setLightingGlobal = (patch: Partial<Pick<UiLighting, "mode" | "brightness">>) => {
    if (!info?.lighting) return;
    const desired = { ...info.lighting, ...patch };
    return write("lighting", () => api.setLighting(desired), (l) => { if (info) info = { ...info, lighting: l }; });
  };
  const setSleep = (s: number) => write("sleep", () => api.setSleepDelay(s), (v) => { if (power) power = { ...power, sleepDelay: v }; });
  const setStandby = (s: number) => write("standby", () => api.setStandbyDelay(s), (v) => { if (power) power = { ...power, standbyDelay: v }; });
  const setMic = (on: boolean) => write("mic", () => api.setMicEnabled(on), (v) => {
    if (power) power = { ...power, micEnabled: v };
    if (v !== on) rowError = { ...rowError, mic: t("设备未接受") };
  });
  const setNoise = (on: boolean) => write("noise", () => api.setNoiseReduction(on ? 1 : 0), (v) => { if (power) power = { ...power, noiseReduction: v }; });
  const setDefaultInput = () => write("audio", () => api.setDefaultAudioInput(), (name) => { audioSource = name; });
  const readDevice = () => write("deviceKeys", () => api.readDeviceKeys(), (k) => { deviceKeys = k; writeReport = []; });
  const writeDevice = () => write("deviceKeys", () => api.writeDeviceKeys(), (r) => { writeReport = r; deviceKeys = null; });
  async function changeIdle(e: Event) {
    idle = Number((e.target as HTMLSelectElement).value);
    try { await api.setSettings({ idleMinutes: idle }); } catch (err) { note = String(err); }
  }
  async function setHalo(patch: Partial<HaloSettings>) {
    try { halo = (await api.setSettings({ halo: patch })).halo; rowError = { ...rowError, halo: "" }; } catch (e) { rowError = { ...rowError, halo: String(e) }; }
  }
  async function setClose(tray: boolean) {
    try { closeToTray = (await api.setSettings({ closeToTray: tray })).closeToTray; } catch (e) { note = String(e); }
  }
  async function changeAutostart() {
    try { autostart = await api.setAutostart(autostart); } catch (e) { note = String(e); }
  }
</script>

<div class="page">
  <div class="ph"><h2>{t("设置")}</h2><label class="lang"><Icon name="globe" size={15} /> {t("界面语言")} <select value={lang()} onchange={(e) => setLang((e.target as HTMLSelectElement).value as Lang)}><option value="en">English</option><option value="zh">简体中文</option><!-- i18n-ignore: 语言自称 --><option value="ja">日本語</option></select></label></div>

  <section class="card">
    <h3>{t("Vibe Key")} <span class="r"><button onclick={load}>{t("刷新状态")}</button></span></h3>
    <div class="row"><span class="k">{t("状态")}</span><span class="v">{snapshot?.connected ? "● " + (snapshot.online === false ? t("本体离线") : t("本体在线")) : "○ " + t("未连接")}{#if snapshot?.battery} · <Icon name="battery" size={15} level={snapshot.battery[0]} /> {snapshot.battery[0] ?? "?"}% · {snapshot.battery[1]} mV{/if}</span><span class="h">{t("心跳")} {snapshot?.heartbeats ?? 0} · {t("回显")} {snapshot?.echoes ?? 0}</span></div>
    <div class="row"><span class="k">{t("固件")}</span><span class="v mono">{info?.firmware ?? "—"} / {info?.dongleFirmware ?? "—"}</span></div>
    <div class="row"><span class="k">{t("空闲后交还控制")}</span><span class="v"><select data-role="idle" value={String(idle)} onchange={changeIdle}>{#each IDLE as m}<option value={String(m)}>{m === 0 ? t("永不") : `${m} ${t("分钟")}`}</option>{/each}</select></span></div>
    <div class="row"><span class="k">{t("设备自身键位")}</span><span class="v"><button data-role="write-device" disabled={!snapshot?.connected} onclick={writeDevice}>{t("写入设备")}</button><button data-role="read-device" disabled={!snapshot?.connected} onclick={readDevice}>{t("从设备读取")}</button></span>{#if rowError.deviceKeys}<span class="h bad">{rowError.deviceKeys}</span>{/if}</div>
    {#if deviceKeys}
      <ul class="keys">{#each deviceKeys as [index, entries]}<li><span class="k">{t(CONTROL_NAMES[index] ?? "")}</span><span class="mono">{entriesLabel(entries.map(([type, code]) => ({ type, code })))}</span></li>{/each}</ul>
    {/if}
    {#if writeReport.length}<ul class="keys">{#each writeReport as line}<li class="mono">{line}</li>{/each}</ul>{/if}
    <details class="help"><summary>{t("连接帮助")}</summary>
      <ol>
        <li>{t("Linux 需要 udev 规则")}{colon()}<span class="mono">/etc/udev/rules.d/70-halokey.rules</span>{t("（.deb 会自动安装；手装见 packaging/linux）")}</li>
        <li>{t("装好规则后重新插拔接收器")}</li>
        <li>{t("本体待机后按任意键唤醒；电源键长按会关机")}</li>
      </ol>
    </details>
  </section>

  <div class="two">
    <section class="card">
      <h3>{t("灯光")}</h3>
      {#if hasBacklight}
        <div class="row"><span class="k">{t("灯光模式")}</span><span class="v">{info?.lighting ? t(modeName[info.lighting.mode] ?? "—") : "—"}</span></div>
        <div class="row"><span class="k">{t("全亮亮度")}</span><span class="v mono">{info?.lighting?.brightness ?? "—"} / 20</span></div>
        <div class="row"><span class="k"></span><span class="v muted">{t("由分层背光接管：各灯在键位页按 Layer 设置")}</span></div>
      {:else}
        <div class="row"><span class="k">{t("灯光模式")}</span><span class="v"><span class="seg">{#each [0, 1, 2] as m}<button data-light-mode={m} class:on={info?.lighting?.mode === m} disabled={!info?.lighting} onclick={() => setLightingGlobal({ mode: m })}>{t(modeName[m]!)}</button>{/each}</span></span>{#if rowError.lighting}<span class="h bad">{rowError.lighting}</span>{/if}</div>
        <div class="row"><span class="k">{t("全亮亮度")}</span><span class="v"><input type="range" data-role="all-on" min="0" max="20" value={info?.lighting?.brightness ?? 0} disabled={!info?.lighting} onchange={(e) => setLightingGlobal({ brightness: Number((e.target as HTMLInputElement).value) })} /><span class="mono">{info?.lighting?.brightness ?? "—"} / 20</span></span></div>
        <div class="row"><span class="k"></span><span class="v muted">{t("各灯效果在键位页按 Layer 设置")}</span></div>
      {/if}
    </section>
    <section class="card">
      <h3>{t("麦克风")}</h3>
      <div class="row"><span class="k">{t("收音")}</span><span class="v"><input class="switch" type="checkbox" data-role="mic" aria-label={t("收音")} checked={!!power?.micEnabled} disabled={!power} onchange={(e) => setMic((e.target as HTMLInputElement).checked)}></span>{#if rowError.mic}<span class="h bad">{rowError.mic}</span>{/if}</div>
      <div class="row"><span class="k">{t("硬件降噪")}</span><span class="v"><input class="switch" type="checkbox" data-role="noise" aria-label={t("硬件降噪")} checked={!!power?.noiseReduction} disabled={!power} onchange={(e) => setNoise((e.target as HTMLInputElement).checked)}></span>{#if rowError.noise}<span class="h bad">{rowError.noise}</span>{/if}</div>
      <div class="row"><span class="k">{t("系统默认输入")}</span><span class="v"><button data-role="default-input" onclick={setDefaultInput}>{t("设为系统默认输入")}</button></span>{#if rowError.audio}<span class="h bad">{rowError.audio}</span>{:else if audioSource}<span class="h mono">{audioSource}</span>{/if}</div>
    </section>
  </div>

  <div class="two">
    <section class="card">
      <h3>{t("电源")}</h3>
      <div class="row"><span class="k">{t("休眠 / 自动关机")}</span><span class="v"><select data-role="sleep" value={String(power?.sleepDelay ?? 3600)} disabled={!power} onchange={(e) => setSleep(Number((e.target as HTMLSelectElement).value))}>{#each SLEEP as s}<option value={String(s)}>{sleepLabel(s)}</option>{/each}</select></span>{#if rowError.sleep}<span class="h bad">{rowError.sleep}</span>{/if}</div>
      <div class="row"><span class="k">{t("待机延时")}</span><span class="v"><select data-role="standby" value={String(power?.standbyDelay ?? 300)} disabled={!power} onchange={(e) => setStandby(Number((e.target as HTMLSelectElement).value))}>{#each STANDBY as s}<option value={String(s)}>{s} {t("秒")}</option>{/each}</select></span>{#if rowError.standby}<span class="h bad">{rowError.standby}</span>{/if}</div>
    </section>
    <section class="card">
      <h3>{t("光环转盘")}</h3>
      <div class="row"><span class="k">{t("出现位置")}</span><span class="v"><span class="seg">
        <button class:on={halo.position === "pointer"} onclick={() => setHalo({ position: "pointer" })}>{t("指针周围")}</button>
        <button class:on={halo.position === "center"} onclick={() => setHalo({ position: "center" })}>{t("屏幕中央")}</button></span></span></div>
      <div class="row"><span class="k">{t("自动关闭")}</span><span class="v"><select data-role="halo-timeout" value={String(halo.timeoutSeconds)} onchange={(e) => setHalo({ timeoutSeconds: Number((e.target as HTMLSelectElement).value) })}>{#each [2, 4, 6, 10] as s}<option value={String(s)}>{s} {t("秒")}</option>{/each}</select></span></div>
      <div class="row"><span class="k">{t("大小")}</span><span class="v"><span class="seg">
        {#each SIZES as [v, name]}<button class:on={halo.size === v} onclick={() => setHalo({ size: v })}>{t(name)}</button>{/each}</span></span>{#if rowError.halo}<span class="h bad">{rowError.halo}</span>{/if}</div>
    </section>
  </div>

  <!-- Task 4 在这里插入"键位配置"卡 -->

  <section class="card">
    <h3>{t("通用")}</h3>
    <div class="row"><span class="k">{t("登录时启动")}</span><span class="v"><input class="switch" type="checkbox" data-role="autostart" aria-label={t("登录时启动")} bind:checked={autostart} onchange={changeAutostart}></span></div>
    <div class="row"><span class="k">{t("关闭窗口时")}</span><span class="v"><span class="seg">
      <button data-close="tray" class:on={closeToTray} onclick={() => setClose(true)}>{t("留在托盘")}</button>
      <button data-close="quit" class:on={!closeToTray} onclick={() => setClose(false)}>{t("退出")}</button></span></span></div>
    <div class="row"><span class="k">{t("外观")}</span><span class="v"><span class="seg">
      {#each THEMES as [v, name]}<button data-theme-choice={v} class:on={theme === v} onclick={() => { theme = v; setTheme(v); }}>{t(name)}</button>{/each}</span></span></div>
  </section>
  {#if note}<p class="bad">{note}</p>{/if}
  <p class="about mono">halokey {snapshot?.version ?? "—"} · {snapshot?.configPath ?? "—"}</p>
</div>
```

样式追加：`.keys { list-style: none; margin: 4px 0 6px 154px; padding: 0; display: flex; flex-direction: column; gap: 2px; font-size: 13px; } .keys li { display: flex; gap: 14px; } .keys .k { width: 100px; } .help { margin-top: 8px; font-size: 13px; color: var(--muted); } .help summary { cursor: pointer; } .help ol { margin: 6px 0 0; padding-left: 20px; } .h.bad { color: var(--bad); } .v input[type="range"] { width: 160px; }`；`.card h3 .r button` 已有描边样式。

`App.svelte`：`<Settings {snapshot} dirty={draft.dirty} hasBacklight={!!draft.map?.lighting || (draft.map?.layers ?? []).some((l) => !!l.lighting)} onKeymapReplaced={(m) => { draft = loadSaved(initialDraft(), m); }} />`。

i18n（EN / JA 各一份，键相同）：`"设备自身键位": "Device key table" / "デバイス側のキー表"`, `"写入设备": "Write to device" / "デバイスに書き込む"`, `"从设备读取": "Read from device" / "デバイスから読む"`, `"连接帮助": "Connection help" / "接続のヘルプ"`, `"Linux 需要 udev 规则": "Linux needs a udev rule" / "Linux では udev ルールが必要です"`, `"（.deb 会自动安装；手装见 packaging/linux）": " (the .deb installs it; see packaging/linux for manual setup)" / "（.deb が自動で入れます。手動は packaging/linux 参照）"`, `"装好规则后重新插拔接收器": "Replug the receiver after installing the rule" / "ルールを入れたらレシーバーを挿し直す"`, `"本体待机后按任意键唤醒；电源键长按会关机": "Press any key to wake the unit from standby; holding the power key turns it off" / "スタンバイ中は任意のキーで復帰。電源キー長押しは電源オフ"`, `"由分层背光接管：各灯在键位页按 Layer 设置": "Owned by layer backlight: set each light per layer on the Keymap page" / "レイヤーのバックライトが管理中：各ライトはキーマップページでレイヤーごとに設定"`, `"设备未接受": "Device did not accept it" / "デバイスが受け付けませんでした"`, `"系统默认输入": "System default input" / "システム既定の入力"`, `"设为系统默认输入": "Set as default input" / "既定の入力にする"`, `"小时": "h" / "時間"`, `"关闭窗口时": "On window close" / "ウィンドウを閉じたとき"`, `"留在托盘": "Stay in tray" / "トレイに残す"`, `"退出": "Quit" / "終了"`。删除不再使用的 `"其余写入项在后续计划中补全"`（EN / JA 都删）。

- [ ] **Step 4: 验证** `npm test && npm run check && npm run build`；浏览器 mock 里把灯光模式点成"全亮"、休眠改 2 小时、收音开关点一下看到红字。
- [ ] **Step 5: 提交** `git add ui && git commit -m "feat(ui): settings page writes lighting, power, mic, device keys and close behaviour"`。

---

### Task 4: 设置页 — 键位配置卡（导入 / 导出 / 载入默认 / 从设备生成）

**Files:**
- Modify: `ui/src/pages/Settings.svelte`, `ui/src/lib/i18n.ts`
- Test: `ui/src/pages/__tests__/settings.test.svelte.ts`（追加）

**Interfaces（Consumes）:** Task 2 的 `api.importKeymap / exportKeymap / loadDefaultKeymap / keymapFromDevice`；Task 3 的 props `dirty / onKeymapReplaced`。

- 卡片"键位配置"：摘要行（`当前文件` 路径 mono）；操作行四个按钮：`导入…`（`data-role="import"`）、`导出…`（`data-role="export"`）、`载入默认键位`（`data-role="load-default"`）、`从设备键位生成`（`data-role="from-device"`）。`dirty` 为真时 导入 / 载入默认 / 从设备 三个禁用，右侧提示 `t("先保存或撤销键位页的改动")`；导出不受影响。
- 结果：导入 / 默认 / 设备成功 → `onKeymapReplaced(map)` + 行内 muted `t("已替换键位文件")`；取消（`null`）→ 什么都不显示；失败 → 行内红字。导出成功 → muted `已导出到 {path}`；取消 → 无提示。

- [ ] **Step 1: 写失败的测试（追加到 settings.test.svelte.ts）**

```ts
  it("replaces the keymap file from import / default / device, and guards unsaved edits", async () => {
    setLang("zh");
    const replaced: unknown[] = [];
    app = mount(Settings, { target: document.body, props: { snapshot, onKeymapReplaced: (m: unknown) => replaced.push(m) } });
    await settle();
    q<HTMLButtonElement>("button[data-role=load-default]").click();
    await settle();
    expect(replaced).toHaveLength(1);
    expect(document.body.textContent).toContain("已替换键位文件");
    vi.spyOn(api, "importKeymap").mockResolvedValue(null);
    q<HTMLButtonElement>("button[data-role=import]").click();
    await settle();
    expect(replaced).toHaveLength(1);
    vi.spyOn(api, "importKeymap").mockRejectedValue("键位文件需要六个控件");
    q<HTMLButtonElement>("button[data-role=import]").click();
    await settle();
    expect(replaced).toHaveLength(1);
    expect(document.body.textContent).toContain("六个控件");
    vi.spyOn(api, "exportKeymap").mockResolvedValue(null);
    q<HTMLButtonElement>("button[data-role=export]").click();
    await settle();
    expect(document.body.textContent).not.toContain("已导出");
    vi.spyOn(api, "exportKeymap").mockResolvedValue("/home/u/keymap.json");
    q<HTMLButtonElement>("button[data-role=export]").click();
    await settle();
    expect(document.body.textContent).toContain("/home/u/keymap.json");
    unmount(app); app = undefined; document.body.innerHTML = "";
    app = mount(Settings, { target: document.body, props: { snapshot, dirty: true } });
    await settle();
    expect(q<HTMLButtonElement>("button[data-role=import]").disabled).toBe(true);
    expect(q<HTMLButtonElement>("button[data-role=from-device]").disabled).toBe(true);
    expect(q<HTMLButtonElement>("button[data-role=export]").disabled).toBe(false);
    expect(document.body.textContent).toContain("先保存或撤销键位页的改动");
  });
```

- [ ] **Step 2: 运行确认失败** → `button[data-role=load-default]` 为 null。

- [ ] **Step 3: 实现**

script 追加：

```ts
  let fileNote = $state("");
  let fileError = $state("");
  async function replaceWith(run: () => Promise<HostKeymap | null>) {
    fileNote = ""; fileError = "";
    try {
      const map = await run();
      if (!map) return;
      onKeymapReplaced?.(map);
      fileNote = t("已替换键位文件");
    } catch (e) { fileError = String(e); }
  }
  async function exportFile() {
    fileNote = ""; fileError = "";
    try { const p = await api.exportKeymap(); if (p) fileNote = `${t("已导出到")} ${p}`; } catch (e) { fileError = String(e); }
  }
```

模板在"通用"卡之前插入：

```svelte
  <section class="card">
    <h3>{t("键位配置")}</h3>
    <div class="row"><span class="k">{t("当前文件")}</span><span class="v mono">{snapshot?.configPath ?? "—"}</span></div>
    <div class="row"><span class="k"></span><span class="v wrap">
      <button data-role="import" disabled={dirty} onclick={() => replaceWith(api.importKeymap)}>{t("导入…")}</button>
      <button data-role="export" onclick={exportFile}>{t("导出…")}</button>
      <button data-role="load-default" disabled={dirty} onclick={() => replaceWith(api.loadDefaultKeymap)}>{t("载入默认键位")}</button>
      <button data-role="from-device" disabled={dirty || !snapshot?.connected} onclick={() => replaceWith(api.keymapFromDevice)}>{t("从设备键位生成")}</button>
    </span>{#if dirty}<span class="h">{t("先保存或撤销键位页的改动")}</span>{:else if fileError}<span class="h bad">{fileError}</span>{:else if fileNote}<span class="h">{fileNote}</span>{/if}</div>
  </section>
```

样式：`.v.wrap { flex-wrap: wrap; }`。i18n：`"导入…": "Import…" / "インポート…"`, `"导出…": "Export…" / "エクスポート…"`, `"载入默认键位": "Load default keymap" / "既定のキーマップを読み込む"`, `"从设备键位生成": "Build from device keys" / "デバイスのキー表から作る"`, `"先保存或撤销键位页的改动": "Save or discard the Keymap page's changes first" / "先にキーマップページの変更を保存するか破棄してください"`, `"已替换键位文件": "Keymap file replaced" / "キーマップファイルを置き換えました"`, `"已导出到": "Exported to" / "エクスポート先"`。

- [ ] **Step 4: 验证** `npm test && npm run check && npm run build`。
- [ ] **Step 5: 提交** `git add ui && git commit -m "feat(ui): keymap file card with import, export, default and device-derived keymaps"`。

---

### Task 5: 文档与截图

**Files:**
- Modify: `docs/03-config-format.md`, `docs/03-config-format.zh.md`, `README.md`, `README.zh.md`
- Create: `docs/screenshots/keymap-en.png`, `docs/screenshots/keymap-zh.png`, `docs/screenshots/settings-en.png`

- [ ] **Step 1: 截图**（Vite 开发服务器 5177，mock 数据，深色，视口 1100×760）：用 Playwright 打开 `http://127.0.0.1:5177/`，`localStorage.setItem("halokey.lang","en")` 后刷新，键位页截图存 `docs/screenshots/keymap-en.png`；切到设置页截 `settings-en.png`；再把语言设为 zh 截 `keymap-zh.png`。文件用 `pngquant`（若没装就原样）控制在 200 KB 内。

- [ ] **Step 2: README**：`## Try it now` 段落开头各插一张图（英文版 `![Keymap page](docs/screenshots/keymap-en.png)`，中文版 `![键位页](docs/screenshots/keymap-zh.png)`）；同段末尾加一句：EN "The Settings page writes lighting, power, microphone and the device's own key table directly, and can import / export the keymap file or rebuild it from the device." / ZH "设置页可直接写灯光、电源、麦克风与设备自身键位，也能导入 / 导出键位文件或从设备重建。"

- [ ] **Step 3: 配置格式文档**：`docs/03-config-format.md` 末尾新增 `## 6. settings.json`：

```markdown
## 6. settings.json

Application preferences live next to the keymap in `settings.json` and never enter the keymap file:

| Key | Value | Default |
|---|---|---|
| `idleMinutes` | minutes of inactivity before halokey hands the device back to its own firmware; `0` = never | `5` |
| `halo.position` | `pointer` or `center` | `pointer` |
| `halo.timeoutSeconds` | seconds before the halo wheel closes | `4` |
| `halo.size` | `small`, `medium`, `large` | `medium` |
| `closeToTray` | closing the settings window hides it to the tray (`true`) or quits (`false`) | `true` |

Language and appearance are per-machine browser preferences, not part of this file.
```

中文版同结构（表头 `键 / 值 / 默认`，最后一句"界面语言与外观是本机偏好，不进这个文件。"）。

- [ ] **Step 4: 校验** `python3 tools/check_docs.py` 全过（相对链接要能解析：截图文件先于文档提交）。
- [ ] **Step 5: 提交** `git add docs README.md README.zh.md && git commit -m "docs: settings.json reference, README screenshots of the new UI"`。

---

### Task 6: 版本 0.1.0、.deb 与真机清单

**Files:**
- Modify: `Cargo.toml`（workspace `version = "0.1.0"`）、`apps/halokey/tauri.conf.json`（`"version": "0.1.0"`）、`ui/src/lib/tauri.ts` mock `version` 已是 0.1.0

- [ ] **Step 1: 版本号** 两处改 0.1.0；`cargo build -p halokey-app` 让 `Cargo.lock` 更新；`cargo test --workspace`。
- [ ] **Step 2: 打包** `cd ui && npm run build && cd .. && cargo tauri build --bundles deb 2>&1 | tail -5` → `target/release/bundle/deb/halokey_0.1.0_arm64.deb`；`dpkg-deb -c` 里确认有 `usr/lib/udev/rules.d/70-halokey.rules` 与 `usr/bin/halokey-app`（或 `halokey`，以实际为准）。
- [ ] **Step 3: 提交并推送** `git add -A && git commit -m "chore: release 0.1.0" && git push origin main`。
- [ ] **Step 4: 真机清单（本机 GNOME/X11；需要用户配合的标 👤）**

1. 设置页 → 灯光模式点"全亮"：三盏可见灯常亮；亮度拖到 5：明显变暗；点"工作"：恢复按层效果。再给 Layer 1 加一条背光保存 → 灯光两行变只读并显示"由分层背光接管"。
2. 休眠改 2 小时、待机改 120 秒：刷新状态后读回一致。
3. 收音开关点开：行右侧红字"设备未接受"，开关回到关（硬件行为）；硬件降噪关 / 开各一次读回一致。
4. "设为系统默认输入"：行右侧出现 `alsa_input.usb-AU05_…`，`pactl get-default-source` 一致；本体关机后再点：红字"没有找到 Vibe Key 的麦克风"。
5. "从设备读取"：六行与出厂值（无效码 / Enter / Esc / Print Screen / → / ⌫）一致；"写入设备"：报告六行，之后退出 halokey 直接按键（设备自身映射）与键位页基础层一致。
6. 导出… 到 `~/Desktop/keymap-test.json`，文件可读；把文件里 `version` 改成 99 再导入… → 红字拒绝、原文件不变；改回后导入成功，键位页刷新。
7. 键位页改一个键不保存 → 设置页三个替换按钮禁用并提示；撤销后恢复。
8. 载入默认键位 → 键位页六个控件回到出厂；从设备键位生成 → 同上（设备表 = 出厂）。
9. 关闭窗口时选"退出" → 关窗后托盘图标消失、进程退出；改回"留在托盘"。
10. 👤 `sudo dpkg -i target/release/bundle/deb/halokey_0.1.0_arm64.deb`，从应用菜单启动 halokey，托盘出现、设备连上、`ls /usr/lib/udev/rules.d/70-halokey.rules` 存在；关于行显示 0.1.0。

---

## 自检记录

- 规格覆盖：§6 七张卡 —— Vibe Key（状态 / 固件 / 空闲 / 设备键位 / 连接帮助 / 刷新）→ Task 3；"断开连接"运行时没有对应命令，**不做**（记入 ruling）；灯光 → Task 3（含 §7 待决结论）；麦克风三项 → Task 3 + Task 1；电源 → Task 3；光环转盘已在第 3 期；键位配置四项 → Task 1 + 4；通用（登录启动已有、关闭窗口 → Task 1 + 3、主题已在设计审查里做、关于 → Task 3；"打开日志"没有日志文件，**不做**）；§8 新命令 → Task 1；§9 行内红字 → Task 3/4；§11 清理：旧 UI 已不存在、`check_docs` 已跳过 `superpowers/`、示例已含 `lighting`，剩余 README 截图 → Task 5；§12 第 5 期剩余项 + 第 6 期 → 本计划。
- 类型一致性：`api.*` 名称在 Task 2 定义、Task 3/4 同名；`AppSettings.closeToTray` Rust camelCase 与 TS 一致；`UiSnapshot.version` 两侧同名；`Settings` props `dirty / hasBacklight / onKeymapReplaced` 在 Task 3 声明、Task 4 与 App 同用；`rowError` 键名在模板与函数一致。
- Review Focus 五项：1 → Task 1 校验先于写入 + Task 4 拒绝测试；2 → Task 3 mic 测试；3 → Task 3 `hasBacklight` 测试；4 → Task 1 `pick_vibe_source` None 测试 + Task 3 拒绝测试；5 → Task 1 `Ok(None)` + Task 4 导出取消测试。
- 无自动化测试的部分：对话框本体、`pactl` 调用、`.deb` 安装 —— 真机清单 4、6、10。
