//! `halokey run`：Linux 上的无界面守护进程。
//!
//! 流程：主机接管（`01 01 10 00 03`）→ 心跳 0.8 s 一次 → 厂商按键事件进手势状态机 →
//! 转换交给键桥注入 uinput。退出时松开所有键、停止心跳并交还控制（`01 01 10 00 00`）。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use halokey_hid::Transport;
use halokey_host::{GestureRouter, HostKeymap, HostProfile, KeyEntry};
use halokey_input::{Injector, KeyBridge, RecordingInjector};
use halokey_proto::{status, Control, VendorKeyEvent};

const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(800);
const PUMP_TIMEOUT: Duration = Duration::from_millis(20);
const REPLY_TIMEOUT: Duration = Duration::from_millis(1500);

/// 运行参数。
pub struct Options {
    /// 配置文件（`HostProfile` 或 `HostKeymap` JSON）；缺省用设备里的六格表。
    pub config: Option<PathBuf>,
    /// 只打印，不注入。
    pub dry_run: bool,
    /// 打印每个事件。
    pub verbose: bool,
}

fn load_config(path: &PathBuf) -> Result<HostKeymap, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    match HostProfile::from_json(&bytes) {
        Ok(profile) => {
            println!("已加载配置「{}」", profile.name);
            Ok(profile.keymap)
        }
        Err(_) => Ok(HostKeymap::from_json(&bytes)?),
    }
}

fn keymap_from_device(transport: &mut Transport) -> Result<HostKeymap, Box<dyn std::error::Error>> {
    let mut bindings: Vec<(u8, Vec<KeyEntry>)> = Vec::new();
    for control in Control::ALL {
        let reply = transport.read_key(control.index(), REPLY_TIMEOUT)?;
        let entries: Vec<KeyEntry> = reply
            .entries
            .iter()
            .map(|e| KeyEntry {
                kind: e.kind.byte(),
                code: e.code,
            })
            .collect();
        bindings.push((control.index(), entries));
    }
    Ok(HostKeymap::from_device_bindings(&bindings)?)
}

/// 阻塞运行直到 Ctrl-C。
pub fn run(transport: &mut Transport, options: Options) -> Result<(), Box<dyn std::error::Error>> {
    let keymap = match &options.config {
        Some(path) => load_config(path)?,
        None => {
            let map = keymap_from_device(transport)?;
            println!("未指定配置，使用设备里的六格表作为主机映射。");
            map
        }
    };
    let mut injector: Box<dyn Injector> = if options.dry_run {
        Box::new(RecordingInjector::default())
    } else {
        halokey_input::open_default()?
    };
    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        ctrlc::set_handler(move || stop.store(true, Ordering::SeqCst))?;
    }
    let mut router = GestureRouter::new();
    router.configure(Some(keymap));
    let mut bridge = KeyBridge::new();
    let start = Instant::now();
    let now = |start: Instant| start.elapsed().as_secs_f64();

    transport.set_software_online(true)?;
    transport.heartbeat()?;
    let mut last_heartbeat = Instant::now();
    let mut echoes: u64 = 0;
    let mut last_echo: Option<Instant> = None;
    println!(
        "守护进程运行中，Ctrl-C 退出。{}",
        if options.dry_run {
            "（只打印，不注入）"
        } else {
            ""
        }
    );

    while !stop.load(Ordering::SeqCst) {
        if last_heartbeat.elapsed() >= HEARTBEAT_INTERVAL {
            transport.heartbeat()?;
            last_heartbeat = Instant::now();
        }
        match transport.recv(PUMP_TIMEOUT) {
            Ok(Some(frame)) => {
                if let Some(event) = VendorKeyEvent::decode(&frame) {
                    if options.verbose {
                        println!(
                            "{:8.3}s  {} {}",
                            now(start),
                            event.control.name(),
                            if event.pressed { "按下" } else { "松开" }
                        );
                    }
                    let transitions = router.receive(event, now(start));
                    handle(
                        &mut bridge,
                        &mut router,
                        injector.as_mut(),
                        transitions,
                        options.verbose,
                    );
                } else if frame[..5] == status::HEARTBEAT {
                    echoes += 1;
                    last_echo = Some(Instant::now());
                } else if options.verbose && status::is_battery_notice(&frame) {
                    if let Ok(b) = status::parse_battery_notice(&frame) {
                        println!(
                            "{:8.3}s  电量广播 {:?}% {} mV",
                            now(start),
                            b.percent,
                            b.millivolts
                        );
                    }
                }
            }
            Ok(None) => {}
            Err(e) => {
                // Ctrl-C 会打断阻塞读取（EINTR），这不是故障。
                if !stop.load(Ordering::SeqCst) {
                    eprintln!("✗ 读取失败：{e}");
                }
                break;
            }
        }
        let transitions = router.advance(now(start));
        handle(
            &mut bridge,
            &mut router,
            injector.as_mut(),
            transitions,
            options.verbose,
        );
        if let Some(t) = last_echo {
            if t.elapsed() > Duration::from_secs(5) {
                eprintln!("⚠ 5 秒没有收到心跳回显，接收器可能已断开。");
                last_echo = None;
            }
        }
    }

    println!("正在退出：松开所有键、停止心跳、交还控制。");
    router.cancel(false);
    for (index, e) in bridge.release_all(injector.as_mut()) {
        eprintln!("✗ 控件 {index} 松开失败：{e}");
    }
    transport.set_software_online(false)?;
    println!(
        "已退出。心跳回显 {echoes} 次，注入边沿 {} 次。",
        bridge.events
    );
    Ok(())
}

fn handle(
    bridge: &mut KeyBridge,
    router: &mut GestureRouter,
    injector: &mut dyn Injector,
    transitions: Vec<halokey_host::Transition>,
    verbose: bool,
) {
    if transitions.is_empty() {
        return;
    }
    if verbose {
        for t in &transitions {
            println!("           → {t:?}");
        }
    }
    let applied = bridge.apply(injector, transitions);
    for (index, action) in applied.execute {
        println!("  (控件 {index} 的动作 {action:?} 尚未支持执行)");
    }
    for (index, e) in applied.failures {
        eprintln!("✗ 控件 {index} 注入失败：{e}");
        router.cancel_index(index);
    }
}
