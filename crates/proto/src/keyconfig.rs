//! 设备端可编程按键表（`01 06 50`）。
//!
//! ```text
//! 读:  → 01 06 50 01 <index>
//!      ← 81 06 50 11 <index> 01 <num> (<type|sign<<7> <code>) ×num
//! 写:  → 01 06 50 04 <index> 01 <num> (<type|sign<<7> <code>) ×num
//!      ← 81 06 50 14 …                 access 0x14 = 写确认
//! ```
//!
//! [CONFIRMED] 类型 `0x02` 键盘页（键码 = HID Usage ID），类型 `0x07` 鼠标页
//! （键码 N = 鼠标按键 N+1，`0x05` / `0x06` = 滚轮上 / 下，没有指针移动）。
//! [INFERRED] 类型 `0x03` 系统 / 多媒体页的取值范围、`num > 1` 组合键的实际行为、
//! `sign` 位的语义都未实测；本模块只原样保留它们，不做解释。

use crate::Error;

/// 读回复的 access 字节。
pub const ACCESS_READ_REPLY: u8 = 0x11;
/// 写确认的 access 字节。
pub const ACCESS_WRITE_ACK: u8 = 0x14;
/// 一个控件最多保存的条目数（厂商 SDK 的上限，组合键行为未实测）。
pub const MAX_ENTRIES: usize = 4;
/// 可读的 index 上限（6、7 存在但未使用）。
pub const MAX_READ_INDEX: u8 = 7;
/// 可写的 index 上限：只允许六个真实控件。
pub const MAX_WRITE_INDEX: u8 = 5;

/// 条目类型字节（已去掉 sign 位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKind {
    /// `0x02`：键盘页，键码为 HID Usage ID。
    Keyboard,
    /// `0x03`：系统 / 多媒体页，取值范围未实测。
    System,
    /// `0x07`：鼠标页。
    Mouse,
    /// 其他未见过的类型，原样保留。
    Other(u8),
}

impl EntryKind {
    /// 由类型字节解析，自动忽略 sign 位。
    pub const fn from_byte(byte: u8) -> EntryKind {
        match byte & 0x7F {
            0x02 => EntryKind::Keyboard,
            0x03 => EntryKind::System,
            0x07 => EntryKind::Mouse,
            other => EntryKind::Other(other),
        }
    }

    /// 类型字节（不含 sign 位）。
    pub const fn byte(self) -> u8 {
        match self {
            EntryKind::Keyboard => 0x02,
            EntryKind::System => 0x03,
            EntryKind::Mouse => 0x07,
            EntryKind::Other(b) => b & 0x7F,
        }
    }
}

/// 鼠标页键码。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseCode {
    /// 鼠标按键 1–5；[INFERRED] 1–3 为左 / 右 / 中键，[CONFIRMED] 4、5 实测。
    Button(u8),
    /// `0x05`。
    WheelUp,
    /// `0x06`。
    WheelDown,
}

impl MouseCode {
    /// 由键码解析；`0x07` 及以上未定义。
    pub const fn from_code(code: u8) -> Option<MouseCode> {
        match code {
            0x00..=0x04 => Some(MouseCode::Button(code + 1)),
            0x05 => Some(MouseCode::WheelUp),
            0x06 => Some(MouseCode::WheelDown),
            _ => None,
        }
    }

    /// 键码。
    pub const fn code(self) -> u8 {
        match self {
            MouseCode::Button(n) => n - 1,
            MouseCode::WheelUp => 0x05,
            MouseCode::WheelDown => 0x06,
        }
    }

    /// 按键在标准 HID 鼠标报文（report ID 0x02）按钮字节中的位；滚轮没有对应位。
    pub const fn report_bit(self) -> Option<u8> {
        match self {
            MouseCode::Button(n) => Some(1 << (n - 1)),
            _ => None,
        }
    }
}

/// 一个条目：类型、键码与 sign 位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEntry {
    /// 条目类型。
    pub kind: EntryKind,
    /// 键码；含义随类型而定。
    pub code: u8,
    /// 类型字节的最高位，语义未实测，原样保留。
    pub sign: bool,
}

impl KeyEntry {
    /// 键盘页条目。
    pub const fn keyboard(code: u8) -> KeyEntry {
        KeyEntry {
            kind: EntryKind::Keyboard,
            code,
            sign: false,
        }
    }

    /// 鼠标页条目。
    pub const fn mouse(code: MouseCode) -> KeyEntry {
        KeyEntry {
            kind: EntryKind::Mouse,
            code: code.code(),
            sign: false,
        }
    }

    /// 由线上的两个字节解析。
    pub const fn from_bytes(kind: u8, code: u8) -> KeyEntry {
        KeyEntry {
            kind: EntryKind::from_byte(kind),
            code,
            sign: kind & 0x80 != 0,
        }
    }

    /// 线上的两个字节。
    pub const fn to_bytes(self) -> [u8; 2] {
        [
            self.kind.byte() | if self.sign { 0x80 } else { 0 },
            self.code,
        ]
    }

    /// 鼠标页条目的解释；非鼠标页或未定义键码返回 `None`。
    pub const fn mouse_code(self) -> Option<MouseCode> {
        match self.kind {
            EntryKind::Mouse => MouseCode::from_code(self.code),
            _ => None,
        }
    }
}

/// [CONFIRMED] 键盘页里已确认可写入的键码：无效码 `0x00` / `0x01`、字母数字符号、F1–F24、修饰键。
pub const fn is_allowed_keyboard_code(code: u8) -> bool {
    matches!(code, 0x00 | 0x01 | 0x04..=0x63 | 0x65 | 0x68..=0x73 | 0xE0..=0xE7)
}

/// 读请求 `01 06 50 01 <index>`。
pub fn read_request(index: u8) -> Result<[u8; 5], Error> {
    if index > MAX_READ_INDEX {
        return Err(Error::InvalidControl);
    }
    Ok([0x01, 0x06, 0x50, 0x01, index])
}

/// 写请求 `01 06 50 04 <index> 01 <num> …`。
///
/// 只允许六个真实控件、1–[`MAX_ENTRIES`] 个条目，键盘页键码受 [`is_allowed_keyboard_code`]
/// 约束，鼠标页键码限于 `0x00`–`0x06`；系统页与未知类型不做范围检查。
pub fn write_request(index: u8, entries: &[KeyEntry]) -> Result<Vec<u8>, Error> {
    if index > MAX_WRITE_INDEX {
        return Err(Error::InvalidControl);
    }
    if entries.is_empty() || entries.len() > MAX_ENTRIES {
        return Err(Error::InvalidEntries);
    }
    for entry in entries {
        let ok = match entry.kind {
            EntryKind::Keyboard => is_allowed_keyboard_code(entry.code),
            EntryKind::Mouse => MouseCode::from_code(entry.code).is_some(),
            EntryKind::System | EntryKind::Other(_) => true,
        };
        if !ok {
            return Err(Error::UnsupportedKey);
        }
    }
    let mut request = vec![0x01, 0x06, 0x50, 0x04, index, 0x01, entries.len() as u8];
    for entry in entries {
        request.extend_from_slice(&entry.to_bytes());
    }
    Ok(request)
}

/// 解析后的读回复。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyConfigReply {
    /// 控件 index。
    pub index: u8,
    /// 条目列表，可能为空（未配置）。
    pub entries: Vec<KeyEntry>,
}

/// 帧是否为指定 access 与 index 的按键表回复。
pub fn matches(frame: &[u8], access: u8, index: u8) -> bool {
    frame.len() >= 5
        && frame[0] & 0x1F == 0x01
        && frame[1] == 0x06
        && frame[2] == 0x50
        && frame[3] == access
        && frame[4] == index
}

/// 是否为指定 index 的写确认。
pub fn is_write_ack(frame: &[u8], index: u8) -> bool {
    matches(frame, ACCESS_WRITE_ACK, index)
}

/// 解析任意 index 的读回复（`81 06 50 11 …`）。
pub fn parse_read_reply(frame: &[u8]) -> Result<KeyConfigReply, Error> {
    if frame.len() < 7 || frame[0] & 0x1F != 0x01 || frame[1] != 0x06 || frame[2] != 0x50 {
        return Err(Error::InvalidFrame);
    }
    if frame[3] != ACCESS_READ_REPLY || frame[5] != 0x01 {
        return Err(Error::InvalidEntries);
    }
    let count = frame[6] as usize;
    if count > 24 || 7 + count * 2 > frame.len() {
        return Err(Error::InvalidEntries);
    }
    let entries = (0..count)
        .map(|i| KeyEntry::from_bytes(frame[7 + i * 2], frame[8 + i * 2]))
        .collect();
    Ok(KeyConfigReply {
        index: frame[4],
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_kinds_ignore_sign_bit_and_round_trip() {
        assert_eq!(EntryKind::from_byte(0x82), EntryKind::Keyboard);
        assert_eq!(EntryKind::from_byte(0x07), EntryKind::Mouse);
        assert_eq!(EntryKind::from_byte(0x13), EntryKind::Other(0x13));
        let signed = KeyEntry::from_bytes(0x82, 0xE3);
        assert!(signed.sign);
        assert_eq!(signed.to_bytes(), [0x82, 0xE3]);
        assert_eq!(KeyEntry::keyboard(0x28).to_bytes(), [0x02, 0x28]);
        assert_eq!(
            KeyEntry::mouse(MouseCode::WheelDown).to_bytes(),
            [0x07, 0x06]
        );
    }

    #[test]
    fn mouse_codes_map_to_buttons_and_report_bits() {
        assert_eq!(MouseCode::from_code(0x03), Some(MouseCode::Button(4)));
        assert_eq!(MouseCode::Button(4).report_bit(), Some(0x08));
        assert_eq!(MouseCode::Button(5).report_bit(), Some(0x10));
        assert_eq!(MouseCode::Button(1).report_bit(), Some(0x01));
        assert_eq!(MouseCode::WheelUp.report_bit(), None);
        assert_eq!(MouseCode::from_code(0x07), None);
        for code in 0..=6u8 {
            assert_eq!(MouseCode::from_code(code).unwrap().code(), code);
        }
        assert_eq!(
            KeyEntry::from_bytes(0x07, 0x05).mouse_code(),
            Some(MouseCode::WheelUp)
        );
        assert_eq!(KeyEntry::keyboard(0x05).mouse_code(), None);
    }

    #[test]
    fn read_reply_validation_and_compound_entries() {
        let frame = [0x81, 6, 0x50, 0x11, 3, 1, 2, 0x82, 0xE3, 2, 0x28];
        assert!(matches(&frame, ACCESS_READ_REPLY, 3));
        assert!(!matches(&frame, ACCESS_WRITE_ACK, 3));
        assert!(!matches(&frame, ACCESS_READ_REPLY, 2));
        let reply = parse_read_reply(&frame).unwrap();
        assert_eq!(reply.index, 3);
        assert_eq!(
            reply.entries,
            vec![KeyEntry::from_bytes(0x82, 0xE3), KeyEntry::keyboard(0x28)]
        );
        assert_eq!(
            parse_read_reply(&frame[..frame.len() - 1]),
            Err(Error::InvalidEntries)
        );
        assert_eq!(
            parse_read_reply(&[0x81, 6, 0x50, 0x11, 0, 1, 25]),
            Err(Error::InvalidEntries)
        );
        assert_eq!(
            parse_read_reply(&[0x81, 6, 0x50, 0x14, 0, 1, 0]),
            Err(Error::InvalidEntries)
        );
        assert_eq!(
            parse_read_reply(&[0x06, 3, 0x0A, 0x11, 1, 0, 0]),
            Err(Error::InvalidFrame)
        );
        let empty = parse_read_reply(&[0x81, 6, 0x50, 0x11, 6, 1, 0]).unwrap();
        assert!(empty.entries.is_empty());
    }

    #[test]
    fn requests_restricted_to_real_controls_and_known_codes() {
        assert_eq!(read_request(7).unwrap(), [1, 6, 0x50, 1, 7]);
        assert_eq!(read_request(8), Err(Error::InvalidControl));
        assert_eq!(
            write_request(0, &[KeyEntry::keyboard(0x01)]).unwrap(),
            vec![1, 6, 0x50, 4, 0, 1, 1, 2, 1]
        );
        assert_eq!(
            write_request(0, &[KeyEntry::mouse(MouseCode::Button(4))]).unwrap(),
            vec![1, 6, 0x50, 4, 0, 1, 1, 7, 3]
        );
        let combo = [KeyEntry::keyboard(0xE0), KeyEntry::keyboard(0x06)];
        assert_eq!(
            write_request(1, &combo).unwrap()[6..],
            [2, 2, 0xE0, 2, 0x06]
        );
        assert_eq!(
            write_request(6, &[KeyEntry::keyboard(4)]),
            Err(Error::InvalidControl)
        );
        assert_eq!(write_request(0, &[]), Err(Error::InvalidEntries));
        assert_eq!(
            write_request(0, &[KeyEntry::keyboard(4); 5]),
            Err(Error::InvalidEntries)
        );
        assert_eq!(
            write_request(0, &[KeyEntry::keyboard(0xFF)]),
            Err(Error::UnsupportedKey)
        );
        assert_eq!(
            write_request(0, &[KeyEntry::keyboard(0x02)]),
            Err(Error::UnsupportedKey)
        );
        assert_eq!(
            write_request(
                0,
                &[KeyEntry {
                    kind: EntryKind::Mouse,
                    code: 0x07,
                    sign: false
                }]
            ),
            Err(Error::UnsupportedKey)
        );
        assert!(write_request(
            0,
            &[KeyEntry {
                kind: EntryKind::System,
                code: 0xE2,
                sign: false
            }]
        )
        .is_ok());
    }
}
