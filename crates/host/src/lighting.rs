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
///
/// 反序列化时先收任意长度的数组，长度在 [`LayerLighting::validate`] 里检查，这样错误是中文而不是 serde 的英文。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "LayerLightingWire", into = "LayerLightingWire")]
pub struct LayerLighting {
    pub lights: [LightEffect; 4],
}

#[derive(Serialize, Deserialize)]
struct LayerLightingWire {
    lights: Vec<LightEffect>,
}

impl TryFrom<LayerLightingWire> for LayerLighting {
    type Error = Error;
    fn try_from(w: LayerLightingWire) -> Result<LayerLighting, Error> {
        let lights: [LightEffect; 4] = w
            .lights
            .try_into()
            .map_err(|_| Error::InvalidLightingCount)?;
        Ok(LayerLighting { lights })
    }
}

impl From<LayerLighting> for LayerLightingWire {
    fn from(l: LayerLighting) -> LayerLightingWire {
        LayerLightingWire {
            lights: l.lights.to_vec(),
        }
    }
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
    /// 全亮亮度与呼吸参数保持现值；全局模式切到 2（工作），否则单灯设置不显示。
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
        // 有记录就切到工作模式：模式 0 / 1 都会忽略单灯设置，四盏全关也要靠 kind=0 才真的灭。
        out.mode = 2;
        out
    }
}
