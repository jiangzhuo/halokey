//! HID 键盘页 Usage ID 到各平台键码的映射。
//!
//! Linux 列来自内核 `hid-input.c` 的 `hid_keyboard[]` 表；`0x01`（ErrorRollOver，
//! olanzi 用作 Fn）映射到 `KEY_FN`。`0x00` 表示不执行动作，没有映射。

/// HID Usage ID → Linux evdev 键码（`KEY_*`）。
pub fn linux_keycode(usage: u8) -> Option<u16> {
    Some(match usage {
        0x01 => 0x1d0, // KEY_FN
        0x04 => 30,    // A
        0x05 => 48,    // B
        0x06 => 46,    // C
        0x07 => 32,    // D
        0x08 => 18,    // E
        0x09 => 33,    // F
        0x0A => 34,    // G
        0x0B => 35,    // H
        0x0C => 23,    // I
        0x0D => 36,    // J
        0x0E => 37,    // K
        0x0F => 38,    // L
        0x10 => 50,    // M
        0x11 => 49,    // N
        0x12 => 24,    // O
        0x13 => 25,    // P
        0x14 => 16,    // Q
        0x15 => 19,    // R
        0x16 => 31,    // S
        0x17 => 20,    // T
        0x18 => 22,    // U
        0x19 => 47,    // V
        0x1A => 17,    // W
        0x1B => 45,    // X
        0x1C => 21,    // Y
        0x1D => 44,    // Z
        0x1E => 2,     // 1
        0x1F => 3,     // 2
        0x20 => 4,     // 3
        0x21 => 5,     // 4
        0x22 => 6,     // 5
        0x23 => 7,     // 6
        0x24 => 8,     // 7
        0x25 => 9,     // 8
        0x26 => 10,    // 9
        0x27 => 11,    // 0
        0x28 => 28,    // Enter
        0x29 => 1,     // Esc
        0x2A => 14,    // Backspace
        0x2B => 15,    // Tab
        0x2C => 57,    // Space
        0x2D => 12,    // -
        0x2E => 13,    // =
        0x2F => 26,    // [
        0x30 => 27,    // ]
        0x31 => 43,    // backslash
        0x32 => 43,    // Non-US #
        0x33 => 39,    // ;
        0x34 => 40,    // '
        0x35 => 41,    // `
        0x36 => 51,    // ,
        0x37 => 52,    // .
        0x38 => 53,    // /
        0x39 => 58,    // Caps Lock
        0x3A => 59,    // F1
        0x3B => 60,
        0x3C => 61,
        0x3D => 62,
        0x3E => 63,
        0x3F => 64,
        0x40 => 65,
        0x41 => 66,
        0x42 => 67,
        0x43 => 68,  // F10
        0x44 => 87,  // F11
        0x45 => 88,  // F12
        0x46 => 99,  // Print Screen / SysRq
        0x47 => 70,  // Scroll Lock
        0x48 => 119, // Pause
        0x49 => 110, // Insert
        0x4A => 102, // Home
        0x4B => 104, // Page Up
        0x4C => 111, // Delete
        0x4D => 107, // End
        0x4E => 109, // Page Down
        0x4F => 106, // Right
        0x50 => 105, // Left
        0x51 => 108, // Down
        0x52 => 103, // Up
        0x53 => 69,  // Num Lock
        0x54 => 98,  // KP /
        0x55 => 55,  // KP *
        0x56 => 74,  // KP -
        0x57 => 78,  // KP +
        0x58 => 96,  // KP Enter
        0x59 => 79,  // KP 1
        0x5A => 80,
        0x5B => 81,
        0x5C => 75,
        0x5D => 76,
        0x5E => 77,
        0x5F => 71,
        0x60 => 72,
        0x61 => 73,  // KP 9
        0x62 => 82,  // KP 0
        0x63 => 83,  // KP .
        0x65 => 127, // Menu / Compose
        0x68 => 183, // F13
        0x69 => 184,
        0x6A => 185,
        0x6B => 186,
        0x6C => 187,
        0x6D => 188,
        0x6E => 189,
        0x6F => 190,
        0x70 => 191,
        0x71 => 192,
        0x72 => 193,
        0x73 => 194, // F24
        0xE0 => 29,  // Left Ctrl
        0xE1 => 42,  // Left Shift
        0xE2 => 56,  // Left Alt
        0xE3 => 125, // Left Meta
        0xE4 => 97,  // Right Ctrl
        0xE5 => 54,  // Right Shift
        0xE6 => 100, // Right Alt
        0xE7 => 126, // Right Meta
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_allowed_device_code_except_no_action_has_a_linux_key() {
        for code in 0u8..=255 {
            let allowed = halokey_proto::keyconfig::is_allowed_keyboard_code(code);
            match (allowed, linux_keycode(code)) {
                (true, None) => assert_eq!(code, 0, "允许的键码 0x{code:02X} 缺少 Linux 映射"),
                (false, Some(_)) => panic!("未允许的键码 0x{code:02X} 不该有映射"),
                _ => {}
            }
        }
        assert_eq!(linux_keycode(0x28), Some(28));
        assert_eq!(linux_keycode(0x68), Some(183));
        assert_eq!(linux_keycode(0xE3), Some(125));
    }
}
