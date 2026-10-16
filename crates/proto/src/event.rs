//! 厂商按键事件：持续发送心跳后，设备不再直出 HID 键码，而是按物理控件上报。
//!
//! [CONFIRMED] 帧为 `8b 10 <action> <pressed> <index>`：`frame[2]` 是逻辑动作号，
//! 不能当作 HID 键码；`frame[3]` 为按下（1）/ 松开（0）；`frame[4]` 为控件 index。

use crate::Control;

/// 一次物理控件事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VendorKeyEvent {
    /// 哪个控件。
    pub control: Control,
    /// 按下为 `true`，松开为 `false`。旋钮转动是极短的按下 + 松开。
    pub pressed: bool,
}

impl VendorKeyEvent {
    /// 解析厂商按键事件；不是该帧或字段越界时返回 `None`。
    pub fn decode(frame: &[u8]) -> Option<VendorKeyEvent> {
        if frame.len() < 5 || frame[0] & 0x1F != 0x0B || frame[1] != 0x10 || frame[3] > 1 {
            return None;
        }
        let control = Control::from_index(frame[4]).ok()?;
        Some(VendorKeyEvent {
            control,
            pressed: frame[3] == 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_press_and_release_by_physical_index() {
        assert_eq!(
            VendorKeyEvent::decode(&[0x8B, 0x10, 0x02, 0x01, 0x04]),
            Some(VendorKeyEvent {
                control: Control::KnobRight,
                pressed: true
            })
        );
        assert_eq!(
            VendorKeyEvent::decode(&[0x8B, 0x10, 0x00, 0x00, 0x00]),
            Some(VendorKeyEvent {
                control: Control::Key1,
                pressed: false
            })
        );
        assert_eq!(
            VendorKeyEvent::decode(&[0x8B, 0x10, 0x00, 0x02, 0x00]),
            None
        );
        assert_eq!(
            VendorKeyEvent::decode(&[0x8B, 0x10, 0x00, 0x01, 0x06]),
            None
        );
        assert_eq!(
            VendorKeyEvent::decode(&[0x8B, 0x11, 0x00, 0x01, 0x00]),
            None
        );
        assert_eq!(VendorKeyEvent::decode(&[0x8B, 0x10, 0x00, 0x01]), None);
    }
}
