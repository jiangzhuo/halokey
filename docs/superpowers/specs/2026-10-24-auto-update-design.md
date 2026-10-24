# halokey 自动更新 · 设计文档

> 2026-10-24 · 依据 #halokey 里与 jiangzhuo 的讨论：更新方式按"提醒 → 用户确认 → 下载安装并重启"；更新签名密钥由 Halo 在 LittleBlack 上生成；Linux 只发 deb 包、只做 deb 的自动更新，发布里去掉 AppImage。
> 2026-10-24 jiangzhuo 确认设计（"按照你建议的来"）。本文是实施计划的依据；内部工作文档，只有中文版。

## 1. 目标与范围

**目标**：用户安装 halokey 之后，GitHub 上发布新的 Release 时，应用能提醒用户；用户点确认后自动下载、校验、安装并重启，不用再去网页下载。

**成功标准**

- 通过安装包安装的应用（Linux deb、macOS 的 .app、Windows 安装版），在新 Release 发布后，最迟到下一次定时检查（6 小时内）或下一次启动时收到提醒。
- 点"下载并安装"后全程在应用里完成。Linux deb 只需在系统授权框里输一次密码；macOS 只在 .app 所在目录不可写时要密码；Windows 只有安装程序的进度窗。
- 更新包都经过签名校验，校验不过就不安装。
- 推送 `v*` 标签只产出**草稿** Release；jiangzhuo 在 GitHub 上点"发布"之后，用户才会收到更新。

**非目标（本期不做）**

- 完全静默安装（Linux deb 本来就需要授权；也不符合"确认后安装"的约定）。
- AppImage 包（发布里去掉；它不能安装 udev 规则、自启路径会变、启动的子程序会继承它打包的库路径，见 #halokey 讨论）和 rpm。
- macOS 公证、macOS / Windows 代码签名（另一项工作，不影响更新本身）。
- 更新通道（beta / stable）、增量更新、回滚界面。
- 从源码运行（`cargo run`、直接跑 `target/*/halokey-app`）时的自动更新：这种运行方式不检查更新，设置页写明"从源码运行，不自动更新"。

**反目标（做出来但出现这些就算失败）**

- 没经过用户确认就下载安装或重启。
- 离线、GitHub 不可达等检查失败时弹窗或发通知打扰用户。
- 同一个版本反复发通知。
- 发布流程在拿不到私钥时悄悄产出没有签名的 Release（必须直接失败）。
- 安装更新时设备停在厂商事件模式、按键没反应（安装前必须像正常退出一样把控制权交还固件）。

## 2. 用户体验

### 2.1 检查

- 应用启动 30 秒后检查一次，之后每 6 小时检查一次；设置页另有"检查更新"按钮随时手动检查。
- 设置里新增"自动检查更新"开关，默认开。关掉后不做启动和定时检查，手动按钮仍然可用。
- 当前运行方式不支持自动更新（见 §3.1 的支持判断）时，不做任何网络请求。

### 2.2 发现新版本

- **系统通知**，每个版本只发一次："halokey vX.Y.Z 可以更新了 · 打开设置安装"。已通知过的版本号记在设置文件里，重启应用也不重复通知。点通知能打开设置窗口就打开；平台不支持点击回调时只显示通知。
- **托盘菜单**多一项"安装更新 vX.Y.Z…"，点了打开设置窗口并定位到"更新"一节。没有可用更新时这一项不出现。
- **设置页"更新"一节**：当前版本、自动检查开关、"检查更新"按钮、状态行。有新版本时显示新版本号、"发布说明"链接（用系统浏览器打开 `https://github.com/jiangzhuo/halokey/releases/tag/vX.Y.Z`）和"下载并安装"按钮。

### 2.3 下载与安装

- 点"下载并安装"后，状态行显示下载进度（已下载 / 总大小）。下载完成并校验签名后：
  1. 先停掉设备服务（与托盘"退出"相同：停心跳、交还固件），再安装。
  2. 各平台安装方式：
     - **Linux deb**：系统弹授权框（pkexec），授权后安装新包，然后重启应用。
     - **macOS**：替换 .app 后重启应用。
     - **Windows**：启动安装程序（passive 模式，只有进度条），应用随即退出，由安装程序完成后重新启动应用。
  3. 用户取消授权、或安装失败时：设备服务重新启动，状态行显示原因（例如"已取消授权，可以稍后再试"），应用继续正常运行。
- 下载、校验、安装的错误只显示在设置页状态行，不发通知。检查失败只写日志。

## 3. 架构

### 3.1 应用侧（`apps/halokey`、`ui/`）

**依赖**

- `tauri-plugin-updater` 2.x（目前 2.14.0），`tauri-plugin-notification` 2.x（目前 2.7.0），`tauri-plugin-opener` 2.x（打开发布说明链接）。
- 只用 Rust API，前端通过本应用自己的命令和事件拿状态，不给前端开放插件的 JS 权限（与现有 `api.*` 调用方式一致）。

**新模块 `apps/halokey/src/update.rs`**

- 更新状态：`Unsupported(原因)` / `Idle` / `Checking` / `UpToDate` / `Available { version, notes }` / `Downloading { downloaded, total }` / `Installing` / `Error(消息)`。状态保存在应用状态里，变化时向设置窗口发事件。
- 支持判断：用 `tauri::utils::platform::bundle_type()`（打包时由 tauri-cli 写入二进制）。`Deb`、`App`、`Nsis`、`Msi` 视为支持；`None`（从源码运行）、`AppImage`、`Rpm` 视为不支持。不支持时不检查，状态为 `Unsupported`。
- 纯逻辑放成可单测的函数：是否需要通知（新版本号与已通知版本号比较）、支持判断、设置里 `updates` 段的合并与校验。
- 后台检查：`tauri::async_runtime` 里的循环（启动后 30 秒，之后每 6 小时；每次先读设置里的开关）。
- 新命令：`update_status`、`check_update`（手动检查）、`install_update`（下载并安装当前可用版本）。新事件：设置窗口订阅的更新状态事件。
- 托盘：可用更新出现时给托盘菜单加"安装更新 vX.Y.Z…"项。

**配置 `apps/halokey/tauri.conf.json`**

- `bundle.createUpdaterArtifacts: true`
- `plugins.updater.pubkey`：更新签名公钥的内容
- `plugins.updater.endpoints`：`["https://github.com/jiangzhuo/halokey/releases/latest/download/latest.json"]`
- `plugins.updater.windows.installMode: "passive"`

**设置文件**

- `AppSettings` 增加 `updates: { autoCheck: bool, notifiedVersion: string | null }`，默认 `autoCheck = true`。沿用现有的深合并与保留未知键的写法。

**语言**

- 界面语言除了存在设置窗口的 localStorage，也写进 `settings.json` 的 `lang`（`en` / `zh` / `ja`，默认 `en`，与界面默认一致）。设置窗口启动时把 localStorage 里已有的选择同步过去，切换语言时也同步。
- 托盘菜单（现有的"打开设置""退出"和新的"安装更新"）和更新通知都按 `lang` 显示；切换语言后托盘菜单立即重建。顺带修好托盘菜单一直只有中文的问题。

**前端（`ui/`）**

- 设置页新增"更新"一节（§2.2、§2.3 的内容）；字符串进 i18n，中、英、日三语。
- `ui/src/lib/tauri.ts` 增加对应的 `api` 方法和类型；浏览器里打开开发服务器时（没有 Tauri）用 mock，和现有写法一致。

### 3.2 发布侧（`.github/workflows/release.yml`、`tools/`）

- **去掉 `--no-sign`**：tauri-cli 2.12.3 的 `src/bundle.rs` 里，这个参数会让更新签名一起跳过（"Updater signing is skipped due to --no-sign flag."）。构建时通过 secrets 提供 `TAURI_SIGNING_PRIVATE_KEY` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`；拿不到私钥时 tauri-cli 会直接报错，构建失败。
- **构建矩阵**：
  - `linux-x86_64`：`ubuntu-22.04`，只产出 deb（去掉现在的 AppImage）
  - `linux-aarch64`（新增）：`ubuntu-22.04-arm`，产出 deb。在 22.04 上打包是为了不抬高 glibc 要求
  - `macos-arm64`、`macos-x86_64`：产出 dmg（首次安装用）和 `.app.tar.gz`（更新用）
  - `windows-x86_64`：产出 NSIS 安装包和 MSI，自动更新走 NSIS
  - 每个更新包都带一个同名 `.sig`
  - 两个 macOS 任务产出的更新包同名，`publish` job 按架构改名后再上传
- **触发方式**：推送 `v*` 标签时，构建并由 `publish` job 创建草稿 Release。手动 `workflow_dispatch` 只做演练：同样构建、签名并生成 `latest.json`（版本取 `tauri.conf.json`），但只作为工作流产物上传，不创建 Release。
- **新的 `publish` job**（依赖全部构建 job）：下载所有构建产物，运行 `python3 tools/make_latest_json.py` 生成 `latest.json`，再创建草稿 Release，上传全部安装包、更新包、`.sig` 和 `latest.json`。用单独的 job 一次性生成，避免多个构建 job 并发改同一个 `latest.json`。
- **`tools/make_latest_json.py`**：
  - `version` 取自标签（去掉 `v`），必须与 `tauri.conf.json` 和 workspace `Cargo.toml` 的版本一致，否则失败。
  - `platforms` 的键按 updater 的查找规则写：先查 `{os}-{arch}-{installer}`，再查 `{os}-{arch}`。需要写：`linux-x86_64-deb`、`linux-aarch64-deb`、`darwin-aarch64-app`、`darwin-x86_64-app`、`windows-x86_64-nsis`、`windows-x86_64-msi`（MSI 安装的用户用 MSI 更新，不会装出两份）。（Linux 不写不带安装类型的键，免得 AppImage 或源码运行的版本误拿到 deb。）
  - 不写 `notes`：Release 是草稿时就生成了 `latest.json`，之后在 GitHub 上编辑的说明进不去；应用里改为链接到 Release 页面（见 §2.2）。
  - `url` 指向 `https://github.com/jiangzhuo/halokey/releases/download/vX.Y.Z/<文件名>`，`signature` 是 `.sig` 文件的内容。
  - 缺任何一个平台的更新包或签名就失败，因为 updater 会先校验整份 `latest.json` 再比较版本，一个平台坏了所有平台都收不到更新。
- **版本号**：发版前把 `tauri.conf.json` 和 workspace `Cargo.toml` 的版本改成新版本并提交，然后推同名标签。发版步骤写进 README（中英两版）。

### 3.3 更新签名密钥

- Halo 在 LittleBlack 上用 `cargo tauri signer generate` 生成密钥对，私钥放 `~/.tauri/halokey-updater.key`（权限 600），密码随机生成，同样只存本机。
- 私钥和密码通过 `gh secret set` 写进 `jiangzhuo/halokey` 的仓库 secrets（执行前在 #halokey 单独确认）。公钥提交进 `tauri.conf.json`。
- 私钥和密码不出现在聊天、日志、提交里。jiangzhuo 需要另外备份一份（例如密码管理器）：私钥丢了，已安装的用户就再也收不到更新。公钥不要随意更换。

## 4. 测试与验证

- **Rust 单元测试**：是否通知、支持判断、`updates` 设置的合并和默认值；`tauri.conf.json` 里开启了 `createUpdaterArtifacts`、endpoint 是 HTTPS 的 GitHub 地址、公钥非空。
- **Python 单元测试**：`make_latest_json` 的平台键、签名读取、版本不一致报错、缺平台报错。CI 里跑。
- **前端 vitest**：更新一节在各状态下的显示和按钮行为。
- **发布流程演练**：用 `workflow_dispatch` 跑一遍，检查产物齐全、`latest.json` 正确、草稿 Release 内容正确。同时确认去掉 `--no-sign` 后，没有证书的 macOS 和 Windows 仍能打包。
- **端到端**：要发两个真实版本（例如 v0.1.0 作为第一个带更新功能的基线，再发 v0.1.1），在 LittleBlack 上装 v0.1.0 的 deb，发布 v0.1.1 后看提醒、授权、安装、重启是否顺利。发布时间由 jiangzhuo 决定。

## 5. 风险

- 第一个带更新功能的版本之前安装的用户收不到自动更新。目前还没有正式发布过，影响基本只有从源码运行的人。
- Linux deb 的授权依赖 polkit 授权代理。GNOME、KDE 都有；极简桌面没有时，插件会退回 zenity、kdialog 或终端 sudo。
- macOS 应用没签名：首次安装要右键打开。更新替换 .app 之后 Gatekeeper 的表现需要实测。[INFERRED] 应用自己下载的包不带隔离属性，应该不会再被拦。
- Windows 应用没签名：首次安装会有 SmartScreen 提示；passive 安装是否需要 UAC，取决于 NSIS 的安装范围，需要实测。
- `ubuntu-22.04-arm` 是 GitHub 的 arm runner，公开仓库可用。它不可用时 `publish` 会因为缺 `linux-aarch64-deb` 而失败，这是有意的：不发缺平台的 Release，届时再决定是否去掉 arm64。

## 6. 留给实施者决定

- "更新"一节和托盘菜单项的具体样式与文案。
- 检查间隔和启动延迟常量的具体实现方式（数值按 §2.1）。
- 通知点击行为在各平台的实现细节。
- `make_latest_json.py` 的命令行参数形式。

## 7. 实施者的判断（Halo 的决定，jiangzhuo 审阅时可推翻）

- 发布说明不进 `latest.json`，应用里链接到 GitHub 的 Release 页面（原因见 §3.2）。
- MSI 安装也支持自动更新，`latest.json` 写 `windows-x86_64-msi`。
- 手动 `workflow_dispatch` 只做演练，不创建 Release。
- arm64 的 deb 必须由 CI 构建；缺了就不发版。

## 8. 待 jiangzhuo 确认

- ~~系统通知和托盘菜单用什么语言~~：已定，跟着界面语言走（2026-10-24）。见 §3.1"语言"。
- ~~发布流程里的 AppImage~~：已定，去掉（2026-10-24）。
