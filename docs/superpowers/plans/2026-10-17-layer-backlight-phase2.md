# 分层背光 · 第 2 期实施计划（模型、运行时、UI）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 每个 Layer 可以有自己的四盏灯效果（关 / 常亮 / 呼吸 + 亮度 0–20），按住 MO 切层时设备灯光跟着变；键位页的 Layer 条右侧显示并编辑当前层的背光，渲染图上的灯圈实时预览。

**Architecture:** 背光记录进键位文件（`lighting` 在顶层与每个 `layers[]` 里，可缺省）。Rust `host` 负责解析、校验、"活动层集合 → 有效背光"的纯函数和到设备记录的映射；`runtime` 在活动层变化时经 100 ms 防抖把有效背光写进设备（只改四盏灯的 kind / 亮度，沿用现有最小差异写序列），并把活动层与有效背光放进快照与事件；前端只编辑草稿并预览。

**Tech Stack:** Rust workspace（`halokey-proto` / `halokey-host` / `halokey-runtime` / `apps/halokey`），Svelte 5 + TypeScript + vitest（`ui/`）。

**Spec:** `docs/superpowers/specs/2026-10-17-settings-ui-redesign-design.md` §4.1 背光条、§4.2 灯圈预览、§7 数据模型、§8 `snapshot.effectiveLighting` 与事件 `layers`、§10 测试、§12 第 2 期。

## Global Constraints

- 配置版本：含 `lighting` 的文件 `required_version()` 为 6；版本由 Rust 决定，前端不改版本号。
- `LightEffect { mode: off|solid|breathe, brightness: 0–20 }`；映射到设备记录：`kind` = 0/1/2，`brightness` 同时写入 `always_on_brightness` 与 `breathe_brightness`；`work_time`、`breathe_level` 保持设备现值。
- 运行时规则（§7）：活动层集合变化时取编号最高且带背光的活动层，否则 Layer 0；与上次写入的有效背光比较，不同才写设备；写入经 100 ms 防抖；所有层都没有背光时不碰灯。
- 灯光模式 / 全亮亮度不在键位文件里；运行时只改四盏灯的 `kind` 与两个亮度字段，不碰 `mode` / `brightness`。
- 四盏灯顺序固定：键 1、键 2、键 3、旋钮（与 `proto::Lighting.lights` 一致）。
- 界面文字中文为源，全部经 `t()`，英文表同步；`npm test`、`npm run check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`、`python3 tools/check_docs.py` 每个任务结束全绿；提交英文，末尾 `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`。
- 不新增依赖。
- 调试版加载 Vite 开发服务器（`cd ui && npm run dev`），发布版 `cargo build --release -p halokey-app` 自动重编嵌入 `ui/dist`（先 `npm run build`）。

## Review Focus

1. **旧文件没有 `lighting`**：打开、保存、运行都必须和第 1 期完全一样，设备灯光一次都不被改写。→ Task 1 往返测试（无字段 → 序列化仍无字段）、Task 3 调度器测试（无背光 → 从不写）。
2. **按住 MO 松开后灯光回不到 Layer 0**（或快速按放两次只写一次）：松开时活动层集合为空 → 有效背光回到 Layer 0 → 100 ms 后写回。→ Task 3 调度器用假时钟测"按下 → 松开 → 一次写回"与"按下 → 快速松开 → 合并"。
3. **设备写入失败（本体睡着、读回不一致）**：只记一条错误，不重试风暴；下一次活动层变化再试。→ Task 4 对失败分支的处理：`fail()` 一次并清掉 pending，`last_written` 置 `None`。
4. **Layer 1 "跟随 Layer 0" 开关来回切**：关掉即创建记录（内容复制当前有效背光），打开即删除记录；来回切不留垃圾字段。→ Task 6 reducer 测试。
5. **亮度越界 / 灯数不对的文件**：Rust 校验拒绝并给中文错误，前端保存时底栏红字。→ Task 1 校验测试 + Task 8 mock 校验。

---

## 文件结构

```
crates/host/src/lighting.rs        LightEffect / LayerLighting：类型、serde、校验、到 proto::Lighting 的映射
crates/host/src/keymap.rs          HostKeymap.lighting / HostLayer.lighting、Wire、required_version、validate、effective_lighting()
crates/host/tests/lighting.rs      纯函数测试
crates/runtime/src/lighting.rs     LightingScheduler：防抖 + 去重（假时钟可测）
crates/runtime/src/lib.rs          Snapshot.effective_lighting、Event::Layers、调度器接线、写设备
apps/halokey/src/main.rs           UiSnapshot.effective_lighting、UiEvent::Layers
ui/src/model/keymap.ts             LightEffect / LayerLighting 类型
ui/src/model/draft.ts              effectiveLighting / setLightEffect / setFollowBase / clearBaseLighting
ui/src/lib/tauri.ts                UiSnapshot.effectiveLighting、UiEvent layers
ui/src/components/BacklightStrip.svelte   Layer 条右侧的状态条 + 展开编辑器
ui/src/components/LayerBar.svelte  传入 right 插槽（已有）
ui/src/pages/Keymap.svelte         接背光条、渲染图灯圈取有效背光
ui/src/lib/i18n.ts                 新字符串
docs/03-config-format.md (+zh)     lighting 字段
docs/examples/layer-backlight.json 示例：Layer 0 常亮、Layer 1 呼吸、键 1 = MO(1)
```

---

### Task 1: 模型 — `LightEffect` / `LayerLighting`、键位文件字段、校验、版本

**Files:**
- Create: `crates/host/src/lighting.rs`
- Modify: `crates/host/src/keymap.rs`（`HostKeymap`、`HostLayer`、`Wire`、`required_version`、`validate`、`new`）, `crates/host/src/lib.rs`
- Test: `crates/host/tests/lighting.rs`

**Interfaces:**
- Produces（`halokey_host` 导出）:
  - `pub enum LightMode { Off, Solid, Breathe }`（serde `rename_all = "lowercase"`）
  - `pub struct LightEffect { pub mode: LightMode, pub brightness: u8 }`
  - `pub struct LayerLighting { pub lights: [LightEffect; 4] }`；`LayerLighting::validate(&self) -> Result<(), Error>`（亮度 ≤ 20）；`LayerLighting::device_lighting(&self, current: &halokey_proto::Lighting) -> halokey_proto::Lighting`
  - `HostKeymap.lighting: Option<LayerLighting>`、`HostLayer.lighting: Option<LayerLighting>`
  - `HostKeymap::effective_lighting(&self, active: &[u8]) -> Option<&LayerLighting>`
  - `Error::InvalidLighting`（中文 Display："背光亮度必须在 0–20 之间。"）

- [ ] **Step 1: 写失败的测试**

`crates/host/tests/lighting.rs`：

```rust
//! 分层背光：字段往返、版本、校验、有效背光解析、到设备记录的映射。
use halokey_host::{Error, HostAction, HostKeymap, HostLayer, LayerLighting, LightEffect, LightMode};
use halokey_proto::{IndicatorLight, Lighting};

fn effects(mode: LightMode, brightness: u8) -> LayerLighting {
    LayerLighting { lights: [LightEffect { mode, brightness }; 4] }
}

#[test]
fn files_without_lighting_round_trip_unchanged() {
    let map = HostKeymap::default_keymap();
    let json = String::from_utf8(map.to_json().unwrap()).unwrap();
    assert!(!json.contains("lighting"));
    let back = HostKeymap::from_json(json.as_bytes()).unwrap();
    assert_eq!(back.lighting, None);
    assert_eq!(back.version, 1);
}

#[test]
fn lighting_round_trips_and_requires_version_6() {
    let mut map = HostKeymap::default_keymap();
    map.lighting = Some(effects(LightMode::Solid, 14));
    map.controls[0].press_action = Some(HostAction::MomentaryLayer(1));
    map.layers.push(HostLayer {
        id: 1,
        controls: Vec::new(),
        lighting: Some(effects(LightMode::Breathe, 20)),
    });
    map.normalize_version();
    assert_eq!(map.version, 6);
    let json = map.to_json().unwrap();
    let text = String::from_utf8(json.clone()).unwrap();
    assert!(text.contains("\"mode\": \"breathe\"") || text.contains("\"mode\":\"breathe\""));
    let back = HostKeymap::from_json(&json).unwrap();
    assert_eq!(back.lighting, map.lighting);
    assert_eq!(back.layers[0].lighting, map.layers[0].lighting);
}

#[test]
fn brightness_above_20_is_rejected() {
    let mut map = HostKeymap::default_keymap();
    map.lighting = Some(effects(LightMode::Solid, 21));
    map.normalize_version();
    assert!(matches!(map.validate(), Err(Error::InvalidLighting)));
    let json = br#"{"version":6,"controls":[],"lighting":{"lights":[{"mode":"solid","brightness":3}]},"doublePressWindow":0.25,"longPressThreshold":0.5}"#;
    assert!(HostKeymap::from_json(json).is_err(), "三盏灯的记录必须被拒绝");
}

#[test]
fn effective_lighting_picks_highest_active_layer_with_a_record() {
    let mut map = HostKeymap::default_keymap();
    map.controls[0].press_action = Some(HostAction::MomentaryLayer(1));
    map.controls[1].press_action = Some(HostAction::MomentaryLayer(3));
    map.layers.push(HostLayer { id: 1, controls: Vec::new(), lighting: Some(effects(LightMode::Breathe, 5)) });
    map.layers.push(HostLayer { id: 3, controls: Vec::new(), lighting: None });
    assert_eq!(map.effective_lighting(&[]), None, "Layer 0 没记录 → 不管灯");
    assert_eq!(map.effective_lighting(&[3]), None, "Layer 3 没记录且 Layer 0 没记录");
    assert_eq!(map.effective_lighting(&[1, 3]).unwrap().lights[0].brightness, 5);
    map.lighting = Some(effects(LightMode::Solid, 9));
    assert_eq!(map.effective_lighting(&[3]).unwrap().lights[0].brightness, 9, "Layer 3 没记录 → 回到 Layer 0");
    assert_eq!(map.effective_lighting(&[1]).unwrap().lights[0].brightness, 5);
}

#[test]
fn device_mapping_keeps_work_time_and_breathe_level() {
    let current = Lighting {
        mode: 2,
        brightness: 14,
        lights: [IndicatorLight { kind: 1, work_time: 7, breathe_level: 3, breathe_brightness: 1, always_on_brightness: 2 }; 4],
    };
    let mut want = effects(LightMode::Breathe, 20);
    want.lights[3] = LightEffect { mode: LightMode::Off, brightness: 0 };
    let out = want.device_lighting(&current);
    assert_eq!(out.mode, 2);
    assert_eq!(out.brightness, 14);
    assert_eq!(out.lights[0], IndicatorLight { kind: 2, work_time: 7, breathe_level: 3, breathe_brightness: 20, always_on_brightness: 20 });
    assert_eq!(out.lights[3].kind, 0);
    assert_eq!(out.lights[3].always_on_brightness, 0);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cd ~/Desktop/halokey && source ~/.cargo/env && cargo test -p halokey-host --test lighting 2>&1 | grep -E "^error|cannot find" | head -3`
Expected: 编译错误，`LayerLighting` / `LightEffect` / `LightMode` 不存在。

- [ ] **Step 3: 实现 `crates/host/src/lighting.rs`**

```rust
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
        if self.lights.iter().any(|l| l.brightness > Self::MAX_BRIGHTNESS) {
            return Err(Error::InvalidLighting);
        }
        Ok(())
    }

    /// 套到设备当前记录上：只改 `kind` 与两个亮度字段，模式 / 全亮亮度 / 工作时长 / 呼吸参数保持现值。
    pub fn device_lighting(&self, current: &Lighting) -> Lighting {
        let mut out = current.clone();
        for (slot, effect) in out.lights.iter_mut().zip(self.lights.iter()) {
            *slot = IndicatorLight {
                kind: effect.mode.kind(),
                work_time: slot.work_time,
                breathe_level: slot.breathe_level,
                breathe_brightness: effect.brightness,
                always_on_brightness: effect.brightness,
            };
        }
        out
    }
}
```

`crates/host/src/error.rs`：在 `InvalidPressTapCount` 后加变体 `InvalidLighting`，`Display` 里加 `Error::InvalidLighting => f.write_str("背光亮度必须在 0–20 之间。"),`。`crates/host/src/lib.rs`：`pub mod lighting;` 与 `pub use lighting::{LayerLighting, LightEffect, LightMode};`。

- [ ] **Step 4: 接进 `keymap.rs`**

- `HostLayer` 加字段：`#[serde(default, skip_serializing_if = "Option::is_none")] pub lighting: Option<LayerLighting>,`（`HostLayer` 直接派生 serde，所以字段属性写在结构体上）。现有三处 `HostLayer { id, controls }` 字面量补 `lighting: None`：`crates/host/tests/gesture.rs:332`、`:350`、`crates/host/tests/keymap_json.rs:196`。
- `HostKeymap` 加字段 `pub lighting: Option<LayerLighting>`；`new()` 里 `lighting: None`；`Wire` 加 `#[serde(default, skip_serializing_if = "Option::is_none")] lighting: Option<LayerLighting>` 并在 `From<HostKeymap> for Wire` 与 `from_value` 两处搬运。
- `required_version()` 开头加：

```rust
        if self.lighting.is_some() || self.layers.iter().any(|l| l.lighting.is_some()) {
            return 6;
        }
```

- `validate()` 在控件循环之前加：

```rust
        if let Some(l) = &self.lighting {
            l.validate()?;
        }
        for layer in &self.layers {
            if let Some(l) = &layer.lighting {
                l.validate()?;
            }
        }
```

  `LayerLighting.lights` 是定长数组，三盏灯的 JSON 在 serde 反序列化时就会失败（`Error::Json`），测试里 `is_err()` 即可。
- 新方法：

```rust
    /// 活动层集合（从底到顶）→ 有效背光：编号最高且带记录的活动层，否则 Layer 0 的记录。
    pub fn effective_lighting(&self, active: &[u8]) -> Option<&LayerLighting> {
        active
            .iter()
            .rev()
            .find_map(|id| self.layers.iter().find(|l| l.id == *id).and_then(|l| l.lighting.as_ref()))
            .or(self.lighting.as_ref())
    }
```

- [ ] **Step 5: 运行测试**

Run: `cargo test -p halokey-host 2>&1 | grep -E "test result|FAILED|panicked"`
Expected: 全部 `ok`（含新文件 6 个用例）。再跑 `cargo clippy --workspace --all-targets -- -D warnings` 与 `cargo test --workspace`，确认其它 crate 里 `HostLayer {` 的构造都补齐了。

- [ ] **Step 6: 提交**

```bash
git add crates/host && git commit -m "feat(host): per-layer backlight model with validation, version 6 and effective-lighting lookup

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: 示例与文档

**Files:**
- Create: `docs/examples/layer-backlight.json`
- Modify: `docs/03-config-format.md`, `docs/03-config-format.zh.md`
- Test: `crates/host/tests/lighting.rs`（加一条读示例的用例）

**Interfaces:** 无新接口；示例会被 Task 6 的前端往返测试引用。

- [ ] **Step 1: 写失败的测试（追加到 `crates/host/tests/lighting.rs`）**

```rust
#[test]
fn example_layer_backlight_parses() {
    let bytes = include_bytes!("../../../docs/examples/layer-backlight.json");
    let map = HostKeymap::from_json(bytes).unwrap();
    assert_eq!(map.version, 6);
    assert_eq!(map.effective_lighting(&[1]).unwrap().lights[0].mode, LightMode::Breathe);
    assert_eq!(map.effective_lighting(&[]).unwrap().lights[0].mode, LightMode::Solid);
}
```

Run: `cargo test -p halokey-host --test lighting example 2>&1 | grep -E "error|panicked|test result" | head -3` → 编译错误（文件不存在）。

- [ ] **Step 2: 写示例 `docs/examples/layer-backlight.json`**

```json
{
  "version": 6,
  "controls": [
    { "index": 0, "press": [{ "type": 2, "code": 0 }], "pressAction": { "momentaryLayer": { "_0": 1 } } },
    { "index": 1, "press": [{ "type": 2, "code": 40 }] },
    { "index": 2, "press": [{ "type": 2, "code": 41 }] },
    { "index": 3, "press": [{ "type": 2, "code": 70 }] },
    { "index": 4, "press": [{ "type": 7, "code": 6 }] },
    { "index": 5, "press": [{ "type": 7, "code": 5 }] }
  ],
  "lighting": {
    "lights": [
      { "mode": "solid", "brightness": 10 },
      { "mode": "solid", "brightness": 10 },
      { "mode": "solid", "brightness": 10 },
      { "mode": "off", "brightness": 0 }
    ]
  },
  "layers": [
    {
      "id": 1,
      "controls": [
        { "index": 1, "press": [{ "type": 2, "code": 43 }], "inheritedGestures": ["doublePress", "longPress"] }
      ],
      "lighting": {
        "lights": [
          { "mode": "breathe", "brightness": 20 },
          { "mode": "breathe", "brightness": 20 },
          { "mode": "breathe", "brightness": 20 },
          { "mode": "solid", "brightness": 20 }
        ]
      }
    }
  ],
  "doublePressWindow": 0.25,
  "longPressThreshold": 0.5
}
```

含义：按住键 1 进 Layer 1，键 2 变 Tab，三键呼吸、旋钮常亮；松开回 Layer 0 三键常亮 10、旋钮灭。

- [ ] **Step 3: 文档**

`docs/03-config-format.md` §1 表格加一行：

```markdown
| `lighting` | Optional per-layer backlight: `{ "lights": [ { "mode": "off|solid|breathe", "brightness": 0–20 } ×4 ] }` in the order key 1, key 2, key 3, knob. Also allowed inside each `layers[]` entry. The runtime writes the highest active layer's record (falling back to the top level) to the device's four indicator lights, keeping `work_time` / `breathe_level` and the global mode / brightness untouched. Omitted everywhere = never touch the lights. Requires version 6. |
```

§5 Examples 加一行指向 `docs/examples/layer-backlight.json`（"hold key 1 to breathe all keys"）。中文版 `docs/03-config-format.zh.md` 同位置同结构：

```markdown
| `lighting` | 可选的分层背光：`{ "lights": [ { "mode": "off|solid|breathe", "brightness": 0–20 } ×4 ] }`，顺序为键 1、键 2、键 3、旋钮。`layers[]` 里的每一层也可以带。运行时把编号最高的活动层的记录（没有则用顶层的）写进设备四盏指示灯，`work_time` / `breathe_level` 与全局模式 / 亮度保持不动。全部缺省 = 从不碰灯。需要版本 6。 |
```

- [ ] **Step 4: 验证**

Run: `cargo test -p halokey-host 2>&1 | grep "test result" && python3 tools/check_docs.py | tail -1`
Expected: 全部 ok；`✅ 全部通过`。

- [ ] **Step 5: 提交**

```bash
git add docs crates/host/tests && git commit -m "docs: document the lighting field and add a layer-backlight example

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: 运行时调度器（防抖 + 去重，纯逻辑）

**Files:**
- Create: `crates/runtime/src/lighting.rs`
- Modify: `crates/runtime/src/lib.rs`（`mod lighting;`）
- Test: 在 `crates/runtime/src/lighting.rs` 内 `#[cfg(test)] mod tests`

**Interfaces:**
- Produces: `pub(crate) struct LightingScheduler`；
  - `LightingScheduler::new() -> Self`
  - `fn note(&mut self, desired: Option<LayerLighting>, now: Instant)`：记录"当前应该是什么"，重置防抖计时
  - `fn due(&mut self, now: Instant) -> Option<LayerLighting>`：防抖到期且与上次成功写入不同时返回要写的值（返回即视为"正在写"，调用方随后调 `written` 或 `failed`）
  - `fn written(&mut self, value: LayerLighting)`、`fn failed(&mut self)`
  - `pub(crate) const DEBOUNCE: Duration = Duration::from_millis(100)`

- [ ] **Step 1: 写失败的测试**

```rust
//! 分层背光写入调度：100 ms 防抖，只在有效背光变化时写设备。
use std::time::{Duration, Instant};

use halokey_host::LayerLighting;

pub(crate) const DEBOUNCE: Duration = Duration::from_millis(100);

#[cfg(test)]
mod tests {
    use super::*;
    use halokey_host::{LightEffect, LightMode};

    fn l(b: u8) -> LayerLighting {
        LayerLighting { lights: [LightEffect { mode: LightMode::Solid, brightness: b }; 4] }
    }

    #[test]
    fn nothing_is_written_without_a_record() {
        let t0 = Instant::now();
        let mut s = LightingScheduler::new();
        s.note(None, t0);
        assert_eq!(s.due(t0 + Duration::from_secs(1)), None);
    }

    #[test]
    fn writes_once_after_debounce_and_not_again_for_the_same_value() {
        let t0 = Instant::now();
        let mut s = LightingScheduler::new();
        s.note(Some(l(5)), t0);
        assert_eq!(s.due(t0 + Duration::from_millis(50)), None, "防抖未到");
        assert_eq!(s.due(t0 + DEBOUNCE), Some(l(5)));
        s.written(l(5));
        s.note(Some(l(5)), t0 + Duration::from_millis(300));
        assert_eq!(s.due(t0 + Duration::from_millis(500)), None, "值没变不重写");
    }

    #[test]
    fn press_and_quick_release_collapse_into_one_write() {
        let t0 = Instant::now();
        let mut s = LightingScheduler::new();
        s.note(Some(l(1)), t0);
        s.written(l(1));
        s.note(Some(l(2)), t0 + Duration::from_millis(10)); // 按下 MO
        s.note(Some(l(1)), t0 + Duration::from_millis(40)); // 很快松开
        assert_eq!(s.due(t0 + Duration::from_millis(200)), None, "回到原值，一次都不用写");
    }

    #[test]
    fn release_writes_layer0_back() {
        let t0 = Instant::now();
        let mut s = LightingScheduler::new();
        s.note(Some(l(1)), t0);
        assert_eq!(s.due(t0 + DEBOUNCE), Some(l(1)));
        s.written(l(1));
        s.note(Some(l(2)), t0 + Duration::from_secs(1));
        assert_eq!(s.due(t0 + Duration::from_secs(1) + DEBOUNCE), Some(l(2)));
        s.written(l(2));
        s.note(Some(l(1)), t0 + Duration::from_secs(2));
        assert_eq!(s.due(t0 + Duration::from_secs(2) + DEBOUNCE), Some(l(1)));
    }

    #[test]
    fn failure_forgets_last_written_so_the_next_change_retries() {
        let t0 = Instant::now();
        let mut s = LightingScheduler::new();
        s.note(Some(l(3)), t0);
        assert_eq!(s.due(t0 + DEBOUNCE), Some(l(3)));
        s.failed();
        assert_eq!(s.due(t0 + Duration::from_secs(1)), None, "失败后不自动重试风暴");
        s.note(Some(l(3)), t0 + Duration::from_secs(2));
        assert_eq!(s.due(t0 + Duration::from_secs(2) + DEBOUNCE), Some(l(3)), "下一次变化再试");
    }
}
```

Run: `cargo test -p halokey-runtime lighting 2>&1 | grep -E "^error|cannot find|test result" | head -3`（先在 `lib.rs` 加 `mod lighting;`）
Expected: 编译错误，`LightingScheduler` 不存在。

- [ ] **Step 2: 实现（放在同文件测试模块之上）**

```rust
/// 把"活动层变化"变成"该写什么、什么时候写"。
pub(crate) struct LightingScheduler {
    /// 当前应有的有效背光；`None` = 不管灯。
    desired: Option<LayerLighting>,
    /// 最近一次 `note` 的时刻；防抖从这里起算。
    changed_at: Option<Instant>,
    /// 最近一次成功写入的值；失败后清空。
    last_written: Option<LayerLighting>,
}

impl LightingScheduler {
    pub(crate) fn new() -> Self {
        LightingScheduler { desired: None, changed_at: None, last_written: None }
    }

    pub(crate) fn note(&mut self, desired: Option<LayerLighting>, now: Instant) {
        if desired != self.desired {
            self.desired = desired;
            self.changed_at = Some(now);
        }
    }

    pub(crate) fn due(&mut self, now: Instant) -> Option<LayerLighting> {
        let changed_at = self.changed_at?;
        if now.duration_since(changed_at) < DEBOUNCE {
            return None;
        }
        self.changed_at = None;
        let desired = self.desired?;
        if Some(desired) == self.last_written {
            return None;
        }
        Some(desired)
    }

    pub(crate) fn written(&mut self, value: LayerLighting) {
        self.last_written = Some(value);
    }

    pub(crate) fn failed(&mut self) {
        self.last_written = None;
    }
}
```

注意 `press_and_quick_release` 用例：两次 `note` 后 `desired` 回到 `l(1)` 且 `changed_at` 被重置，`due` 时与 `last_written` 相同 → `None`。`LayerLighting` 需要 `Copy`（Task 1 已派生）。

- [ ] **Step 3: 运行测试** → `cargo test -p halokey-runtime lighting` 5 个用例 ok；clippy 绿。
- [ ] **Step 4: 提交** `git add crates/runtime && git commit -m "feat(runtime): debounced, deduplicated backlight scheduler"`（带 Co-Authored-By）。

---

### Task 4: 运行时接线 — 活动层 → 写设备、快照与事件

**Files:**
- Modify: `crates/runtime/src/lib.rs`, `apps/halokey/src/main.rs`
- Test: `apps/halokey` 无新测试；行为由 Task 3 单测 + 真机清单覆盖（见 Ruling 说明）。

**Interfaces:**
- Produces: `Snapshot.effective_lighting: Option<LayerLighting>`；`Event::Layers(Vec<u8>)`（活动层变化时发出，附当前集合）；`UiSnapshot.effectiveLighting: UiLayerLighting | null`（`{ lights: [{ mode: "off"|"solid"|"breathe", brightness }×4] }`，直接 serde `LayerLighting`）；`UiEvent::Layers { active: Vec<u8> }`（前端 `kind: "layers"`）。

- [ ] **Step 1: `Snapshot` 与 `Event`**

`Snapshot` 加 `pub effective_lighting: Option<LayerLighting>,`；`Event` 加变体 `/// 活动层集合变化（从底到顶）。\n    Layers(Vec<u8>),`。`Worker` 加字段 `lighting: LightingScheduler,`（`new()` 里 `lighting: LightingScheduler::new()`）。

- [ ] **Step 2: 在活动层变化处喂调度器**

把 `apply()` 里现有的

```rust
        let layers = self.router.active_layers();
        self.update(|s| s.active_layers = layers);
```

改成一个方法并在 `apply()` 与 `set_keymap()`（配置替换后）两处调用：

```rust
    /// 活动层集合刷新：快照、事件、背光调度。
    fn refresh_layers(&mut self) {
        let layers = self.router.active_layers();
        let changed = self.snapshot().active_layers != layers;
        let desired = self
            .snapshot()
            .keymap
            .as_ref()
            .and_then(|m| m.effective_lighting(&layers).copied());
        self.lighting.note(desired, Instant::now());
        self.update(|s| {
            s.active_layers = layers.clone();
            s.effective_lighting = desired;
        });
        if changed {
            (self.on_event)(Event::Layers(layers));
        }
    }
```

`set_keymap()` 里在 `self.update(|s| s.keymap = Some(map))` 之后调用 `self.refresh_layers()`（这样新配置的 Layer 0 背光会在 100 ms 后写入）。

- [ ] **Step 3: 在连接循环里写设备**

连接循环中 `self.pump_runner(now);` 之后加：

```rust
            if !self.paused {
                if let Some(desired) = self.lighting.due(Instant::now()) {
                    match write_layer_lighting(transport, &desired) {
                        Ok(()) => self.lighting.written(desired),
                        Err(e) => {
                            self.lighting.failed();
                            self.fail(format!("背光写入失败：{e}"));
                        }
                    }
                    self.drain_stray();
                }
            }
```

辅助函数（放在 `set_lighting` 旁边；读一次现值、只写有差异的灯、不做读回核对以省两次往返）：

```rust
/// 把一层的背光套到设备现值上写入：只改四盏灯的 kind 与亮度，每条写等确认。
fn write_layer_lighting(transport: &mut Transport, desired: &LayerLighting) -> Result<(), String> {
    let current = read_lighting(transport)?;
    let target = desired.device_lighting(&current);
    let writes = settings::lighting_writes(&target, &current).map_err(|e| e.to_string())?;
    for request in &writes {
        transport
            .transact(request, REPLY_TIMEOUT, |f| settings::matches_lighting_write_reply(f, request))
            .map_err(|e| format!("指示灯写入未确认：{e}"))?;
    }
    Ok(())
}
```

暂停（已交还控制）期间不写；恢复后下一次层变化再写。断开重连后 `last_written` 应清空：在进入连接循环的开头（`Connected` 事件发出处附近）加 `self.lighting.failed();`，并 `self.refresh_layers();` 让 Layer 0 背光在重连后重新写入一次。

- [ ] **Step 4: 应用层转发**

`apps/halokey/src/main.rs`：`UiSnapshot` 加 `effective_lighting: Option<halokey_host::LayerLighting>`（serde 派生已带 camelCase → `effectiveLighting`），`from()` 里搬运；`UiEvent` 加 `Layers { active: Vec<u8> }`，`forward()` 里 `Event::Layers(active) => UiEvent::Layers { active }`。

- [ ] **Step 5: 验证**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace 2>&1 | grep -E "FAILED|panicked"; cargo build --release -p halokey-app`
Expected: clippy 无警告、测试无失败、构建成功。

真机快测（本机，设备在线）：把 `docs/examples/layer-backlight.json` 复制到 `~/.config/halokey/keymap.json`，重启正式版：三键应变常亮 10、旋钮灭；按住键 1 → 三键呼吸、旋钮常亮；松开 → 回到常亮。`HALOKEY_DEBUG=1` 下 stderr 不应出现"背光写入失败"。记录结果到提交信息。

- [ ] **Step 6: 提交** `git add -A && git commit -m "feat(runtime,app): write the active layer's backlight to the device; expose effective lighting and a layers event"`（带 Co-Authored-By）。

---

### Task 5: 前端类型与桥

**Files:**
- Modify: `ui/src/model/keymap.ts`, `ui/src/lib/tauri.ts`
- Test: `ui/src/model/__tests__/keymap.test.ts`（往返加示例）, `ui/src/lib/__tests__/tauri.test.ts`

**Interfaces:**
- Produces: `type LightMode = "off" | "solid" | "breathe"`、`type LightEffect = { mode: LightMode; brightness: number }`、`type LayerLighting = { lights: LightEffect[] }`；`HostKeymap.lighting?: LayerLighting | null`、`HostLayer.lighting?: LayerLighting | null`；`UiSnapshot.effectiveLighting: LayerLighting | null`；`UiEvent` 加 `{ kind: "layers"; active: number[] }`；`LIGHT_NAMES = ["键 1", "键 2", "键 3", "旋钮"] as const`（在 `labels.ts`，经 `t()` 显示）。

- [ ] **Step 1: 写失败的测试**

`keymap.test.ts` 的往返用例把文件列表加上 `"layer-backlight.json"`，并新增：

```ts
  it("exposes lighting records", () => {
    const map = example("layer-backlight.json");
    expect(map.lighting?.lights).toHaveLength(4);
    expect(map.layers?.[0]?.lighting?.lights[0]?.mode).toBe("breathe");
  });
```

`tauri.test.ts` 加：

```ts
  it("mock snapshot carries effectiveLighting", async () => {
    const snap = await api.snapshot();
    expect("effectiveLighting" in snap).toBe(true);
  });
```

Run: `cd ui && npx vitest run src/model/__tests__/keymap.test.ts src/lib/__tests__/tauri.test.ts 2>&1 | grep -E "×|Tests "` → 2 个失败（类型层面 `npm run check` 也会报）。

- [ ] **Step 2: 实现**

`keymap.ts` 加类型并在 `HostLayer` / `HostKeymap` 上加 `lighting?: LayerLighting | null`。`labels.ts` 加 `export const LIGHT_NAMES = ["键 1", "键 2", "键 3", "旋钮"] as const;`。`tauri.ts`：`UiSnapshot` 加 `effectiveLighting: LayerLighting | null`（mock 里 `effectiveLighting: null`），`UiEvent` 加 `| { kind: "layers"; active: number[] }`。

- [ ] **Step 3: 运行** `npm test && npm run check` 全绿。
- [ ] **Step 4: 提交** `git add ui && git commit -m "feat(ui): lighting types in the keymap model and bridge"`（带 Co-Authored-By）。

---

### Task 6: 草稿 reducer — 背光编辑

**Files:**
- Modify: `ui/src/model/draft.ts`
- Test: `ui/src/model/__tests__/draft.test.ts`

**Interfaces:**
- Produces:
  - `effectiveLighting(map: HostKeymap, layer: number): LayerLighting | null`（该层记录，否则 Layer 0 记录，否则 `null`）
  - `hasOwnLighting(map, layer): boolean`
  - `setLightEffect(d: Draft, light: number, patch: Partial<LightEffect>): Draft`（写当前选中层；该层没记录时先以"当前有效背光"或全关创建记录）
  - `setFollowBase(d: Draft, follow: boolean): Draft`（非基础层：`true` 删除本层记录，`false` 以有效背光创建记录；基础层无操作）
  - `clearBaseLighting(d: Draft): Draft`（基础层：删除记录 = 不管灯；非基础层无操作）
  - `DEFAULT_LIGHTING: LayerLighting`（四盏全 `off` / 0）

- [ ] **Step 1: 写失败的测试（追加到 `draft.test.ts`）**

```ts
import { effectiveLighting, hasOwnLighting, setLightEffect, setFollowBase, clearBaseLighting } from "../draft";
// …（放到文件顶部的 import 里）

describe("draft lighting", () => {
  const loadLit = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/layer-backlight.json"), "utf8"));
  it("resolves effective lighting per layer", () => {
    const m = loadLit();
    expect(effectiveLighting(m, 0)!.lights[0]!.mode).toBe("solid");
    expect(effectiveLighting(m, 1)!.lights[0]!.mode).toBe("breathe");
    expect(effectiveLighting(m, 2)!.lights[0]!.mode).toBe("solid");
    expect(hasOwnLighting(m, 2)).toBe(false);
    expect(effectiveLighting(load(), 0)).toBeNull();
  });
  it("edits create a record for the selected layer and mark dirty", () => {
    let d = loadSaved(initialDraft(), loadLit());
    d = setLightEffect(select(d, { layer: 2, index: 0, gesture: "press" }), 3, { mode: "breathe", brightness: 7 });
    expect(d.dirty).toBe(true);
    const l2 = d.map!.layers!.find((l) => l.id === 2)!;
    expect(l2.lighting!.lights[3]).toEqual({ mode: "breathe", brightness: 7 });
    expect(l2.lighting!.lights[0]).toEqual({ mode: "solid", brightness: 10 }); // 其余灯复制自有效背光
    expect(l2.controls).toEqual([]);
  });
  it("follow toggles create and delete the record cleanly", () => {
    let d = loadSaved(initialDraft(), loadLit());
    d = select(d, { layer: 1, index: 0, gesture: "press" });
    d = setFollowBase(d, true);
    expect(d.map!.layers!.find((l) => l.id === 1)!.lighting).toBeUndefined();
    d = setFollowBase(d, false);
    expect(d.map!.layers!.find((l) => l.id === 1)!.lighting!.lights[0]).toEqual({ mode: "solid", brightness: 10 });
    const before = JSON.stringify(d.map);
    d = setFollowBase(select(d, { layer: 0, index: 0, gesture: "press" }), true);
    expect(JSON.stringify(d.map)).toBe(before);
  });
  it("base layer can drop its record and edits start from all-off when nothing exists", () => {
    let d = loadSaved(initialDraft(), loadLit());
    d = clearBaseLighting(select(d, { layer: 0, index: 0, gesture: "press" }));
    expect(d.map!.lighting).toBeUndefined();
    d = setLightEffect(d, 0, { brightness: 12 });
    expect(d.map!.lighting!.lights[0]).toEqual({ mode: "off", brightness: 12 });
    expect(d.map!.lighting!.lights[1]).toEqual({ mode: "off", brightness: 0 });
  });
  it("clamps brightness to 0–20", () => {
    let d = loadSaved(initialDraft(), loadLit());
    d = setLightEffect(d, 0, { brightness: 99 });
    expect(d.map!.lighting!.lights[0]!.brightness).toBe(20);
    d = setLightEffect(d, 0, { brightness: -4 });
    expect(d.map!.lighting!.lights[0]!.brightness).toBe(0);
  });
});
```

Run: `npx vitest run src/model/__tests__/draft.test.ts` → 失败（函数不存在）。

- [ ] **Step 2: 实现（追加到 `draft.ts`）**

```ts
import type { LayerLighting, LightEffect } from "./keymap";

export const DEFAULT_LIGHTING: LayerLighting = { lights: Array.from({ length: 4 }, () => ({ mode: "off" as const, brightness: 0 })) };

export function effectiveLighting(map: HostKeymap, layer: number): LayerLighting | null {
  const own = layer === 0 ? map.lighting : map.layers?.find((l) => l.id === layer)?.lighting;
  return own ?? map.lighting ?? null;
}
export const hasOwnLighting = (map: HostKeymap, layer: number): boolean =>
  !!(layer === 0 ? map.lighting : map.layers?.find((l) => l.id === layer)?.lighting);

/** 修改当前层某盏灯；层没有记录时先用有效背光（或全关）建一条。 */
export function setLightEffect(d: Draft, light: number, patch: Partial<LightEffect>): Draft {
  if (!d.map || light < 0 || light > 3) return d;
  const map = clone(d.map);
  const layer = d.selection.layer;
  const record: LayerLighting = clone(effectiveLighting(map, layer) ?? DEFAULT_LIGHTING);
  const cur = record.lights[light]!;
  record.lights[light] = { mode: patch.mode ?? cur.mode, brightness: Math.min(20, Math.max(0, Math.round(patch.brightness ?? cur.brightness))) };
  writeLighting(map, layer, record);
  return { ...d, map, dirty: true };
}

/** 非基础层：跟随 Layer 0 = 删除本层记录；不跟随 = 以有效背光建一条。 */
export function setFollowBase(d: Draft, follow: boolean): Draft {
  if (!d.map || d.selection.layer === 0) return d;
  const map = clone(d.map);
  const layer = d.selection.layer;
  if (follow) {
    const l = map.layers?.find((x) => x.id === layer);
    if (!l?.lighting) return d;
    delete l.lighting;
    if (!l.controls.length) map.layers = map.layers!.filter((x) => x.id !== layer);
    if (map.layers && !map.layers.length) delete map.layers;
  } else {
    if (hasOwnLighting(map, layer)) return d;
    writeLighting(map, layer, clone(effectiveLighting(map, layer) ?? DEFAULT_LIGHTING));
  }
  return { ...d, map, dirty: true };
}

/** 基础层：删除记录 = 不管灯。 */
export function clearBaseLighting(d: Draft): Draft {
  if (!d.map || d.selection.layer !== 0 || !d.map.lighting) return d;
  const map = clone(d.map);
  delete map.lighting;
  return { ...d, map, dirty: true };
}

function writeLighting(map: HostKeymap, layer: number, record: LayerLighting) {
  if (layer === 0) { map.lighting = record; return; }
  map.layers ??= [];
  let l = map.layers.find((x) => x.id === layer);
  if (!l) { l = { id: layer, controls: [] }; map.layers.push(l); map.layers.sort((a, b) => a.id - b.id); }
  l.lighting = record;
}
```

`HostLayer.controls` 在 Rust 里 `#[serde(default)]`，空数组可省，但 `writable()` 创建的层已经带 `controls: []`，保持一致。

- [ ] **Step 3: 运行** `npm test && npm run check` 全绿（原有用例不受影响）。
- [ ] **Step 4: 提交** `git add ui && git commit -m "feat(ui): lighting edits in the keymap draft reducer"`（带 Co-Authored-By）。

---

### Task 7: 背光条组件 + 渲染图灯圈预览

**Files:**
- Create: `ui/src/components/BacklightStrip.svelte`
- Modify: `ui/src/pages/Keymap.svelte`, `ui/src/lib/i18n.ts`, `ui/src/components/DeviceView.svelte`（灯圈亮度）
- Test: `ui/src/pages/__tests__/backlight.test.svelte.ts`

**Interfaces:**
- `BacklightStrip` props：`{ layer: number; lighting: LayerLighting | null; own: boolean; onEffect: (light: number, patch: Partial<LightEffect>) => void; onFollow: (follow: boolean) => void; onClearBase: () => void }`；内部 `expanded` 状态。
- `DeviceView` props 改为 `{ lights: LightEffect[] | null; selectedIndex; onPick }`：`null` 或 `mode === "off"` 不亮；`solid` 亮，透明度随亮度（0.3–1）；`breathe` 呼吸动画，幅度随亮度。

- [ ] **Step 1: 写失败的组件测试**

```ts
import { describe, it, expect, afterEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import Keymap from "../Keymap.svelte";
import { initialDraft, loadSaved, type Draft } from "../../model/draft";
import type { HostKeymap } from "../../model/keymap";
import { setLang } from "../../lib/i18n";

const load = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/layer-backlight.json"), "utf8"));
let app: ReturnType<typeof mount> | undefined;
afterEach(() => { if (app) unmount(app); app = undefined; document.body.innerHTML = ""; });
const buttons = () => [...document.querySelectorAll("button")] as HTMLButtonElement[];
const byText = (t: string) => buttons().find((b) => b.textContent?.trim() === t);

describe("backlight strip", () => {
  it("shows the effective lighting, expands an editor and writes the draft", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null } });
    flushSync();
    expect(document.body.textContent).toContain("本层背光");
    expect(document.querySelectorAll(".strip .dot[data-mode=solid]")).toHaveLength(3);
    expect(document.querySelectorAll(".device .spot[data-mode=solid]")).toHaveLength(3);
    byText("编辑…")!.click();
    flushSync();
    const rows = document.querySelectorAll(".editor .light");
    expect(rows).toHaveLength(4);
    (rows[3]!.querySelector("button[data-mode=breathe]") as HTMLButtonElement).click();
    flushSync();
    expect(draft.v.map!.lighting!.lights[3]!.mode).toBe("breathe");
    expect(draft.v.dirty).toBe(true);
    expect(document.querySelectorAll(".device .spot[data-mode=breathe]")).toHaveLength(1);
  });
  it("layer 1 follows layer 0 when its record is removed", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null } });
    flushSync();
    ([...document.querySelectorAll(".seg button")].find((b) => b.textContent?.trim() === "1") as HTMLButtonElement).click();
    flushSync();
    expect(document.querySelectorAll(".strip .dot[data-mode=breathe]")).toHaveLength(3);
    byText("编辑…")!.click();
    flushSync();
    const follow = document.querySelector(".editor input[data-role=follow]") as HTMLInputElement;
    expect(follow.checked).toBe(false);
    follow.click();
    flushSync();
    expect(draft.v.map!.layers!.find((l) => l.id === 1)!.lighting).toBeUndefined();
    expect(document.querySelectorAll(".strip .dot[data-mode=solid]")).toHaveLength(3);
  });
});
```

Run: `npx vitest run src/pages/__tests__/backlight.test.svelte.ts` → 失败（没有背光条）。

- [ ] **Step 2: `BacklightStrip.svelte`**

```svelte
<script lang="ts">
  import { t } from "../lib/i18n";
  import { LIGHT_NAMES } from "../model/labels";
  import type { LayerLighting, LightEffect, LightMode } from "../model/keymap";
  let { layer, lighting, own, onEffect, onFollow, onClearBase }: { layer: number; lighting: LayerLighting | null; own: boolean; onEffect: (light: number, patch: Partial<LightEffect>) => void; onFollow: (follow: boolean) => void; onClearBase: () => void } = $props();
  let expanded = $state(false);
  const MODES: LightMode[] = ["off", "solid", "breathe"];
  const modeName: Record<LightMode, string> = { off: "关", solid: "常亮", breathe: "呼吸" };
  const summary = $derived(!lighting ? (layer === 0 ? t("未设置（不管灯）") : t("跟随 Layer 0")) : lighting.lights.map((l, i) => `${t(LIGHT_NAMES[i]!)} ${t(modeName[l.mode])}`).join(" · "));
  const maxBrightness = $derived(lighting ? Math.max(...lighting.lights.map((l) => l.brightness)) : 0);
</script>

<div class="wrap">
  <div class="strip">
    <span class="title">{t("本层背光")}</span>
    {#if lighting}
      {#each lighting.lights as l, i}
        <span class="dot" data-mode={l.mode} title={`${t(LIGHT_NAMES[i]!)} · ${t(modeName[l.mode])} · ${l.brightness}`}></span>
      {/each}
    {/if}
    <span class="sum">{summary}</span>
    {#if lighting}<span class="muted">{t("亮度")} {maxBrightness}</span>{/if}
    {#if layer !== 0 && !own}<span class="muted">▽</span>{/if}
    <button onclick={() => (expanded = !expanded)}>{expanded ? t("收起") : t("编辑…")}</button>
  </div>
  {#if expanded}
    <div class="editor">
      {#each [0, 1, 2, 3] as i}
        {@const l = lighting?.lights[i] ?? { mode: "off", brightness: 0 }}
        <div class="light">
          <span class="name">{t(LIGHT_NAMES[i]!)}</span>
          <span class="seg">
            {#each MODES as m}<button data-mode={m} class:on={l.mode === m} onclick={() => onEffect(i, { mode: m })}>{t(modeName[m])}</button>{/each}
          </span>
          <input type="range" min="0" max="20" value={l.brightness} disabled={l.mode === "off"} oninput={(e) => onEffect(i, { brightness: Number((e.target as HTMLInputElement).value) })} />
          <span class="mono">{l.brightness}</span>
        </div>
      {/each}
      <div class="foot">
        {#if layer !== 0}
          <label><input type="checkbox" data-role="follow" checked={!own} onchange={(e) => onFollow((e.target as HTMLInputElement).checked)} /> {t("跟随 Layer 0")}</label>
        {:else if lighting}
          <button onclick={onClearBase}>{t("不管灯（删除本层背光）")}</button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .wrap { display: flex; flex-direction: column; align-items: flex-end; gap: 6px; }
  .strip { display: flex; align-items: center; gap: 8px; background: var(--card); border-radius: 8px; padding: 6px 10px; font-size: 13px; }
  .title { color: var(--muted); }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: #55555d; display: inline-block; }
  .dot[data-mode="solid"] { background: var(--accent); }
  .dot[data-mode="breathe"] { background: var(--accent); animation: breathe 1.8s ease-in-out infinite; }
  .sum { max-width: 360px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .muted { color: var(--muted); }
  .strip button { padding: 4px 10px; font-size: 13px; }
  .editor { background: var(--panel); border: 1px solid var(--line); border-radius: 10px; padding: 10px 14px; display: flex; flex-direction: column; gap: 8px; min-width: 460px; }
  .light { display: grid; grid-template-columns: 48px auto 1fr 28px; align-items: center; gap: 10px; }
  .name { color: var(--muted); font-size: 13px; }
  .seg { display: inline-flex; background: var(--card); border-radius: 8px; padding: 3px; gap: 2px; }
  .seg button { background: none; padding: 4px 10px; font-size: 13px; }
  .seg button.on { background: var(--accent); color: var(--accent-text); }
  .mono { font-family: var(--mono); font-size: 12.5px; text-align: right; }
  .foot { display: flex; justify-content: flex-end; font-size: 13px; color: var(--muted); }
  @keyframes breathe { 0%, 100% { opacity: .3; } 50% { opacity: 1; } }
</style>
```

`i18n.ts` 的 `EN` 补：`"未设置（不管灯）": "Not set (lights untouched)"`, `"亮度": "Brightness"`, `"收起": "Collapse"`, `"关": "Off"`, `"常亮": "Solid"`, `"呼吸": "Breathe"`, `"不管灯（删除本层背光）": "Leave lights alone (remove this layer's backlight)"`, `"键 1": "Key 1"`, `"键 2": "Key 2"`, `"键 3": "Key 3"`, `"旋钮": "Knob"`。（`"本层背光"`, `"编辑…"`, `"跟随 Layer 0"` 已在表里。）`modeName[...]` 与 `LIGHT_NAMES[i]` 不是字面量调用，所以要手动确认这些键都在 `EN` 里；Task 6 的 i18n 扫描测试只能兜住字面量。

- [ ] **Step 3: `DeviceView` 灯圈按效果与亮度**

props 改为 `lights: LightEffect[] | null`；`ring(i)` 返回 `lights?.[i]?.mode ?? "off"`；灯圈样式加亮度：

```svelte
<button class="spot" class:sel={selectedIndex === i} data-mode={ring(i)} style:--glow={String(0.3 + 0.7 * ((lights?.[i]?.brightness ?? 0) / 20))} … >
```

CSS：`.spot[data-mode="solid"] { box-shadow: 0 0 0 3px var(--accent), 0 0 14px 3px rgba(242,201,181,.55); opacity: var(--glow, 1); }`，`breathe` 的动画保持，`opacity` 由动画控制时用 `animation` 覆盖即可（呼吸幅度不随亮度，简单起见）。

- [ ] **Step 4: 接进 `Keymap.svelte`**

```svelte
  import BacklightStrip from "../components/BacklightStrip.svelte";
  import { effectiveLighting, hasOwnLighting, setLightEffect, setFollowBase, clearBaseLighting } from "../model/draft";
  const lighting = $derived(map ? effectiveLighting(map, sel.layer) : null);
  const lights = $derived(lighting?.lights ?? null);
```

删除旧的 `lights` 占位行；`<LayerBar …>` 改为：

```svelte
  <LayerBar layer={sel.layer} onSelect={(l) => { draft = select(draft, { ...sel, layer: l }); }}>
    {#snippet right()}
      <BacklightStrip layer={sel.layer} {lighting} own={hasOwnLighting(map, sel.layer)}
        onEffect={(i, p) => (draft = setLightEffect(draft, i, p))}
        onFollow={(f) => (draft = setFollowBase(draft, f))}
        onClearBase={() => (draft = clearBaseLighting(draft))} />
    {/snippet}
  </LayerBar>
```

`DeviceView` 的 `{lights}` 传法不变（类型已变）。

- [ ] **Step 5: 运行** `npm test && npm run check && npm run build`；在浏览器（`npm run dev`，mock）里看一眼：Layer 条右侧有背光条，点"编辑…"展开四行，切 Layer 1 时渲染图三键呼吸。
- [ ] **Step 6: 提交** `git add ui && git commit -m "feat(ui): per-layer backlight strip with inline editor and live light rings"`（带 Co-Authored-By）。

---

### Task 8: 运行时活动层高亮、mock 校验、真机清单

**Files:**
- Modify: `ui/src/components/LayerBar.svelte`（活动层小圆点）, `ui/src/App.svelte`（订阅 `layers` 事件）, `ui/src/pages/Keymap.svelte`（透传 `activeLayers`）, `ui/src/lib/tauri.ts`（mock 校验加亮度与灯数）
- Test: `ui/src/lib/__tests__/tauri.test.ts`, `ui/src/pages/__tests__/keymap.test.svelte.ts`

**Interfaces:**
- `LayerBar` props 加 `active?: number[]`：集合里的层按钮带一个小圆点（设备当前正按着的 MO 层）。
- `Keymap.svelte` props 加 `activeLayers: number[]`。

- [ ] **Step 1: 写失败的测试**

`tauri.test.ts` 追加：

```ts
  it("mock validation rejects bad lighting like Rust does", async () => {
    const map = await api.getKeymap();
    await expect(api.validateKeymap({ ...map, lighting: { lights: [{ mode: "solid", brightness: 21 }, { mode: "off", brightness: 0 }, { mode: "off", brightness: 0 }, { mode: "off", brightness: 0 }] } })).rejects.toMatch(/0–20/);
    await expect(api.validateKeymap({ ...map, lighting: { lights: [{ mode: "solid", brightness: 1 }] } })).rejects.toMatch(/四盏/);
  });
```

`keymap.test.svelte.ts` 追加一个用例：

```ts
  it("marks active layers on the layer bar", () => {
    setLang("zh");
    const draft = $state<{ v: Draft }>({ v: loadSaved(initialDraft(), load()) });
    app = mount(Keymap, { target: document.body, props: { get draft() { return draft.v; }, set draft(d: Draft) { draft.v = d; }, snapshot: null, activeLayers: [2] } });
    flushSync();
    const two = [...document.querySelectorAll(".seg button")].find((b) => b.textContent?.trim() === "2")!;
    expect(two.querySelector(".live")).toBeTruthy();
  });
```

Run 两个文件 → 失败。

- [ ] **Step 2: 实现**

`tauri.ts` 的 `mockValidate` 加：

```ts
  for (const rec of [map.lighting, ...(map.layers ?? []).map((l) => l.lighting)]) {
    if (!rec) continue;
    if (rec.lights.length !== 4) throw "背光记录必须有四盏灯。";
    if (rec.lights.some((l) => l.brightness < 0 || l.brightness > 20)) throw "背光亮度必须在 0–20 之间。";
  }
```

`LayerBar.svelte`：props 加 `active = [] as number[]`，按钮内 `{l}{#if active.includes(l)}<i class="live"></i>{/if}`，样式 `.live { display:inline-block; width:6px; height:6px; border-radius:50%; background: var(--ok); margin-left: 6px; vertical-align: middle; }`。

`Keymap.svelte`：props 加 `activeLayers = [] as number[]`，传给 `LayerBar`。`App.svelte`：`let activeLayers = $state<number[]>([])`；`refresh()` 里 `activeLayers = snapshot?.activeLayers ?? []`；`subscribe` 回调里 `if (e.kind === "layers") activeLayers = e.active;`；`<Keymap bind:draft {snapshot} {activeLayers} />`。

- [ ] **Step 3: 运行** `npm test && npm run check && npm run build`；`cargo build --release -p halokey-app`。

- [ ] **Step 4: 真机清单（本机，逐项记录到提交信息）**

1. 用现有 `~/.config/halokey/keymap.json`（无 `lighting`）启动：设备灯光与启动前完全一样；`HALOKEY_DEBUG=1` 的 stderr 没有任何"背光"字样。
2. 键位页 Layer 0：背光条显示"未设置（不管灯）"；点"编辑…"→ 把键 1 设为常亮 20 → 保存 → 设备键 1 常亮（其余灯不变）。
3. Layer 1：背光条显示"跟随 Layer 0 ▽"；编辑 → 取消跟随 → 三键呼吸 → 保存；给键 3 的单击分配 MO(1)（Layer 0）→ 保存；按住键 3：三键呼吸、Layer 条上 1 出现绿点、渲染图灯圈跟着变；松开：回到 Layer 0 的灯，绿点消失。
4. 快速连按键 3 五次：灯光最终停在 Layer 0 状态，stderr 无"背光写入失败"。
5. 本体睡着时按住键 3（先等它进待机）：第一下由设备直出，之后灯光正确切换，日志至多一条失败。
6. 拔掉接收器再插回：Layer 0 背光在重连后自动写回一次。
7. 英文界面：背光条、编辑器、灯名全部英文。

- [ ] **Step 5: 提交并推送**

```bash
git add -A && git commit -m "feat(ui): live active-layer marks; mock lighting validation; phase 2 complete

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>" && git push origin main
```

---

## 自检记录

- 规格覆盖：§4.1 背光条（Task 7）、§4.2 灯圈实时预览（Task 7）、§7 数据模型与运行时规则（Task 1、3、4）、§8 `effectiveLighting` 与 `layers` 事件（Task 4、5、8）、§10 Rust 测试三项（Task 1 解析、Task 1 往返、Task 3 假时钟防抖）与 TS reducer 测试（Task 6）、§11 示例升级（Task 2）、§12 第 2 期范围。设置页的"灯光模式 / 全亮亮度"写入属第 5 期，本期不碰。
- 类型一致性：`LayerLighting { lights: [LightEffect; 4] }` ↔ TS `{ lights: LightEffect[] }`；`LightMode` 的 serde `lowercase` ↔ TS `"off"|"solid"|"breathe"`；`Snapshot.effective_lighting` ↔ `UiSnapshot.effectiveLighting`；`Event::Layers(Vec<u8>)` ↔ `{ kind: "layers", active }`；Task 6 的函数名在 Task 7 / 8 中同名。
- Review Focus 五项：1 → Task 1 往返 + Task 3 `nothing_is_written_without_a_record`；2 → Task 3 `release_writes_layer0_back` / `press_and_quick_release_collapse_into_one_write`；3 → Task 3 `failure_forgets_last_written…` + Task 4 失败分支；4 → Task 6 `follow toggles…`；5 → Task 1 `brightness_above_20_is_rejected` + Task 8 mock 校验。
- 执行后回填（2026-10-18）：本计划两处假设被真机推翻——(1) §7 "保持 `work_time`、不碰全局模式"做不出可见背光：工作模式下灯效只显示 `work_time` 秒，模式 0 / 1 忽略单灯设置；实现改为亮灯写 `work_time=255`、有记录就写模式 2；(2) MO 按下 / 松开不产生手势过渡，`refresh_layers()` 必须在每个按键事件后调用，不能只挂在 `apply()` 里。另外 AU05 的键 1 灯不可见；亮度 2 / 20 肉眼有差。审查修正：失败重试加 10 s 退避；灯数错误走原始 JSON 预检给中文错误；设置页直接写灯后运行时作废缓存。
- 已知无自动化测试的部分：Task 4 的运行时接线（需要假设备后端），由真机清单 1–6 覆盖；执行时按 Ruling 记录。
