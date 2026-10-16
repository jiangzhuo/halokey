# AGENTS.md · 项目 Memory

> 项目级记忆，每次会话开始时自动加载。修改约定前先读这里。本文件不翻译。

---

## 1. 语言约定

| 场景 | 语言 |
|---|---|
| 与用户对话 | **中文** |
| 代码注释 | 中文 |
| 提交信息 | **英文**（仓库公开） |
| 文档 | **中英双语（强制）** |

## 2. 文档双语规则

- 英文是默认入口，文件名不带语言后缀；中文版加 `.zh.md`，同目录兄弟文件。
- 新增文档必须同时产出两份；改一份必须同步另一份。
- 两版 Markdown 结构一一对应：标题层级、表格形状、代码块数量。
- 每份 `docs/` 文档头部要有 `📚` 文档集导航行和 `🌐` 语言互链。
- 十六进制、字节序列、命令、路径、符号名、错误码、`[CONFIRMED]` / `[INFERRED]` 标签一律不翻译。
- 改完必须跑 `python3 tools/check_docs.py`，不通过就是没做完。

## 3. 技术栈

Rust workspace + Tauri 2。crate 划分与数据流见 `docs/01-architecture.md`。
平台相关代码只允许出现在 `crates/input`、`crates/wm`、`crates/hid` 的平台模块和 `apps/halokey` 里。

## 4. 协议不变量（直接用，不要重新推导）

- 设备 AU05，VID `0xFFF1` / PID `0x00DD`。接口 2 = 标准 HID（Consumer `0x01` / Mouse `0x02` / Keyboard `0x03`），接口 3 = 厂商通道（usage page `0xFFFC`，report ID `0x55`）。
- TEA / ECB / 8 字节分组 / 32 轮，delta `0x9E3779B9`，解密 sum 起始 `0xC6EF3720`，密钥 `ca ba a5 ca 6d 8a 2a bc ba 9e 5a ca ca 8b b8 9b`，自检 `TEA_Enc(00×8) == 38 90 c4 99 a3 60 aa ad`。
- 明文 64 字节，线上为 report ID + 前 63 字节密文；回复只解前 7 个分组。
- 按键表：读 `01 06 50 01 <index>`，写 `01 06 50 04 <index> 01 <num> <type> <code>`，写确认 access `0x14`，读回 access `0x11`。
- 条目类型：`0x02` 键盘，`0x03` 系统/多媒体，`0x07` 鼠标页（键码 N = 鼠标按键 N+1，`0x05`/`0x06` = 滚轮上/下，**没有指针移动**）。
- 控件 index：0 键 1（上）、1 键 2（中）、2 键 3（下）、3 旋钮按下、4 右拧、5 左拧。
- 旋钮转动 = 0–10 ms 脉冲，人手按键 = 80–2000 ms，阈值 25 ms。
- 设备唯一标识（deviceSn、flashId）不公开，文档里写 `<REDACTED>`。
- 完整协议记录在 olanzi 仓库 `docs/02-vibekey-protocol.md` 与 `docs/05-verification-log.md`。

## 5. 踩过的坑

| 约束 | 说明 |
|---|---|
| macOS 不要用 `IOHIDManagerOpen` | 全有全无；hidapi 按设备逐个 `IOHIDDeviceOpen`，验证后再信 |
| Linux `/dev/hidraw*` 默认 `root:root 0600` | 开发时 `chmod a+rw`，发布附 udev 规则；`/dev/uinput` 同理 |
| Windows 输出报告必须等长 | 含 report ID 补满 64 字节 |
| 判断设备离线看原始明文 | 真离线：`06 03 0a 11 00`；代码 bug：一行都不打印 |
| 输入监控 / 辅助功能权限失败要大声报错 | 静默跳过会变成"按了没反应" |
| `pkill -f` 会杀掉包含同样字符串的当前 shell | 用 `pgrep` 精确匹配 |

## 6. 安全规则

- 读命令（查询、读键位、读电量）随时可发。
- 写键位持久化但可还原；试验先选"改坏也没关系"的槽位，并先读回原值留底。
- 其他 `access=0x04` 写命令（亮度、麦克风、灯、OTA）参数格式未明前不要乱发。
- 电源键长按可能直接关机，不要测。

## 7. 证据标准

- **[CONFIRMED]**：有直接证据（实测数据、字面量、设备回复）。
- **[INFERRED]**：间接推断，待验证。
- 永远不把推断写成结论；失败尝试也要留档。

## 8. 协作偏好

- 用户愿意动手验证；需要实测时直接说明按哪个控件、什么顺序、间隔多久。
- 用户的现场观察优先于推断。
- 对外行为（开 PR、发 issue、评论）每次单独确认。

## 9. Rules (English)

- Chat and code comments in Chinese; commit messages in English.
- Every document under `README` and `docs/` exists in English (unsuffixed) and Chinese (`.zh.md`), with identical structure; run `python3 tools/check_docs.py` after any change.
- Never translate hex, byte sequences, commands, paths, symbols, error codes or the `[CONFIRMED]` / `[INFERRED]` tags.
- Platform-specific code is confined to the adapter crates and the Tauri app.
- Device-unique identifiers are never published.
- Follow the pitfalls in §5 and the safety rules in §6.
