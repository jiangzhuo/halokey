//! 带层的键位表 `HostKeymap`，与 olanzi 的 JSON 格式兼容（版本 1–5）。
//!
//! Layer 0 是完整的六控件基础映射；Layer 1–3 是稀疏覆盖，未记录的控件或标记为
//! 透明继承的手势从较低的活动层继承。版本号由内容决定：使用了哪些特性就写哪个版本。
//! 版本 6 是 halokey 扩展（指针移动、鼠标页条目），olanzi 会拒绝，这正是想要的。

use std::collections::{BTreeSet, HashSet};

use serde::de::Error as _;
use serde::{Deserialize, Serialize};

use crate::action::{validate_entries, Halo, HostAction, KeyEntry, NamedHostAction};
use crate::Error;

/// 键盘动作的输出方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LongPressBehavior {
    /// 跟随物理按住 / 松开（长按：阈值到达后保持到松开）。
    Hold,
    /// 短按一次。
    Tap,
    /// 连按 N 次。
    Burst,
}

/// 可标记为透明继承的手势。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ControlGesture {
    Press,
    DoublePress,
    LongPress,
}

/// 一个控件在某一层的动作与输出设置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlActionMap {
    /// 控件 index 0–5。
    pub index: u8,
    /// 单击的键盘条目；`press_action` 存在时被覆盖。
    pub press: Vec<KeyEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub double_press: Option<Vec<KeyEntry>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_press: Option<Vec<KeyEntry>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub press_action: Option<HostAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub double_press_action: Option<HostAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_press_action: Option<HostAction>,
    #[serde(default = "ControlActionMap::hold")]
    pub press_behavior: LongPressBehavior,
    #[serde(default = "ControlActionMap::two")]
    pub press_tap_count: u32,
    #[serde(default = "ControlActionMap::tap")]
    pub double_press_behavior: LongPressBehavior,
    #[serde(default = "ControlActionMap::two")]
    pub double_press_tap_count: u32,
    /// 旧配置没有该字段，其长按动作一直保持到物理松开。
    #[serde(default = "ControlActionMap::hold")]
    pub long_press_behavior: LongPressBehavior,
    #[serde(default = "ControlActionMap::two")]
    pub long_press_tap_count: u32,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub inherited_gestures: BTreeSet<ControlGesture>,
}

impl ControlActionMap {
    const fn hold() -> LongPressBehavior {
        LongPressBehavior::Hold
    }
    const fn tap() -> LongPressBehavior {
        LongPressBehavior::Tap
    }
    const fn two() -> u32 {
        2
    }

    /// 只有单击键盘条目的控件。
    pub fn new(index: u8, press: Vec<KeyEntry>) -> ControlActionMap {
        ControlActionMap {
            index,
            press,
            double_press: None,
            long_press: None,
            press_action: None,
            double_press_action: None,
            long_press_action: None,
            press_behavior: LongPressBehavior::Hold,
            press_tap_count: 2,
            double_press_behavior: LongPressBehavior::Tap,
            double_press_tap_count: 2,
            long_press_behavior: LongPressBehavior::Hold,
            long_press_tap_count: 2,
            inherited_gestures: BTreeSet::new(),
        }
    }

    /// 单击的有效动作。
    pub fn effective_press(&self) -> HostAction {
        self.press_action
            .clone()
            .unwrap_or_else(|| HostAction::Keyboard(self.press.clone()))
    }

    /// 双击的有效动作；`None` 表示关闭。
    pub fn effective_double_press(&self) -> Option<HostAction> {
        self.double_press_action
            .clone()
            .or_else(|| self.double_press.clone().map(HostAction::Keyboard))
    }

    /// 长按的有效动作；`None` 表示关闭。
    pub fn effective_long_press(&self) -> Option<HostAction> {
        self.long_press_action
            .clone()
            .or_else(|| self.long_press.clone().map(HostAction::Keyboard))
    }

    fn uses_non_default_output(&self) -> bool {
        self.press_behavior != LongPressBehavior::Hold
            || self.press_tap_count != 2
            || self.double_press_behavior != LongPressBehavior::Tap
            || self.double_press_tap_count != 2
            || !self.inherited_gestures.is_empty()
    }

    fn actions(&self) -> [&Option<HostAction>; 3] {
        [
            &self.press_action,
            &self.double_press_action,
            &self.long_press_action,
        ]
    }
}

/// 稀疏层：未配置的控件继续查找较低的活动层，最后回到基础层。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostLayer {
    /// 1–3。
    pub id: u8,
    #[serde(default)]
    pub controls: Vec<ControlActionMap>,
}

/// 主机键位表。
#[derive(Debug, Clone, PartialEq)]
pub struct HostKeymap {
    /// 1–5，由内容决定；见 [`HostKeymap::normalize_version`]。
    pub version: u32,
    pub controls: Vec<ControlActionMap>,
    pub action_library: Vec<NamedHostAction>,
    pub layers: Vec<HostLayer>,
    /// 光环菜单（halokey 扩展）。
    pub halos: Vec<Halo>,
    /// 双击判定窗口，秒。
    pub double_press_window: f64,
    /// 长按阈值，秒。
    pub long_press_threshold: f64,
}

impl HostKeymap {
    /// JSON 大小上限。
    pub const MAX_JSON_BYTES: usize = 32 * 1024;

    /// 六个控件各一个单键动作，默认时序。
    pub fn new(controls: Vec<ControlActionMap>) -> HostKeymap {
        let mut map = HostKeymap {
            version: 1,
            controls,
            action_library: Vec::new(),
            layers: Vec::new(),
            halos: Vec::new(),
            double_press_window: 0.25,
            long_press_threshold: 0.5,
        };
        map.normalize_version();
        map
    }

    /// 出厂六键的主机映射，作为首次编辑的草稿。
    pub fn default_keymap() -> HostKeymap {
        HostKeymap::new(
            halokey_proto::Control::FACTORY_CODES
                .iter()
                .enumerate()
                .map(|(i, code)| ControlActionMap::new(i as u8, vec![KeyEntry::new(*code)]))
                .collect(),
        )
    }

    /// 从设备读回的六格表生成基础映射；条目原样保留（含组合键），按 index 排序。
    pub fn from_device_bindings(bindings: &[(u8, Vec<KeyEntry>)]) -> Result<HostKeymap, Error> {
        let mut sorted: Vec<&(u8, Vec<KeyEntry>)> = bindings.iter().collect();
        sorted.sort_by_key(|b| b.0);
        let map = HostKeymap::new(
            sorted
                .into_iter()
                .map(|(i, e)| ControlActionMap::new(*i, e.clone()))
                .collect(),
        );
        map.validate()?;
        Ok(map)
    }

    fn all_controls(&self) -> impl Iterator<Item = &ControlActionMap> {
        self.controls
            .iter()
            .chain(self.layers.iter().flat_map(|l| l.controls.iter()))
    }

    /// 内容所需的最低版本。
    pub fn required_version(&self) -> u32 {
        let extended = self.all_controls().any(|c| {
            c.press
                .iter()
                .chain(c.double_press.iter().flatten())
                .chain(c.long_press.iter().flatten())
                .any(|e| e.kind != 0x02)
                || c.actions()
                    .iter()
                    .filter_map(|a| a.as_ref())
                    .any(HostAction::uses_halokey_extensions)
        }) || self
            .action_library
            .iter()
            .any(|i| i.action.uses_halokey_extensions());
        if extended {
            return 6;
        }
        if self
            .all_controls()
            .any(ControlActionMap::uses_non_default_output)
        {
            return 5;
        }
        if !self.layers.is_empty()
            || self.all_controls().any(|c| {
                c.actions()
                    .iter()
                    .any(|a| matches!(a, Some(HostAction::MomentaryLayer(_))))
            })
        {
            return 4;
        }
        if !self.action_library.is_empty()
            || self.controls.iter().any(|c| {
                c.actions()
                    .iter()
                    .any(|a| matches!(a, Some(HostAction::Library(_))))
            })
        {
            return 3;
        }
        if self
            .controls
            .iter()
            .any(|c| c.actions().iter().any(|a| a.is_some()))
        {
            2
        } else {
            1
        }
    }

    /// 支持的最高版本。
    pub const MAX_VERSION: u32 = 6;

    /// 把版本号对齐到内容；版本已越界时保留原值，由校验报错。
    pub fn normalize_version(&mut self) {
        if (1..=Self::MAX_VERSION).contains(&self.version) {
            self.version = self.required_version();
        }
    }

    /// 完整校验，与 olanzi 一致。
    pub fn validate(&self) -> Result<(), Error> {
        if !(1..=Self::MAX_VERSION).contains(&self.version)
            || self.version < self.required_version()
        {
            return Err(Error::UnsupportedVersion);
        }
        let indices: HashSet<u8> = self.controls.iter().map(|c| c.index).collect();
        if self.controls.len() != 6 || indices != (0..6).collect() {
            return Err(Error::InvalidControls);
        }
        if self
            .controls
            .iter()
            .any(|c| !c.inherited_gestures.is_empty())
        {
            return Err(Error::InvalidInheritedGestures);
        }
        let layer_ids: HashSet<u8> = self.layers.iter().map(|l| l.id).collect();
        let layers_ok = layer_ids.len() == self.layers.len()
            && self.layers.iter().all(|layer| {
                let ids: HashSet<u8> = layer.controls.iter().map(|c| c.index).collect();
                (1..=3).contains(&layer.id)
                    && ids.len() == layer.controls.len()
                    && ids.iter().all(|i| *i < 6)
            });
        if !layers_ok {
            return Err(Error::InvalidLayer);
        }
        let (d, l) = (self.double_press_window, self.long_press_threshold);
        if !d.is_finite()
            || !l.is_finite()
            || !(0.15..=0.5).contains(&d)
            || !(0.3..=2.0).contains(&l)
            || l <= d
        {
            return Err(Error::InvalidTiming);
        }
        let ids: HashSet<&str> = self.action_library.iter().map(|i| i.id.as_str()).collect();
        if ids.len() != self.action_library.len() {
            return Err(Error::DuplicateLibraryEntry);
        }
        let mut slots = HashSet::new();
        for item in &self.action_library {
            item.validate()?;
            if !slots.insert((item.is_macro(), item.slot)) {
                return Err(Error::DuplicateLibraryEntry);
            }
        }
        let halo_ids: HashSet<&str> = self.halos.iter().map(|h| h.id.as_str()).collect();
        if halo_ids.len() != self.halos.len() {
            return Err(Error::InvalidHalo);
        }
        for halo in &self.halos {
            halo.validate()?;
            for option in &halo.options {
                if let HostAction::Halo(id) = &option.action {
                    if !halo_ids.contains(id.as_str()) {
                        return Err(Error::MissingHalo);
                    }
                }
            }
        }
        for control in self.all_controls() {
            let press = control.effective_press();
            let double = control.effective_double_press();
            let long = control.effective_long_press();
            if let HostAction::MomentaryLayer(target) = press {
                if control.index >= 4 || !(1..=3).contains(&target) {
                    return Err(Error::InvalidLayerAction);
                }
            }
            if matches!(double, Some(HostAction::MomentaryLayer(_))) {
                return Err(Error::InvalidLayerAction);
            }
            if let Some(HostAction::MomentaryLayer(target)) = long {
                if control.index >= 4 || !(1..=3).contains(&target) {
                    return Err(Error::InvalidLayerAction);
                }
            }
            if !(2..=20).contains(&control.press_tap_count)
                || !(2..=20).contains(&control.double_press_tap_count)
            {
                return Err(Error::InvalidPressTapCount);
            }
            if control.double_press_behavior == LongPressBehavior::Hold {
                return Err(Error::InvalidDoublePressBehavior);
            }
            if !(2..=20).contains(&control.long_press_tap_count) {
                return Err(Error::InvalidLongPressTapCount);
            }
            if control.index >= 4
                && (double.is_some()
                    || long.is_some()
                    || control.long_press_behavior != LongPressBehavior::Hold)
            {
                return Err(Error::UnsupportedGesture);
            }
            for action in [Some(press), double, long].into_iter().flatten() {
                let resolved = self.resolve(&action).ok_or(Error::MissingLibraryAction)?;
                resolved.validate()?;
                if let HostAction::Halo(id) = &resolved {
                    if !halo_ids.contains(id.as_str()) {
                        return Err(Error::MissingHalo);
                    }
                }
            }
            validate_entries(&control.press)?;
            if let Some(entries) = &control.double_press {
                validate_entries(entries)?;
            }
            if let Some(entries) = &control.long_press {
                validate_entries(entries)?;
            }
        }
        // 分别继承后可能拼出 MO + 双击；覆盖所有三个附加层的活动组合。
        for mask in 0u8..8 {
            let active: Vec<u8> = (1..=3u8).filter(|l| mask & (1 << (l - 1)) != 0).collect();
            for index in 0..4u8 {
                if let Some(control) = self.resolved_control(index, &active) {
                    if control.effective_press().is_momentary_layer()
                        && control.effective_double_press().is_some()
                    {
                        return Err(Error::InvalidLayerAction);
                    }
                }
            }
        }
        Ok(())
    }

    /// 按 id 查光环菜单。
    pub fn halo(&self, id: &str) -> Option<&Halo> {
        self.halos.iter().find(|h| h.id == id)
    }

    /// 只解析一层引用；无效目标或嵌套引用返回 `None`，由 [`validate`](Self::validate) 给出明确错误。
    pub fn resolve(&self, action: &HostAction) -> Option<HostAction> {
        match action {
            HostAction::Library(id) => {
                let item = self.action_library.iter().find(|i| &i.id == id)?;
                match item.action {
                    HostAction::Application(_) | HostAction::Macro(_) => Some(item.action.clone()),
                    _ => None,
                }
            }
            other => Some(other.clone()),
        }
    }

    /// 精确查找某层的控件记录；缺失返回 `None`，不把继承动作误当成覆盖。
    pub fn control(&self, index: u8, layer: u8) -> Option<&ControlActionMap> {
        if layer == 0 {
            return self.controls.iter().find(|c| c.index == index);
        }
        self.layers
            .iter()
            .find(|l| l.id == layer)?
            .controls
            .iter()
            .find(|c| c.index == index)
    }

    /// 从基础层向高层逐个叠加；动作与该手势的输出设置一起继承。
    pub fn resolved_control(&self, index: u8, active_layers: &[u8]) -> Option<ControlActionMap> {
        let mut result = self.control(index, 0)?.clone();
        let mut layers: Vec<u8> = active_layers
            .iter()
            .copied()
            .filter(|l| (1..=3).contains(l))
            .collect();
        layers.sort_unstable();
        layers.dedup();
        for layer in layers {
            let Some(overlay) = self.control(index, layer) else {
                continue;
            };
            if !overlay.inherited_gestures.contains(&ControlGesture::Press) {
                result.press = overlay.press.clone();
                result.press_action = overlay.press_action.clone();
                result.press_behavior = overlay.press_behavior;
                result.press_tap_count = overlay.press_tap_count;
            }
            if !overlay
                .inherited_gestures
                .contains(&ControlGesture::DoublePress)
            {
                result.double_press = overlay.double_press.clone();
                result.double_press_action = overlay.double_press_action.clone();
                result.double_press_behavior = overlay.double_press_behavior;
                result.double_press_tap_count = overlay.double_press_tap_count;
            }
            if !overlay
                .inherited_gestures
                .contains(&ControlGesture::LongPress)
            {
                result.long_press = overlay.long_press.clone();
                result.long_press_action = overlay.long_press_action.clone();
                result.long_press_behavior = overlay.long_press_behavior;
                result.long_press_tap_count = overlay.long_press_tap_count;
            }
        }
        result.inherited_gestures.clear();
        Some(result)
    }

    /// 从 JSON 导入：先限制大小，再检查版本，再解码并校验。
    pub fn from_json(bytes: &[u8]) -> Result<HostKeymap, Error> {
        if bytes.len() > Self::MAX_JSON_BYTES {
            return Err(Error::TooLarge);
        }
        let value: serde_json::Value = serde_json::from_slice(bytes)?;
        HostKeymap::from_value(value)
    }

    fn from_value(value: serde_json::Value) -> Result<HostKeymap, Error> {
        // 新版动作可能无法解码，先报告版本不兼容，避免误报配置损坏。
        let version = value
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .ok_or(Error::UnsupportedVersion)?;
        if !(1..=u64::from(Self::MAX_VERSION)).contains(&version) {
            return Err(Error::UnsupportedVersion);
        }
        let wire: Wire = serde_json::from_value(value)?;
        let mut map = HostKeymap {
            version: wire.version,
            controls: wire.controls,
            action_library: wire.action_library,
            layers: wire.layers,
            halos: wire.halos,
            double_press_window: wire.double_press_window,
            long_press_threshold: wire.long_press_threshold,
        };
        map.validate()?;
        map.normalize_version();
        Ok(map)
    }

    /// 导出 JSON：先校验，空的层与动作库不写出。
    pub fn to_json(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let mut map = self.clone();
        map.normalize_version();
        Ok(serde_json::to_vec(&Wire::from(map))?)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wire {
    version: u32,
    controls: Vec<ControlActionMap>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    layers: Vec<HostLayer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    action_library: Vec<NamedHostAction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    halos: Vec<Halo>,
    double_press_window: f64,
    long_press_threshold: f64,
}

impl From<HostKeymap> for Wire {
    fn from(m: HostKeymap) -> Self {
        Wire {
            version: m.version,
            controls: m.controls,
            layers: m.layers,
            action_library: m.action_library,
            halos: m.halos,
            double_press_window: m.double_press_window,
            long_press_threshold: m.long_press_threshold,
        }
    }
}

impl Serialize for HostKeymap {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut map = self.clone();
        map.normalize_version();
        Wire::from(map).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HostKeymap {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        HostKeymap::from_value(value).map_err(D::Error::custom)
    }
}
