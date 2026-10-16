//! AU05（Ulanzi Vibe Key）厂商通道协议的纯逻辑实现。
//!
//! 本 crate 不做任何 I/O，也不含平台代码；它只负责把请求编码成线上的 64 字节报文、
//! 把设备回复解码成明文，并解释已经实测确认的几类帧。
//! 所有常量与字段布局来自 olanzi 仓库的 `docs/02-vibekey-protocol.md` 与
//! `docs/05-verification-log.md`；每个模块的文档注释都标明了证据强度。

#![forbid(unsafe_code)]

pub mod control;
pub mod error;
pub mod event;
pub mod frame;
pub mod keyconfig;
pub mod settings;
pub mod status;
pub mod tea;

pub use control::Control;
pub use error::Error;
pub use event::VendorKeyEvent;
pub use keyconfig::{EntryKind, KeyConfigReply, KeyEntry, MouseCode};
pub use settings::{IndicatorLight, Lighting, NoiseReduction, Version};
pub use status::Battery;

/// USB 厂商 ID。
pub const VENDOR_ID: u16 = 0xFFF1;
/// USB 产品 ID。
pub const PRODUCT_ID: u16 = 0x00DD;
/// 厂商通道所在接口的 usage page（接口 3）。
pub const VENDOR_USAGE_PAGE: u16 = 0xFFFC;
/// 标准 HID 接口（接口 2）上鼠标报文的 report ID。
pub const MOUSE_REPORT_ID: u8 = 0x02;
/// 标准 HID 接口（接口 2）上键盘报文的 report ID。
pub const KEYBOARD_REPORT_ID: u8 = 0x03;
