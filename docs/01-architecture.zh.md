# 01 · 架构
> 🌐 [English](01-architecture.md)

> 📚 文档集：[README](../README.zh.md) · **01 架构** · [02 协议笔记](02-protocol-notes.zh.md) · [03 配置格式](03-config-format.zh.md)

halokey 的设计笔记。除非标注 **[CONFIRMED]**（表示在真机上实测过），这里的一切都是计划。

---

## 1. 目标

1. 一套代码覆盖 Linux、macOS 和 Windows；平台相关代码只存在于适配 crate 里。
2. 不含厂商二进制：TEA 协议按公开的逆向记录自行实现。
3. 主机侧执行：halokey 运行时设备只上报原始控件事件，由主机决定发生什么，分层、手势、光环菜单和指针移动都靠这一点。
4. halokey 不运行时设备照样是一个普通键盘，使用最后一次写入的六格表。

## 2. Crate 划分

```text
crates/proto     TEA（32 轮、ECB、小端半字）、64 字节帧、命令表、
                 按键配置编解码、条目类型 0x02 / 0x03 / 0x07。无 I/O，无平台代码。
crates/hid       hidapi 传输。打开接口 3（usage page 0xFFFC），写 report ID 0x55
                 + 63 字节，读 64 字节回复，跑心跳，轮询热插拔。
crates/host      HostKeymap 模型（与 olanzi 的 JSON 兼容）、手势路由
                 （单击 / 双击 / 长按、旋钮格数）、四层、QMK 风格宏、
                 动作执行。纯逻辑、确定性，用假时钟做单元测试。
crates/input     trait Injector { key_down, key_up, mouse_button, wheel, pointer_move }
                 Linux：uinput · macOS：CGEvent · Windows：SendInput
crates/wm        trait Launcher { launch(app) -> Activation, is_frontmost(app), poll() }
                 Linux：先做 X11 EWMH，Wayland 之后按合成器适配 · macOS：NSWorkspace · Windows：Win32
crates/runtime   设备服务：一个线程拥有传输、心跳、路由、注入器、执行器；
                 命令进、快照与事件出。命令行与桌面应用共用。
apps/halokey     Tauri 2：托盘、设置窗（webview）、光环悬浮窗、CLI 子命令
ui/              两个窗口共用的 Svelte 5 + Vite 前端（`index.html`、`halo.html` → `ui/dist`）
```

依赖方向严格向下：`apps` → `runtime` → `host` → `proto`；`hid`、`input`、`wm` 是叶子，由 `runtime` 组装。

## 3. 数据流

```text
device ──vendor event──▶ hid ──▶ host (gesture router, layers) ──▶ action
                                                                   ├─▶ input (keys, mouse)
                                                                   ├─▶ wm (switch app)
                                                                   ├─▶ halo overlay (show / rotate / pick)
                                                                   └─▶ shell command
```

心跳让设备保持在厂商事件模式。halokey 停止、挂起或机器睡眠时交还控制权，固件重新按存储的表直出。

## 4. 光环悬浮层

- 第二个 Tauri 窗口：透明、无边框、置顶、不进任务栏、**不可聚焦**。它绝不能抢焦点，因为选中的动作要注入到旋钮刚开始转动时处于活动状态的那个应用。
- 旋钮转动切换选项，旋钮按下确认，超时或按键取消。
- 每个光环八个选项；每个选项可以是任意主机动作，所以一个光环也能打开另一个光环。
- 平台说明：Linux X11 直接可用，Wayland 需要 layer-shell 协议而 GNOME 不支持；macOS 需要不激活的面板；Windows 需要不激活的窗口样式。

## 5. 沿用的协议事实

| 事实 | 来源 |
|---|---|
| **[CONFIRMED]** TEA 密钥、delta `0x9E3779B9`、32 轮，`TEA_Enc(00×8) == 38 90 c4 99 a3 60 aa ad` | olanzi 文档 02 |
| **[CONFIRMED]** 明文 64 字节，report ID 之后只发前 63 字节；回复只解密前 56 字节 | olanzi 文档 02 |
| **[CONFIRMED]** 按键表读 `01 06 50 01 <index>`，写 `01 06 50 04 <index> 01 <num> <type> <code>`，确认 `81 06 50 14` | olanzi 文档 02 §6 |
| **[CONFIRMED]** 条目类型 `0x07` 是鼠标页：键码 N = 按键 N+1，`0x05` / `0x06` = 滚轮上 / 下，没有指针移动 | olanzi 文档 05 §10，2026-10-16 Linux 实测 |
| **[CONFIRMED]** Linux 把接口 3 暴露为 `/dev/hidraw`；不发心跳也能读 | olanzi 文档 05 §10 |

## 6. 已知风险

| 风险 | 对策 |
|---|---|
| Windows HID 要求输出报告长度与描述符完全一致 | 含 report ID 补满 64 字节，发布前真机验证 |
| macOS：`IOHIDManagerOpen` 全有全无 | hidapi 按设备逐个 `IOHIDDeviceOpen`；要验证，绝不退回 manager 调用 |
| Wayland 没有置顶窗 | 有 layer-shell 的用 layer-shell，其他环境记录降级行为 |
| NVIDIA 上 WebKitGTK 的 DMA-BUF 渲染器会把窗口画成空白 | 应用启动时若未设置则自动设 `WEBKIT_DISABLE_DMABUF_RENDERER=1` |
| `/dev/hidraw` 和 `/dev/uinput` 默认只有 root 可用 | 附带 udev 规则和用户组说明 |
| 主机崩溃后设备停留在厂商事件模式不出声 | 心跳看门狗，并记录社区报告的固件超时恢复行为 |

## 7. 里程碑

1. `proto` 带基于证据日志的单元测试；`hid` 在三个平台上读出六格表。**Linux 已于 2026-10-16 完成**；macOS 与 Windows 仍待真机运行。
2. 无界面守护进程：心跳、厂商事件、键盘和鼠标注入、导入 olanzi 键位。**Linux 的 `halokey run` 已于 2026-10-16 真机验证**（键 1 打出 `f`，键 2 发出 Enter，旋钮滚动页面，换上 `docs/examples/knob-moves-pointer.json` 后旋钮移动了指针））。切换应用与宏已于 2026-10-16 在 GNOME/X11 上用 `docs/examples/switch-apps-and-macro.json` 验证：按键拉起 Text Editor 与 Chrome，旋钮按下切回编辑器并打出 `hi`。**里程碑 2 在 Linux 上完成。**
3. 托盘和设置窗。**Linux 已于 2026-10-16 完成**（`apps/halokey`，Tauri 2.12，原生 HTML/JS，JSON 编辑器加表单式动作与光环编辑器、设备面板、登录启动开关、写入设备备用）。**前端已于 2026-10-17 重做**（重设计第 1 期）：`ui/` 改为 Svelte 5 + Vite + TypeScript，olanzi 风格的键位页（Layer 条、实物渲染图、手势卡片、分类键码面板、带 Rust 校验的保存 / 撤销）和只读的设置页；分层背光、光环编辑器和其余设置写入在后续各期。
4. 光环悬浮层。**Linux 已于 2026-10-16 完成**：不可聚焦的透明窗口由运行时事件驱动；旋转选择、按下确认、按键取消、空闲 4 秒关闭；多级光环已验证。
5. 灯光、麦克风、电源设置；打包、签名、自动更新。**灯光读写与设备信息已于 2026-10-16 完成**；`cargo tauri build` 可产出带 udev 规则的 `.deb`。麦克风与待机写入已完成；签名、自动更新待做。
