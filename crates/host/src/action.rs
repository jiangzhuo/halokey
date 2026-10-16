//! 主机动作：键盘条目、切换应用、宏、动作库引用、临时切层。
//!
//! JSON 编码沿用 Swift Codable 的外部标签形式：`{"keyboard": {"_0": [...]}}`。

use serde::{Deserialize, Serialize};

use crate::Error;

/// 键盘条目；`type` 为设备端类型字节，主机动作目前只接受键盘页 `0x02`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyEntry {
    /// 条目类型；缺省为键盘页。
    #[serde(rename = "type", default = "KeyEntry::default_kind")]
    pub kind: u8,
    /// HID Usage ID；`0` 表示不执行动作。
    pub code: u8,
}

impl KeyEntry {
    const fn default_kind() -> u8 {
        0x02
    }

    /// 键盘页条目。
    pub const fn new(code: u8) -> KeyEntry {
        KeyEntry { kind: 0x02, code }
    }

    /// 是否为修饰键 `0xE0`–`0xE7`。
    pub const fn is_modifier(self) -> bool {
        self.kind == 0x02 && matches!(self.code, 0xE0..=0xE7)
    }

    /// 鼠标页条目（类型 `0x07`）。
    pub const fn mouse(code: halokey_proto::MouseCode) -> KeyEntry {
        KeyEntry {
            kind: 0x07,
            code: code.code(),
        }
    }

    /// 鼠标页条目的解释；键盘页返回 `None`。
    pub const fn mouse_code(self) -> Option<halokey_proto::MouseCode> {
        if self.kind == 0x07 {
            halokey_proto::MouseCode::from_code(self.code)
        } else {
            None
        }
    }
}

/// 一个动作最多包含的键盘条目数。
pub const MAX_KEY_ENTRIES: usize = 24;

/// 校验一组条目：键盘页只接受已确认可用的键码，鼠标页只接受按键 1–5 与滚轮；允许为空。
///
/// 鼠标页条目是 halokey 对 olanzi 格式的扩展：设备自己的表就能存它们，主机侧由注入器执行。
pub fn validate_entries(entries: &[KeyEntry]) -> Result<(), Error> {
    if entries.len() > MAX_KEY_ENTRIES {
        return Err(Error::TooManyKeyEntries);
    }
    for entry in entries {
        let ok = match entry.kind {
            0x02 => halokey_proto::keyconfig::is_allowed_keyboard_code(entry.code),
            0x07 => halokey_proto::MouseCode::from_code(entry.code).is_some(),
            _ => false,
        };
        if !ok {
            return Err(Error::UnsupportedKeyEntry(entry.kind, entry.code));
        }
    }
    Ok(())
}

/// 切换 / 启动的目标应用。平台适配层决定用 bundle、路径还是名称去找。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationTarget {
    /// macOS bundle ID；其他平台可为空。
    pub bundle_identifier: String,
    /// 可执行文件或 .app 路径；可为空。
    pub path: String,
    /// 显示名称。
    pub name: String,
}

impl ApplicationTarget {
    /// 名称非空且不超过 200 字，路径与标识至少一个非空，不含 NUL。
    pub fn validate(&self) -> Result<(), Error> {
        let name_ok = !self.name.trim().is_empty() && self.name.chars().count() <= 200;
        let sizes_ok = self.bundle_identifier.len() <= 255 && self.path.len() <= 4096;
        let locator_ok = !self.bundle_identifier.is_empty() || !self.path.is_empty();
        let no_nul = ![&self.bundle_identifier, &self.path, &self.name]
            .iter()
            .any(|s| s.contains('\0'));
        if name_ok && sizes_ok && locator_ok && no_nul {
            Ok(())
        } else {
            Err(Error::InvalidApplication)
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Arg<T> {
    #[serde(rename = "_0")]
    value: T,
}

/// 宏的一步。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "MacroStepWire", into = "MacroStepWire")]
pub enum MacroStep {
    /// 按一组键。
    Keyboard(Vec<KeyEntry>),
    /// 切换到应用。
    Application(ApplicationTarget),
    /// 等待秒数。
    Delay(f64),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum MacroStepWire {
    Keyboard(Arg<Vec<KeyEntry>>),
    Application(Arg<ApplicationTarget>),
    Delay(Arg<f64>),
}

impl From<MacroStepWire> for MacroStep {
    fn from(w: MacroStepWire) -> Self {
        match w {
            MacroStepWire::Keyboard(a) => MacroStep::Keyboard(a.value),
            MacroStepWire::Application(a) => MacroStep::Application(a.value),
            MacroStepWire::Delay(a) => MacroStep::Delay(a.value),
        }
    }
}

impl From<MacroStep> for MacroStepWire {
    fn from(s: MacroStep) -> Self {
        match s {
            MacroStep::Keyboard(v) => MacroStepWire::Keyboard(Arg { value: v }),
            MacroStep::Application(v) => MacroStepWire::Application(Arg { value: v }),
            MacroStep::Delay(v) => MacroStepWire::Delay(Arg { value: v }),
        }
    }
}

/// 指针相对位移（像素）。halokey 扩展，olanzi 不识别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PointerDelta {
    pub dx: i32,
    pub dy: i32,
}

impl PointerDelta {
    /// 单次位移的绝对值上限。
    pub const MAX_STEP: i32 = 1000;
}

/// 控件可绑定的动作。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "HostActionWire", into = "HostActionWire")]
pub enum HostAction {
    /// 键盘条目（可为组合键），也可含鼠标页条目。
    Keyboard(Vec<KeyEntry>),
    /// 切换 / 启动应用。
    Application(ApplicationTarget),
    /// 顺序宏。
    Macro(Vec<MacroStep>),
    /// 引用动作库中的一项（UUID 字符串）。
    Library(String),
    /// 按住期间激活 Layer 1–3。
    MomentaryLayer(u8),
    /// 每次触发把指针移动给定像素；旋钮每格一次。halokey 扩展，配置版本 6。
    PointerMove(PointerDelta),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum HostActionWire {
    Keyboard(Arg<Vec<KeyEntry>>),
    Application(Arg<ApplicationTarget>),
    Macro(Arg<Vec<MacroStep>>),
    Library(Arg<String>),
    MomentaryLayer(Arg<u8>),
    PointerMove(Arg<PointerDelta>),
}

impl From<HostActionWire> for HostAction {
    fn from(w: HostActionWire) -> Self {
        match w {
            HostActionWire::Keyboard(a) => HostAction::Keyboard(a.value),
            HostActionWire::Application(a) => HostAction::Application(a.value),
            HostActionWire::Macro(a) => HostAction::Macro(a.value),
            HostActionWire::Library(a) => HostAction::Library(a.value),
            HostActionWire::MomentaryLayer(a) => HostAction::MomentaryLayer(a.value),
            HostActionWire::PointerMove(a) => HostAction::PointerMove(a.value),
        }
    }
}

impl From<HostAction> for HostActionWire {
    fn from(a: HostAction) -> Self {
        match a {
            HostAction::Keyboard(v) => HostActionWire::Keyboard(Arg { value: v }),
            HostAction::Application(v) => HostActionWire::Application(Arg { value: v }),
            HostAction::Macro(v) => HostActionWire::Macro(Arg { value: v }),
            HostAction::Library(v) => HostActionWire::Library(Arg { value: v }),
            HostAction::MomentaryLayer(v) => HostActionWire::MomentaryLayer(Arg { value: v }),
            HostAction::PointerMove(v) => HostActionWire::PointerMove(Arg { value: v }),
        }
    }
}

impl HostAction {
    /// 校验动作本身；库引用的目标由整份配置校验。
    pub fn validate(&self) -> Result<(), Error> {
        match self {
            HostAction::Library(_) => Ok(()),
            HostAction::PointerMove(delta) => {
                if delta.dx.abs() <= PointerDelta::MAX_STEP
                    && delta.dy.abs() <= PointerDelta::MAX_STEP
                    && (delta.dx, delta.dy) != (0, 0)
                {
                    Ok(())
                } else {
                    Err(Error::InvalidPointerDelta)
                }
            }
            HostAction::MomentaryLayer(layer) => {
                if (1..=3).contains(layer) {
                    Ok(())
                } else {
                    Err(Error::InvalidLayer)
                }
            }
            HostAction::Keyboard(entries) => validate_entries(entries),
            HostAction::Application(target) => target.validate(),
            HostAction::Macro(steps) => {
                if !(1..=32).contains(&steps.len()) {
                    return Err(Error::InvalidStepCount);
                }
                for step in steps {
                    match step {
                        MacroStep::Keyboard(entries) => validate_entries(entries)?,
                        MacroStep::Application(target) => target.validate()?,
                        MacroStep::Delay(seconds) => {
                            if !seconds.is_finite() || !(0.05..=10.0).contains(seconds) {
                                return Err(Error::InvalidDelay);
                            }
                        }
                    }
                }
                Ok(())
            }
        }
    }

    /// 是否为临时切层。
    pub const fn is_momentary_layer(&self) -> bool {
        matches!(self, HostAction::MomentaryLayer(_))
    }

    /// 是否用到了 halokey 对 olanzi 格式的扩展（指针移动、鼠标页条目）。
    pub fn uses_halokey_extensions(&self) -> bool {
        match self {
            HostAction::PointerMove(_) => true,
            HostAction::Keyboard(entries) => entries.iter().any(|e| e.kind != 0x02),
            HostAction::Macro(steps) => steps
                .iter()
                .any(|s| matches!(s, MacroStep::Keyboard(e) if e.iter().any(|k| k.kind != 0x02))),
            _ => false,
        }
    }
}

/// 动作库中的一项；控件通过 UUID 绑定，改名或改内容不影响绑定。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedHostAction {
    /// UUID 字符串（8-4-4-4-12）。
    pub id: String,
    /// 显示名称。
    pub name: String,
    /// 同类动作内的编号 0–15。
    pub slot: u8,
    /// 只能是应用或宏。
    pub action: HostAction,
}

/// 松散的 UUID 格式检查：36 个字符、四个连字符位置正确、其余为十六进制。
pub fn is_uuid(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

impl NamedHostAction {
    /// 是否为宏（否则为应用）。
    pub const fn is_macro(&self) -> bool {
        matches!(self.action, HostAction::Macro(_))
    }

    /// 名称 1–80 字且不含 NUL，编号 0–15，动作只能是应用或宏，id 必须是 UUID。
    pub fn validate(&self) -> Result<(), Error> {
        if self.name.trim().is_empty() || self.name.chars().count() > 80 || self.name.contains('\0')
        {
            return Err(Error::InvalidLibraryName);
        }
        if self.slot > 15 {
            return Err(Error::InvalidLibrarySlot);
        }
        if !is_uuid(&self.id) {
            return Err(Error::DuplicateLibraryEntry);
        }
        match self.action {
            HostAction::Application(_) | HostAction::Macro(_) => self.action.validate(),
            _ => Err(Error::InvalidLibraryAction),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swift_style_json_round_trips() {
        let json = r#"{"keyboard":{"_0":[{"type":2,"code":40},{"code":4}]}}"#;
        let action: HostAction = serde_json::from_str(json).unwrap();
        assert_eq!(
            action,
            HostAction::Keyboard(vec![KeyEntry::new(40), KeyEntry::new(4)])
        );
        let layer: HostAction = serde_json::from_str(r#"{"momentaryLayer":{"_0":2}}"#).unwrap();
        assert_eq!(layer, HostAction::MomentaryLayer(2));
        let macro_json = r#"{"macro":{"_0":[{"delay":{"_0":0.5}},{"application":{"_0":{"bundleIdentifier":"a.b","path":"","name":"A"}}}]}}"#;
        let m: HostAction = serde_json::from_str(macro_json).unwrap();
        assert_eq!(serde_json::to_string(&m).unwrap(), macro_json);
        assert!(serde_json::from_str::<HostAction>(r#"{"futureAction":{"_0":2}}"#).is_err());
        assert_eq!(
            serde_json::to_string(&HostAction::Library("x".into())).unwrap(),
            r#"{"library":{"_0":"x"}}"#
        );
        let mv = HostAction::PointerMove(PointerDelta { dx: 20, dy: -5 });
        let text = serde_json::to_string(&mv).unwrap();
        assert_eq!(text, r#"{"pointerMove":{"_0":{"dx":20,"dy":-5}}}"#);
        assert_eq!(serde_json::from_str::<HostAction>(&text).unwrap(), mv);
        assert!(mv.validate().is_ok());
        assert!(mv.uses_halokey_extensions());
        assert_eq!(
            HostAction::PointerMove(PointerDelta { dx: 0, dy: 0 }).validate(),
            Err(Error::InvalidPointerDelta)
        );
        assert_eq!(
            HostAction::PointerMove(PointerDelta { dx: 1001, dy: 0 }).validate(),
            Err(Error::InvalidPointerDelta)
        );
    }

    #[test]
    fn validation_mirrors_olanzi() {
        assert!(HostAction::Keyboard(vec![KeyEntry::new(0)])
            .validate()
            .is_ok());
        assert_eq!(
            HostAction::Keyboard(vec![KeyEntry {
                kind: 3,
                code: 0x28
            }])
            .validate(),
            Err(Error::UnsupportedKeyEntry(3, 0x28))
        );
        assert_eq!(
            HostAction::Keyboard(vec![KeyEntry::new(4); 25]).validate(),
            Err(Error::TooManyKeyEntries)
        );
        assert_eq!(
            HostAction::MomentaryLayer(4).validate(),
            Err(Error::InvalidLayer)
        );
        assert_eq!(
            HostAction::Macro(vec![]).validate(),
            Err(Error::InvalidStepCount)
        );
        assert_eq!(
            HostAction::Macro(vec![MacroStep::Delay(0.01)]).validate(),
            Err(Error::InvalidDelay)
        );
        assert_eq!(
            HostAction::Macro(vec![MacroStep::Delay(f64::NAN)]).validate(),
            Err(Error::InvalidDelay)
        );
        let app = ApplicationTarget {
            bundle_identifier: String::new(),
            path: String::new(),
            name: "x".into(),
        };
        assert_eq!(app.validate(), Err(Error::InvalidApplication));
        let item = NamedHostAction {
            id: "123e4567-e89b-12d3-a456-426614174000".into(),
            name: "切换".into(),
            slot: 15,
            action: HostAction::Macro(vec![MacroStep::Delay(1.0)]),
        };
        assert!(item.validate().is_ok());
        assert!(item.is_macro());
        let mut bad = item.clone();
        bad.action = HostAction::Keyboard(vec![]);
        assert_eq!(bad.validate(), Err(Error::InvalidLibraryAction));
        bad = item.clone();
        bad.slot = 16;
        assert_eq!(bad.validate(), Err(Error::InvalidLibrarySlot));
        bad = item;
        bad.id = "nope".into();
        assert_eq!(bad.validate(), Err(Error::DuplicateLibraryEntry));
        assert!(is_uuid("123E4567-E89B-12D3-A456-426614174000"));
        assert!(!is_uuid("123e4567e89b12d3a456426614174000"));
    }
}
