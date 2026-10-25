//! 设备服务：一个专门的线程拥有厂商通道、心跳、手势状态机、注入器和动作执行器。
//!
//! 命令行的 `halokey run` 和桌面应用共用这一层。外部通过 [`Service`] 发命令、读快照、
//! 收事件；设备线程从不阻塞在界面上，界面也从不直接碰设备。
//!
//! 生命周期：枚举接收器 → 打开 → 主机接管 `01 01 10 00 03` → 心跳 0.8 s 一次 →
//! 事件循环 → 停止时松开所有键、交还控制 `01 01 10 00 00`。接收器不见了就回到枚举，每秒重试。

#![forbid(unsafe_code)]

mod lighting;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use halokey_hid::{HidApi, InputWatch, Transport};
use halokey_host::runner::KEYBOARD_OWNER;
use halokey_host::{
    ActionRunner, GestureRouter, Halo, HostAction, HostKeymap, KeyEntry, Launcher, LayerLighting,
    MacroKeys, Transition,
};
use halokey_input::{Injector, KeyBridge, RecordingInjector};
use halokey_proto::frame::Plaintext;
use halokey_proto::settings::NoiseReductionSetting;
use halokey_proto::{settings, status, Battery, Control, KeyConfigReply, Lighting, VendorKeyEvent};
use lighting::{layer_backlight_allowed, LightingScheduler};

/// 心跳间隔。
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(800);
/// 连续多久没有心跳回显就视为接收器断开。
pub const ECHO_TIMEOUT: Duration = Duration::from_secs(5);
/// 没有接收器时的重试间隔。
pub const RETRY_INTERVAL: Duration = Duration::from_secs(1);
/// 本体在线状态的复查间隔；按键事件和电量广播也会即时把状态置为在线。
pub const ONLINE_RECHECK: Duration = Duration::from_secs(10);
/// 光环菜单无操作多久后自动关闭。
pub const HALO_TIMEOUT: Duration = Duration::from_secs(4);
/// 光环菜单选项执行时使用的按键所有者编号。
pub const HALO_OWNER: u8 = 0xFE;
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
    /// 设备无操作多久后交还控制、停止心跳，让固件自己待机；`None` 永不。
    pub idle_timeout: Option<Duration>,
    /// 光环转盘无操作多久自动关闭。
    pub halo_timeout: Duration,
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
    /// 空闲暂停中：已交还控制，等待设备活动。
    pub paused: bool,
    /// 设备报告的待机状态（通知 0x0D）。
    pub standby: Option<bool>,
    /// 本体链路（通知 0x0B）。
    pub link: Option<bool>,
    /// 当前按住的 MO 层，从底到顶；空表示只有基础层。
    pub active_layers: Vec<u8>,
    /// 当前活动层集合对应的有效背光；`None` = 不管灯。
    pub effective_lighting: Option<LayerLighting>,
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
    /// 打开光环菜单，附初始选中项。
    HaloShow(Halo, usize),
    /// 光环菜单选中项变化。
    HaloSelect(usize),
    /// 光环菜单关闭（确认、取消或超时）。
    HaloHide,
    /// 空闲超时，已交还控制。
    Paused,
    /// 检测到活动，重新接管。
    Resumed,
    /// 错误（注入、动作、设备）。
    Error(String),
    /// 服务已停止。
    Stopped,
    /// 活动层集合变化（从底到顶）。
    Layers(Vec<u8>),
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
    /// 读设备信息（版本、降噪、指示灯），只读安全。
    QueryInfo(Sender<DeviceInfo>),
    /// 写指示灯参数：传入期望值，服务按当前读回算出最小写序列并逐条确认。
    SetLighting(Lighting, Sender<Result<Lighting, String>>),
    /// 改空闲超时；`None` 永不暂停。
    SetIdleTimeout(Option<Duration>),
    /// 改光环转盘的自动关闭时间。
    SetHaloTimeout(Duration),
    /// 读电源与麦克风设置。
    QueryPower(Sender<PowerInfo>),
    /// 写休眠延时（秒，0 永不），读回核对。
    SetSleepDelay(u32, Sender<Result<u32, String>>),
    /// 写待机延时（秒），读回核对。
    SetStandbyDelay(u32, Sender<Result<u32, String>>),
    /// 麦克风开关，读回核对。
    SetMicEnabled(bool, Sender<Result<bool, String>>),
    /// 降噪等级 0 / 1（低高参数用 Studio 固定值），读回核对。
    SetNoiseReduction(u8, Sender<Result<NoiseReductionSetting, String>>),
    /// 停止。
    Stop,
}

/// 电源与麦克风设置；读不到的项为 `None`。
#[derive(Debug, Clone, Default)]
pub struct PowerInfo {
    pub standby: Option<bool>,
    pub standby_delay: Option<u32>,
    pub sleep_delay: Option<u32>,
    pub mic_enabled: Option<bool>,
    pub noise_reduction: Option<NoiseReductionSetting>,
    pub errors: Vec<String>,
}

/// 只读设备信息；查不到的项为 `None`。
#[derive(Debug, Clone, Default)]
pub struct DeviceInfo {
    pub firmware: Option<String>,
    pub dongle_firmware: Option<String>,
    pub noise_reduction: Option<[u8; 4]>,
    pub lighting: Option<Lighting>,
    pub errors: Vec<String>,
}

/// 注入器工厂。可以多次调用：第一次失败（例如 macOS 还没有辅助功能权限）后会定期重试。
pub type InjectorFactory =
    Box<dyn FnMut() -> Result<Box<dyn Injector>, halokey_input::Error> + Send>;

/// 构造注入器与启动器的工厂；在设备线程内调用。
pub struct Backends {
    pub injector: InjectorFactory,
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

/// 注入器打开失败后的重试：有按键时才试，两次之间至少隔 [`InjectorRetry::INTERVAL`]。
struct InjectorRetry {
    factory: InjectorFactory,
    last_attempt: Instant,
    /// 打开失败时写进 `last_error` 的文本；恢复后只清掉这一条，不覆盖之后的其他错误。
    error: String,
}

impl InjectorRetry {
    const INTERVAL: Duration = Duration::from_secs(2);

    /// 距上次尝试不到间隔时返回 `None`，否则再调一次工厂。
    fn attempt(&mut self, now: Instant) -> Option<Result<Box<dyn Injector>, halokey_input::Error>> {
        if now.duration_since(self.last_attempt) < Self::INTERVAL {
            return None;
        }
        self.last_attempt = now;
        Some((self.factory)())
    }
}

/// `last_error` 仍是 `text` 时才清掉；之后又出了别的错就保留那条。
fn clear_error_if(snapshot: &mut Snapshot, text: &str) {
    if snapshot.last_error.as_deref() == Some(text) {
        snapshot.last_error = None;
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
    injector_retry: Option<InjectorRetry>,
    runner: Option<ActionRunner<Box<dyn Launcher>>>,
    start: Instant,
    stray: (Sender<Plaintext>, Receiver<Plaintext>),
    halo: Option<HaloState>,
    halo_swallowed: std::collections::BTreeSet<u8>,
    idle_timeout: Option<Duration>,
    halo_timeout: Duration,
    last_activity: Instant,
    paused: bool,
    lighting: LightingScheduler,
}

struct HaloState {
    halo: Halo,
    selected: usize,
    last_input: Instant,
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
        let idle_timeout = config.idle_timeout;
        let halo_timeout = config.halo_timeout;
        Worker {
            config,
            backends: Some(backends),
            commands,
            snapshot,
            on_event,
            router: GestureRouter::new(),
            bridge: KeyBridge::new(),
            injector: None,
            injector_retry: None,
            runner: None,
            start: Instant::now(),
            stray: mpsc::channel(),
            halo: None,
            halo_swallowed: std::collections::BTreeSet::new(),
            idle_timeout,
            halo_timeout,
            last_activity: Instant::now(),
            paused: false,
            lighting: LightingScheduler::new(),
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

    /// 注入器之前打开失败时再试一次（例如用户刚授予了辅助功能权限）；成功就换掉记录器。
    fn retry_injector(&mut self) {
        let Some(retry) = self.injector_retry.as_mut() else {
            return;
        };
        // 还没到间隔或依旧失败：继续用记录器，下次按键再试。
        if let Some(Ok(injector)) = retry.attempt(Instant::now()) {
            if let Some(old) = self.injector.as_mut() {
                self.bridge.release_all(old.as_mut());
            }
            self.injector = Some(injector);
            if let Some(retry) = self.injector_retry.take() {
                self.update(|s| clear_error_if(s, &retry.error));
            }
        }
    }

    fn fail(&self, text: String) {
        self.update(|s| s.last_error = Some(text.clone()));
        (self.on_event)(Event::Error(text));
    }

    fn run(&mut self) {
        // 后端在这里创建；注入器失败就先用记录器并报错，不让服务起不来，之后有按键时再重试。
        if let Some(mut backends) = self.backends.take() {
            self.injector = Some(if self.config.dry_run {
                Box::new(RecordingInjector::default())
            } else {
                match (backends.injector)() {
                    Ok(i) => i,
                    Err(e) => {
                        let error = format!("输入注入不可用，改为只记录：{e}");
                        self.fail(error.clone());
                        self.injector_retry = Some(InjectorRetry {
                            factory: backends.injector,
                            last_attempt: Instant::now(),
                            error,
                        });
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
            let watch = InputWatch::open(&api, &id).ok();
            match self.session(&mut transport, watch) {
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
                Ok(Command::QueryInfo(reply)) => {
                    let _ = reply.send(DeviceInfo {
                        errors: vec!["接收器未连接".into()],
                        ..DeviceInfo::default()
                    });
                }
                Ok(Command::SetLighting(_, reply)) => {
                    let _ = reply.send(Err("接收器未连接".into()));
                }
                Ok(Command::SetIdleTimeout(timeout)) => self.idle_timeout = timeout,
                Ok(Command::SetHaloTimeout(t)) => self.halo_timeout = t,
                Ok(Command::QueryPower(reply)) => {
                    let _ = reply.send(PowerInfo {
                        errors: vec!["接收器未连接".into()],
                        ..PowerInfo::default()
                    });
                }
                Ok(Command::SetSleepDelay(_, reply)) | Ok(Command::SetStandbyDelay(_, reply)) => {
                    let _ = reply.send(Err("接收器未连接".into()));
                }
                Ok(Command::SetMicEnabled(_, reply)) => {
                    let _ = reply.send(Err("接收器未连接".into()));
                }
                Ok(Command::SetNoiseReduction(_, reply)) => {
                    let _ = reply.send(Err("接收器未连接".into()));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => return false,
                Err(mpsc::RecvTimeoutError::Disconnected) => return true,
            }
        }
    }

    fn set_keymap(&mut self, map: HostKeymap) {
        self.close_halo();
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
        self.refresh_layers();
    }

    /// 活动层集合刷新：快照、事件、背光调度。
    fn refresh_layers(&mut self) {
        let layers = self.router.active_layers();
        let snapshot = self.snapshot();
        let changed = snapshot.active_layers != layers;
        let desired = snapshot
            .keymap
            .as_ref()
            .and_then(|m| m.effective_lighting(&layers).copied());
        self.lighting.note(desired, Instant::now());
        self.update(|s| {
            s.active_layers = layers.clone();
            s.effective_lighting = desired;
        });
        if changed {
            (self.on_event)(Event::Layers(layers));
        }
    }

    fn session(&mut self, transport: &mut Transport, mut watch: Option<InputWatch>) -> Flow {
        // 本体可能在待机，读不到键位不算失败：先用默认表接管，等它醒来再读。
        let mut keymap_pending = false;
        if self.config.keymap.is_none() && self.snapshot().keymap.is_none() {
            match read_keymap(transport) {
                Ok(map) => self.set_keymap(map),
                Err(e) => {
                    self.fail(format!(
                        "本体暂时没有回复键位（可能在待机），先用出厂映射：{e}"
                    ));
                    self.set_keymap(HostKeymap::default_keymap());
                    keymap_pending = true;
                }
            }
        } else if let Some(map) = self.snapshot().keymap {
            let _ = self.router.configure(Some(map));
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
        // 重连后设备灯光状态未知：忘掉上次写入，让 Layer 0 背光重新写一次。
        self.lighting.failed();
        self.refresh_layers();
        match transport.query_online(REPLY_TIMEOUT) {
            Ok(online) => self.update(|s| s.online = Some(online)),
            Err(e) => self.fail(format!("在线查询失败：{e}")),
        }
        self.drain_stray();
        let mut last_online_check = Instant::now();
        self.last_activity = Instant::now();
        self.paused = false;
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
                Ok(Command::QueryInfo(reply)) => {
                    let _ = reply.send(query_info(transport));
                    self.drain_stray();
                }
                Ok(Command::SetLighting(desired, reply)) => {
                    let _ = reply.send(set_lighting(transport, &desired));
                    // 设置页直接改了设备记录（含总开关）：缓存作废并立刻按当前层重算，
                    // 切回"工作"时分层背光马上恢复，不用等下一次按键。
                    self.lighting.failed();
                    self.refresh_layers();
                    self.drain_stray();
                }
                Ok(Command::SetIdleTimeout(timeout)) => {
                    self.idle_timeout = timeout;
                    self.last_activity = Instant::now();
                }
                Ok(Command::SetHaloTimeout(t)) => self.halo_timeout = t,
                Ok(Command::QueryPower(reply)) => {
                    let _ = reply.send(query_power(transport));
                    self.drain_stray();
                }
                Ok(Command::SetSleepDelay(seconds, reply)) => {
                    let _ = reply.send(write_u32_setting(
                        transport,
                        settings::sleep_delay_write(seconds),
                        &settings::SLEEP_DELAY_REQUEST,
                        settings::is_sleep_delay_reply,
                        seconds,
                    ));
                    self.drain_stray();
                }
                Ok(Command::SetStandbyDelay(seconds, reply)) => {
                    let _ = reply.send(write_u32_setting(
                        transport,
                        settings::standby_delay_write(seconds),
                        &settings::STANDBY_DELAY_REQUEST,
                        settings::is_standby_delay_reply,
                        seconds,
                    ));
                    self.drain_stray();
                }
                Ok(Command::SetMicEnabled(enabled, reply)) => {
                    let _ = reply.send(write_bool_setting(
                        transport,
                        settings::mic_enable_write(enabled),
                        &settings::MIC_ENABLE_REQUEST,
                        settings::is_mic_enable_reply,
                        enabled,
                    ));
                    self.drain_stray();
                }
                Ok(Command::SetNoiseReduction(level, reply)) => {
                    let _ = reply.send(set_noise_reduction(transport, level));
                    self.drain_stray();
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => return Flow::Stop,
            }
            if !self.paused && last_heartbeat.elapsed() >= HEARTBEAT_INTERVAL {
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
            let layers_before = self.router.active_layers();
            let transitions = self.router.advance(now);
            // 长按 MO 到阈值、OSL 的旋钮转动停下都只改活动层、不产生过渡，apply 不会替它刷新。
            let layers_changed = self.router.active_layers() != layers_before;
            self.apply(transitions);
            if layers_changed {
                self.refresh_layers();
            }
            self.pump_runner(now);
            if !self.paused {
                if let Some(desired) = self.lighting.due(Instant::now()) {
                    match write_layer_lighting(transport, &desired) {
                        Ok(()) => self.lighting.written(desired),
                        Err(e) => {
                            self.lighting.failed_now(Instant::now());
                            self.fail(format!("背光写入失败：{e}"));
                        }
                    }
                    self.drain_stray();
                }
            }
            if self
                .halo
                .as_ref()
                .is_some_and(|h| h.last_input.elapsed() > self.halo_timeout)
            {
                self.close_halo();
            }
            // 接收器刚插上时无线链路可能还没恢复，首次查询会答"离线"；定期复查。
            if last_online_check.elapsed() >= ONLINE_RECHECK {
                last_online_check = Instant::now();
                match transport.query_online(REPLY_TIMEOUT) {
                    Ok(online) => {
                        let was_online = self.snapshot().online == Some(true);
                        self.update(|s| s.online = Some(online));
                        if online && !was_online {
                            // 本体刚上线：之前失败的背光写入在这里补一次。
                            self.refresh_layers();
                        }
                    }
                    Err(e) => self.fail(format!("在线查询失败：{e}")),
                }
                self.drain_stray();
            }
            // 本体醒来后补读设备键位（只在启动时没读到、且用户没给配置的情况下）。
            if keymap_pending && self.snapshot().online == Some(true) {
                if let Ok(map) = read_keymap(transport) {
                    self.set_keymap(map);
                    keymap_pending = false;
                }
                self.drain_stray();
            }
            if self.paused {
                // 暂停期间只被动监听标准 HID 接口；设备活动就重新接管。
                let activity = watch
                    .as_mut()
                    .map_or(Ok(false), |w| w.saw_activity(Duration::ZERO));
                match activity {
                    Ok(true) => {
                        if let Err(e) = transport
                            .set_software_online(true)
                            .and_then(|()| transport.heartbeat())
                        {
                            self.fail(format!("重新接管失败：{e}"));
                            return Flow::Reconnect;
                        }
                        last_heartbeat = Instant::now();
                        last_echo = Instant::now();
                        self.last_activity = Instant::now();
                        self.paused = false;
                        self.update(|s| {
                            s.paused = false;
                            s.heartbeats += 1;
                        });
                        (self.on_event)(Event::Resumed);
                    }
                    Ok(false) => {}
                    Err(e) => {
                        self.fail(format!("监听设备活动失败：{e}"));
                        watch = None;
                    }
                }
                continue;
            }
            if let Some(timeout) = self.idle_timeout {
                if self.last_activity.elapsed() >= timeout
                    && !self.bridge.is_holding()
                    && watch.is_some()
                {
                    self.close_halo();
                    if let Some(runner) = self.runner.as_mut() {
                        runner.cancel();
                    }
                    let transitions = self.router.cancel(false);
                    self.apply(transitions);
                    if let Err(e) = transport.set_software_online(false) {
                        self.fail(format!("交还控制失败：{e}"));
                    }
                    self.paused = true;
                    self.update(|s| s.paused = true);
                    (self.on_event)(Event::Paused);
                    continue;
                }
            }
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
            // 本体发来按键，必然在线，也不算空闲。
            self.last_activity = Instant::now();
            self.update(|s| s.online = Some(true));
            self.retry_injector();
            let now = self.now();
            (self.on_event)(Event::Key(event, now));
            if self.intercept_for_halo(event) {
                return false;
            }
            let transitions = self.router.receive(event, now);
            self.apply(transitions);
            // 切层（MO 按下 / 松开、TG / TO / OSL）本身不产生过渡，活动层集合却变了：这里再刷一次。
            self.refresh_layers();
            return false;
        }
        if let Some((kind, value)) = settings::parse_state_notice(frame) {
            match kind {
                settings::NOTICE_STANDBY => self.update(|s| s.standby = Some(value == 1)),
                _ => self.update(|s| s.link = Some(value == 1)),
            }
            return false;
        }
        if status::is_battery_notice(frame) {
            self.update(|s| s.online = Some(true));
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

    /// 光环菜单打开时接管旋钮与按键；返回 `true` 表示事件已被消费，不再交给手势路由。
    fn intercept_for_halo(&mut self, event: VendorKeyEvent) -> bool {
        let index = event.control.index();
        if !event.pressed {
            // 被吞掉的按下，其松开也吞掉；其余松开照常路由，保持物理状态一致。
            return self.halo_swallowed.remove(&index);
        }
        let Some(state) = self.halo.as_mut() else {
            return false;
        };
        state.last_input = Instant::now();
        match event.control {
            Control::KnobRight | Control::KnobLeft => {
                let n = state.halo.options.len();
                state.selected = if event.control == Control::KnobRight {
                    (state.selected + 1) % n
                } else {
                    (state.selected + n - 1) % n
                };
                (self.on_event)(Event::HaloSelect(state.selected));
                true
            }
            Control::KnobPress => {
                self.halo_swallowed.insert(index);
                let action = state.halo.options[state.selected].action.clone();
                self.close_halo();
                self.execute_halo_action(action);
                true
            }
            Control::Key1 | Control::Key2 | Control::Key3 => {
                self.halo_swallowed.insert(index);
                self.close_halo();
                true
            }
        }
    }

    fn open_halo(&mut self, id: &str) {
        let Some(halo) = self
            .snapshot()
            .keymap
            .as_ref()
            .and_then(|m| m.halo(id).cloned())
        else {
            self.fail(format!("光环菜单「{id}」不存在"));
            return;
        };
        self.halo = Some(HaloState {
            halo: halo.clone(),
            selected: 0,
            last_input: Instant::now(),
        });
        (self.on_event)(Event::HaloShow(halo, 0));
    }

    fn close_halo(&mut self) {
        if self.halo.take().is_some() {
            (self.on_event)(Event::HaloHide);
        }
    }

    fn execute_halo_action(&mut self, action: HostAction) {
        match action {
            HostAction::Keyboard(entries) => {
                let Some(mut injector) = self.injector.take() else {
                    return;
                };
                let result = self
                    .bridge
                    .press(injector.as_mut(), HALO_OWNER, &entries)
                    .and_then(|()| self.bridge.release(injector.as_mut(), HALO_OWNER));
                self.injector = Some(injector);
                if let Err(e) = result {
                    self.fail(format!("光环选项按键失败：{e}"));
                }
            }
            HostAction::PointerMove(delta) => {
                if let Some(injector) = self.injector.as_mut() {
                    if let Err(e) = injector.pointer_move(delta.dx, delta.dy) {
                        self.fail(format!("光环选项指针移动失败：{e}"));
                    }
                }
            }
            HostAction::Scroll(delta) => {
                if let Some(injector) = self.injector.as_mut() {
                    // 注入接口的垂直正值向上，配置里 dy 正值向下。
                    if let Err(e) = injector.wheel(-delta.dy, delta.dx) {
                        self.fail(format!("光环选项滚动失败：{e}"));
                    }
                }
            }
            HostAction::Halo(id) => self.open_halo(&id),
            other => {
                if let Some(runner) = self.runner.as_mut() {
                    if let Err(e) = runner.enqueue(HALO_OWNER, other) {
                        self.fail(format!("光环选项无法排队：{e}"));
                    }
                }
            }
        }
    }

    fn apply(&mut self, transitions: Vec<Transition>) {
        if transitions.is_empty() {
            return;
        }
        (self.on_event)(Event::Transitions(transitions.clone()));
        self.refresh_layers();
        let Some(mut injector) = self.injector.take() else {
            return;
        };
        let applied = self.bridge.apply(injector.as_mut(), transitions);
        let injected = self.bridge.events;
        self.update(|s| s.injected = injected);
        let mut errors = Vec::new();
        let mut halos = Vec::new();
        for (index, action) in applied.execute {
            match action {
                HostAction::Halo(id) => halos.push(id),
                HostAction::PointerMove(delta) => {
                    if let Err(e) = injector.pointer_move(delta.dx, delta.dy) {
                        errors.push(format!("控件 {index} 指针移动失败：{e}"));
                    }
                }
                HostAction::Scroll(delta) => {
                    // 一格旋钮一次注入 N 格，不走连按队列，转得快也不积压。
                    if let Err(e) = injector.wheel(-delta.dy, delta.dx) {
                        errors.push(format!("控件 {index} 滚动失败：{e}"));
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
        for id in halos {
            self.open_halo(&id);
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
        self.close_halo();
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
        self.paused = false;
        self.update(|s| {
            s.connected = false;
            s.paused = false;
        });
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

fn query_info(transport: &mut Transport) -> DeviceInfo {
    let mut info = DeviceInfo::default();
    match transport.transact(&settings::DONGLE_VERSION_REQUEST, REPLY_TIMEOUT, |f| {
        settings::is_dongle_version_reply(f)
    }) {
        Ok(f) => match settings::parse_version(&f) {
            Ok(v) => info.dongle_firmware = Some(v.to_string()),
            Err(e) => info.errors.push(format!("接收器版本：{e}")),
        },
        Err(e) => info.errors.push(format!("接收器版本：{e}")),
    }
    match transport.transact(&settings::FIRMWARE_VERSION_REQUEST, REPLY_TIMEOUT, |f| {
        settings::is_firmware_version_reply(f)
    }) {
        Ok(f) => match settings::parse_version(&f) {
            Ok(v) => info.firmware = Some(v.to_string()),
            Err(e) => info.errors.push(format!("本体版本：{e}")),
        },
        Err(e) => info.errors.push(format!("本体版本：{e}")),
    }
    match transport.transact(&settings::NOISE_REDUCTION_REQUEST, REPLY_TIMEOUT, |f| {
        settings::is_noise_reduction_reply(f)
    }) {
        Ok(f) => match settings::parse_noise_reduction(&f) {
            Ok(n) => info.noise_reduction = Some(n.raw),
            Err(e) => info.errors.push(format!("降噪：{e}")),
        },
        Err(e) => info.errors.push(format!("降噪：{e}")),
    }
    match read_lighting(transport) {
        Ok(l) => info.lighting = Some(l),
        Err(e) => info.errors.push(format!("指示灯：{e}")),
    }
    info
}

fn read_lighting(transport: &mut Transport) -> Result<Lighting, String> {
    let frame = transport
        .transact(&settings::LIGHTING_REQUEST, REPLY_TIMEOUT, |f| {
            settings::is_lighting_reply(f)
        })
        .map_err(|e| e.to_string())?;
    settings::parse_lighting(&frame).map_err(|e| e.to_string())
}

/// 把一层的背光套到设备现值上写入：只改四盏灯的 kind 与亮度，每条写等确认（不读回核对，省两次往返）。
fn write_layer_lighting(transport: &mut Transport, desired: &LayerLighting) -> Result<(), String> {
    let current = read_lighting(transport)?;
    // 全局模式不是"工作"：用户在设置页选了全灭 / 全亮，分层背光让位，什么都不写。
    if !layer_backlight_allowed(current.mode) {
        return Ok(());
    }
    let target = desired.device_lighting(&current);
    let mut writes = settings::lighting_writes(&target, &current).map_err(|e| e.to_string())?;
    // 固件只让这次被写到的灯（重新）开始显示，没写到的灯会被打断：亮着的灯不管有没有变化都补写
    // 灯效与工作时长，让它们一起重新开始。
    for (i, light) in target.lights.iter().enumerate() {
        if light.kind != 0 {
            writes.push(settings::lighting_write_frame(&target, 0x04, i as u8));
            writes.push(settings::lighting_write_frame(&target, 0x08, i as u8));
        }
    }
    if std::env::var_os("HALOKEY_DEBUG").is_some() {
        eprintln!("[halokey] 背光：现值 {current:?}\n[halokey] 背光：目标 {target:?}\n[halokey] 背光：{} 条写入", writes.len());
    }
    for request in &writes {
        transport
            .transact(request, REPLY_TIMEOUT, |f| {
                settings::matches_lighting_write_reply(f, request)
            })
            .map_err(|e| format!("指示灯写入未确认：{e}"))?;
    }
    Ok(())
}

/// 先读回当前值，只写有差异的字段，每条等确认，最后再读回核对。
fn set_lighting(transport: &mut Transport, desired: &Lighting) -> Result<Lighting, String> {
    let current = read_lighting(transport)?;
    let writes = settings::lighting_writes(desired, &current).map_err(|e| e.to_string())?;
    for request in &writes {
        transport
            .transact(request, REPLY_TIMEOUT, |f| {
                settings::matches_lighting_write_reply(f, request)
            })
            .map_err(|e| format!("指示灯写入未确认：{e}"))?;
    }
    let after = read_lighting(transport)?;
    if &after != desired {
        return Err("指示灯写入后读回与期望不一致".into());
    }
    Ok(after)
}

fn read_u32(
    transport: &mut Transport,
    request: &[u8],
    matches: fn(&[u8]) -> bool,
) -> Result<u32, String> {
    let frame = transport
        .transact(request, REPLY_TIMEOUT, |f| matches(f))
        .map_err(|e| e.to_string())?;
    settings::parse_u32_reply(&frame).map_err(|e| e.to_string())
}

fn read_bool(
    transport: &mut Transport,
    request: &[u8],
    matches: fn(&[u8]) -> bool,
) -> Result<bool, String> {
    let frame = transport
        .transact(request, REPLY_TIMEOUT, |f| matches(f))
        .map_err(|e| e.to_string())?;
    settings::parse_bool_reply(&frame).map_err(|e| e.to_string())
}

/// 读电源与麦克风设置；每项独立失败。
fn query_power(transport: &mut Transport) -> PowerInfo {
    let mut info = PowerInfo::default();
    match read_bool(
        transport,
        &settings::STANDBY_STATE_REQUEST,
        settings::is_standby_state_reply,
    ) {
        Ok(v) => info.standby = Some(v),
        Err(e) => info.errors.push(format!("待机状态：{e}")),
    }
    match read_u32(
        transport,
        &settings::STANDBY_DELAY_REQUEST,
        settings::is_standby_delay_reply,
    ) {
        Ok(v) => info.standby_delay = Some(v),
        Err(e) => info.errors.push(format!("待机延时：{e}")),
    }
    match read_u32(
        transport,
        &settings::SLEEP_DELAY_REQUEST,
        settings::is_sleep_delay_reply,
    ) {
        Ok(v) => info.sleep_delay = Some(v),
        Err(e) => info.errors.push(format!("休眠延时：{e}")),
    }
    match read_bool(
        transport,
        &settings::MIC_ENABLE_REQUEST,
        settings::is_mic_enable_reply,
    ) {
        Ok(v) => info.mic_enabled = Some(v),
        Err(e) => info.errors.push(format!("麦克风开关：{e}")),
    }
    match transport.transact(&settings::NOISE_REDUCTION_REQUEST, REPLY_TIMEOUT, |f| {
        settings::is_noise_reduction_reply(f)
    }) {
        Ok(f) => match settings::parse_noise_reduction_setting(&f) {
            Ok(v) => info.noise_reduction = Some(v),
            Err(e) => info.errors.push(format!("降噪：{e}")),
        },
        Err(e) => info.errors.push(format!("降噪：{e}")),
    }
    info
}

/// 写入后不等确认帧（格式未知），直接读回核对。
fn write_u32_setting(
    transport: &mut Transport,
    request: [u8; 8],
    read_request: &[u8],
    matches: fn(&[u8]) -> bool,
    expected: u32,
) -> Result<u32, String> {
    transport.send(&request).map_err(|e| e.to_string())?;
    std::thread::sleep(Duration::from_millis(150));
    let actual = read_u32(transport, read_request, matches)?;
    if actual != expected {
        return Err(format!("写入后读回为 {actual}，期望 {expected}"));
    }
    Ok(actual)
}

fn write_bool_setting(
    transport: &mut Transport,
    request: [u8; 5],
    read_request: &[u8],
    matches: fn(&[u8]) -> bool,
    expected: bool,
) -> Result<bool, String> {
    transport.send(&request).map_err(|e| e.to_string())?;
    std::thread::sleep(Duration::from_millis(150));
    let actual = read_bool(transport, read_request, matches)?;
    if actual != expected {
        return Err(format!("写入后读回为 {actual}，期望 {expected}"));
    }
    Ok(actual)
}

fn set_noise_reduction(
    transport: &mut Transport,
    level: u8,
) -> Result<NoiseReductionSetting, String> {
    if level > 1 {
        return Err("降噪等级只能是 0 或 1".into());
    }
    let (low, high) = settings::NOISE_REDUCTION_STUDIO_PARAMS;
    transport
        .send(&settings::noise_reduction_write(level, low, high))
        .map_err(|e| e.to_string())?;
    std::thread::sleep(Duration::from_millis(150));
    let frame = transport
        .transact(&settings::NOISE_REDUCTION_REQUEST, REPLY_TIMEOUT, |f| {
            settings::is_noise_reduction_reply(f)
        })
        .map_err(|e| e.to_string())?;
    let actual = settings::parse_noise_reduction_setting(&frame).map_err(|e| e.to_string())?;
    if actual.level != level {
        return Err(format!("写入后读回等级为 {}，期望 {level}", actual.level));
    }
    Ok(actual)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn injector_recovery_keeps_a_newer_error() {
        let mut s = Snapshot {
            last_error: Some("注入器".into()),
            ..Snapshot::default()
        };
        clear_error_if(&mut s, "注入器");
        assert_eq!(s.last_error, None);
        s.last_error = Some("键位读取失败".into());
        clear_error_if(&mut s, "注入器");
        assert_eq!(s.last_error.as_deref(), Some("键位读取失败"));
    }

    #[test]
    fn injector_retry_waits_for_the_interval_then_calls_the_factory() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let start = Instant::now();
        let mut retry = InjectorRetry {
            factory: Box::new(move || {
                // 第一次重试还失败，第二次成功。
                if counter.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err(halokey_input::Error::Unsupported("测试"))
                } else {
                    Ok(Box::new(RecordingInjector::default()) as Box<dyn Injector>)
                }
            }),
            last_attempt: start,
            error: String::new(),
        };
        assert!(retry.attempt(start + Duration::from_millis(500)).is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0, "间隔内不调用工厂");
        assert!(matches!(
            retry.attempt(start + InjectorRetry::INTERVAL),
            Some(Err(_))
        ));
        assert!(retry
            .attempt(start + InjectorRetry::INTERVAL + Duration::from_secs(1))
            .is_none());
        assert!(matches!(
            retry.attempt(start + InjectorRetry::INTERVAL * 2),
            Some(Ok(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
