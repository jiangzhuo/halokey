//! halokey 桌面应用：托盘、设置窗口、光环悬浮层。设备逻辑全部在 `halokey-runtime` 的线程里。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::sync::Mutex;

use halokey_host::{HostAction, HostKeymap, HostProfile};
use halokey_proto::{IndicatorLight, Lighting};
use halokey_runtime::{Backends, Command, Config, DeviceInfo, Event, Service, Snapshot};
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
    Stopped,
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

fn forward(app: &AppHandle, event: Event) {
    let ui = match event {
        Event::Connected => UiEvent::Connected,
        Event::Disconnected => UiEvent::Disconnected,
        Event::Key(e, at) => UiEvent::Key {
            control: e.control.index(),
            name: e.control.name().to_string(),
            pressed: e.pressed,
            at,
        },
        Event::Transitions(_) => return,
        Event::Battery(b) => UiEvent::Battery {
            percent: b.percent,
            millivolts: b.millivolts,
            charging: b.charging,
        },
        Event::HaloShow(halo, selected) => {
            if let Some(w) = app.get_webview_window("halo") {
                let _ = w.center();
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
        Event::Error(text) => UiEvent::Error { text },
        Event::Stopped => UiEvent::Stopped,
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

#[tauri::command]
fn save_keymap(state: State<'_, App>, keymap: serde_json::Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(&keymap).map_err(|e| e.to_string())?;
    let map = HostKeymap::from_json(&bytes).map_err(|e| e.to_string())?;
    let pretty = serde_json::to_vec_pretty(&keymap).map_err(|e| e.to_string())?;
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
fn read_device_keys(state: State<'_, App>) -> Result<DeviceKeys, String> {
    let guard = state.service.lock().map_err(|e| e.to_string())?;
    let service = guard.as_ref().ok_or("服务未运行")?;
    let (tx, rx) = std::sync::mpsc::channel();
    if !service.send(Command::ReadKeys(tx)) {
        return Err("设备线程已退出".into());
    }
    let replies = rx
        .recv_timeout(std::time::Duration::from_secs(12))
        .map_err(|_| "读取超时")??;
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
fn device_info(state: State<'_, App>) -> Result<UiDeviceInfo, String> {
    let guard = state.service.lock().map_err(|e| e.to_string())?;
    let service = guard.as_ref().ok_or("服务未运行")?;
    let (tx, rx) = std::sync::mpsc::channel();
    if !service.send(Command::QueryInfo(tx)) {
        return Err("设备线程已退出".into());
    }
    let info = rx
        .recv_timeout(std::time::Duration::from_secs(12))
        .map_err(|_| "读取超时")?;
    Ok(info.into())
}

#[tauri::command]
fn set_lighting(state: State<'_, App>, lighting: UiLighting) -> Result<UiLighting, String> {
    let desired = Lighting::try_from(lighting)?;
    let guard = state.service.lock().map_err(|e| e.to_string())?;
    let service = guard.as_ref().ok_or("服务未运行")?;
    let (tx, rx) = std::sync::mpsc::channel();
    if !service.send(Command::SetLighting(desired, tx)) {
        return Err("设备线程已退出".into());
    }
    let result = rx
        .recv_timeout(std::time::Duration::from_secs(20))
        .map_err(|_| "写入超时")??;
    Ok(result.into())
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
fn write_device_keys(state: State<'_, App>) -> Result<Vec<String>, String> {
    let guard = state.service.lock().map_err(|e| e.to_string())?;
    let service = guard.as_ref().ok_or("服务未运行")?;
    let map = service.snapshot().keymap.ok_or("还没有键位")?;
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
        let (tx, rx) = std::sync::mpsc::channel();
        if !service.send(Command::WriteKey(control.index, entries, tx)) {
            return Err("设备线程已退出".into());
        }
        match rx
            .recv_timeout(std::time::Duration::from_secs(12))
            .map_err(|_| "写入超时")?
        {
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
            save_keymap,
            read_device_keys,
            device_info,
            set_lighting,
            get_autostart,
            set_autostart,
            write_device_keys
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let service = Service::spawn(
                Config {
                    keymap: initial,
                    dry_run: false,
                    serial: None,
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
