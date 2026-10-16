//! `halokey` 命令行：枚举接收器、查状态、读写按键表、监听厂商通道。
//!
//! 读命令随时安全；`set-key` 会持久化写入设备，写后自动读回核对。

mod run;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand, ValueEnum};
use halokey_hid::{enumerate, HidApi, Transport};
use halokey_proto::frame::Plaintext;
use halokey_proto::{
    status, Control, EntryKind, KeyConfigReply, KeyEntry, MouseCode, VendorKeyEvent,
};

const REPLY_TIMEOUT: Duration = Duration::from_millis(1500);

#[derive(Parser)]
#[command(name = "halokey", version, about = "Ulanzi Vibe Key (AU05) 命令行工具")]
struct Cli {
    /// 接收器有多个时，用 USB 序列号选择。
    #[arg(long, global = true)]
    serial: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 列出找到的厂商通道接口。
    List,
    /// 查询本体在线状态与电量。
    Status,
    /// 读出六个控件的设备端按键配置。
    Keys,
    /// 写一个控件的按键配置并读回核对（持久化到设备）。
    SetKey {
        /// 控件 index：0 键 1、1 键 2、2 键 3、3 旋钮按下、4 右拧、5 左拧。
        index: u8,
        /// 条目类型。
        kind: Kind,
        /// 键码：键盘页为 HID Usage ID（如 0x28），鼠标页为 button1–button5 / wheel-up / wheel-down 或数字。
        code: String,
    },
    /// 监听厂商通道并解码打印；可选同时发心跳让设备切到厂商事件模式。
    Monitor {
        /// 每 800 ms 发一次 Studio 专用心跳。
        #[arg(long)]
        heartbeat: bool,
        /// 监听秒数，0 表示直到 Ctrl-C。
        #[arg(long, default_value_t = 30)]
        seconds: u64,
    },
    /// 把设备里的六格表导出为主机键位 JSON（可编辑后用 `run --config` 加载）。
    ExportKeymap,
    /// 无界面守护进程：心跳 + 厂商事件 → 手势 → 注入（Linux uinput）。
    Run {
        /// HostProfile 或 HostKeymap JSON；缺省用设备里的六格表。
        #[arg(long)]
        config: Option<PathBuf>,
        /// 只打印转换，不注入。
        #[arg(long)]
        dry_run: bool,
        /// 打印每个事件与转换。
        #[arg(long, short)]
        verbose: bool,
    },
    /// 发送主机接管（on）或交还（off）命令。
    Online {
        #[arg(value_enum)]
        state: OnOff,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Kind {
    /// 键盘页（0x02）。
    Kb,
    /// 鼠标页（0x07）。
    Mouse,
    /// 系统 / 多媒体页（0x03），取值范围未验证。
    Sys,
}

#[derive(Clone, Copy, ValueEnum)]
enum OnOff {
    On,
    Off,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("✗ {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let api = HidApi::new()?;
    if let Command::List = cli.command {
        let devices = enumerate(&api);
        if devices.is_empty() {
            println!(
                "没有找到 Vibe Key 接收器的厂商通道（VID fff1 / PID 00dd，usage page fffc）。"
            );
            println!("Linux 上请确认 /dev/hidraw* 可读写，见 packaging/linux/70-halokey.rules。");
        }
        for d in devices {
            println!(
                "{}  接口 {}  usage page {:#06x}  序列号 {}",
                d.path.to_string_lossy(),
                d.interface,
                d.usage_page,
                d.serial.as_deref().unwrap_or("-")
            );
        }
        return Ok(());
    }
    let id = enumerate(&api)
        .into_iter()
        .find(|d| {
            cli.serial
                .as_ref()
                .is_none_or(|s| d.serial.as_deref() == Some(s))
        })
        .ok_or(halokey_hid::Error::NoDevice)?;
    let mut transport = Transport::open(&api, &id)?;
    transport.set_sink(Some(Box::new(|frame: &Plaintext| {
        println!("  (其他帧) {}", describe(frame));
    })));
    match cli.command {
        Command::List => unreachable!(),
        Command::Status => {
            let online = transport.query_online(REPLY_TIMEOUT)?;
            println!(
                "本体在线：{}",
                if online {
                    "是"
                } else {
                    "否（接收器已应答，status=0）"
                }
            );
            match transport.query_battery(REPLY_TIMEOUT) {
                Ok(b) => println!(
                    "电量：{}  电压 {} mV  充电 {}",
                    b.percent.map_or("未知".to_string(), |p| format!("{p}%")),
                    b.millivolts,
                    b.charging.map_or("未知", |c| if c { "是" } else { "否" })
                ),
                Err(halokey_hid::Error::Timeout) => println!("电量：无回复（本体关机时正常）"),
                Err(e) => return Err(e.into()),
            }
        }
        Command::Keys => {
            println!("{:<6} {:<10} 条目", "index", "控件");
            for control in Control::ALL {
                let line = match transport.read_key(control.index(), REPLY_TIMEOUT) {
                    Ok(reply) => entries_text(&reply),
                    Err(halokey_hid::Error::Timeout) => "(无回复)".to_string(),
                    Err(e) => return Err(e.into()),
                };
                println!("{:<6} {:<10} {}", control.index(), control.name(), line);
            }
        }
        Command::SetKey { index, kind, code } => {
            let entry = parse_entry(kind, &code)?;
            let before = transport.read_key(index, REPLY_TIMEOUT)?;
            println!("写入前：{}", entries_text(&before));
            let after = transport.write_key(index, &[entry], REPLY_TIMEOUT)?;
            println!("写入后：{}  ✓ 已读回核对", entries_text(&after));
        }
        Command::Monitor { heartbeat, seconds } => {
            let start = Instant::now();
            let mut last_heartbeat: Option<Instant> = None;
            println!(
                "监听 {}{}…",
                if seconds == 0 {
                    "直到 Ctrl-C".to_string()
                } else {
                    format!("{seconds} s")
                },
                if heartbeat { "，并发送心跳" } else { "" }
            );
            transport.set_sink(Some(Box::new(move |frame: &Plaintext| {
                println!(
                    "{:8.3}s  {}",
                    start.elapsed().as_secs_f64(),
                    describe(frame)
                );
            })));
            while seconds == 0 || start.elapsed() < Duration::from_secs(seconds) {
                if heartbeat
                    && last_heartbeat.is_none_or(|t| t.elapsed() >= Duration::from_millis(800))
                {
                    transport.heartbeat()?;
                    last_heartbeat = Some(Instant::now());
                }
                transport.pump(Duration::from_millis(200))?;
            }
            if heartbeat {
                println!("共发送心跳 {} 次", transport.heartbeats_sent());
            }
        }
        Command::ExportKeymap => {
            let keymap = run::keymap_from_device(&mut transport)?;
            let bytes = keymap.to_json()?;
            let value: serde_json::Value = serde_json::from_slice(&bytes)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Command::Run {
            config,
            dry_run,
            verbose,
        } => {
            // 服务自己打开接收器；先释放这里为其他命令打开的句柄。
            drop(transport);
            run::run(run::Options {
                config,
                dry_run,
                verbose,
                serial: cli.serial.clone(),
            })?;
        }
        Command::Online { state } => {
            transport.set_software_online(matches!(state, OnOff::On))?;
            println!(
                "已发送软件{}命令",
                if matches!(state, OnOff::On) {
                    "在线"
                } else {
                    "离线"
                }
            );
        }
    }
    Ok(())
}

fn parse_entry(kind: Kind, code: &str) -> Result<KeyEntry, String> {
    let number = |text: &str| -> Result<u8, String> {
        let parsed = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            Some(hex) => u8::from_str_radix(hex, 16),
            None => text.parse::<u8>(),
        };
        parsed.map_err(|_| format!("无法解析键码：{text}"))
    };
    Ok(match kind {
        Kind::Kb => KeyEntry::keyboard(number(code)?),
        Kind::Sys => KeyEntry {
            kind: EntryKind::System,
            code: number(code)?,
            sign: false,
        },
        Kind::Mouse => {
            let mouse = match code.to_ascii_lowercase().as_str() {
                "wheel-up" | "wheelup" => MouseCode::WheelUp,
                "wheel-down" | "wheeldown" => MouseCode::WheelDown,
                other => match other.strip_prefix("button") {
                    Some(n) => MouseCode::Button(
                        n.parse().map_err(|_| format!("无法解析鼠标按键：{code}"))?,
                    ),
                    None => MouseCode::from_code(number(code)?)
                        .ok_or(format!("鼠标页没有键码 {code}"))?,
                },
            };
            KeyEntry::mouse(mouse)
        }
    })
}

fn entries_text(reply: &KeyConfigReply) -> String {
    if reply.entries.is_empty() {
        return "未配置".to_string();
    }
    reply
        .entries
        .iter()
        .map(entry_text)
        .collect::<Vec<_>>()
        .join(" + ")
}

fn entry_text(entry: &KeyEntry) -> String {
    let sign = if entry.sign { " sign" } else { "" };
    match entry.kind {
        EntryKind::Keyboard => format!("键盘 {:#04x}{sign}", entry.code),
        EntryKind::System => format!("系统/多媒体 {:#04x}{sign}", entry.code),
        EntryKind::Mouse => match entry.mouse_code() {
            Some(MouseCode::Button(n)) => format!("鼠标按键 {n}{sign}"),
            Some(MouseCode::WheelUp) => format!("滚轮↑{sign}"),
            Some(MouseCode::WheelDown) => format!("滚轮↓{sign}"),
            None => format!("鼠标 {:#04x}{sign}", entry.code),
        },
        EntryKind::Other(kind) => format!("类型 {kind:#04x} 键码 {:#04x}{sign}", entry.code),
    }
}

fn describe(frame: &Plaintext) -> String {
    let hex = frame[..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    if let Some(event) = VendorKeyEvent::decode(frame) {
        return format!(
            "按键事件 {} {}  [{hex}]",
            event.control.name(),
            if event.pressed { "按下" } else { "松开" }
        );
    }
    if status::is_online_reply(frame) {
        return format!("在线状态 {:?}  [{hex}]", status::parse_online(frame));
    }
    if status::is_battery_reply(frame) {
        return format!("电量 {:?}  [{hex}]", status::parse_battery(frame));
    }
    if frame[0] & 0x1F == 0x0B {
        return format!("通知 子类型 {:#04x}  [{hex}]", frame[1]);
    }
    format!("cmd {:#04x}  [{hex}]", frame[0] & 0x1F)
}
