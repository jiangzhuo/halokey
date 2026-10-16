//! 设备设置类命令：指示灯参数、麦克风降噪、固件 / 接收器版本。
//!
//! 读命令随时安全。写命令只实现 olanzi 在 macOS 上实测过的指示灯写入；
//! 降噪等其他写命令参数格式未明，不提供。

use crate::Error;

/// 指示灯参数查询 `01 0b 88 01`。
pub const LIGHTING_REQUEST: [u8; 4] = [0x01, 0x0B, 0x88, 0x01];
/// 麦克风降噪查询 `01 01 90 01`。
pub const NOISE_REDUCTION_REQUEST: [u8; 4] = [0x01, 0x01, 0x90, 0x01];
/// 本体固件版本查询 `01 04 04 01`（本体关机时无回复）。
pub const FIRMWARE_VERSION_REQUEST: [u8; 4] = [0x01, 0x04, 0x04, 0x01];
/// 接收器固件版本查询 `06 02 03 01`（接收器应答）。
pub const DONGLE_VERSION_REQUEST: [u8; 4] = [0x06, 0x02, 0x03, 0x01];

/// 一盏指示灯的参数（键 1–3 与旋钮各一组，共 4 组，每组 5 字节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IndicatorLight {
    /// 灯效：0 关、1 常亮、2 呼吸。
    pub kind: u8,
    pub work_time: u8,
    pub breathe_level: u8,
    /// 呼吸亮度 0–20。
    pub breathe_brightness: u8,
    /// 常亮亮度 0–20。
    pub always_on_brightness: u8,
}

impl IndicatorLight {
    const fn bytes(self) -> [u8; 5] {
        [
            self.kind,
            self.work_time,
            self.breathe_level,
            self.breathe_brightness,
            self.always_on_brightness,
        ]
    }
}

/// 指示灯整体参数。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lighting {
    /// 灯光模式 0–2。
    pub mode: u8,
    /// 全亮亮度 0–20。
    pub brightness: u8,
    /// 四盏灯：键 1、键 2、键 3、旋钮。
    pub lights: [IndicatorLight; 4],
}

/// 是否为指示灯读回复 `81 0b 88 11 …`。
pub fn is_lighting_reply(frame: &[u8]) -> bool {
    frame.len() >= 4 && frame[0] & 0x1F == 0x01 && frame[1..4] == [0x0B, 0x88, 0x11]
}

/// 解析指示灯读回复：模式在偏移 6、亮度在 7，四组灯参数从 8 起每组 5 字节。
///
/// [CONFIRMED] 布局来自 olanzi 的 macOS 实测与 Studio 反汇编；未知字段原样保留。
pub fn parse_lighting(frame: &[u8]) -> Result<Lighting, Error> {
    if !is_lighting_reply(frame) || frame.len() < 28 {
        return Err(Error::InvalidFrame);
    }
    let light = |i: usize| {
        let b = 8 + 5 * i;
        IndicatorLight {
            kind: frame[b],
            work_time: frame[b + 1],
            breathe_level: frame[b + 2],
            breathe_brightness: frame[b + 3],
            always_on_brightness: frame[b + 4],
        }
    };
    Ok(Lighting {
        mode: frame[6],
        brightness: frame[7],
        lights: [light(0), light(1), light(2), light(3)],
    })
}

/// 生成把 `expected` 改成 `desired` 所需的写命令序列（每条命令只改一个字段，但携带完整记录）。
///
/// [CONFIRMED] 掩码 1 模式、2 全亮亮度、4 灯效、0x20 呼吸亮度、0x40 常亮亮度；偏移 5 为灯序号。
/// 工作时长与呼吸参数不允许修改。
pub fn lighting_writes(desired: &Lighting, expected: &Lighting) -> Result<Vec<Vec<u8>>, Error> {
    let frame = |mask: u8, focus: u8| -> Vec<u8> {
        let mut f = vec![
            0x01,
            0x0B,
            0x88,
            0x04,
            mask,
            focus,
            desired.mode,
            desired.brightness,
        ];
        for light in &desired.lights {
            f.extend_from_slice(&light.bytes());
        }
        f
    };
    let mut writes = Vec::new();
    if desired.mode != expected.mode {
        if desired.mode > 2 {
            return Err(Error::InvalidFrame);
        }
        writes.push(frame(1, 0));
    }
    if desired.brightness != expected.brightness {
        if desired.brightness > 20 {
            return Err(Error::InvalidFrame);
        }
        writes.push(frame(2, 0));
    }
    for (i, (new, old)) in desired
        .lights
        .iter()
        .zip(expected.lights.iter())
        .enumerate()
    {
        if old.work_time != new.work_time || old.breathe_level != new.breathe_level {
            return Err(Error::InvalidFrame);
        }
        if new.breathe_brightness != old.breathe_brightness {
            if new.breathe_brightness > 20 {
                return Err(Error::InvalidFrame);
            }
            writes.push(frame(0x20, i as u8));
        }
        if new.always_on_brightness != old.always_on_brightness {
            if new.always_on_brightness > 20 {
                return Err(Error::InvalidFrame);
            }
            writes.push(frame(0x40, i as u8));
        }
        if new.kind != old.kind {
            // 0 关、1 常亮、2 呼吸。olanzi 只在键 1 上实测过呼吸；doitian 的 Windows 工具
            // 用厂商 SDK 读回确认四个通道都接受 workType=2，故四盏灯都放开。
            if new.kind > 2 {
                return Err(Error::InvalidFrame);
            }
            writes.push(frame(4, i as u8));
        }
    }
    Ok(writes)
}

/// 是否为与给定写请求匹配的指示灯写确认（access 0x14，掩码、灯序号与被改字段一致）。
pub fn matches_lighting_write_reply(frame: &[u8], request: &[u8]) -> bool {
    if frame.len() < 28
        || request.len() < 28
        || frame[0] & 0x1F != 0x01
        || frame[1..4] != [0x0B, 0x88, 0x14]
    {
        return false;
    }
    if frame[4] != request[4] || frame[5] != request[5] || request[5] >= 4 {
        return false;
    }
    let base = 8 + 5 * usize::from(request[5]);
    let offset = match request[4] {
        1 => 6,
        2 => 7,
        4 => base,
        0x20 => base + 3,
        0x40 => base + 4,
        _ => return false,
    };
    frame[offset] == request[offset]
}

/// 麦克风降噪读回复：两个麦克风各一对（低、高）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoiseReduction {
    pub raw: [u8; 4],
}

/// 是否为降噪读回复 `81 01 90 11 …`。
pub fn is_noise_reduction_reply(frame: &[u8]) -> bool {
    frame.len() >= 8 && frame[0] & 0x1F == 0x01 && frame[1..4] == [0x01, 0x90, 0x11]
}

/// 解析降噪读回复；[INFERRED] 四个字节为"双麦各 低 / 高"，样本 `00 64 00 64`。
pub fn parse_noise_reduction(frame: &[u8]) -> Result<NoiseReduction, Error> {
    if !is_noise_reduction_reply(frame) {
        return Err(Error::InvalidFrame);
    }
    Ok(NoiseReduction {
        raw: [frame[4], frame[5], frame[6], frame[7]],
    })
}

/// 版本号三元组。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version(pub u8, pub u8, pub u8);

impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// 是否为本体固件版本回复 `81 04 04 11 …`。
pub fn is_firmware_version_reply(frame: &[u8]) -> bool {
    frame.len() >= 7 && frame[0] & 0x1F == 0x01 && frame[1..4] == [0x04, 0x04, 0x11]
}

/// 是否为接收器版本回复 `06 02 03 11 …`。
pub fn is_dongle_version_reply(frame: &[u8]) -> bool {
    frame.len() >= 7 && frame[0] & 0x1F == 0x06 && frame[1..4] == [0x02, 0x03, 0x11]
}

/// 解析版本回复：payload 前三字节为主、次、修订号（样本 `04 04 02` = 4.4.2）。
///
/// [INFERRED] 回复里版本三元组的精确偏移只由 olanzi 的 `… 04 04 02 …` 样本推断为紧跟 access 字节；
/// 本函数先在偏移 4 取，若那里是 0 则向后找第一个非零三元组，待真机确认。
pub fn parse_version(frame: &[u8]) -> Result<Version, Error> {
    if !(is_firmware_version_reply(frame) || is_dongle_version_reply(frame)) {
        return Err(Error::InvalidFrame);
    }
    let payload = &frame[4..];
    let start = payload
        .iter()
        .position(|b| *b != 0)
        .ok_or(Error::InvalidFrame)?;
    if start + 3 > payload.len() {
        return Err(Error::InvalidFrame);
    }
    Ok(Version(
        payload[start],
        payload[start + 1],
        payload[start + 2],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_lighting() -> Vec<u8> {
        // olanzi 文档样本：`00 00 02 07 02 | 0a 02 07 02 02 | 0a 02 07 02 01 | 0a 02 07 02 00` 前置 `81 0b 88 11 00 00 mode brightness`
        let mut f = vec![0x81, 0x0B, 0x88, 0x11, 0, 0, 1, 10];
        f.extend_from_slice(&[
            0, 0, 2, 7, 2, 0x0a, 2, 7, 2, 2, 0x0a, 2, 7, 2, 1, 0x0a, 2, 7, 2, 0,
        ]);
        f
    }

    #[test]
    fn lighting_round_trip_and_write_masks() {
        let frame = sample_lighting();
        let lighting = parse_lighting(&frame).unwrap();
        assert_eq!((lighting.mode, lighting.brightness), (1, 10));
        assert_eq!(lighting.lights[1].kind, 0x0a);
        assert_eq!(lighting.lights[3].always_on_brightness, 0);
        let mut desired = lighting.clone();
        desired.brightness = 15;
        desired.lights[0].kind = 2;
        desired.lights[3].always_on_brightness = 5;
        let writes = lighting_writes(&desired, &lighting).unwrap();
        assert_eq!(writes.len(), 3);
        // 顺序：模式、全亮亮度，然后按灯序号逐灯（呼吸亮度、常亮亮度、灯效）。
        assert_eq!(&writes[0][..6], &[1, 0x0B, 0x88, 4, 2, 0]);
        assert_eq!(&writes[1][..6], &[1, 0x0B, 0x88, 4, 4, 0]);
        assert_eq!(&writes[2][..6], &[1, 0x0B, 0x88, 4, 0x40, 3]);
        assert_eq!(writes[0].len(), 28);
        let mut ack = writes[2].clone();
        ack[0] = 0x81;
        ack[3] = 0x14;
        assert!(matches_lighting_write_reply(&ack, &writes[2]));
        ack[8 + 5 * 3 + 4] = 9;
        assert!(!matches_lighting_write_reply(&ack, &writes[2]));
        let mut bad = lighting.clone();
        bad.brightness = 21;
        assert_eq!(lighting_writes(&bad, &lighting), Err(Error::InvalidFrame));
        bad = lighting.clone();
        bad.lights[2].kind = 3;
        assert_eq!(lighting_writes(&bad, &lighting), Err(Error::InvalidFrame));
        let mut breathe = lighting.clone();
        breathe.lights[2].kind = 2;
        assert_eq!(lighting_writes(&breathe, &lighting).unwrap().len(), 1);
        bad = lighting.clone();
        bad.lights[0].work_time = 9;
        assert_eq!(lighting_writes(&bad, &lighting), Err(Error::InvalidFrame));
        assert_eq!(parse_lighting(&frame[..27]), Err(Error::InvalidFrame));
    }

    #[test]
    fn noise_reduction_and_versions() {
        let nr = parse_noise_reduction(&[0x81, 1, 0x90, 0x11, 0, 0x64, 0, 0x64]).unwrap();
        assert_eq!(nr.raw, [0, 0x64, 0, 0x64]);
        assert_eq!(
            parse_version(&[0x81, 4, 4, 0x11, 4, 4, 2, 0])
                .unwrap()
                .to_string(),
            "4.4.2"
        );
        assert_eq!(
            parse_version(&[0x06, 2, 3, 0x11, 0, 0, 4, 4, 2]).unwrap(),
            Version(4, 4, 2)
        );
        assert_eq!(
            parse_version(&[0x06, 2, 3, 0x11, 0, 0, 0]),
            Err(Error::InvalidFrame)
        );
        assert_eq!(
            parse_version(&[0x81, 1, 2, 0x11, 4, 4, 2]),
            Err(Error::InvalidFrame)
        );
    }
}
