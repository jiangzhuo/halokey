# halokey

> 🌐 [English](README.md)

Ulanzi Vibe Key（AU05）的跨平台开源控制中心：重映射三个按键和旋钮，执行主机侧动作，用屏幕上的环形菜单做选择，管理灯光、麦克风和电源设置。一套代码覆盖 Linux、macOS 和 Windows。

**状态：pre-alpha，设计阶段。** 协议已验证，应用代码尚未开始。

## 为什么再做一个

Ulanzi Studio 只支持 Windows 和 macOS。目前所有社区项目要么只做 macOS，要么只做 Linux，而且没有一个实现了 Studio 的环形菜单。halokey 用 Rust 和 Tauri 2 重新开始，设备逻辑只写一遍，平台之间只差薄薄的适配层。

## 计划功能

| 功能 | 状态 |
|---|---|
| 厂商通道（TEA 加密 HID，report ID `0x55`） | 协议已在 Linux `hidraw` 和 macOS IOKit 上验证 |
| 按键与旋钮重映射、分层、单击 / 双击 / 长按 | 设计沿用 olanzi |
| 主机侧动作：键盘、鼠标按键、滚轮、指针移动、切换应用、宏、shell | 计划中 |
| 环形"光环"菜单：由旋钮驱动的八选项悬浮层 | 计划中 |
| 灯光、降噪、待机时间、电量读取 | 协议已知，计划中 |
| 托盘 / 菜单栏图标、登录启动、自动更新 | 计划中 |

## 平台矩阵

| 组件 | Linux | macOS | Windows |
|---|---|---|---|
| HID 传输 | hidapi 走 `hidraw` | hidapi 走 IOKit | hidapi 走 Win32 HID |
| 输入注入 | uinput | CGEvent | SendInput |
| 切换应用 | X11 EWMH，Wayland 按合成器适配 | NSWorkspace | SetForegroundWindow |
| 悬浮窗 | X11 可用，Wayland 需要 layer-shell | 不抢焦点的面板 | 不激活的窗口 |

## 仓库布局（计划）

```text
crates/proto     TEA、帧编解码、命令表、键码表 – 纯逻辑，重点单元测试
crates/hid       hidapi 传输、心跳、热插拔
crates/host      键位模型、手势路由、宏、动作执行
crates/input     uinput / CGEvent / SendInput 后端
crates/wm        切换应用后端
apps/halokey     Tauri 2 外壳：托盘、设置窗、光环悬浮层
docs/            设计与协议笔记，英文为默认，`.zh.md` 为中文版
tools/           仓库检查脚本
```

## 与其他项目的关系

- [olanzi](https://github.com/MrCroxx/olanzi) 保存着逆向记录和一个原生 macOS 应用；halokey 复用它的协议文档和键位格式。
- [vibed](https://github.com/ihavespoons/vibed) 记录了从厂商 SDK 恢复的鼠标页条目类型，halokey 已在真机上确认。

halokey 是独立的社区项目，与 Ulanzi 或 Kehwin 无关联、未获其背书，不包含也不加载任何厂商二进制。

## 文档

| # | 文档 | 内容 |
|---|---|---|
| 01 | [架构](docs/01-architecture.zh.md) | crate 划分、平台适配、悬浮层、已知风险 |

每份文档都有英文版（默认文件名）和中文版（`.zh.md`）；`python3 tools/check_docs.py` 负责检查两版是否同步。

## 许可证

[MIT](LICENSE)
