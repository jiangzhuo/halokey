//! 分层背光：每层四盏灯的效果，与设备指示灯记录的映射。

use halokey_proto::{IndicatorLight, Lighting};
use serde::{Deserialize, Serialize};

use crate::Error;

/// 灯效，对应设备 `kind` 0 / 1 / 2。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LightMode {
    Off,
    Solid,
    Breathe,
}

impl LightMode {
    pub const fn kind(self) -> u8 {
        match self {
            LightMode::Off => 0,
            LightMode::Solid => 1,
            LightMode::Breathe => 2,
        }
    }
}

/// 一盏灯：效果 + 亮度 0–20。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightEffect {
    pub mode: LightMode,
    pub brightness: u8,
}

/// 一层的四盏灯：键 1、键 2、键 3、旋钮。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerLighting {
    pub lights: [LightEffect; 4],
}

impl LayerLighting {
    pub const MAX_BRIGHTNESS: u8 = 20;

    pub fn validate(&self) -> Result<(), Error> {
        if self
            .lights
            .iter()
            .any(|l| l.brightness > Self::MAX_BRIGHTNESS)
        {
            return Err(Error::InvalidLighting);
        }
        Ok(())
    }

    /// 工作模式下灯效显示的时长；255 约等于常显（2026-10-18 实测，vibed 同样取值）。
    pub const PERSISTENT_WORK_TIME: u8 = 255;

    /// 套到设备当前记录上：改 `kind`、两个亮度字段和工作时长（亮的灯写 255 使其常显），
    /// 全亮亮度与呼吸参数保持现值；只要有亮的灯，全局模式切到 2（工作），否则单灯设置不显示。
    pub fn device_lighting(&self, current: &Lighting) -> Lighting {
        let mut out = current.clone();
        for (slot, effect) in out.lights.iter_mut().zip(self.lights.iter()) {
            let lit = effect.mode != LightMode::Off;
            *slot = IndicatorLight {
                kind: effect.mode.kind(),
                work_time: if lit {
                    Self::PERSISTENT_WORK_TIME
                } else {
                    slot.work_time
                },
                breathe_level: slot.breathe_level,
                breathe_brightness: effect.brightness,
                always_on_brightness: effect.brightness,
            };
        }
        if self.lights.iter().any(|l| l.mode != LightMode::Off) {
            out.mode = 2;
        }
        out
    }
}
