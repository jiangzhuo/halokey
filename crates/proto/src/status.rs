//! 接收器 / 本体状态类命令：在线查询、电量、心跳、主机接管。
//!
//! [CONFIRMED] 在线查询由接收器应答，所以永远有回复；要看回复里的 status 字节，
//! 不能把"有回复"当成"本体在线"。

use crate::Error;

/// Studio 专用心跳 `06 01 23 00 01`，约每秒一次；持续发送后按键改走厂商事件。
pub const HEARTBEAT: [u8; 5] = [0x06, 0x01, 0x23, 0x00, 0x01];
/// 设备在线状态查询 `06 03 0a 01`（接收器应答）。
pub const ONLINE_REQUEST: [u8; 4] = [0x06, 0x03, 0x0A, 0x01];
/// 电量查询 `01 01 02 01`（本体应答，关机时无回复）。
pub const BATTERY_REQUEST: [u8; 4] = [0x01, 0x01, 0x02, 0x01];

/// 主机接管 / 交还（VibeKey Lite 的用法）：`01 01 10 00 03` 软件在线，`01 01 10 00 00` 软件离线。
pub const fn software_online(online: bool) -> [u8; 5] {
    [0x01, 0x01, 0x10, 0x00, if online { 0x03 } else { 0x00 }]
}

/// 是否为在线状态回复 `06 03 0a 11 …`。
pub fn is_online_reply(frame: &[u8]) -> bool {
    frame.len() >= 5 && frame[0] & 0x1F == 0x06 && frame[1..4] == [0x03, 0x0A, 0x11]
}

/// 解析在线状态；status 字节只接受 0 / 1。
pub fn parse_online(frame: &[u8]) -> Result<bool, Error> {
    if !is_online_reply(frame) || frame[4] > 1 {
        return Err(Error::InvalidFrame);
    }
    Ok(frame[4] == 1)
}

/// 电量读数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Battery {
    /// 电压，毫伏。
    pub millivolts: u16,
    /// 百分比；超出 0–100 视为未知。
    pub percent: Option<u8>,
    /// 充电状态；字段缺失或不是 0 / 1 视为未知。
    pub charging: Option<bool>,
}

/// 是否为电量回复 `81 01 02 11 …`。
pub fn is_battery_reply(frame: &[u8]) -> bool {
    frame.len() >= 4 && frame[0] & 0x1F == 0x01 && frame[1..4] == [0x01, 0x02, 0x11]
}

/// 解析电量回复。
///
/// [CONFIRMED] Studio 的解析器按小端 16 位读取电压（帧偏移 4–5）与百分比（6–7），
/// 充电状态在偏移 10。百分比不是由电压估算的。
pub fn parse_battery(frame: &[u8]) -> Result<Battery, Error> {
    if !is_battery_reply(frame) || frame.len() < 8 {
        return Err(Error::InvalidFrame);
    }
    let millivolts = u16::from_le_bytes([frame[4], frame[5]]);
    if millivolts == 0xFFFF {
        return Err(Error::InvalidFrame);
    }
    let level = u16::from_le_bytes([frame[6], frame[7]]);
    let charging = frame.get(10).and_then(|b| match b {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    });
    Ok(Battery {
        millivolts,
        percent: (level <= 100).then_some(level as u8),
        charging,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn online_status_uses_explicit_byte_not_heartbeat_ack() {
        assert_eq!(parse_online(&[6, 3, 0x0A, 0x11, 1]), Ok(true));
        assert_eq!(parse_online(&[6, 3, 0x0A, 0x11, 0]), Ok(false));
        assert_eq!(
            parse_online(&[6, 3, 0x0A, 0x11, 2]),
            Err(Error::InvalidFrame)
        );
        assert_eq!(parse_online(&HEARTBEAT), Err(Error::InvalidFrame));
        assert_eq!(software_online(true), [1, 1, 0x10, 0, 3]);
        assert_eq!(software_online(false), [1, 1, 0x10, 0, 0]);
    }

    #[test]
    fn battery_fields_follow_studio_parser() {
        let frame = [0x81, 1, 2, 0x11, 0xF6, 0x0C, 0x0A, 0, 0xC8, 1, 1, 0];
        assert_eq!(
            parse_battery(&frame),
            Ok(Battery {
                millivolts: 3318,
                percent: Some(10),
                charging: Some(true)
            })
        );
        let short = [0x81, 1, 2, 0x11, 0xDD, 0x0D, 0x1E, 0, 0xF1, 1];
        assert_eq!(
            parse_battery(&short),
            Ok(Battery {
                millivolts: 3549,
                percent: Some(30),
                charging: None
            })
        );
        let odd = [0x81, 1, 2, 0x11, 0x00, 0x10, 0xC8, 0, 0, 0, 7];
        assert_eq!(
            parse_battery(&odd),
            Ok(Battery {
                millivolts: 4096,
                percent: None,
                charging: None
            })
        );
        assert_eq!(
            parse_battery(&[0x81, 1, 2, 0x11, 0xFF, 0xFF, 0, 0]),
            Err(Error::InvalidFrame)
        );
        assert_eq!(
            parse_battery(&[0x81, 1, 2, 0x11, 0, 0]),
            Err(Error::InvalidFrame)
        );
        assert_eq!(
            parse_battery(&[0x81, 1, 2, 0x01, 0, 0, 0, 0]),
            Err(Error::InvalidFrame)
        );
    }
}
