//! 把主机动作变成真实输入事件的平台后端。
//!
//! [`Injector`] 是平台无关的接口：键盘条目按 HID Usage ID 传入，鼠标按键与滚轮、
//! 指针移动用相对量。Linux 用 uinput 虚拟设备（X11 与 Wayland 通吃）；
//! macOS 用 CGEvent（需要辅助功能权限），Windows 用 SendInput。
//! macOS 与 Windows 后端只经编译检查，尚未在真机验证。

#![deny(unsafe_code)]

use std::fmt;

use halokey_host::KeyEntry;

pub use bridge::KeyBridge;

pub mod bridge;
pub mod hid_usage;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

/// 注入错误。
#[derive(Debug)]
pub enum Error {
    /// 当前平台还没有后端。
    Unsupported(&'static str),
    /// 没有对应平台键码的 HID 键码。
    UnmappedKey(u8),
    /// 底层 I/O 失败（通常是没有 `/dev/uinput` 权限）。
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unsupported(platform) => write!(f, "{platform} 上的输入注入后端尚未实现"),
            Error::UnmappedKey(code) => write!(f, "键码 0x{code:02X} 没有对应的平台按键"),
            Error::Io(e) => write!(f, "输入注入失败：{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// 鼠标按键编号 1–5（左、右、中、后退、前进）。
pub type MouseButton = u8;

/// 平台无关的注入接口。实现者负责把一组条目按顺序按下、按相反顺序松开。
pub trait Injector {
    /// 按下一组键盘条目（修饰键先于普通键）。
    fn press(&mut self, entries: &[KeyEntry]) -> Result<(), Error>;
    /// 松开一组键盘条目（普通键先于修饰键）。
    fn release(&mut self, entries: &[KeyEntry]) -> Result<(), Error>;
    /// 鼠标按键按下 / 松开。
    fn mouse_button(&mut self, button: MouseButton, down: bool) -> Result<(), Error>;
    /// 滚轮：垂直正值向上，水平正值向右。
    fn wheel(&mut self, vertical: i32, horizontal: i32) -> Result<(), Error>;
    /// 相对移动指针。
    fn pointer_move(&mut self, dx: i32, dy: i32) -> Result<(), Error>;
}

/// 打开当前平台的默认后端。
pub fn open_default() -> Result<Box<dyn Injector>, Error> {
    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(linux::UinputInjector::open()?))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::CgEventInjector::open()?))
    }
    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(windows::SendInputInjector::open()?))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(Error::Unsupported("此平台"))
    }
}

/// 把条目排成按下顺序：修饰键在前，其余保持原序；松开时反转即可。
pub fn press_order(entries: &[KeyEntry]) -> Vec<KeyEntry> {
    let mut ordered: Vec<KeyEntry> = entries
        .iter()
        .copied()
        .filter(|e| e.is_modifier())
        .collect();
    ordered.extend(entries.iter().copied().filter(|e| !e.is_modifier()));
    ordered
}

/// 记录在案的注入器：只记录调用，用于测试守护进程逻辑。
#[derive(Debug, Default)]
pub struct RecordingInjector {
    /// `(条目, 按下)` 的顺序记录。
    pub keys: Vec<(KeyEntry, bool)>,
    /// 鼠标与滚轮、移动调用的文字记录。
    pub others: Vec<String>,
}

impl Injector for RecordingInjector {
    fn press(&mut self, entries: &[KeyEntry]) -> Result<(), Error> {
        self.keys
            .extend(press_order(entries).into_iter().map(|e| (e, true)));
        Ok(())
    }
    fn release(&mut self, entries: &[KeyEntry]) -> Result<(), Error> {
        self.keys
            .extend(press_order(entries).into_iter().rev().map(|e| (e, false)));
        Ok(())
    }
    fn mouse_button(&mut self, button: MouseButton, down: bool) -> Result<(), Error> {
        self.others.push(format!(
            "button{button} {}",
            if down { "down" } else { "up" }
        ));
        Ok(())
    }
    fn wheel(&mut self, vertical: i32, horizontal: i32) -> Result<(), Error> {
        self.others.push(format!("wheel {vertical} {horizontal}"));
        Ok(())
    }
    fn pointer_move(&mut self, dx: i32, dy: i32) -> Result<(), Error> {
        self.others.push(format!("move {dx} {dy}"));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_go_down_first_and_up_last() {
        let combo = [
            KeyEntry::new(0x04),
            KeyEntry::new(0xE3),
            KeyEntry::new(0xE1),
        ];
        let mut rec = RecordingInjector::default();
        rec.press(&combo).unwrap();
        rec.release(&combo).unwrap();
        let codes: Vec<(u8, bool)> = rec.keys.iter().map(|(e, d)| (e.code, *d)).collect();
        assert_eq!(
            codes,
            vec![
                (0xE3, true),
                (0xE1, true),
                (0x04, true),
                (0x04, false),
                (0xE1, false),
                (0xE3, false)
            ]
        );
    }
}
