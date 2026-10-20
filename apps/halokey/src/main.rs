//! halokey 桌面应用：托盘、设置窗口、光环悬浮层。设备逻辑全部在 `halokey-runtime` 的线程里。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::sync::Mutex;

use halokey_host::{HostAction, HostKeymap, HostProfile};
use halokey_proto::{IndicatorLight, Lighting};
use halokey_runtime::{Backends, Command, Config, DeviceInfo, Event, PowerInfo, Service, Snapshot};
use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State};

/// 应用状态：设备服务与配置路径。
struct App {
    service: Mutex<Option<Service>>,
    config_path: PathBuf,
}

#[derive(Serialize, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum UiEvent {
    Connected,
    Disconnected,
    Key {
        control: u8,
        name: String,
        pressed: bool,
        at: f64,
    },
    Battery {
        percent: Option<u8>,
        millivolts: u16,
        charging: Option<bool>,
    },
    HaloShow {
        halo: halokey_host::Halo,
        selected: usize,
        /// 转盘外径像素。
        size: u32,
    },
    HaloSelect {
        selected: usize,
    },
    HaloHide,
    Error {
        text: String,
    },
    Paused,
    Resumed,
    Stopped,
    Layers {
        active: Vec<u8>,
    },
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UiSnapshot {
    connected: bool,
    online: Option<bool>,
    battery: Option<(Option<u8>, u16, Option<bool>)>,
    heartbeats: u64,
    echoes: u64,
    injected: u64,
    last_error: Option<String>,
    config_path: String,
    paused: bool,
    standby: Option<bool>,
    link: Option<bool>,
    idle_minutes: u64,
    /// 当前按住的 MO 层，从底到顶。
    active_layers: Vec<u8>,
    effective_lighting: Option<halokey_host::LayerLighting>,
    /// 应用版本（Cargo.toml），设置页"关于"行显示。
    version: String,
}

impl UiSnapshot {
    fn from(s: Snapshot, config_path: &std::path::Path) -> UiSnapshot {
        UiSnapshot {
            connected: s.connected,
            online: s.online,
            battery: s.battery.map(|b| (b.percent, b.millivolts, b.charging)),
            heartbeats: s.heartbeats,
            echoes: s.echoes,
            injected: s.injected,
            last_error: s.last_error,
            config_path: config_path.display().to_string(),
            paused: s.paused,
            standby: s.standby,
            link: s.link,
            idle_minutes: 0,
            active_layers: s.active_layers,
            effective_lighting: s.effective_lighting,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

fn config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("halokey").join("keymap.json")
}

fn load_keymap(path: &std::path::Path) -> Result<Option<HostKeymap>, String> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    match HostProfile::from_json(&bytes) {
        Ok(profile) => Ok(Some(profile.keymap)),
        Err(_) => HostKeymap::from_json(&bytes)
            .map(Some)
            .map_err(|e| e.to_string()),
    }
}

/// `HALOKEY_DEBUG=1` 时把每个按键事件与手势过渡打到 stderr（排障用）。
fn debug_log() -> bool {
    std::env::var_os("HALOKEY_DEBUG").is_some()
}

fn forward(app: &AppHandle, event: Event) {
    let ui = match event {
        Event::Connected => UiEvent::Connected,
        Event::Disconnected => UiEvent::Disconnected,
        Event::Key(e, at) => {
            if debug_log() {
                eprintln!(
                    "[halokey] 按键事件：{} pressed={} at={at:.3}",
                    e.control.name(),
                    e.pressed
                );
            }
            UiEvent::Key {
                control: e.control.index(),
                name: e.control.name().to_string(),
                pressed: e.pressed,
                at,
            }
        }
        Event::Transitions(ts) => {
            if debug_log() {
                eprintln!("[halokey] 手势过渡：{ts:?}");
            }
            return;
        }
        Event::Battery(b) => UiEvent::Battery {
            percent: b.percent,
            millivolts: b.millivolts,
            charging: b.charging,
        },
        Event::HaloShow(halo, selected) => {
            let settings = load_settings(&app.state::<App>().config_path);
            let size = settings.halo.size_px();
            if let Some(w) = app.get_webview_window("halo") {
                // 尺寸是网页里的 CSS 像素 = 逻辑像素；指针位置是物理像素，按缩放换算。
                let _ = w.set_size(tauri::LogicalSize::new(
                    f64::from(size),
                    f64::from(size + HALO_HINT_HEIGHT),
                ));
                let scale = w.scale_factor().unwrap_or(1.0);
                // 按设置围着鼠标指针出现；拿不到指针位置或设置为居中时放屏幕中央。
                let placed = settings.halo.position == HaloPosition::Pointer
                    && match app.cursor_position() {
                        Ok(pos) => {
                            let (x, y) = halo_window_origin(pos.x, pos.y, size, scale);
                            w.set_position(tauri::PhysicalPosition::new(x, y)).is_ok()
                        }
                        Err(_) => false,
                    };
                if !placed {
                    let _ = w.center();
                }
                let _ = w.show();
            }
            UiEvent::HaloShow {
                halo,
                selected,
                size,
            }
        }
        Event::HaloSelect(selected) => UiEvent::HaloSelect { selected },
        Event::HaloHide => {
            if let Some(w) = app.get_webview_window("halo") {
                let _ = w.hide();
            }
            UiEvent::HaloHide
        }
        Event::Error(text) => {
            eprintln!("[halokey] 运行时错误：{text}");
            UiEvent::Error { text }
        }
        Event::Paused => {
            eprintln!("[halokey] 空闲，已交还控制");
            UiEvent::Paused
        }
        Event::Resumed => {
            eprintln!("[halokey] 设备活动，重新接管");
            UiEvent::Resumed
        }
        Event::Stopped => UiEvent::Stopped,
        Event::Layers(active) => {
            if debug_log() {
                eprintln!("[halokey] 活动层：{active:?}");
            }
            UiEvent::Layers { active }
        }
    };
    let _ = app.emit("halokey", ui);
}

#[tauri::command]
fn snapshot(state: State<'_, App>) -> Result<UiSnapshot, String> {
    let guard = state.service.lock().map_err(|e| e.to_string())?;
    let service = guard.as_ref().ok_or("服务未运行")?;
    Ok(UiSnapshot::from(service.snapshot(), &state.config_path))
}

#[tauri::command]
fn get_keymap(state: State<'_, App>) -> Result<serde_json::Value, String> {
    let guard = state.service.lock().map_err(|e| e.to_string())?;
    let service = guard.as_ref().ok_or("服务未运行")?;
    let map = service
        .snapshot()
        .keymap
        .ok_or("还没有键位（等待接收器）")?;
    let bytes = map.to_json().map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

/// 当前顶层窗口列表，供 APP 编辑器挑选；拿不到或不全时 `error` 给原因而不是失败。
/// X11 往返是同步的，放到阻塞线程池里，不占 async 运行时的工作线程。
#[tauri::command]
async fn list_windows() -> halokey_wm::WindowList {
    tauri::async_runtime::spawn_blocking(halokey_wm::list_windows)
        .await
        .unwrap_or_else(|e| halokey_wm::WindowList { windows: Vec::new(), error: Some(e.to_string()) })
}

/// 只校验不保存，供界面在保存前显示错误。
#[tauri::command]
fn validate_keymap(keymap: serde_json::Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(&keymap).map_err(|e| e.to_string())?;
    HostKeymap::from_json(&bytes)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_keymap(state: State<'_, App>, keymap: serde_json::Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(&keymap).map_err(|e| e.to_string())?;
    let map = HostKeymap::from_json(&bytes).map_err(|e| e.to_string())?;
    for c in &map.controls {
        eprintln!(
            "[halokey] 保存键位：控件 {} 单击 = {:?}",
            c.index,
            c.effective_press()
        );
    }
    install_keymap(&state, map).map(|_| ())
}

/// 退出：先停设备线程（松开按住的键、把控制交还设备），再退出进程。托盘"退出"与关窗退出都走这里。
fn quit_app(app: &AppHandle) {
    if let Some(service) = app
        .state::<App>()
        .service
        .lock()
        .ok()
        .and_then(|mut g| g.take())
    {
        service.stop();
    }
    app.exit(0);
}

/// 把一份已校验的键位写到配置文件（归一化 JSON，版本号由内容决定）并下发运行时；
/// 保存、导入、载入默认、从设备生成都走这里。
fn install_keymap(state: &App, map: HostKeymap) -> Result<serde_json::Value, String> {
    let normalized: serde_json::Value =
        serde_json::from_slice(&map.to_json().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let pretty = serde_json::to_vec_pretty(&normalized).map_err(|e| e.to_string())?;
    // 先确认服务还在，再落盘，尽量不出现"文件换了、运行时没换"的半失败。
    let guard = state.service.lock().map_err(|e| e.to_string())?;
    let service = guard.as_ref().ok_or("服务未运行，文件未改动")?;
    if let Some(dir) = state.config_path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&state.config_path, pretty)
        .map_err(|e| format!("写入 {} 失败：{e}", state.config_path.display()))?;
    if !service.send(Command::SetKeymap(map)) {
        return Err("文件已替换，但设备线程已退出；重启 halokey 后生效".into());
    }
    Ok(normalized)
}

/// 导入键位文件：对话框选 JSON → Rust 校验 → 替换配置文件并下发。取消返回 `None`。
#[tauri::command]
async fn import_keymap(
    app: AppHandle,
    state: State<'_, App>,
) -> Result<Option<serde_json::Value>, String> {
    use tauri_plugin_dialog::DialogExt;
    let handle = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .add_filter("JSON", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(file) = picked else { return Ok(None) };
    let path = file.into_path().map_err(|e| e.to_string())?;
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if size > 1 << 20 {
        return Err(format!("{} 太大（{} 字节），不像键位文件", path.display(), size));
    }
    let bytes =
        std::fs::read(&path).map_err(|e| format!("读取 {} 失败：{e}", path.display()))?;
    let map = HostKeymap::from_json(&bytes).map_err(|e| e.to_string())?;
    install_keymap(&state, map).map(Some)
}

/// 导出键位文件：保存对话框 → 原样拷贝当前配置文件。取消返回 `None`。
#[tauri::command]
async fn export_keymap(app: AppHandle, state: State<'_, App>) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let handle = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .add_filter("JSON", &["json"])
            .set_file_name("keymap.json")
            .blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(file) = picked else { return Ok(None) };
    let target = file.into_path().map_err(|e| e.to_string())?;
    if !state.config_path.exists() {
        return Err("还没有键位文件：先在键位页保存一次".into());
    }
    std::fs::copy(&state.config_path, &target)
        .map_err(|e| format!("写入 {} 失败：{e}", target.display()))?;
    Ok(Some(target.display().to_string()))
}

/// 出厂六键的主机映射替换当前文件。
#[tauri::command]
fn load_default_keymap(state: State<'_, App>) -> Result<serde_json::Value, String> {
    install_keymap(&state, HostKeymap::default_keymap())
}

/// 读设备六格表，生成基础映射替换当前文件。
#[tauri::command]
async fn keymap_from_device(state: State<'_, App>) -> Result<serde_json::Value, String> {
    let replies = ask(&state, Command::ReadKeys, 12)??;
    let bindings: Vec<(u8, Vec<halokey_host::KeyEntry>)> = replies
        .into_iter()
        .map(|r| {
            (
                r.index,
                r.entries
                    .iter()
                    .map(|e| halokey_host::KeyEntry {
                        kind: e.kind.byte(),
                        code: e.code,
                    })
                    .collect(),
            )
        })
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

/// `pactl list sources short` 的结果判定：连不上声音服务要说清楚，别冒充"没找到麦克风"。
fn vibe_source_from_pactl(ok: bool, stdout: &str, stderr: &str) -> Result<String, String> {
    if !ok {
        return Err(format!("pactl 失败：{}", stderr.trim()));
    }
    pick_vibe_source(stdout).ok_or_else(|| "没有找到 Vibe Key 的麦克风（本体开着、收音打开了吗？）".to_string())
}

/// Linux：把 Vibe Key 设为系统默认输入（PipeWire / PulseAudio 都认 pactl）。
#[tauri::command]
async fn set_default_audio_input() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let out = std::process::Command::new("pactl")
            .args(["list", "sources", "short"])
            .output()
            .map_err(|e| format!("找不到 pactl：{e}"))?;
        let name = vibe_source_from_pactl(
            out.status.success(),
            &String::from_utf8_lossy(&out.stdout),
            &String::from_utf8_lossy(&out.stderr),
        )?;
        let out = std::process::Command::new("pactl")
            .args(["set-default-source", &name])
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!(
                "pactl set-default-source 失败：{}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(name)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 设备六格表：`(index, [(type, code)])`。
type DeviceKeys = Vec<(u8, Vec<(u8, u8)>)>;

#[tauri::command]
async fn read_device_keys(state: State<'_, App>) -> Result<DeviceKeys, String> {
    let replies = ask(&state, Command::ReadKeys, 12)??;
    Ok(replies
        .into_iter()
        .map(|r| {
            (
                r.index,
                r.entries.iter().map(|e| (e.kind.byte(), e.code)).collect(),
            )
        })
        .collect())
}

#[derive(Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UiLight {
    kind: u8,
    work_time: u8,
    breathe_level: u8,
    breathe_brightness: u8,
    always_on_brightness: u8,
}

#[derive(Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UiLighting {
    mode: u8,
    brightness: u8,
    lights: Vec<UiLight>,
}

impl From<Lighting> for UiLighting {
    fn from(l: Lighting) -> UiLighting {
        UiLighting {
            mode: l.mode,
            brightness: l.brightness,
            lights: l
                .lights
                .iter()
                .map(|x| UiLight {
                    kind: x.kind,
                    work_time: x.work_time,
                    breathe_level: x.breathe_level,
                    breathe_brightness: x.breathe_brightness,
                    always_on_brightness: x.always_on_brightness,
                })
                .collect(),
        }
    }
}

impl TryFrom<UiLighting> for Lighting {
    type Error = String;
    fn try_from(u: UiLighting) -> Result<Lighting, String> {
        if u.lights.len() != 4 {
            return Err("指示灯参数必须有四盏灯".into());
        }
        let light = |x: &UiLight| IndicatorLight {
            kind: x.kind,
            work_time: x.work_time,
            breathe_level: x.breathe_level,
            breathe_brightness: x.breathe_brightness,
            always_on_brightness: x.always_on_brightness,
        };
        Ok(Lighting {
            mode: u.mode,
            brightness: u.brightness,
            lights: [
                light(&u.lights[0]),
                light(&u.lights[1]),
                light(&u.lights[2]),
                light(&u.lights[3]),
            ],
        })
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UiDeviceInfo {
    firmware: Option<String>,
    dongle_firmware: Option<String>,
    noise_reduction: Option<[u8; 4]>,
    lighting: Option<UiLighting>,
    errors: Vec<String>,
}

impl From<DeviceInfo> for UiDeviceInfo {
    fn from(i: DeviceInfo) -> UiDeviceInfo {
        UiDeviceInfo {
            firmware: i.firmware,
            dongle_firmware: i.dongle_firmware,
            noise_reduction: i.noise_reduction,
            lighting: i.lighting.map(UiLighting::from),
            errors: i.errors,
        }
    }
}

#[tauri::command]
async fn device_info(state: State<'_, App>) -> Result<UiDeviceInfo, String> {
    ask(&state, Command::QueryInfo, 12).map(UiDeviceInfo::from)
}

#[tauri::command]
async fn set_lighting(state: State<'_, App>, lighting: UiLighting) -> Result<UiLighting, String> {
    let desired = Lighting::try_from(lighting)?;
    let result = ask(&state, |tx| Command::SetLighting(desired, tx), 20)??;
    Ok(result.into())
}

#[derive(Serialize, serde::Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
enum HaloPosition {
    Pointer,
    Center,
}

#[derive(Serialize, serde::Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
enum HaloSize {
    Small,
    Medium,
    Large,
}

#[derive(Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
struct HaloSettings {
    position: HaloPosition,
    timeout_seconds: u64,
    size: HaloSize,
}

impl Default for HaloSettings {
    fn default() -> Self {
        HaloSettings {
            position: HaloPosition::Pointer,
            timeout_seconds: 4,
            size: HaloSize::Medium,
        }
    }
}

impl HaloSettings {
    /// 转盘外径像素：小 / 中 / 大。
    fn size_px(&self) -> u32 {
        match self.size {
            HaloSize::Small => 280,
            HaloSize::Medium => 360,
            HaloSize::Large => 440,
        }
    }
}

/// 转盘窗口在环下面还有一行提示。
const HALO_HINT_HEIGHT: u32 = 28;

/// 转盘窗口的物理坐标左上角：指针（物理像素）减去窗口逻辑尺寸 × 缩放的一半，不出屏幕左上角。
fn halo_window_origin(cursor_x: f64, cursor_y: f64, size: u32, scale: f64) -> (f64, f64) {
    let half = f64::from(size) * scale / 2.0;
    ((cursor_x - half).max(0.0), (cursor_y - half).max(0.0))
}

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
        AppSettings {
            idle_minutes: 5,
            halo: HaloSettings::default(),
            close_to_tray: true,
        }
    }
}

/// 深合并：patch 里出现的键覆盖，其余保留（包括本版本不认识的键）。
fn deep_merge(base: &mut serde_json::Value, patch: &serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(b), serde_json::Value::Object(p)) => {
            for (k, v) in p {
                deep_merge(b.entry(k.clone()).or_insert(serde_json::Value::Null), v);
            }
        }
        (b, p) => *b = p.clone(),
    }
}

fn merge_settings(
    mut current: serde_json::Value,
    patch: &serde_json::Value,
) -> Result<AppSettings, String> {
    if !current.is_object() {
        current = serde_json::json!({});
    }
    deep_merge(&mut current, patch);
    let settings: AppSettings =
        serde_json::from_value(current).map_err(|e| format!("设置格式无效：{e}"))?;
    if !(1..=60).contains(&settings.halo.timeout_seconds) {
        return Err("转盘自动关闭时间应为 1–60 秒".into());
    }
    if settings.idle_minutes > 1440 {
        return Err("空闲时间最多 1440 分钟".into());
    }
    Ok(settings)
}

fn settings_path(config_path: &std::path::Path) -> PathBuf {
    config_path.with_file_name("settings.json")
}

/// 原始 JSON（保留本版本不认识的键）；读不到就是空对象。
fn load_settings_value(config_path: &std::path::Path) -> serde_json::Value {
    std::fs::read(settings_path(config_path))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn load_settings(config_path: &std::path::Path) -> AppSettings {
    merge_settings(load_settings_value(config_path), &serde_json::json!({})).unwrap_or_default()
}

/// 把设置写回文件：先合并进原始 JSON，未知键不丢。
fn save_settings(config_path: &std::path::Path, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(config_path);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut raw = load_settings_value(config_path);
    deep_merge(
        &mut raw,
        &serde_json::to_value(settings).map_err(|e| e.to_string())?,
    );
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&raw).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(state: State<'_, App>) -> AppSettings {
    load_settings(&state.config_path)
}

/// 合并一段设置（只覆盖给出的键），写文件并即时生效。
#[tauri::command]
fn set_settings(state: State<'_, App>, patch: serde_json::Value) -> Result<AppSettings, String> {
    let settings = merge_settings(load_settings_value(&state.config_path), &patch)?;
    save_settings(&state.config_path, &settings)?;
    let guard = state.service.lock().map_err(|e| e.to_string())?;
    let service = guard.as_ref().ok_or("服务未运行")?;
    let idle = (settings.idle_minutes > 0)
        .then(|| std::time::Duration::from_secs(settings.idle_minutes * 60));
    if !service.send(Command::SetIdleTimeout(idle))
        || !service.send(Command::SetHaloTimeout(std::time::Duration::from_secs(
            settings.halo.timeout_seconds,
        )))
    {
        return Err("设备线程已退出".into());
    }
    Ok(settings)
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UiPower {
    standby: Option<bool>,
    standby_delay: Option<u32>,
    sleep_delay: Option<u32>,
    mic_enabled: Option<bool>,
    noise_reduction: Option<u8>,
    errors: Vec<String>,
}

impl From<PowerInfo> for UiPower {
    fn from(p: PowerInfo) -> UiPower {
        UiPower {
            standby: p.standby,
            standby_delay: p.standby_delay,
            sleep_delay: p.sleep_delay,
            mic_enabled: p.mic_enabled,
            noise_reduction: p.noise_reduction.map(|n| n.level),
            errors: p.errors,
        }
    }
}

/// 只在发送命令时持锁；等待回复不占锁，免得 `snapshot` 等快命令被设备超时拖住。
fn send_command(state: &State<'_, App>, command: Command) -> Result<(), String> {
    let guard = state.service.lock().map_err(|e| e.to_string())?;
    let service = guard.as_ref().ok_or("服务未运行")?;
    if !service.send(command) {
        return Err("设备线程已退出".into());
    }
    Ok(())
}

/// 发一条带回复通道的命令并等待；调用方必须是 async 命令，这样等待发生在工作线程而不是主线程。
fn ask<T: Send + 'static>(
    state: &State<'_, App>,
    make: impl FnOnce(std::sync::mpsc::Sender<T>) -> Command,
    secs: u64,
) -> Result<T, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    send_command(state, make(tx))?;
    rx.recv_timeout(std::time::Duration::from_secs(secs))
        .map_err(|_| "设备没有在限定时间内回复".to_string())
}

#[tauri::command]
async fn power_info(state: State<'_, App>) -> Result<UiPower, String> {
    ask(&state, Command::QueryPower, 15).map(UiPower::from)
}

#[tauri::command]
async fn set_sleep_delay(state: State<'_, App>, seconds: u32) -> Result<u32, String> {
    ask(&state, |tx| Command::SetSleepDelay(seconds, tx), 15)?
}

#[tauri::command]
async fn set_standby_delay(state: State<'_, App>, seconds: u32) -> Result<u32, String> {
    ask(&state, |tx| Command::SetStandbyDelay(seconds, tx), 15)?
}

#[tauri::command]
async fn set_noise_reduction(state: State<'_, App>, level: u8) -> Result<u8, String> {
    ask(&state, |tx| Command::SetNoiseReduction(level, tx), 15)?.map(|n| n.level)
}

fn autostart_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("autostart").join("halokey.desktop")
}

#[tauri::command]
fn get_autostart() -> bool {
    autostart_path().exists()
}

/// 登录启动：写 / 删 XDG autostart 条目，Exec 指向当前可执行文件。
#[tauri::command]
fn set_autostart(enabled: bool) -> Result<bool, String> {
    let path = autostart_path();
    if !enabled {
        match std::fs::remove_file(&path) {
            Ok(()) => return Ok(false),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e.to_string()),
        }
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=halokey\nComment=Ulanzi Vibe Key control center\nExec={}\nIcon=halokey-app\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        exe.display()
    );
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, entry).map_err(|e| e.to_string())?;
    Ok(true)
}

/// 把基础层六个控件的单击条目写进设备自己的表，应用不运行时设备照此直出。
/// 只写纯键盘 / 鼠标页条目；其他动作（应用、宏、光环、指针）对应的控件保持设备现值不变。
#[tauri::command]
async fn write_device_keys(state: State<'_, App>) -> Result<Vec<String>, String> {
    let map = {
        let guard = state.service.lock().map_err(|e| e.to_string())?;
        let service = guard.as_ref().ok_or("服务未运行")?;
        service.snapshot().keymap.ok_or("还没有键位")?
    };
    let mut report = Vec::new();
    for control in &map.controls {
        let Some(HostAction::Keyboard(entries)) = Some(control.effective_press()) else {
            report.push(format!(
                "控件 {}：非键盘动作，设备表保持不变",
                control.index
            ));
            continue;
        };
        let entries: Vec<halokey_proto::KeyEntry> = entries
            .iter()
            .filter(|e| e.code != 0 || e.kind != 2)
            .map(|e| halokey_proto::KeyEntry::from_bytes(e.kind, e.code))
            .collect();
        let entries = if entries.is_empty() {
            vec![halokey_proto::KeyEntry::keyboard(0x01)]
        } else {
            entries
        };
        if entries.len() > halokey_proto::keyconfig::MAX_ENTRIES {
            report.push(format!(
                "控件 {}：条目超过 {} 个，跳过",
                control.index,
                halokey_proto::keyconfig::MAX_ENTRIES
            ));
            continue;
        }
        match ask(
            &state,
            |tx| Command::WriteKey(control.index, entries, tx),
            12,
        )? {
            Ok(reply) => report.push(format!(
                "控件 {}：已写入 {} 个条目并读回核对",
                reply.index,
                reply.entries.len()
            )),
            Err(e) => report.push(format!("控件 {}：失败 {e}", control.index)),
        }
    }
    Ok(report)
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn main() {
    // NVIDIA 上 WebKitGTK 的 DMA-BUF 渲染会出现空白窗口；未显式设置时关掉它。
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    let path = config_path();
    let settings = load_settings(&path);
    let idle_minutes = settings.idle_minutes;
    let halo_timeout = std::time::Duration::from_secs(settings.halo.timeout_seconds);
    let initial = match load_keymap(&path) {
        Ok(map) => map,
        Err(e) => {
            eprintln!("配置读取失败，将使用设备表：{e}");
            None
        }
    };
    tauri::Builder::default()
        .manage(App {
            service: Mutex::new(None),
            config_path: path,
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            snapshot,
            get_keymap,
            validate_keymap,
            list_windows,
            save_keymap,
            read_device_keys,
            device_info,
            set_lighting,
            get_autostart,
            set_autostart,
            write_device_keys,
            get_settings,
            set_settings,
            import_keymap,
            export_keymap,
            load_default_keymap,
            keymap_from_device,
            set_default_audio_input,
            power_info,
            set_sleep_delay,
            set_standby_delay,
            set_noise_reduction
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            // 调试用：HALOKEY_SHOW_MAIN=1 时启动即显示设置窗口，省去点托盘。
            if std::env::var_os("HALOKEY_SHOW_MAIN").is_some() {
                show_main(&handle);
            }
            let service = Service::spawn(
                Config {
                    keymap: initial,
                    dry_run: false,
                    serial: None,
                    idle_timeout: (idle_minutes > 0)
                        .then(|| std::time::Duration::from_secs(idle_minutes * 60)),
                    halo_timeout,
                },
                Backends::platform(),
                Box::new(move |event| forward(&handle, event)),
            );
            *app.state::<App>().service.lock().expect("state lock") = Some(service);

            let open = MenuItem::with_id(app, "open", "打开设置", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            let icon = app.default_window_icon().cloned().expect("图标");
            TrayIconBuilder::with_id("tray")
                .icon(icon)
                .tooltip("halokey")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => show_main(app),
                    "quit" => quit_app(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // 关闭设置窗：按设置留在托盘（只隐藏）或退出。
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let close_to_tray =
                        load_settings(&window.state::<App>().config_path).close_to_tray;
                    if close_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        quit_app(window.app_handle());
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("halokey 启动失败");
}

#[cfg(test)]
mod tests {
    #[test]
    fn picks_the_vibe_key_capture_source_and_skips_monitors() {
        let list = "94\talsa_output.platform-hdmi.monitor\tPipeWire\ts32le 2ch 48000Hz\tRUNNING\n\
                    4779424\talsa_input.usb-AU05_AU05_202609xxxxxx-00.analog-stereo\tPipeWire\ts24le 2ch 48000Hz\tSUSPENDED\n\
                    4779425\talsa_input.usb-AU05_AU05_202609xxxxxx-00.analog-stereo.monitor\tPipeWire\ts24le 2ch 48000Hz\tSUSPENDED\n";
        assert_eq!(
            pick_vibe_source(list).as_deref(),
            Some("alsa_input.usb-AU05_AU05_202609xxxxxx-00.analog-stereo")
        );
        assert_eq!(
            pick_vibe_source("1\talsa_input.usb-Other-00.analog-stereo\tPipeWire\n"),
            None
        );
        assert_eq!(pick_vibe_source(""), None);
    }

    #[test]
    fn pactl_failures_report_the_real_reason() {
        assert_eq!(
            vibe_source_from_pactl(false, "", "Connection failure: Connection refused\n"),
            Err("pactl 失败：Connection failure: Connection refused".to_string())
        );
        assert_eq!(
            vibe_source_from_pactl(true, "1\talsa_input.usb-Other-00.analog-stereo\tPipeWire\n", ""),
            Err("没有找到 Vibe Key 的麦克风（本体开着、收音打开了吗？）".to_string())
        );
        assert_eq!(
            vibe_source_from_pactl(true, "2\talsa_input.usb-AU05_AU05_202609xxxxxx-00.analog-stereo\tPipeWire\n", ""),
            Ok("alsa_input.usb-AU05_AU05_202609xxxxxx-00.analog-stereo".to_string())
        );
    }

    #[test]
    fn settings_default_close_to_tray_and_merge_keeps_it() {
        let s = AppSettings::default();
        assert!(s.close_to_tray);
        let merged: AppSettings =
            serde_json::from_value(serde_json::json!({ "idleMinutes": 3 })).unwrap();
        assert!(merged.close_to_tray, "缺省字段用默认值");
        let off: AppSettings =
            serde_json::from_value(serde_json::json!({ "closeToTray": false })).unwrap();
        assert!(!off.close_to_tray);
    }

    use super::*;

    #[test]
    fn halo_window_origin_uses_physical_pixels_for_logical_size() {
        // 2× 缩放：逻辑 360 px 的窗口在物理坐标里是 720 px 宽，围着指针居中。
        assert_eq!(halo_window_origin(1000.0, 800.0, 360, 2.0), (640.0, 440.0));
        assert_eq!(
            halo_window_origin(100.0, 50.0, 360, 1.0),
            (0.0, 0.0),
            "左上角夹取到 0"
        );
    }

    #[test]
    fn settings_merge_keeps_unknown_and_defaults() {
        let old: serde_json::Value = serde_json::json!({ "idleMinutes": 10, "future": true });
        let patch = serde_json::json!({ "halo": { "size": "large" } });
        let merged = merge_settings(old, &patch).unwrap();
        assert_eq!(merged.idle_minutes, 10);
        assert_eq!(merged.halo.size, HaloSize::Large);
        assert_eq!(merged.halo.timeout_seconds, 4, "缺省段用默认值");
        assert_eq!(merged.halo.position, HaloPosition::Pointer);
        let json = serde_json::to_value(&merged).unwrap();
        assert_eq!(json["halo"]["size"], "large");
        assert_eq!(json["halo"]["timeoutSeconds"], 4);
        assert!(merge_settings(
            serde_json::json!({}),
            &serde_json::json!({ "halo": { "timeoutSeconds": 0 } })
        )
        .is_err());
        assert_eq!(HaloSettings::default().size_px(), 360);
    }

    #[test]
    fn validate_keymap_accepts_examples_and_rejects_bad_shapes() {
        let good: serde_json::Value =
            serde_json::from_str(include_str!("../../../docs/examples/halo-menu.json")).unwrap();
        assert!(validate_keymap(good.clone()).is_ok());
        let mut bad = good.clone();
        bad["controls"].as_array_mut().unwrap().pop();
        assert!(validate_keymap(bad).is_err());
        let mut hold_on_double = good;
        hold_on_double["controls"][0]["doublePressBehavior"] = "hold".into();
        assert!(validate_keymap(hold_on_double).is_err());
    }
}
