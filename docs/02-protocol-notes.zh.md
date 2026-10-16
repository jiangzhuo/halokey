# 02 · 协议笔记
> 🌐 [English](02-protocol-notes.md)

> 📚 文档集：[README](../README.zh.md) · [01 架构](01-architecture.zh.md) · **02 协议笔记**

halokey 在 olanzi 记录之外自己确认的 AU05 厂商通道事实。除非另有说明，这里的一切都是在 Linux 上通过 `/dev/hidraw` 用 `halokey` 命令行实测的。原始日志在 [evidence/](evidence/)。

---

## 1. Linux 传输

[CONFIRMED] hidapi 的纯 Rust hidraw 后端枚举接收器时带 usage page：接口 2 暴露 usage page `0x000c`、`0x0001`（usage 1、2、6），接口 3 暴露 `0xfffc` usage 1。halokey 按 usage page 选厂商通道，只有后端报不出 usage page 时才退回接口号 3。

[CONFIRMED] 读取不需要心跳：设备空闲数分钟后，`status` 和 `keys` 仍在毫秒内应答。

## 2. 心跳回显

[CONFIRMED] 主机发出的每条心跳 `06 01 23 00 01` 都会在约 1 ms 后被接收器原样回显，回复位和 access 半字节都不置位。olanzi 把心跳列为"无已验证应答"，这个回显就是那个应答。[2026-10-16-linux-vendor-events.log](evidence/2026-10-16-linux-vendor-events.log) 里 119 条心跳对应 119 条回显。因此回显缺失可以当作接收器健康信号，守护进程应当盯住它。

## 3. 六个控件的厂商按键事件

[CONFIRMED] 心跳运行时，每个控件都产生 `8b 10 <action> <pressed> <index>`：

| 控件 | index | action | 每个手势的帧 |
|---|---|---|---|
| 键 1 | `00` | `6f` | 按下 + 松开 |
| 键 2 | `01` | `70` | 按下 + 松开 |
| 键 3 | `02` | `71` | 按下 + 松开 |
| 旋钮按下 | `03` | `6e` | 按下 + 松开 |
| 旋钮 → 右拧 | `04` | `72` | **每格一帧 `pressed = 1`，没有松开** |
| 旋钮 ← 左拧 | `05` | `73` | **每格一帧 `pressed = 1`，没有松开** |

action 字节对每个控件固定，但不是键码；主机按 `index` 路由。本次抓取中每感知到一格转动产生两帧（3 格 6 帧，4 格 8 帧），所以主机必须把每一帧当作一步，绝不能等松开。

## 4. 电量广播（通知 `0x7b`）

[INFERRED] 设备每隔数秒主动广播 `0b 7b <v_lo> <v_hi> 00 <percent> <?> 01`，不需要请求。四个样本中，2–3 字节的小端电压和第 5 字节的百分比都与随后 `01 01 02 01` 的读数完全一致（`cf 0e` / `d0 0e` = 3791 / 3792 mV，`3c` = 60%；更早的 `15 0f` = 3861 mV，`46` = 70%）。第 6 字节（`c6`、`c1`、`c0`、`bb`）与第 7 字节（恒为 `01`）含义未解，所以 halokey 不从广播推导充电状态。`halokey-proto` 以 `status::parse_battery_notice` 暴露它。

## 5. 鼠标页（类型 `0x07`）

[CONFIRMED] 见 olanzi 文档 02 §6 与 05 §10：键码 N 是鼠标按键 N+1，`0x05` / `0x06` 是滚轮上 / 下，没有指针移动条目。`halokey keys` 把它们显示为 `鼠标按键 N` / `滚轮↑` / `滚轮↓`。

## 6. 灯光：四个通道都能呼吸

[CONFIRMED] olanzi 只允许键 1 使用 `workType = 2`（呼吸），因为它只实测过那一盏灯。halokey 对四盏灯都发 `01 0b 88 04 04 <light> …`；2026-10-16 在设置窗口把旋钮灯（index 3）设为呼吸，写入得到确认，读回一致，用户看到旋钮灯在呼吸。这与 Windows 工具 `doitian/ulanzi-vibekey` 的 SDK 读回结果一致：四个通道都接受该值。

## 7. 未解问题

- Linux 上心跳运行时标准 HID 接口是否保持沉默（macOS 上是）。本平台尚未抓取。
- 电量广播第 6–7 字节的含义。
- 本体关机时接收器是否仍回显心跳（可用来区分接收器健康与本体在场）。
