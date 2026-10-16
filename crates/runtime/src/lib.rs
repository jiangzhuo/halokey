//! 设备服务：一个专门的线程拥有厂商通道、心跳、手势状态机、注入器和动作执行器。
//!
//! 命令行的 `halokey run` 和桌面应用共用这一层。外部通过 [`Service`] 发命令、读快照、
//! 收事件；设备线程从不阻塞在界面上，界面也从不直接碰设备。
//!
//! 生命周期：枚举接收器 → 打开 → 主机接管 `01 01 10 00 03` → 心跳 0.8 s 一次 →
//! 事件循环 → 停止时松开所有键、交还控制 `01 01 10 00 00`。接收器不见了就回到枚举，每秒重试。

#![forbid(unsafe_code)]

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use halokey_hid::{HidApi, Transport};
use halokey_host::runner::KEYBOARD_OWNER;
use halokey_host::{
    ActionRunner, GestureRouter, HostAction, HostKeymap, KeyEntry, Launcher, MacroKeys, Transition,
};
use halokey_input::{Injector, KeyBridge, RecordingInjector};
use halokey_proto::frame::Plaintext;
use halokey_proto::{status, Battery, Control, KeyConfigReply, VendorKeyEvent};

/// 心跳间隔。
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(800);
/// 连续多久没有心跳回显就视为接收器断开。
pub const ECHO_TIMEOUT: Duration = Duration::from_secs(5);
/// 没有接收器时的重试间隔。
pub const RETRY_INTERVAL: Duration = Duration::from_secs(1);
const PUMP_TIMEOUT: Duration = Duration::from_millis(20);
const REPLY_TIMEOUT: Duration = Duration::from_millis(1500);

/// 服务配置。
pub struct Config {
    /// 主机键位；`None` 表示启动时从设备六格表生成。
    pub keymap: Option<HostKeymap>,
    /// 只记录、不注入。
    pub dry_run: bool,
    /// 用序列号选择接收器。
    pub serial: Option<String>,
}

/// 外部可读的状态快照。
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// 厂商通道已打开。
    pub connected: bool,
    /// 本体在线（接收器应答里的 status 字节）；未知为 `None`。
    pub online: Option<bool>,
    /// 最近一次电量（查询或广播）。
    pub battery: Option<Battery>,
    /// 当前主机键位。
    pub keymap: Option<HostKeymap>,
    /// 已发送心跳数。
    pub heartbeats: u64,
    /// 已收到的心跳回显数。
    pub echoes: u64,
    /// 已注入的按键边沿数。
    pub injected: u64,
    /// 最近的错误文本。
    pub last_error: Option<String>,
}

/// 推送给外部的事件。
#[derive(Debug, Clone)]
pub enum Event {
    /// 接收器已打开。
    Connected,
    /// 接收器丢失，正在重试。
    Disconnected,
    /// 物理控件事件，附服务启动以来的秒数。
    Key(VendorKeyEvent, f64),
    /// 手势状态机输出。
    Transitions(Vec<Transition>),
    /// 电量更新。
    Battery(Battery),
    /// 错误（注入、动作、设备）。
    Error(String),
    /// 服务已停止。
    Stopped,
}

/// 发给设备线程的命令。
pub enum Command {
    /// 替换主机键位；会先松开所有按住的键。
    SetKeymap(HostKeymap),
    /// 读六格表。
    ReadKeys(Sender<Result<Vec<KeyConfigReply>, String>>),
    /// 写一格并读回。
    WriteKey(
        u8,
        Vec<halokey_proto::KeyEntry>,
        Sender<Result<KeyConfigReply, String>>,
    ),
    /// 查电量。
    QueryBattery(Sender<Result<Battery, String>>),
    /// 停止。
    Stop,
}

/// 构造注入器与启动器的工厂；在设备线程内调用。
pub struct Backends {
    pub injector: Box<dyn FnOnce() -> Result<Box<dyn Injector>, halokey_input::Error> + Send>,
    pub launcher: Box<dyn FnOnce() -> Box<dyn Launcher> + Send>,
}

impl Backends {
    /// 当前平台的默认后端。
    pub fn platform() -> Backends {
        Backends {
            injector: Box::new(halokey_input::open_default),
            launcher: Box::new(halokey_wm::open_default),
        }
    }
}

/// 运行中的服务句柄。
pub struct Service {
    commands: Sender<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    thread: Option<JoinHandle<()>>,
}

impl Service {
    /// 启动设备线程。`on_event` 在设备线程上被调用，必须快速返回。
    pub fn spawn(
        config: Config,
        backends: Backends,
        on_event: Box<dyn Fn(Event) + Send>,
    ) -> Service {
        let (tx, rx) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(Snapshot {
            keymap: config.keymap.clone(),
            ..Snapshot::default()
        }));
        let shared = snapshot.clone();
        let thread = std::thread::Builder::new()
            .name("halokey-device".into())
            .spawn(move || {
                let mut worker = Worker::new(config, backends, rx, shared, on_event);
                worker.run();
            })
            .expect("无法创建设备线程");
        Service {
            commands: tx,
            snapshot,
            thread: Some(thread),
        }
    }

    /// 当前快照的副本。
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// 发送命令；线程已退出时返回 `false`。
    pub fn send(&self, command: Command) -> bool {
        self.commands.send(command).is_ok()
    }

    /// 停止并等待线程结束。
    pub fn stop(mut self) {
        let _ = self.commands.send(Command::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct MacroKeyBridge<'a> {
    bridge: &'a mut KeyBridge,
    injector: &'a mut dyn Injector,
}

impl MacroKeys for MacroKeyBridge<'_> {
    type Error = halokey_input::Error;
    fn press(&mut self, entries: &[KeyEntry]) -> Result<(), Self::Error> {
        self.bridge.press(self.injector, KEYBOARD_OWNER, entries)
    }
    fn release(&mut self) -> Result<(), Self::Error> {
        self.bridge.release(self.injector, KEYBOARD_OWNER)
    }
}

struct Worker {
    config: Config,
    backends: Option<Backends>,
    commands: Receiver<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    on_event: Box<dyn Fn(Event) + Send>,
    router: GestureRouter,
    bridge: KeyBridge,
    injector: Option<Box<dyn Injector>>,
    runner: Option<ActionRunner<Box<dyn Launcher>>>,
    start: Instant,
    stray: (Sender<Plaintext>, Receiver<Plaintext>),
}

enum Flow {
    Reconnect,
    Stop,
}

impl Worker {
    fn new(
        config: Config,
        backends: Backends,
        commands: Receiver<Command>,
        snapshot: Arc<Mutex<Snapshot>>,
        on_event: Box<dyn Fn(Event) + Send>,
    ) -> Worker {
        Worker {
            config,
            backends: Some(backends),
            commands,
            snapshot,
            on_event,
            router: GestureRouter::new(),
            bridge: KeyBridge::new(),
            injector: None,
            runner: None,
            start: Instant::now(),
            stray: mpsc::channel(),
        }
    }

    fn now(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    fn update(&self, f: impl FnOnce(&mut Snapshot)) {
        if let Ok(mut s) = self.snapshot.lock() {
            f(&mut s);
        }
    }

    fn fail(&self, text: String) {
        self.update(|s| s.last_error = Some(text.clone()));
        (self.on_event)(Event::Error(text));
    }

    fn run(&mut self) {
        // 后端只创建一次；失败就一直用记录器并报错，不让服务起不来。
        if let Some(backends) = self.backends.take() {
            self.injector = Some(if self.config.dry_run {
                Box::new(RecordingInjector::default())
            } else {
                match (backends.injector)() {
                    Ok(i) => i,
                    Err(e) => {
                        self.fail(format!("输入注入不可用，改为只记录：{e}"));
                        Box::new(RecordingInjector::default())
                    }
                }
            });
            self.runner = Some(ActionRunner::new((backends.launcher)()));
        }
        loop {
            let api = match HidApi::new() {
                Ok(api) => api,
                Err(e) => {
                    self.fail(format!("HID 初始化失败：{e}"));
                    if self.wait_or_stop(RETRY_INTERVAL) {
                        break;
                    }
                    continue;
                }
            };
            let id = halokey_hid::enumerate(&api).into_iter().find(|d| {
                self.config
                    .serial
                    .as_ref()
                    .is_none_or(|s| d.serial.as_deref() == Some(s))
            });
            let Some(id) = id else {
                if self.wait_or_stop(RETRY_INTERVAL) {
                    break;
                }
                continue;
            };
            let mut transport = match Transport::open(&api, &id) {
                Ok(t) => t,
                Err(e) => {
                    self.fail(format!("打开接收器失败：{e}"));
                    if self.wait_or_stop(RETRY_INTERVAL) {
                        break;
                    }
                    continue;
                }
            };
            let sender = self.stray.0.clone();
            transport.set_sink(Some(Box::new(move |frame: &Plaintext| {
                let _ = sender.send(*frame);
            })));
            match self.session(&mut transport) {
                Flow::Stop => {
                    self.teardown(&mut transport);
                    break;
                }
                Flow::Reconnect => {
                    self.teardown(&mut transport);
                    self.update(|s| {
                        s.connected = false;
                        s.online = None;
                    });
                    (self.on_event)(Event::Disconnected);
                }
            }
        }
        (self.on_event)(Event::Stopped);
    }

    /// 等待期间仍处理命令；返回 `true` 表示收到了停止命令。
    fn wait_or_stop(&mut self, duration: Duration) -> bool {
        let deadline = Instant::now() + duration;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return false;
            }
            match self.commands.recv_timeout(remaining) {
                Ok(Command::Stop) => return true,
                Ok(Command::SetKeymap(map)) => self.set_keymap(map),
                Ok(Command::ReadKeys(reply)) => {
                    let _ = reply.send(Err("接收器未连接".into()));
                }
                Ok(Command::WriteKey(_, _, reply)) => {
                    let _ = reply.send(Err("接收器未连接".into()));
                }
                Ok(Command::QueryBattery(reply)) => {
                    let _ = reply.send(Err("接收器未连接".into()));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => return false,
                Err(mpsc::RecvTimeoutError::Disconnected) => return true,
            }
        }
    }

    fn set_keymap(&mut self, map: HostKeymap) {
        if let Some(runner) = self.runner.as_mut() {
            runner.cancel();
        }
        let transitions = self.router.configure(Some(map.clone()));
        self.apply(transitions);
        if let Some(injector) = self.injector.as_mut() {
            for (index, e) in self.bridge.release_all(injector.as_mut()) {
                self.fail(format!("控件 {index} 松开失败：{e}"));
            }
        }
        self.update(|s| s.keymap = Some(map));
    }

    fn session(&mut self, transport: &mut Transport) -> Flow {
        if self.config.keymap.is_none() && self.snapshot().keymap.is_none() {
            match read_keymap(transport) {
                Ok(map) => self.set_keymap(map),
                Err(e) => {
                    self.fail(format!("读取设备键位失败：{e}"));
                    return Flow::Reconnect;
                }
            }
        } else if let Some(map) = self.snapshot().keymap {
            if self.router.configure(Some(map.clone())).is_empty() {
                // 已配置过则无需重复；configure 对相同配置返回空。
            }
        }
        if let Err(e) = transport
            .set_software_online(true)
            .and_then(|()| transport.heartbeat())
        {
            self.fail(format!("主机接管失败：{e}"));
            return Flow::Reconnect;
        }
        let mut last_heartbeat = Instant::now();
        let mut last_echo = Instant::now();
        self.update(|s| {
            s.connected = true;
            s.heartbeats += 1;
        });
        (self.on_event)(Event::Connected);
        match transport.query_online(REPLY_TIMEOUT) {
            Ok(online) => self.update(|s| s.online = Some(online)),
            Err(e) => self.fail(format!("在线查询失败：{e}")),
        }
        self.drain_stray();
        loop {
            match self.commands.try_recv() {
                Ok(Command::Stop) => return Flow::Stop,
                Ok(Command::SetKeymap(map)) => self.set_keymap(map),
                Ok(Command::ReadKeys(reply)) => {
                    let _ = reply.send(read_keys(transport).map_err(|e| e.to_string()));
                    self.drain_stray();
                }
                Ok(Command::WriteKey(index, entries, reply)) => {
                    let _ = reply.send(
                        transport
                            .write_key(index, &entries, REPLY_TIMEOUT)
                            .map_err(|e| e.to_string()),
                    );
                    self.drain_stray();
                }
                Ok(Command::QueryBattery(reply)) => {
                    let result = transport
                        .query_battery(REPLY_TIMEOUT)
                        .map_err(|e| e.to_string());
                    if let Ok(b) = &result {
                        self.update(|s| s.battery = Some(*b));
                        (self.on_event)(Event::Battery(*b));
                    }
                    let _ = reply.send(result);
                    self.drain_stray();
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => return Flow::Stop,
            }
            if last_heartbeat.elapsed() >= HEARTBEAT_INTERVAL {
                if let Err(e) = transport.heartbeat() {
                    self.fail(format!("心跳发送失败：{e}"));
                    return Flow::Reconnect;
                }
                last_heartbeat = Instant::now();
                self.update(|s| s.heartbeats += 1);
            }
            match transport.recv(PUMP_TIMEOUT) {
                Ok(Some(frame)) => {
                    if self.handle_frame(&frame) {
                        last_echo = Instant::now();
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    if !e.to_string().contains("Interrupted") {
                        self.fail(format!("读取接收器失败：{e}"));
                        return Flow::Reconnect;
                    }
                }
            }
            let now = self.now();
            let transitions = self.router.advance(now);
            self.apply(transitions);
            self.pump_runner(now);
            if last_echo.elapsed() > ECHO_TIMEOUT {
                self.fail("5 秒没有收到心跳回显，接收器可能已断开。".into());
                return Flow::Reconnect;
            }
        }
    }

    fn snapshot(&self) -> Snapshot {
        self.snapshot.lock().map(|s| s.clone()).unwrap_or_default()
    }

    fn drain_stray(&mut self) {
        while let Ok(frame) = self.stray.1.try_recv() {
            self.handle_frame(&frame);
        }
    }

    /// 处理一帧；返回是否为心跳回显。
    fn handle_frame(&mut self, frame: &Plaintext) -> bool {
        if frame[..5] == status::HEARTBEAT {
            self.update(|s| s.echoes += 1);
            return true;
        }
        if let Some(event) = VendorKeyEvent::decode(frame) {
            let now = self.now();
            (self.on_event)(Event::Key(event, now));
            let transitions = self.router.receive(event, now);
            self.apply(transitions);
            return false;
        }
        if status::is_battery_notice(frame) {
            if let Ok(b) = status::parse_battery_notice(frame) {
                let merged = Battery {
                    charging: self.snapshot().battery.and_then(|old| old.charging),
                    ..b
                };
                self.update(|s| s.battery = Some(merged));
                (self.on_event)(Event::Battery(merged));
            }
        }
        false
    }

    fn apply(&mut self, transitions: Vec<Transition>) {
        if transitions.is_empty() {
            return;
        }
        (self.on_event)(Event::Transitions(transitions.clone()));
        let Some(mut injector) = self.injector.take() else {
            return;
        };
        let applied = self.bridge.apply(injector.as_mut(), transitions);
        let injected = self.bridge.events;
        self.update(|s| s.injected = injected);
        let mut errors = Vec::new();
        for (index, action) in applied.execute {
            match action {
                HostAction::PointerMove(delta) => {
                    if let Err(e) = injector.pointer_move(delta.dx, delta.dy) {
                        errors.push(format!("控件 {index} 指针移动失败：{e}"));
                    }
                }
                other => {
                    if let Some(runner) = self.runner.as_mut() {
                        if let Err(e) = runner.enqueue(index, other) {
                            errors.push(format!("控件 {index} 的动作无法排队：{e}"));
                        }
                    }
                }
            }
        }
        self.injector = Some(injector);
        for (index, e) in applied.failures {
            errors.push(format!("控件 {index} 注入失败：{e}"));
            self.router.cancel_index(index);
        }
        for text in errors {
            self.fail(text);
        }
    }

    fn pump_runner(&mut self, now: f64) {
        let (Some(runner), Some(injector)) = (self.runner.as_mut(), self.injector.as_mut()) else {
            return;
        };
        let mut keys = MacroKeyBridge {
            bridge: &mut self.bridge,
            injector: injector.as_mut(),
        };
        if let Err(e) = runner.pump(now, &mut keys) {
            let text = format!("动作执行失败：{e}");
            self.update(|s| s.last_error = Some(text.clone()));
            (self.on_event)(Event::Error(text));
        }
    }

    fn teardown(&mut self, transport: &mut Transport) {
        if let Some(runner) = self.runner.as_mut() {
            runner.cancel();
        }
        self.router.cancel(false);
        if let Some(injector) = self.injector.as_mut() {
            for (index, e) in self.bridge.release_all(injector.as_mut()) {
                self.fail(format!("控件 {index} 松开失败：{e}"));
            }
        }
        let _ = transport.set_software_online(false);
        self.update(|s| s.connected = false);
    }
}

fn read_keys(transport: &mut Transport) -> Result<Vec<KeyConfigReply>, halokey_hid::Error> {
    Control::ALL
        .iter()
        .map(|c| transport.read_key(c.index(), REPLY_TIMEOUT))
        .collect()
}

/// 读设备六格表，生成基础主机映射。
pub fn read_keymap(transport: &mut Transport) -> Result<HostKeymap, Box<dyn std::error::Error>> {
    let replies = read_keys(transport)?;
    let bindings: Vec<(u8, Vec<KeyEntry>)> = replies
        .iter()
        .map(|r| {
            (
                r.index,
                r.entries
                    .iter()
                    .map(|e| KeyEntry {
                        kind: e.kind.byte(),
                        code: e.code,
                    })
                    .collect(),
            )
        })
        .collect();
    Ok(HostKeymap::from_device_bindings(&bindings)?)
}
