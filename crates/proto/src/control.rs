//! AU05 的六个物理控件及其在按键表中的编号。
//!
//! [CONFIRMED] index 0–5 与控件的对应关系及出厂键码；index 6、7 存在但未使用。

use crate::Error;

/// 物理控件。`as u8` 即按键表 index。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Control {
    /// 键 1（上）。
    Key1 = 0,
    /// 键 2（中）。
    Key2 = 1,
    /// 键 3（下）。
    Key3 = 2,
    /// 旋钮按下。
    KnobPress = 3,
    /// 旋钮 → 右拧（顺时针）。
    KnobRight = 4,
    /// 旋钮 ← 左拧（逆时针）。
    KnobLeft = 5,
}

impl Control {
    /// 全部六个控件，按 index 顺序。
    pub const ALL: [Control; 6] = [
        Control::Key1,
        Control::Key2,
        Control::Key3,
        Control::KnobPress,
        Control::KnobRight,
        Control::KnobLeft,
    ];

    /// 出厂键码（类型 0x02）。键 1 的 `0x01` 是 ErrorRollOver，等于无效码。
    pub const FACTORY_CODES: [u8; 6] = [0x01, 0x28, 0x29, 0x46, 0x4F, 0x2A];

    /// 按键表 index。
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// 由 index 得到控件；6、7 不是可用控件。
    pub const fn from_index(index: u8) -> Result<Control, Error> {
        Ok(match index {
            0 => Control::Key1,
            1 => Control::Key2,
            2 => Control::Key3,
            3 => Control::KnobPress,
            4 => Control::KnobRight,
            5 => Control::KnobLeft,
            _ => return Err(Error::InvalidControl),
        })
    }

    /// 出厂键码。
    pub const fn factory_code(self) -> u8 {
        Self::FACTORY_CODES[self as usize]
    }

    /// 是否为旋钮转动：转动只产生 0–10 ms 的脉冲，没有"按住"的概念。
    pub const fn is_rotation(self) -> bool {
        matches!(self, Control::KnobRight | Control::KnobLeft)
    }

    /// 中文名称。
    pub const fn name(self) -> &'static str {
        match self {
            Control::Key1 => "键 1",
            Control::Key2 => "键 2",
            Control::Key3 => "键 3",
            Control::KnobPress => "旋钮按下",
            Control::KnobRight => "旋钮右拧",
            Control::KnobLeft => "旋钮左拧",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_round_trip_and_factory_codes() {
        for (i, control) in Control::ALL.iter().enumerate() {
            assert_eq!(control.index() as usize, i);
            assert_eq!(Control::from_index(i as u8), Ok(*control));
            assert_eq!(control.factory_code(), Control::FACTORY_CODES[i]);
        }
        assert_eq!(Control::from_index(6), Err(Error::InvalidControl));
        assert_eq!(Control::from_index(255), Err(Error::InvalidControl));
        assert!(Control::KnobRight.is_rotation());
        assert!(!Control::KnobPress.is_rotation());
    }
}
