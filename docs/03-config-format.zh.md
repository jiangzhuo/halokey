# 03 · 配置格式
> 🌐 [English](03-config-format.md)

> 📚 文档集：[README](../README.zh.md) · [01 架构](01-architecture.zh.md) · [02 协议笔记](02-protocol-notes.zh.md) · **03 配置格式**

halokey 只保存一个 JSON 文件，Linux 上在 `~/.config/halokey/keymap.json`。设置窗口编辑它；`halokey run --config` 和 `halokey export-keymap` 用同一种结构。格式是 olanzi 的 `HostKeymap` 加两个 halokey 扩展，所以 olanzi 导出的文件可以原样导入。

---

## 1. 顶层

```json
{
  "version": 6,
  "controls": [ … 六项，index 0–5 … ],
  "layers": [ { "id": 1, "controls": [ … ] } ],
  "actionLibrary": [ { "id": "UUID", "name": "…", "slot": 0, "action": { … } } ],
  "halos": [ { "id": "main", "name": "主菜单", "options": [ … ] } ],
  "doublePressWindow": 0.25,
  "longPressThreshold": 0.5
}
```

| 字段 | 含义 |
|---|---|
| `version` | 1–6，保存时按内容决定。1–5 是 olanzi 的；6 表示用到了 halokey 扩展（`halos`、`halo` 或 `pointerMove` 动作、鼠标页条目）。olanzi 会故意拒绝版本 6。 |
| `controls` | 恰好六项，每个控件一项：0 键 1、1 键 2、2 键 3、3 旋钮按下、4 右拧、5 左拧。 |
| `layers` | 可选的稀疏覆盖层 1–3，由 `momentaryLayer` 动作激活。 |
| `actionLibrary` | 可选的可复用应用或宏动作，由 `library` 动作引用。 |
| `halos` | 可选的环形菜单，见 §4。 |
| `lighting` | 可选的分层背光：`{ "lights": [ { "mode": "off|solid|breathe", "brightness": 0–20 } ×4 ] }`，顺序为键 1、键 2、键 3、旋钮。`layers[]` 里的每一层也可以带。运行时把编号最高的活动层的记录（没有则用顶层的）写进设备四盏指示灯，亮着的灯的 `work_time` 写成 255（工作模式下灯效显示 `work_time` 秒，255 即常显），`breathe_level` 与全局亮度保持不动。记录里有亮的灯时运行时会把全局灯光模式切到"工作"（"全灭"和"全亮"都会忽略单灯设置）。AU05 的键 1 没有可见的灯，所以只有键 2、键 3 和旋钮会显示。固件 4.4.2 上亮度 2 与 20 肉眼差别明显（2026-10-18 实测）。全部缺省 = 从不碰灯。需要版本 6。 |
| `doublePressWindow` | 0.15–0.5 秒。 |
| `longPressThreshold` | 0.3–2 秒，必须大于双击窗口。 |

## 2. 一个控件

```json
{
  "index": 3,
  "press": [ { "type": 2, "code": 42 } ],
  "pressAction": { "halo": { "_0": "main" } },
  "doublePress": [ { "type": 2, "code": 40 } ],
  "longPress": null,
  "pressBehavior": "hold",
  "pressTapCount": 2,
  "doublePressBehavior": "tap",
  "doublePressTapCount": 2,
  "longPressBehavior": "hold",
  "longPressTapCount": 2,
  "inheritedGestures": []
}
```

- `press` 是单击的键盘条目列表；有 `pressAction` 时以它为准。`doublePress` / `longPress` 也是同样一对；缺失这一对表示关闭该手势。
- 旋钮转动（index 4、5）只有单击动作，输出方式只能是 `hold`。
- 输出方式：`hold` 跟随物理按住，`tap` 发一次脉冲，`burst` 发 `…TapCount` 次脉冲（2–20）。双击不能是 `hold`。
- `inheritedGestures` 只在 `layers` 里有效；列出的手势透传到更低的层。

## 3. 条目与动作

键盘条目是 `{ "type": 2, "code": <HID usage> }`；修饰键是 `0xE0`–`0xE7`；键码 `0` 表示不执行动作。鼠标页条目是 `{ "type": 7, "code": n }`，`0`–`4` 为鼠标按键 1–5，`5` / `6` 为滚轮上 / 下。

动作沿用 olanzi 的标签形式 `{ "<kind>": { "_0": <value> } }`：

| 类型 | 值 | 说明 |
|---|---|---|
| `keyboard` | 条目列表 | 与普通条目列表相同。 |
| `application` | `{ "bundleIdentifier", "path", "name" }` | Linux X11：`bundleIdentifier` 匹配 `WM_CLASS`，也作为 `gtk-launch` 的桌面文件 id；`path` 是可执行文件。macOS：bundle id / `.app` 路径 / 名称。Windows：可执行文件名 / 路径。 |
| `macro` | 步骤列表 | 步骤为 `keyboard`、`application` 或 `delay`（`{ "delay": { "_0": 0.3 } }`，0.05–10 秒），1–32 步。 |
| `library` | UUID 字符串 | 引用 `actionLibrary`。 |
| `momentaryLayer` | 1–3 | 只限按键与旋钮按下的单击或长按。 |
| `pointerMove` | `{ "dx", "dy" }` | halokey 扩展。每次触发的像素位移，各不超过 ±1000，不能同时为零。 |
| `halo` | 光环 id | halokey 扩展。打开该菜单。 |

## 4. 光环菜单

```json
{
  "id": "main",
  "name": "主菜单",
  "options": [
    { "label": "Chrome", "action": { "application": { "_0": { "bundleIdentifier": "google-chrome", "path": "", "name": "Chrome" } } } },
    { "label": "更多…", "action": { "halo": { "_0": "more" } } }
  ]
}
```

一到八个选项；文字与 id 为 1–80 字；选项动作可以是除 `library` 和 `momentaryLayer` 之外的任何动作。光环打开时，旋钮旋转选择、旋钮按下确认、任意按键取消、空闲 4 秒自动关闭。

## 5. 示例

- [`examples/knob-moves-pointer.json`](examples/knob-moves-pointer.json)：旋钮移动指针。
- [`examples/switch-apps-and-macro.json`](examples/switch-apps-and-macro.json)：按键切换应用，旋钮按下跑宏。
- [`examples/halo-menu.json`](examples/halo-menu.json)：旋钮按下上的两级光环。
- [`examples/layer-backlight.json`](examples/layer-backlight.json)：按住键 1 让三个键呼吸（分层背光）。
