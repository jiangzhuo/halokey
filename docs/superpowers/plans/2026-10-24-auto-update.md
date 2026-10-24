# 自动更新 · 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 通过安装包安装的 halokey（Linux deb、macOS .app、Windows NSIS / MSI）在 GitHub 发布新 Release 后提醒用户，用户确认后自动下载、校验、安装并重启；发布流程产出带签名的更新包和 `latest.json`；托盘菜单与通知跟随界面语言。

**Architecture:** Rust 端新增 `update.rs`（状态机、纯逻辑、后台检查循环、下载安装）和 `text.rs`（托盘与通知的三语文案），用 `tauri-plugin-updater` / `tauri-plugin-notification` / `tauri-plugin-opener` 的 Rust API，前端只经本应用自己的命令和 `halokey-update` 事件拿状态。设置文件新增 `lang` 和 `updates` 两段。发布侧把 `release.yml` 改成"各平台构建 + 签名 → `publish` job 用 `tools/make_latest_json.py` 生成 `latest.json` → 草稿 Release"。

**Tech Stack:** Rust（Tauri 2.12，`tauri-plugin-updater` 2.14、`tauri-plugin-notification` 2.7、`tauri-plugin-opener` 2），Svelte 5 + vitest，Python 3（标准库 + unittest），GitHub Actions。

**Spec:** `docs/superpowers/specs/2026-10-24-auto-update-design.md`（全文；§2 体验、§3 架构、§7 实施者判断、§8 已定事项）。

## Global Constraints

- 检查时机：启动后 30 秒一次，之后每 6 小时一次；设置页"检查更新"随时手动检查。`updates.autoCheck` 默认 `true`，关掉后不做启动和定时检查，手动按钮仍可用。
- 每个版本只发一次系统通知；已通知版本号记在 `settings.json` 的 `updates.notifiedVersion`。
- 检查失败（后台）只写日志，不发通知、不进入错误状态；下载 / 校验 / 安装错误只显示在设置页状态行。
- 安装前先停设备服务（与托盘"退出"相同：停心跳、交还固件）；取消授权或安装失败后重新启动设备服务，应用继续运行。
- 支持自动更新的运行方式：`bundle_type()` 为 `Deb` / `Msi` / `Nsis`，或 macOS 上可执行文件位于 `.app/Contents/MacOS/` 内（macOS 的 `bundle_type()` 无论如何都返回 `App`，不能单独作为判断）。其余（源码运行、AppImage、Rpm）不检查、不发任何网络请求。
- `tauri.conf.json`：`bundle.createUpdaterArtifacts: true`；`plugins.updater.endpoints = ["https://github.com/jiangzhuo/halokey/releases/latest/download/latest.json"]`；`plugins.updater.windows.installMode = "passive"`；`plugins.updater.pubkey` = 公钥内容。
- `latest.json` 键：`linux-x86_64-deb`、`linux-aarch64-deb`、`darwin-aarch64-app`、`darwin-x86_64-app`、`windows-x86_64-nsis`、`windows-x86_64-msi`；不写 Linux 的裸 `{os}-{arch}` 键；不写 `notes`；缺任一平台就失败；`url` 为 `https://github.com/jiangzhuo/halokey/releases/download/vX.Y.Z/<文件名>`，`signature` 为 `.sig` 文件内容。
- 发布说明：设置页用系统浏览器打开 `https://github.com/jiangzhuo/halokey/releases/tag/vX.Y.Z`。
- 发布：只发 deb（去掉 AppImage）；Linux 在 `ubuntu-22.04`（x86_64）和 `ubuntu-22.04-arm`（aarch64）上打包；推 `v*` 标签只建**草稿** Release；`workflow_dispatch` 只做演练，不建 Release；去掉 `--no-sign`；签名私钥来自 secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。
- 语言：`settings.json` 的 `lang` 取 `en` / `zh` / `ja`，未知值按 `en`（与界面 `resolveLang` 一致）；托盘菜单与更新通知按它显示，切换语言后立即重建托盘菜单。
- 私钥和密码不出现在聊天、日志、提交、测试样例里；只存在 `~/.tauri/`（权限 600）和仓库 secrets。
- 代码注释中文，提交信息英文，末尾 `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`；README / docs 改动中英两版同步并跑 `python3 tools/check_docs.py`。
- 新界面字符串全部经 `t()`，EN / JA 表键集合一致；`.svelte` 模板里不留中文字面量（`i18n.test.ts` 强制）。
- 每个任务结束：`cd ui && npm test && npm run check && npm run build`、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo test --workspace --locked`、`python3 tools/check_docs.py` 全绿（Task 6 起再加 `python3 -m unittest discover -s tools/tests`）。

## Review Focus

1. **离线或 GitHub 不可达时的后台检查**：用户不该看到任何弹窗或红字，状态回到 `idle`；手动检查才显示错误。→ Task 3 `after_check_error` 测试。
2. **Linux 上点了安装但在授权框里取消**：设备服务要恢复、应用继续运行，设置页显示"已取消授权，可以稍后再试"，而不是一串英文错误。→ Task 3 `state_after_install_error` 测试（`AuthenticationFailed` → `Cancelled`）；Task 4 安装流程失败分支重启服务。
3. **旧版本或手改过的 `settings.json`**（没有 `lang` / `updates`、`lang` 为 `"fr"`）：其他设置不能被重置，默认值补上。→ Task 1 测试。
4. **连点"下载并安装"、或检查进行中点安装**：只能有一次下载。→ Task 3 `can_install` 测试；Task 5 按钮在 `downloading` / `installing` 时禁用的测试。
5. **下载时服务器没给总大小**（`total` 为 `null`）：进度只显示已下载的 MB，不能出现 `NaN` 或除以零。→ Task 5 测试 `downloading` + `total: null` 显示 `2.0 MB` 且页面里没有 `NaN`。

---

## 文件结构

```
apps/halokey/Cargo.toml                 + tauri-plugin-updater = "2"、tauri-plugin-notification = "2"、tauri-plugin-opener = "2"
apps/halokey/tauri.conf.json            createUpdaterArtifacts、plugins.updater
apps/halokey/src/text.rs     （新）     Lang、resolve_lang、托盘与通知三语文案
apps/halokey/src/update.rs   （新）     UpdateState、纯逻辑、Updates 状态、check / install / spawn_loop、命令
apps/halokey/src/main.rs                AppSettings.lang / updates；build_tray_menu / refresh_tray_menu；spawn_service；
                                        UiEvent::ShowUpdates；注册插件和命令；set_settings 重建托盘
ui/src/lib/tauri.ts                     UpdateState 类型、api.updateStatus / checkUpdate / installUpdate / openReleaseNotes、
                                        subscribeUpdate、AppSettings.lang / updates、mock
ui/src/components/UpdateCard.svelte（新）设置页"更新"一节
ui/src/pages/Settings.svelte            放入 UpdateCard（id="updates"）
ui/src/App.svelte                       同步界面语言到 settings.json；showUpdates 事件切到设置页
ui/src/lib/i18n.ts                      新字符串（EN / JA）
tools/make_latest_json.py    （新）     生成 latest.json，校验版本与平台
tools/tests/test_make_latest_json.py（新）
.github/workflows/release.yml           矩阵、签名、publish job
.github/workflows/docs.yml              跑 tools/tests
README.md / README.zh.md                安装与更新、发版步骤
docs/01-architecture.md (+zh)           里程碑 5：自动更新
AGENTS.md                               本地 `cargo tauri build` 需要 `--no-sign` 或私钥
```

---

### Task 1: 设置文件的 `lang` / `updates` 段与三语托盘菜单

**Files:**
- Create: `apps/halokey/src/text.rs`
- Modify: `apps/halokey/src/main.rs`（`AppSettings`、`merge_settings`、`set_settings`、`setup` 里的托盘、`on_menu_event`）
- Test: `apps/halokey/src/text.rs` 与 `main.rs` 的 `mod tests`

**Interfaces:**
- Produces（`text.rs`）：`pub enum Lang { En, Zh, Ja }`；`pub fn resolve_lang(s: &str) -> Lang`；`pub struct TrayText { pub open: &'static str, pub quit: &'static str }`；`pub fn tray_text(lang: Lang) -> TrayText`；`pub fn install_update_label(lang: Lang, version: &str) -> String`；`pub fn update_notification(lang: Lang, version: &str) -> (String, String)`（标题，正文）。
- Produces（`main.rs`）：`struct UpdateSettings { auto_check: bool, notified_version: Option<String> }`（serde camelCase、`default`，`auto_check` 默认 `true`）；`AppSettings` 新字段 `lang: String`（默认 `"en"`）、`updates: UpdateSettings`；`fn build_tray_menu(app: &AppHandle, lang: Lang, update: Option<&str>) -> tauri::Result<Menu<tauri::Wry>>`（菜单项 id：`open`、`install-update`（仅当 `update` 为 `Some`）、`quit`）；`fn refresh_tray_menu(app: &AppHandle, update: Option<&str>)`（读设置里的 `lang`，给 id 为 `"tray"` 的托盘换菜单）。
- `set_settings` 签名改为 `fn set_settings(app: AppHandle, state: State<'_, App>, patch: serde_json::Value) -> Result<AppSettings, String>`；patch 里有 `lang` 时调用 `refresh_tray_menu`（`update` 参数在 Task 4 之前传 `None`）。

- [ ] **Step 1: 写失败的测试**

```rust
// text.rs
#[test]
fn unknown_language_falls_back_to_english() {
    assert!(matches!(resolve_lang("fr"), Lang::En));
    assert!(matches!(resolve_lang("ja"), Lang::Ja));
    assert_eq!(tray_text(Lang::Zh).open, "打开设置");
    assert_eq!(tray_text(Lang::En).quit, "Quit");
    assert_eq!(tray_text(Lang::Ja).open, "設定を開く");
    assert_eq!(install_update_label(Lang::En, "0.2.0"), "Install update v0.2.0…");
    assert!(update_notification(Lang::Zh, "0.2.0").0.contains("v0.2.0"));
}

// main.rs
#[test]
fn old_settings_files_keep_their_values_and_gain_defaults() {
    let merged = merge_settings(serde_json::json!({ "idleMinutes": 10, "lang": "fr" }), &serde_json::json!({})).unwrap();
    assert_eq!(merged.idle_minutes, 10);
    assert!(matches!(text::resolve_lang(&merged.lang), text::Lang::En));
    assert!(merged.updates.auto_check);
    assert_eq!(merged.updates.notified_version, None);
    let off = merge_settings(serde_json::json!({}), &serde_json::json!({ "updates": { "autoCheck": false } })).unwrap();
    assert!(!off.updates.auto_check);
    assert_eq!(serde_json::to_value(&off).unwrap()["updates"]["notifiedVersion"], serde_json::Value::Null);
}
```

- [ ] **Step 2: 运行，确认失败**

Run: `cargo test -p halokey-app --locked`
Expected: 编译失败（`text` 模块、`updates` 字段不存在）。

- [ ] **Step 3: 实现**

`text.rs` 三语文案（英 / 中 / 日）：托盘 `Open settings / 打开设置 / 設定を開く`、`Quit / 退出 / 終了`；安装项 `Install update v{v}… / 安装更新 v{v}… / アップデート v{v} をインストール…`；通知标题 `halokey v{v} is available / halokey v{v} 可以更新了 / halokey v{v} が利用可能です`，正文 `Open Settings to install it. / 打开设置即可安装。 / 設定を開いてインストールしてください。`。`main.rs`：`AppSettings` 加字段；`setup` 里的托盘菜单改用 `build_tray_menu(…, resolve_lang(&settings.lang), None)`；`set_settings` 在 patch 含 `lang` 时 `refresh_tray_menu(&app, None)`。

- [ ] **Step 4: 运行，确认通过**

Run: `cargo test -p halokey-app --locked`
Expected: 全部 PASS。

- [ ] **Step 5: 提交**

```bash
git add apps/halokey/src/text.rs apps/halokey/src/main.rs
git commit -m "feat(app): settings gain lang and updates; tray menu follows the interface language"
```

---

### Task 2: 界面语言同步到 `settings.json`

**Files:**
- Modify: `ui/src/lib/tauri.ts`（`AppSettings` 类型、`setSettings` patch 类型、mock）、`ui/src/App.svelte`
- Test: `ui/src/lib/__tests__/tauri.test.ts`、`ui/src/__tests__/app.test.svelte.ts`

**Interfaces:**
- Consumes：Task 1 的 `settings.json` 字段 `lang`、`updates`。
- Produces：`AppSettings = { idleMinutes; halo; closeToTray; lang: "en" | "zh" | "ja"; updates: { autoCheck: boolean; notifiedVersion: string | null } }`；`setSettings(patch: { idleMinutes?; closeToTray?; halo?; lang?: Lang; updates?: { autoCheck?: boolean } })`。

- [ ] **Step 1: 写失败的测试**

```ts
// app.test.svelte.ts
it("syncs the interface language to settings on start and on change", async () => {
  const spy = vi.spyOn(api, "setSettings");
  render(App);
  await tick();
  expect(spy).toHaveBeenCalledWith({ lang: "en" });
  setLang("ja");
  await tick();
  expect(spy).toHaveBeenLastCalledWith({ lang: "ja" });
});
// tauri.test.ts
it("mock settings carry lang and updates defaults", async () => {
  resetMocks();
  const s = await api.getSettings();
  expect(s.lang).toBe("en");
  expect(s.updates).toEqual({ autoCheck: true, notifiedVersion: null });
});
```

- [ ] **Step 2: 运行，确认失败**

Run: `cd ui && npx vitest run src/__tests__/app.test.svelte.ts src/lib/__tests__/tauri.test.ts`
Expected: FAIL（没有调用 `setSettings`；mock 没有 `lang`）。

- [ ] **Step 3: 实现**

`App.svelte` 的 `onMount` 里调用一次 `api.setSettings({ lang: lang() })`，`halokey-lang` 事件处理里再调用一次（失败只 `console.error`）。mock 的 `defaultSettings` 补 `lang: "en"`、`updates: { autoCheck: true, notifiedVersion: null }`，`setSettings` mock 合并 `updates`。

- [ ] **Step 4: 运行，确认通过**

Run: `cd ui && npm test && npm run check`
Expected: 全部 PASS，0 errors。

- [ ] **Step 5: 提交**

```bash
git add ui/src/lib/tauri.ts ui/src/App.svelte ui/src/__tests__/app.test.svelte.ts ui/src/lib/__tests__/tauri.test.ts
git commit -m "feat(ui): store the interface language in settings so the tray can follow it"
```

---

### Task 3: 更新签名密钥、updater 配置与更新状态的纯逻辑

**Files:**
- Create: `apps/halokey/src/update.rs`
- Modify: `apps/halokey/Cargo.toml`、`apps/halokey/tauri.conf.json`、`apps/halokey/src/main.rs`（`mod update;`、注册三个插件）
- Test: `update.rs` 的 `mod tests`；`main.rs` 的配置测试

**Interfaces:**
- Produces（`update.rs`）：

```rust
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum UpdateState {
    Unsupported { reason: UnsupportedReason }, Idle, Checking, UpToDate,
    Available { version: String },
    Downloading { downloaded: u64, total: Option<u64> },
    Installing, Cancelled, Error { message: String },
}
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum UnsupportedReason { FromSource, Bundle }
pub const FIRST_CHECK_DELAY: Duration;   // 30 秒
pub const CHECK_INTERVAL: Duration;      // 6 小时
pub fn unsupported_reason(bundle: Option<BundleType>, exe: &Path) -> Option<UnsupportedReason>;
pub fn should_notify(available: &str, notified: Option<&str>) -> bool;
pub fn after_check_error(manual: bool, message: String) -> UpdateState;
pub fn can_install(state: &UpdateState) -> bool;
pub fn state_after_install_error(err: &tauri_plugin_updater::Error) -> UpdateState;
pub fn release_notes_url(version: &str) -> String;
```

- [ ] **Step 1: 生成更新签名密钥（只在 LittleBlack 上做）**

Run:
```bash
mkdir -p ~/.tauri && chmod 700 ~/.tauri
openssl rand -base64 32 | tr -d '\n' > ~/.tauri/halokey-updater.key.password && chmod 600 ~/.tauri/halokey-updater.key.password
cargo tauri signer generate --ci -w ~/.tauri/halokey-updater.key -p "$(cat ~/.tauri/halokey-updater.key.password)"
chmod 600 ~/.tauri/halokey-updater.key
```
Expected: `~/.tauri/halokey-updater.key` 和 `.key.pub` 存在。终端输出里的私钥不复制到任何地方。完成后在 #halokey 提醒 jiangzhuo 备份 `~/.tauri/halokey-updater.key*`。

- [ ] **Step 2: 写失败的测试**

```rust
// update.rs
#[test]
fn only_installed_packages_check_for_updates() {
    let plain = Path::new("/home/u/halokey/target/release/halokey-app");
    let app = Path::new("/Applications/halokey.app/Contents/MacOS/halokey-app");
    assert_eq!(unsupported_reason(Some(BundleType::Deb), plain), None);
    assert_eq!(unsupported_reason(Some(BundleType::Nsis), plain), None);
    assert_eq!(unsupported_reason(Some(BundleType::Msi), plain), None);
    assert_eq!(unsupported_reason(None, plain), Some(UnsupportedReason::FromSource));
    assert_eq!(unsupported_reason(Some(BundleType::AppImage), plain), Some(UnsupportedReason::Bundle));
    assert_eq!(unsupported_reason(Some(BundleType::Rpm), plain), Some(UnsupportedReason::Bundle));
    // macOS 的 bundle_type() 总是 App：要看路径是否在 .app 里
    assert_eq!(unsupported_reason(Some(BundleType::App), app), None);
    assert_eq!(unsupported_reason(Some(BundleType::App), plain), Some(UnsupportedReason::FromSource));
}
#[test]
fn each_version_is_notified_once() {
    assert!(should_notify("0.2.0", None));
    assert!(should_notify("0.2.0", Some("0.1.1")));
    assert!(!should_notify("0.2.0", Some("0.2.0")));
}
#[test]
fn background_check_failures_stay_quiet() {
    assert_eq!(after_check_error(false, "offline".into()), UpdateState::Idle);
    assert_eq!(after_check_error(true, "offline".into()), UpdateState::Error { message: "offline".into() });
}
#[test]
fn install_only_starts_from_available() {
    assert!(can_install(&UpdateState::Available { version: "0.2.0".into() }));
    for s in [UpdateState::Idle, UpdateState::Checking, UpdateState::Installing, UpdateState::Downloading { downloaded: 1, total: None }] {
        assert!(!can_install(&s));
    }
}
#[test]
fn cancelled_authorization_is_not_an_error() {
    assert_eq!(state_after_install_error(&tauri_plugin_updater::Error::AuthenticationFailed), UpdateState::Cancelled);
}
#[test]
fn state_json_matches_the_frontend_contract() {
    assert_eq!(serde_json::to_value(UpdateState::Downloading { downloaded: 5, total: Some(9) }).unwrap(),
               serde_json::json!({ "state": "downloading", "downloaded": 5, "total": 9 }));
    assert_eq!(serde_json::to_value(UpdateState::Unsupported { reason: UnsupportedReason::FromSource }).unwrap(),
               serde_json::json!({ "state": "unsupported", "reason": "fromSource" }));
    assert_eq!(release_notes_url("0.2.0"), "https://github.com/jiangzhuo/halokey/releases/tag/v0.2.0");
}

// main.rs
#[test]
fn updater_is_configured_for_github_releases() {
    let conf = tauri_conf();
    assert_eq!(conf["bundle"]["createUpdaterArtifacts"], true);
    let up = &conf["plugins"]["updater"];
    assert_eq!(up["endpoints"], serde_json::json!(["https://github.com/jiangzhuo/halokey/releases/latest/download/latest.json"]));
    assert_eq!(up["windows"]["installMode"], "passive");
    assert!(up["pubkey"].as_str().is_some_and(|k| k.len() > 40));
}
```

- [ ] **Step 3: 运行，确认失败**

Run: `cargo test -p halokey-app --locked`
Expected: 编译失败（`update` 模块不存在），配置测试失败。

- [ ] **Step 4: 实现**

`Cargo.toml` 加三个插件（`"2"`）；`tauri.conf.json` 加 Global Constraints 里的四项，`pubkey` 取 `~/.tauri/halokey-updater.key.pub` 的内容；`main.rs` 注册 `tauri_plugin_updater::Builder::new().build()`、`tauri_plugin_notification::init()`、`tauri_plugin_opener::init()`；`update.rs` 写上述函数，`unsupported_reason` 在 `App` 时检查路径里是否有 `.app/Contents/MacOS/`；`state_after_install_error` 只把 `Error::AuthenticationFailed`（2.14.0 的变体名，已在 docs.rs 核对）映射为 `Cancelled`，其余（含 `DebInstallFailed`、`PackageInstallFailed`；枚举是 `non_exhaustive`，要有通配分支）为 `Error { message: err.to_string() }`。

- [ ] **Step 5: 运行，确认通过**

Run: `cargo test -p halokey-app --locked && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: 全部 PASS，无警告。

- [ ] **Step 6: 提交**

```bash
git add apps/halokey/Cargo.toml Cargo.lock apps/halokey/tauri.conf.json apps/halokey/src/update.rs apps/halokey/src/main.rs
git commit -m "feat(app): updater config, signing public key and the update state logic"
```

---

### Task 4: 后台检查、通知、托盘项、下载安装与命令

**Files:**
- Modify: `apps/halokey/src/update.rs`、`apps/halokey/src/main.rs`（`spawn_service`、`UiEvent::ShowUpdates`、托盘 `install-update` 菜单事件、`manage(Updates)`、`setup` 里 `update::spawn_loop`、注册命令）
- Test: `cargo test`（纯逻辑已在 Task 3）；本任务以构建、clippy 和 Task 9 的端到端为准

**Interfaces:**
- Consumes：Task 1 的 `refresh_tray_menu`、`update_notification`、`UpdateSettings`；Task 3 的全部函数。
- Produces：
  - `pub struct Updates { state: Mutex<UpdateState>, pending: tauri::async_runtime::Mutex<Option<tauri_plugin_updater::Update>> }`（`main` 里 `.manage(Updates::default())`；初始状态由 `unsupported_reason(bundle_type(), current_exe)` 决定）。
  - `pub fn set_state(app: &AppHandle, state: UpdateState)`：存状态、`app.emit("halokey-update", &state)`、`Available` 时 `refresh_tray_menu(app, Some(&version))`，否则 `refresh_tray_menu(app, None)`。
  - `pub async fn check(app: &AppHandle, manual: bool) -> UpdateState`；`pub async fn install(app: &AppHandle) -> Result<(), String>`；`pub fn spawn_loop(app: AppHandle)`。
  - 命令：`update_status() -> UpdateState`、`check_update() -> UpdateState`、`install_update() -> Result<(), String>`、`open_release_notes(version: String) -> Result<(), String>`。
  - `main.rs`：`fn spawn_service(app: &AppHandle) -> Service`（从 `setup` 抽出：按当前配置文件和设置构建 `Config`）；`UiEvent::ShowUpdates`（`{"kind":"showUpdates"}`）。

- [ ] **Step 1: 抽出 `spawn_service`，`setup` 改为调用它**

Run: `cargo test -p halokey-app --locked`
Expected: 现有测试全部 PASS（纯重构）。

- [ ] **Step 2: 实现 `check`**

不支持时直接返回当前 `Unsupported` 状态，不做网络请求。`Checking` → `app.updater()?.check()`：`Some(update)` 存进 `pending`，状态 `Available`；若 `should_notify` 为真，先设状态（托盘项、事件），再发通知（系统没有通知服务时会失败：结果忽略，只 `eprintln!`，托盘项和设置页提示不受影响），再把 `notifiedVersion` 写回设置文件；`None` → `UpToDate`；错误 → `after_check_error(manual, …)`，同时 `eprintln!`。

- [ ] **Step 3: 实现 `install`**

`can_install` 为假时返回 `Err`。`download(进度回调 → Downloading{…})` → 状态 `Installing` → 取出并 `stop()` 设备服务 → `update.install(bytes)` → 成功则 `app.restart()`；失败则 `state_after_install_error`、`spawn_service` 放回 `App.service`、返回 `Err`。

- [ ] **Step 4: 实现 `spawn_loop`、命令和托盘项**

循环：先睡 `FIRST_CHECK_DELAY`，之后每轮读设置：`autoCheck` 为真才 `check(app, false)`，然后睡 `CHECK_INTERVAL`。托盘 `install-update`：`show_main` + `app.emit("halokey", UiEvent::ShowUpdates)`。`open_release_notes` 用 opener 打开 `release_notes_url(&version)`。

- [ ] **Step 5: 运行检查**

Run: `cargo test --workspace --locked && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all -- --check`
Expected: 全部通过。

- [ ] **Step 6: 本机冒烟（需要 jiangzhuo 同意）**

正在运行的 halokey-app 占着设备，冒烟要先停它。先在 #halokey 征得同意；同意后：`cd ui && npm run build`，`cargo build --release -p halokey-app`，`systemctl --user stop halokey-app`，`HALOKEY_SHOW_MAIN=1` 运行新二进制，确认设置页"更新"一节显示 `Running from source; updates are not automatic`（源码构建）、托盘菜单按界面语言显示、切换语言后托盘菜单跟着变，然后退出并用原来的方式重新启动 app（`systemd-run --user --collect --unit=halokey-app <原二进制>`）。不同意就跳过，并在任务报告里写明没做。

- [ ] **Step 7: 提交**

```bash
git add apps/halokey/src/update.rs apps/halokey/src/main.rs
git commit -m "feat(app): check for updates on start and every 6 h, notify once, install on confirmation"
```

---

### Task 5: 设置页"更新"一节

**Files:**
- Create: `ui/src/components/UpdateCard.svelte`、`ui/src/components/__tests__/update-card.test.svelte.ts`
- Modify: `ui/src/lib/tauri.ts`、`ui/src/pages/Settings.svelte`、`ui/src/App.svelte`、`ui/src/lib/i18n.ts`

**Interfaces:**
- Consumes：Task 4 的命令与 `halokey-update` 事件、`UiEvent` `showUpdates`。
- Produces（`tauri.ts`）：`UpdateState` 类型（与 Task 3 的 JSON 一一对应）；`api.updateStatus(): Promise<UpdateState>`、`api.checkUpdate(): Promise<UpdateState>`、`api.installUpdate(): Promise<void>`、`api.openReleaseNotes(version: string): Promise<void>`；`subscribeUpdate(handler: (s: UpdateState) => void): Promise<() => void>`；测试用 `setMockUpdate(s: UpdateState)`（`resetMocks` 复位为 `{ state: "idle" }`）；`UiEvent` 增加 `{ kind: "showUpdates" }`。

- [ ] **Step 1: 写失败的测试**

```ts
it("shows the version and installs on click", async () => {
  setMockUpdate({ state: "available", version: "0.2.0" });
  const install = vi.spyOn(api, "installUpdate");
  render(UpdateCard);
  await tick();
  expect(screen.getByText(/0\.2\.0/)).toBeTruthy();
  await fireEvent.click(screen.getByRole("button", { name: "Download and install" }));
  expect(install).toHaveBeenCalledOnce();
});
it("disables install while downloading and shows progress", async () => {
  setMockUpdate({ state: "downloading", downloaded: 1048576, total: 4194304 });
  render(UpdateCard); await tick();
  expect(screen.queryByRole("button", { name: "Download and install" })).toBeNull();
  expect(screen.getByText(/1\.0 \/ 4\.0 MB/)).toBeTruthy();
});
it("explains source runs and cancelled authorization", async () => {
  setMockUpdate({ state: "unsupported", reason: "fromSource" });
  render(UpdateCard); await tick();
  expect(screen.getByText("Running from source; updates are not automatic")).toBeTruthy();
});
it("auto-check toggle writes settings", async () => {
  const spy = vi.spyOn(api, "setSettings");
  render(UpdateCard); await tick();
  await fireEvent.click(screen.getByRole("checkbox", { name: "Check for updates automatically" }));
  expect(spy).toHaveBeenCalledWith({ updates: { autoCheck: false } });
});
it("release notes link opens the release page", async () => {
  setMockUpdate({ state: "available", version: "0.2.0" });
  const open = vi.spyOn(api, "openReleaseNotes");
  render(UpdateCard); await tick();
  await fireEvent.click(screen.getByRole("button", { name: "Release notes" }));
  expect(open).toHaveBeenCalledWith("0.2.0");
});
it("progress without a total shows only the downloaded size", async () => {
  setMockUpdate({ state: "downloading", downloaded: 2097152, total: null });
  render(UpdateCard); await tick();
  expect(screen.getByText(/2\.0 MB/)).toBeTruthy();
  expect(document.body.textContent).not.toContain("NaN");
});
```

（`cancelled` 状态显示 `"Authorization cancelled; you can try again later"`；`error` 显示 `message` 原文；`upToDate` 显示 `"You're up to date"`；`checking` 显示 `"Checking…"`；`installing` 显示 `"Installing…"`。在同一文件里各加一条渲染断言。）

- [ ] **Step 2: 运行，确认失败**

Run: `cd ui && npx vitest run src/components/__tests__/update-card.test.svelte.ts`
Expected: FAIL（组件不存在）。

- [ ] **Step 3: 实现**

`UpdateCard.svelte`：当前版本（`api.snapshot().version`）、自动检查开关（`getSettings().updates.autoCheck`）、"检查更新"按钮（`checking` / `downloading` / `installing` 时禁用）、状态行、`available` 时的"发布说明"按钮和"下载并安装"按钮；挂载时 `updateStatus()` + `subscribeUpdate`。字节数按 MB 保留一位小数。`Settings.svelte` 在"通用"卡之后放 `<section class="card" id="updates"><UpdateCard /></section>`。`App.svelte`：收到 `showUpdates` → `page = "settings"`，`tick()` 后 `document.getElementById("updates")?.scrollIntoView()`。新字符串进 `i18n.ts` 的 EN、JA 表（中文为键：`更新`、`当前版本`、`自动检查更新`、`检查更新`、`正在检查…`、`已是最新版本`、`有新版本`、`发布说明`、`下载并安装`、`正在下载`、`正在安装…`、`已取消授权，可以稍后再试`、`从源码运行，不自动更新`、`这种安装方式不支持自动更新`）。

- [ ] **Step 4: 运行，确认通过**

Run: `cd ui && npm test && npm run check && npm run build`
Expected: 全部 PASS（含 `i18n.test.ts` 的键集合与模板字面量检查），0 errors。

- [ ] **Step 5: 提交**

```bash
git add ui/src/components/UpdateCard.svelte ui/src/components/__tests__/update-card.test.svelte.ts ui/src/lib/tauri.ts ui/src/pages/Settings.svelte ui/src/App.svelte ui/src/lib/i18n.ts
git commit -m "feat(ui): Updates section in Settings with progress, release notes and auto-check"
```

---

### Task 6: `tools/make_latest_json.py`

**Files:**
- Create: `tools/make_latest_json.py`、`tools/tests/test_make_latest_json.py`
- Modify: `.github/workflows/docs.yml`（新增一步 `python3 -m unittest discover -s tools/tests -v`）

**Interfaces:**
- Produces：
  - `PLATFORMS: dict[str, str]`（键 → 文件名正则）：`linux-x86_64-deb: r"_amd64\.deb$"`、`linux-aarch64-deb: r"_arm64\.deb$"`、`darwin-aarch64-app: r"_aarch64\.app\.tar\.gz$"`、`darwin-x86_64-app: r"_x64\.app\.tar\.gz$"`、`windows-x86_64-nsis: r"_x64-setup\.exe$"`、`windows-x86_64-msi: r"_x64_en-US\.msi$"`。
  - `check_version(requested: str, conf_version: str, cargo_version: str) -> str`（去掉前导 `v`；三者不一致抛 `ValueError`）。
  - `build_manifest(version: str, artifacts: Path) -> dict`（每个平台恰好一个匹配文件且有同名 `.sig`，否则 `ValueError` 并列出缺的键；`url` = `https://github.com/jiangzhuo/halokey/releases/download/v{version}/{name}`；`signature` = `.sig` 文本原样）。
  - CLI：`python3 tools/make_latest_json.py --artifacts DIR --out FILE (--tag vX.Y.Z | --version-from-conf)`；读 `apps/halokey/tauri.conf.json` 和根 `Cargo.toml` 的版本；失败退出码 1 并打印原因。

- [ ] **Step 1: 写失败的测试**

```python
class MakeLatestJson(unittest.TestCase):
    def test_full_manifest(self):
        d = fake_artifacts("0.2.0")  # 六个包 + .sig，签名内容为 "SIG-<文件名>"
        m = build_manifest("0.2.0", d)
        self.assertEqual(m["version"], "0.2.0")
        self.assertNotIn("notes", m)
        self.assertEqual(set(m["platforms"]), set(PLATFORMS))
        self.assertNotIn("linux-x86_64", m["platforms"])
        deb = m["platforms"]["linux-aarch64-deb"]
        self.assertEqual(deb["url"], "https://github.com/jiangzhuo/halokey/releases/download/v0.2.0/halokey_0.2.0_arm64.deb")
        self.assertEqual(deb["signature"], "SIG-halokey_0.2.0_arm64.deb")
    def test_missing_platform_fails(self):
        d = fake_artifacts("0.2.0", skip="halokey_0.2.0_arm64.deb")
        with self.assertRaisesRegex(ValueError, "linux-aarch64-deb"):
            build_manifest("0.2.0", d)
    def test_missing_signature_fails(self):
        d = fake_artifacts("0.2.0", skip_sig="halokey_0.2.0_x64-setup.exe")
        with self.assertRaisesRegex(ValueError, "windows-x86_64-nsis"):
            build_manifest("0.2.0", d)
    def test_version_must_match_everywhere(self):
        self.assertEqual(check_version("v0.2.0", "0.2.0", "0.2.0"), "0.2.0")
        with self.assertRaises(ValueError):
            check_version("v0.2.1", "0.2.0", "0.2.0")
```

- [ ] **Step 2: 运行，确认失败**

Run: `python3 -m unittest discover -s tools/tests -v`
Expected: ImportError（模块不存在）。

- [ ] **Step 3: 实现**（只用标准库：`argparse`、`json`、`re`、`tomllib`、`pathlib`）

- [ ] **Step 4: 运行，确认通过**

Run: `python3 -m unittest discover -s tools/tests -v && python3 tools/check_docs.py`
Expected: 全部 OK。

- [ ] **Step 5: 提交**

```bash
git add tools/make_latest_json.py tools/tests/test_make_latest_json.py .github/workflows/docs.yml
git commit -m "build: generate latest.json from signed release artifacts"
```

---

### Task 7: 发布流程

**Files:**
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Consumes：Task 6 的 CLI；secrets `TAURI_SIGNING_PRIVATE_KEY`、`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。
- Produces：每个构建 job 上传工作流产物 `halokey-<name>`（安装包、更新包、`.sig`）；`publish` job。

- [ ] **Step 1: 改 `bundle` 矩阵**

| name | os | args |
|---|---|---|
| `linux-x86_64` | `ubuntu-22.04` | `--bundles deb` |
| `linux-aarch64` | `ubuntu-22.04-arm` | `--bundles deb` |
| `macos-arm64` | `macos-latest` | `--target aarch64-apple-darwin --bundles app,dmg` |
| `macos-x86_64` | `macos-latest` | `--target x86_64-apple-darwin --bundles app,dmg` |
| `windows-x86_64` | `windows-latest` | `--bundles msi,nsis` |

构建命令改为 `cargo tauri build ${{ matrix.args }}`（去掉 `--no-sign`），`env` 带两个 secrets。收集：`*.deb`、`*.dmg`、`*.msi`、`*-setup.exe` 及它们的 `.sig`，macOS 的 `halokey.app.tar.gz(.sig)` 改名为 `halokey_<version>_<aarch64|x64>.app.tar.gz(.sig)`（版本从 `tauri.conf.json` 读）。不再收集 AppImage。

- [ ] **Step 2: 新增 `publish` job**

`needs: bundle`，`runs-on: ubuntu-latest`，`permissions: contents: write`。`actions/download-artifact@v4`（`merge-multiple: true`）到 `dist/`；标签触发时 `python3 tools/make_latest_json.py --artifacts dist --out dist/latest.json --tag ${{ github.ref_name }}`，然后 `softprops/action-gh-release@v2`（`draft: true`、`files: dist/*`）；`workflow_dispatch` 时用 `--version-from-conf`，只把 `dist/latest.json` 作为工作流产物上传。workflow 顶层 `permissions` 降为 `contents: read`。

- [ ] **Step 3: 本地校验**

Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml'))" && echo OK`
Expected: `OK`。再用 Task 6 测试里的假产物目录跑一遍 CLI：`python3 tools/make_latest_json.py --artifacts <假目录> --out /tmp/latest.json --version-from-conf`，Expected：退出码 0，六个平台键。

- [ ] **Step 4: 提交**

```bash
git add .github/workflows/release.yml
git commit -m "ci(release): signed deb/app/nsis/msi updater bundles, arm64 deb, latest.json, no AppImage"
```

---

### Task 8: 文档

**Files:**
- Modify: `README.md`、`README.zh.md`、`docs/01-architecture.md`、`docs/01-architecture.zh.md`、`AGENTS.md`

- [ ] **Step 1: README 两版**

功能表 "auto-update planned" 改为已完成并说明方式（提醒 → 确认 → 安装）；安装段改为：Linux 用 Release 里的 deb（x86_64 / arm64），不再提 AppImage；新增"发版"小节：改 `apps/halokey/tauri.conf.json` 与根 `Cargo.toml` 的版本 → 提交 → 推 `vX.Y.Z` 标签 → 在 GitHub 上检查草稿并发布（发布后用户才收到更新）；本地 `cargo tauri build` 没有私钥时加 `--no-sign`。

- [ ] **Step 2: 架构文档两版**

§7 里程碑 5 写明自动更新完成的范围（deb、macOS .app、Windows NSIS / MSI；源码运行不更新）和日期。

- [ ] **Step 3: AGENTS.md §3**

补一句：开启 `createUpdaterArtifacts` 后，本地 `cargo tauri build` 需要 `TAURI_SIGNING_PRIVATE_KEY`，否则加 `--no-sign`。

- [ ] **Step 4: 校验并提交**

Run: `python3 tools/check_docs.py`
Expected: `✅ 全部通过`。

```bash
git add README.md README.zh.md docs/01-architecture.md docs/01-architecture.zh.md AGENTS.md
git commit -m "docs: auto-update, deb-only Linux releases and the release steps"
```

---

### Task 9: 密钥入库、演练与端到端（每一步对外操作前单独确认）

- [ ] **Step 1: 推送分支并开 PR** —— 先在 #halokey 问 jiangzhuo（目标：`jiangzhuo/halokey`，分支 `feat/auto-update` → `main`）。
- [ ] **Step 2: 写入仓库 secrets** —— 先问 jiangzhuo；确认后 `gh secret set TAURI_SIGNING_PRIVATE_KEY -R jiangzhuo/halokey < ~/.tauri/halokey-updater.key`、`gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD -R jiangzhuo/halokey < ~/.tauri/halokey-updater.key.password`。Expected：`gh secret list -R jiangzhuo/halokey` 列出两项。
- [ ] **Step 3: 演练** —— 在 PR 分支上 `gh workflow run release.yml -R jiangzhuo/halokey --ref feat/auto-update`。Expected：五个构建 job 和 `publish` 都成功；下载 `latest.json` 产物，六个平台键齐全、签名非空；macOS / Windows 在没有证书时也打包成功。
- [ ] **Step 4: 端到端** —— 由 jiangzhuo 决定时间：合并后发 v0.1.0（第一个带更新功能的版本）并发布，在 LittleBlack 上安装其 deb；再发 v0.1.1 并发布；确认提醒、授权、安装、重启都正常，设备按键在重启后可用。
