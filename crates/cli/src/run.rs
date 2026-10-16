//! `halokey run`：无界面守护进程，基于 `halokey-runtime` 的设备服务。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use halokey_hid::Transport;
use halokey_host::{HostKeymap, HostProfile};
use halokey_runtime::{Backends, Config, Event, Service};

/// 运行参数。
pub struct Options {
    /// 配置文件（`HostProfile` 或 `HostKeymap` JSON）；缺省用设备里的六格表。
    pub config: Option<PathBuf>,
    /// 只打印，不注入。
    pub dry_run: bool,
    /// 打印每个事件。
    pub verbose: bool,
    /// 接收器序列号。
    pub serial: Option<String>,
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

/// 读设备六格表，生成基础主机映射（供 `export-keymap` 使用）。
pub fn keymap_from_device(
    transport: &mut Transport,
) -> Result<HostKeymap, Box<dyn std::error::Error>> {
    halokey_runtime::read_keymap(transport)
}

/// 阻塞运行直到 Ctrl-C。
pub fn run(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    let keymap = match &options.config {
        Some(path) => Some(load_config(path)?),
        None => {
            println!("未指定配置，使用设备里的六格表作为主机映射。");
            None
        }
    };
    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        ctrlc::set_handler(move || stop.store(true, Ordering::SeqCst))?;
    }
    let verbose = options.verbose;
    let service = Service::spawn(
        Config {
            keymap,
            dry_run: options.dry_run,
            serial: options.serial,
        },
        Backends::platform(),
        Box::new(move |event| match event {
            Event::Connected => println!("接收器已连接，主机已接管。"),
            Event::Disconnected => println!("接收器断开，等待重新连接…"),
            Event::Key(e, t) if verbose => {
                println!(
                    "{t:8.3}s  {} {}",
                    e.control.name(),
                    if e.pressed { "按下" } else { "松开" }
                );
            }
            Event::Transitions(list) if verbose => {
                for t in list {
                    println!("           → {t:?}");
                }
            }
            Event::Battery(b) if verbose => {
                println!("           电量 {:?}% {} mV", b.percent, b.millivolts)
            }
            Event::Error(text) => eprintln!("✗ {text}"),
            Event::Stopped => println!("设备线程已停止。"),
            _ => {}
        }),
    );
    println!(
        "守护进程运行中，Ctrl-C 退出。{}",
        if options.dry_run {
            "（只打印，不注入）"
        } else {
            ""
        }
    );
    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(100));
    }
    println!("正在退出：松开所有键、停止心跳、交还控制。");
    let snapshot = service.snapshot();
    service.stop();
    println!(
        "已退出。心跳 {} 次，回显 {} 次，注入边沿 {} 次。",
        snapshot.heartbeats, snapshot.echoes, snapshot.injected
    );
    Ok(())
}
