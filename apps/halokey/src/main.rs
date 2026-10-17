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
            if let Some(w) = app.get_webview_window("halo") {
                // 光环围着鼠标指针出现；拿不到指针位置就居中。
                match (app.cursor_position(), w.outer_size()) {
                    (Ok(pos), Ok(size)) => {
                        let x = pos.x - f64::from(size.width) / 2.0;
                        let y = pos.y - f64::from(size.height) / 2.0;
                        let _ =
                            w.set_position(tauri::PhysicalPosition::new(x.max(0.0), y.max(0.0)));
                    }
                    _ => {
                        let _ = w.center();
                    }
                }
                let _ = w.show();
            }
            UiEvent::HaloShow { halo, selected }
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
        Event::Layers(active) => UiEvent::Layers { active },
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
    // 落盘归一化后的形状（版本号由内容决定），而不是界面原样回传的 JSON。
    let normalized: serde_json::Value =
        serde_json::from_slice(&map.to_json().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let pretty = serde_json::to_vec_pretty(&normalized).map_err(|e| e.to_string())?;
    if let Some(dir) = state.config_path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&state.config_path, pretty).map_err(|e| e.to_string())?;
    let guard = state.service.lock().map_err(|e| e.to_string())?;
    let service = guard.as_ref().ok_or("服务未运行")?;
    if !service.send(Command::SetKeymap(map)) {
        return Err("设备线程已退出".into());
    }
    Ok(())
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

#[derive(Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
struct AppSettings {
    idle_minutes: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings { idle_minutes: 5 }
    }
}

fn settings_path(config_path: &std::path::Path) -> PathBuf {
    config_path.with_file_name("settings.json")
}

fn load_settings(config_path: &std::path::Path) -> AppSettings {
    std::fs::read(settings_path(config_path))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_settings(config_path: &std::path::Path, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(config_path);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(state: State<'_, App>) -> AppSettings {
    load_settings(&state.config_path)
}

#[tauri::command]
fn set_idle_minutes(state: State<'_, App>, minutes: u64) -> Result<(), String> {
    let settings = AppSettings {
        idle_minutes: minutes,
    };
    save_settings(&state.config_path, &settings)?;
    let guard = state.service.lock().map_err(|e| e.to_string())?;
    let service = guard.as_ref().ok_or("服务未运行")?;
    let timeout = (minutes > 0).then(|| std::time::Duration::from_secs(minutes * 60));
    if !service.send(Command::SetIdleTimeout(timeout)) {
        return Err("设备线程已退出".into());
    }
    Ok(())
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
async fn set_mic_enabled(state: State<'_, App>, enabled: bool) -> Result<bool, String> {
    ask(&state, |tx| Command::SetMicEnabled(enabled, tx), 15)?
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
    let idle_minutes = load_settings(&path).idle_minutes;
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
        .invoke_handler(tauri::generate_handler![
            snapshot,
            get_keymap,
            validate_keymap,
            save_keymap,
            read_device_keys,
            device_info,
            set_lighting,
            get_autostart,
            set_autostart,
            write_device_keys,
            get_settings,
            set_idle_minutes,
            power_info,
            set_sleep_delay,
            set_standby_delay,
            set_mic_enabled,
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
                    "quit" => {
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
            // 关闭设置窗只是隐藏；应用留在托盘。
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("halokey 启动失败");
}

#[cfg(test)]
mod tests {
    use super::*;

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
