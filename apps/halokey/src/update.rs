//! 应用内更新：状态与纯逻辑、后台检查循环、系统通知、下载安装，以及给设置页的命令。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde::Serialize;
use tauri::utils::config::BundleType;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;

use crate::{text, App};

/// 更新状态；序列化后直接给前端（`state` 字段区分）。
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum UpdateState {
    Unsupported { reason: UnsupportedReason },
    Idle,
    Checking,
    UpToDate,
    Available { version: String },
    Downloading { downloaded: u64, total: Option<u64> },
    Installing,
    Cancelled,
    Error { message: String },
}

/// 不支持应用内更新的原因。
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum UnsupportedReason {
    /// 从源码直接运行（不是安装包）。
    FromSource,
    /// 安装包类型不支持（AppImage、Rpm 等）。
    Bundle,
}

/// 启动后首次检查前的等待时间。
pub const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);
/// 两次后台检查的间隔。
pub const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// 状态变化时发给设置窗口的事件，载荷是 `UpdateState` 的 JSON。
const UPDATE_EVENT: &str = "halokey-update";

/// 下载总大小未知时，进度每多少字节报一次。
const UNKNOWN_TOTAL_STEP: u64 = 256 * 1024;

/// 当前运行方式是否支持更新；`None` 表示支持。
/// macOS 的 `bundle_type()` 恒为 `App`，所以还要看可执行文件是否真在 `.app` 里。
pub fn unsupported_reason(bundle: Option<BundleType>, exe: &Path) -> Option<UnsupportedReason> {
    match bundle {
        Some(BundleType::Deb | BundleType::Msi | BundleType::Nsis) => None,
        Some(BundleType::App) => {
            let in_app = exe
                .to_string_lossy()
                .replace('\\', "/")
                .contains(".app/Contents/MacOS/");
            if in_app {
                None
            } else {
                Some(UnsupportedReason::FromSource)
            }
        }
        None => Some(UnsupportedReason::FromSource),
        Some(_) => Some(UnsupportedReason::Bundle),
    }
}

/// 同一个版本只通知一次。
pub fn should_notify(available: &str, notified: Option<&str>) -> bool {
    notified != Some(available)
}

/// 检查失败后的状态：后台检查静默回到 `Idle`，手动检查才报错。
pub fn after_check_error(manual: bool, message: String) -> UpdateState {
    if manual {
        UpdateState::Error { message }
    } else {
        UpdateState::Idle
    }
}

/// 一次检查失败后的状态。之前已经发现的新版本保留（托盘项、安装按钮和 pending 都还在），
/// 后台、手动检查都一样；其余照 `after_check_error`。
pub fn state_after_check_error(
    previous: &UpdateState,
    manual: bool,
    message: String,
) -> UpdateState {
    match previous {
        UpdateState::Available { .. } => previous.clone(),
        _ => after_check_error(manual, message),
    }
}

/// 只有发现新版本之后才能开始安装。
pub fn can_install(state: &UpdateState) -> bool {
    matches!(state, UpdateState::Available { .. })
}

/// 安装出错后的状态：用户取消系统授权不算错误。
pub fn state_after_install_error(err: &tauri_plugin_updater::Error) -> UpdateState {
    match err {
        tauri_plugin_updater::Error::AuthenticationFailed => UpdateState::Cancelled,
        // 枚举是 non_exhaustive，其余一律当作错误展示
        other => UpdateState::Error {
            message: other.to_string(),
        },
    }
}

/// 该版本的发布说明页面。
pub fn release_notes_url(version: &str) -> String {
    format!("https://github.com/jiangzhuo/halokey/releases/tag/v{version}")
}

/// 启动时的状态：不支持的运行方式一直是 `Unsupported`，其余从 `Idle` 开始。
fn initial_state(reason: Option<UnsupportedReason>) -> UpdateState {
    match reason {
        Some(reason) => UpdateState::Unsupported { reason },
        None => UpdateState::Idle,
    }
}

/// 可以安装的版本号；托盘菜单有没有"安装更新"项只看它。
fn available_version(state: &UpdateState) -> Option<&str> {
    match state {
        UpdateState::Available { version } => Some(version),
        _ => None,
    }
}

/// 只有可安装的版本变了（进入、离开 `Available` 或换了版本）才重建托盘菜单，下载进度不重建。
fn tray_needs_refresh(old: &UpdateState, new: &UpdateState) -> bool {
    available_version(old) != available_version(new)
}

/// 正在下载或安装。
fn install_running(state: &UpdateState) -> bool {
    matches!(
        state,
        UpdateState::Downloading { .. } | UpdateState::Installing
    )
}

/// 能否开始一次检查：不支持时不联网；正在检查、下载、安装时不打断，免得覆盖进行中的状态。
fn can_check(state: &UpdateState) -> bool {
    !matches!(
        state,
        UpdateState::Unsupported { .. } | UpdateState::Checking
    ) && !install_running(state)
}

/// 下载进度节流：总大小已知时百分比每变一个整数才报一次，未知时每 256 KiB 报一次。
#[derive(Default)]
struct Progress {
    downloaded: u64,
    /// 上次报告时的刻度（百分比，或 256 KiB 的块数）；一次下载里总大小不变，两种刻度不会混用。
    last_mark: Option<u64>,
}

impl Progress {
    /// 记下一块数据；需要刷新界面时返回新状态。
    fn advance(&mut self, chunk: usize, total: Option<u64>) -> Option<UpdateState> {
        // 服务器给 0 当作不知道，免得除以零、界面显示 x / 0
        let total = total.filter(|&t| t > 0);
        self.downloaded = self
            .downloaded
            .saturating_add(u64::try_from(chunk).unwrap_or(u64::MAX));
        let mark = match total {
            Some(total) => self.downloaded.min(total).saturating_mul(100) / total,
            None => self.downloaded / UNKNOWN_TOTAL_STEP,
        };
        if self.last_mark == Some(mark) {
            return None;
        }
        self.last_mark = Some(mark);
        Some(UpdateState::Downloading {
            downloaded: self.downloaded,
            total,
        })
    }
}

/// 应用里的更新状态；`checking` 为真时有一次检查在跑；`pending` 是检查到、还没安装的更新。
pub struct Updates {
    state: Mutex<UpdateState>,
    checking: AtomicBool,
    pending: tauri::async_runtime::Mutex<Option<tauri_plugin_updater::Update>>,
}

/// 一次检查进行中；丢掉时（检查结束、出错或 panic）放开，下一次检查才能开始。
struct CheckRunning<'a>(&'a AtomicBool);

impl Drop for CheckRunning<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl Default for Updates {
    /// 初始状态由运行方式决定（安装包类型和可执行文件位置）。
    fn default() -> Self {
        let exe = tauri::utils::platform::current_exe().unwrap_or_else(|e| {
            eprintln!("[halokey] 拿不到可执行文件路径，按路径判断的更新支持会失效：{e}");
            PathBuf::new()
        });
        let reason = unsupported_reason(tauri::utils::platform::bundle_type(), &exe);
        Updates {
            state: Mutex::new(initial_state(reason)),
            checking: AtomicBool::new(false),
            pending: Default::default(),
        }
    }
}

impl Updates {
    fn lock(&self) -> MutexGuard<'_, UpdateState> {
        // 状态总是一个完整的值，锁中毒了也照常用
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn current(&self) -> UpdateState {
        self.lock().clone()
    }

    /// 当前可安装的版本号（重建托盘菜单时用）。
    pub fn available_version(&self) -> Option<String> {
        available_version(&self.lock()).map(str::to_string)
    }

    /// 换上新状态，返回旧状态。
    fn replace(&self, state: UpdateState) -> UpdateState {
        std::mem::replace(&mut *self.lock(), state)
    }

    /// `allowed(当前状态)` 为真时换成 `next` 并返回 `Ok(旧状态)`，否则不动并返回 `Err(当前状态)`。
    /// 判断和替换在同一把锁里：连点两次"下载并安装"也只会下载一份。
    fn transition(
        &self,
        allowed: fn(&UpdateState) -> bool,
        next: UpdateState,
    ) -> Result<UpdateState, UpdateState> {
        let mut state = self.lock();
        if allowed(&state) {
            Ok(std::mem::replace(&mut *state, next))
        } else {
            Err(state.clone())
        }
    }

    /// 开始一次检查，返回检查前的状态；不支持、正在下载安装、已有检查在跑时不开始，返回 `Err(当前状态)`。
    /// 手动检查把状态换成 `Checking`；后台检查不改状态，托盘项和安装按钮保持原样。
    fn begin_check(&self, manual: bool) -> Result<(UpdateState, CheckRunning<'_>), UpdateState> {
        let mut state = self.lock();
        if !can_check(&state) || self.checking.swap(true, Ordering::AcqRel) {
            return Err(state.clone());
        }
        let running = CheckRunning(&self.checking);
        let previous = if manual {
            std::mem::replace(&mut *state, UpdateState::Checking)
        } else {
            state.clone()
        };
        Ok((previous, running))
    }

    /// 写入检查结果并返回旧状态。后台检查不占状态，期间用户可能已经开始安装：那时不覆盖，返回 `Err(当前状态)`。
    fn finish_check(&self, result: UpdateState) -> Result<UpdateState, UpdateState> {
        self.transition(|state| !install_running(state), result)
    }
}

/// 换上新状态并通知设置窗口。
pub fn set_state(app: &AppHandle, state: UpdateState) {
    let old = app.state::<Updates>().replace(state.clone());
    publish(app, &old, &state);
}

/// 状态已经换好：发事件；可安装的版本变了才重建托盘菜单。
fn publish(app: &AppHandle, old: &UpdateState, new: &UpdateState) {
    if let Err(e) = app.emit(UPDATE_EVENT, new) {
        eprintln!("[halokey] 发送更新状态失败：{e}");
    }
    if tray_needs_refresh(old, new) {
        crate::refresh_tray_menu(app, available_version(new));
    }
}

async fn find_update(
    app: &AppHandle,
) -> tauri_plugin_updater::Result<Option<tauri_plugin_updater::Update>> {
    app.updater()?.check().await
}

/// 检查一次更新并返回检查后的状态。`manual` 为用户点了"检查更新"：显示"正在检查"，失败才报错；
/// 后台检查不显示过程，失败只写日志。已经发现的新版本不会因为检查失败而丢掉（见 `state_after_check_error`）。
/// 不支持或正在检查、下载、安装时什么也不做，直接返回当前状态。
pub async fn check(app: &AppHandle, manual: bool) -> UpdateState {
    let updates = app.state::<Updates>();
    let (previous, _running) = match updates.begin_check(manual) {
        Ok(started) => started,
        Err(current) => return current,
    };
    if manual {
        publish(app, &previous, &UpdateState::Checking);
    }
    let (result, found) = match find_update(app).await {
        Ok(Some(update)) => (
            UpdateState::Available {
                version: update.version.clone(),
            },
            Some(update),
        ),
        Ok(None) => (UpdateState::UpToDate, None),
        Err(e) => {
            eprintln!("[halokey] 检查更新失败：{e}");
            (
                state_after_check_error(&previous, manual, e.to_string()),
                None,
            )
        }
    };
    // 拿着 pending 的锁写结果：pending 与状态一致，安装要先拿到这把锁才能取走更新包
    let mut pending = updates.pending.lock().await;
    let old = match updates.finish_check(result.clone()) {
        Ok(old) => old,
        Err(current) => return current,
    };
    if let Some(update) = found {
        *pending = Some(update);
    }
    drop(pending);
    // 状态真的变了才通知设置页、重建托盘；先设状态（托盘项、设置页提示），再发通知
    if old != result {
        publish(app, &old, &result);
    }
    if let UpdateState::Available { version } = &result {
        notify_once(app, version);
    }
    result
}

/// 每个版本只发一次系统通知，然后把版本号记进设置文件。
/// 没有通知服务时发送会失败：只写日志，托盘项和设置页提示不受影响。
fn notify_once(app: &AppHandle, version: &str) {
    let config_path = &app.state::<App>().config_path;
    let settings = crate::load_settings(config_path);
    if !should_notify(version, settings.updates.notified_version.as_deref()) {
        return;
    }
    let (title, body) = text::update_notification(text::resolve_lang(&settings.lang), version);
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        eprintln!("[halokey] 系统通知发送失败：{e}");
    }
    let patch = serde_json::json!({ "updates": { "notifiedVersion": version } });
    let saved = crate::merge_settings(crate::load_settings_value(config_path), &patch)
        .and_then(|settings| crate::save_settings(config_path, &settings));
    if let Err(e) = saved {
        eprintln!("[halokey] 记录已通知的版本失败：{e}");
    }
}

/// 下载并安装当前可用的版本。成功后重启应用（Windows 由安装程序退出并重启应用），不会返回。
/// 下载或安装失败、用户取消授权时设备服务照常运行，状态行显示原因，返回 `Err`。
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let updates = app.state::<Updates>();
    let starting = UpdateState::Downloading {
        downloaded: 0,
        total: None,
    };
    let previous = updates
        .transition(can_install, starting.clone())
        .map_err(|_| "没有可安装的更新".to_string())?;
    publish(app, &previous, &starting);
    // 复制而不取走：安装失败后，进行中的后台检查可能把 `Available` 恢复回来，那时更新包还要在
    let pending = updates.pending.lock().await.clone();
    let Some(update) = pending else {
        // 不会发生：进入 Available 之前总是先存好 pending
        let message = "没有可安装的更新".to_string();
        set_state(
            app,
            UpdateState::Error {
                message: message.clone(),
            },
        );
        return Err(message);
    };

    let mut progress = Progress::default();
    let downloaded = update
        .download(
            |chunk, total| {
                if let Some(state) = progress.advance(chunk, total) {
                    set_state(app, state);
                }
            },
            || {},
        )
        .await;
    let bytes = match downloaded {
        Ok(bytes) => bytes,
        Err(e) => {
            // 设备服务还没停，只需报告
            eprintln!("[halokey] 下载更新失败：{e}");
            set_state(app, state_after_install_error(&e));
            return Err(e.to_string());
        }
    };

    set_state(app, UpdateState::Installing);
    // 先停设备服务（与托盘"退出"相同：停心跳、交还固件），再安装。
    // 两步都会阻塞（等设备线程结束、等系统授权），放到阻塞线程池里。
    let handle = app.clone();
    let installed = tauri::async_runtime::spawn_blocking(move || {
        crate::stop_service(&handle);
        update.install(bytes)
    })
    .await;
    let (state, message) = match installed {
        Ok(Ok(())) => app.restart(),
        Ok(Err(e)) => (state_after_install_error(&e), e.to_string()),
        Err(e) => (
            UpdateState::Error {
                message: e.to_string(),
            },
            e.to_string(),
        ),
    };
    eprintln!("[halokey] 安装更新失败：{message}");
    restore_service(app);
    set_state(app, state);
    Err(message)
}

/// 安装没成功：按当前配置重新启动设备服务，应用继续正常运行。
fn restore_service(app: &AppHandle) {
    let state = app.state::<App>();
    let mut service = state.service.lock().unwrap_or_else(PoisonError::into_inner);
    if service.is_none() {
        *service = Some(crate::spawn_service(app));
    }
}

/// 后台检查：启动 30 秒后一次，之后每 6 小时一次；每轮重新读设置里的"自动检查更新"。
/// async 运行时没有暴露计时器，所以在单独的线程里睡，检查本身交给 Tauri 的 async 运行时。
pub fn spawn_loop(app: AppHandle) {
    let spawned = std::thread::Builder::new()
        .name("halokey-update".into())
        .spawn(move || {
            std::thread::sleep(FIRST_CHECK_DELAY);
            loop {
                let auto_check = crate::load_settings(&app.state::<App>().config_path)
                    .updates
                    .auto_check;
                if auto_check {
                    tauri::async_runtime::block_on(check(&app, false));
                }
                std::thread::sleep(CHECK_INTERVAL);
            }
        });
    if let Err(e) = spawned {
        eprintln!("[halokey] 无法启动后台更新检查：{e}");
    }
}

/// 当前更新状态；设置页打开时取一次，之后听 `halokey-update` 事件。
#[tauri::command]
pub fn update_status(updates: State<'_, Updates>) -> UpdateState {
    updates.current()
}

/// 手动检查更新。
#[tauri::command]
pub async fn check_update(app: AppHandle) -> UpdateState {
    check(&app, true).await
}

/// 下载并安装当前可用的版本。
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    install(&app).await
}

/// 用系统浏览器打开该版本的发布说明。
#[tauri::command]
pub fn open_release_notes(app: AppHandle, version: String) -> Result<(), String> {
    app.opener()
        .open_url(release_notes_url(&version), None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tauri::utils::config::BundleType;

    #[test]
    fn only_installed_packages_check_for_updates() {
        let plain = Path::new("/home/u/halokey/target/release/halokey-app");
        let app = Path::new("/Applications/halokey.app/Contents/MacOS/halokey-app");
        assert_eq!(unsupported_reason(Some(BundleType::Deb), plain), None);
        assert_eq!(unsupported_reason(Some(BundleType::Nsis), plain), None);
        assert_eq!(unsupported_reason(Some(BundleType::Msi), plain), None);
        assert_eq!(
            unsupported_reason(None, plain),
            Some(UnsupportedReason::FromSource)
        );
        assert_eq!(
            unsupported_reason(Some(BundleType::AppImage), plain),
            Some(UnsupportedReason::Bundle)
        );
        assert_eq!(
            unsupported_reason(Some(BundleType::Rpm), plain),
            Some(UnsupportedReason::Bundle)
        );
        // macOS 的 bundle_type() 总是 App：要看路径是否在 .app 里
        assert_eq!(unsupported_reason(Some(BundleType::App), app), None);
        assert_eq!(
            unsupported_reason(Some(BundleType::App), plain),
            Some(UnsupportedReason::FromSource)
        );
    }

    #[test]
    fn each_version_is_notified_once() {
        assert!(should_notify("0.2.0", None));
        assert!(should_notify("0.2.0", Some("0.1.1")));
        assert!(!should_notify("0.2.0", Some("0.2.0")));
    }

    #[test]
    fn background_check_failures_stay_quiet() {
        assert_eq!(
            after_check_error(false, "offline".into()),
            UpdateState::Idle
        );
        assert_eq!(
            after_check_error(true, "offline".into()),
            UpdateState::Error {
                message: "offline".into()
            }
        );
    }

    #[test]
    fn install_only_starts_from_available() {
        assert!(can_install(&UpdateState::Available {
            version: "0.2.0".into()
        }));
        for s in [
            UpdateState::Idle,
            UpdateState::Checking,
            UpdateState::Installing,
            UpdateState::Downloading {
                downloaded: 1,
                total: None,
            },
        ] {
            assert!(!can_install(&s));
        }
    }

    #[test]
    fn cancelled_authorization_is_not_an_error() {
        assert_eq!(
            state_after_install_error(&tauri_plugin_updater::Error::AuthenticationFailed),
            UpdateState::Cancelled
        );
    }

    #[test]
    fn unsupported_builds_start_unsupported_others_idle() {
        assert_eq!(initial_state(None), UpdateState::Idle);
        assert_eq!(
            initial_state(Some(UnsupportedReason::Bundle)),
            UpdateState::Unsupported {
                reason: UnsupportedReason::Bundle
            }
        );
    }

    #[test]
    fn tray_menu_is_rebuilt_only_when_the_available_version_changes() {
        let available = |v: &str| UpdateState::Available { version: v.into() };
        let downloading = |n| UpdateState::Downloading {
            downloaded: n,
            total: Some(100),
        };
        assert_eq!(available_version(&available("0.2.0")), Some("0.2.0"));
        assert_eq!(available_version(&UpdateState::Checking), None);
        // 进入、离开 Available、换了版本：重建
        assert!(tray_needs_refresh(
            &UpdateState::Checking,
            &available("0.2.0")
        ));
        assert!(tray_needs_refresh(&available("0.2.0"), &downloading(0)));
        assert!(tray_needs_refresh(&available("0.2.0"), &available("0.2.1")));
        // 下载进度、普通状态之间切换：不重建
        assert!(!tray_needs_refresh(&downloading(1), &downloading(2)));
        assert!(!tray_needs_refresh(
            &UpdateState::Idle,
            &UpdateState::Checking
        ));
        assert!(!tray_needs_refresh(
            &available("0.2.0"),
            &available("0.2.0")
        ));
    }

    #[test]
    fn checks_never_run_unsupported_or_over_a_running_check_or_install() {
        for s in [
            UpdateState::Unsupported {
                reason: UnsupportedReason::FromSource,
            },
            UpdateState::Downloading {
                downloaded: 0,
                total: None,
            },
            UpdateState::Installing,
            UpdateState::Checking,
        ] {
            assert!(!can_check(&s), "{s:?}");
        }
        for s in [
            UpdateState::Idle,
            UpdateState::UpToDate,
            UpdateState::Available {
                version: "0.2.0".into(),
            },
            UpdateState::Cancelled,
            UpdateState::Error {
                message: "offline".into(),
            },
        ] {
            assert!(can_check(&s), "{s:?}");
        }
    }

    fn updates_in(state: UpdateState) -> Updates {
        Updates {
            state: Mutex::new(state),
            checking: AtomicBool::new(false),
            pending: Default::default(),
        }
    }

    #[test]
    fn a_failed_check_keeps_a_known_update() {
        let available = UpdateState::Available {
            version: "0.2.0".into(),
        };
        // 已经发现的新版本：后台、手动检查失败都保留（托盘项和安装按钮不消失）
        assert_eq!(
            state_after_check_error(&available, false, "offline".into()),
            available
        );
        assert_eq!(
            state_after_check_error(&available, true, "offline".into()),
            available
        );
        // 其余照旧：后台静默回到 Idle，手动报错
        assert_eq!(
            state_after_check_error(&UpdateState::Idle, false, "offline".into()),
            UpdateState::Idle
        );
        for previous in [UpdateState::Idle, UpdateState::UpToDate] {
            assert_eq!(
                state_after_check_error(&previous, true, "offline".into()),
                UpdateState::Error {
                    message: "offline".into()
                }
            );
        }
    }

    #[test]
    fn background_checks_keep_the_state_and_only_one_check_runs_at_a_time() {
        let available = UpdateState::Available {
            version: "0.2.0".into(),
        };
        let updates = updates_in(available.clone());
        let (previous, running) = updates.begin_check(false).unwrap();
        assert_eq!(previous, available);
        assert_eq!(updates.current(), available, "后台检查不显示 Checking");
        // 检查还在跑：再来一次（手动或后台）不开始，状态不变
        assert_eq!(updates.begin_check(true).err(), Some(available.clone()));
        assert_eq!(updates.begin_check(false).err(), Some(available.clone()));
        assert_eq!(updates.current(), available);
        drop(running);
        // 上一次结束后，手动检查显示 Checking
        let (previous, _running) = updates.begin_check(true).unwrap();
        assert_eq!(previous, available);
        assert_eq!(updates.current(), UpdateState::Checking);
        // 不支持、正在下载时不检查
        let downloading = UpdateState::Downloading {
            downloaded: 1,
            total: None,
        };
        assert_eq!(
            updates_in(downloading.clone()).begin_check(false).err(),
            Some(downloading)
        );
    }

    #[test]
    fn a_check_result_never_overwrites_a_running_install() {
        // 后台检查期间用户点了安装：结果作废，下载状态不动
        let downloading = UpdateState::Downloading {
            downloaded: 5,
            total: Some(9),
        };
        let updates = updates_in(downloading.clone());
        assert_eq!(
            updates.finish_check(UpdateState::UpToDate),
            Err(downloading.clone())
        );
        assert_eq!(updates.current(), downloading);
        let updates = updates_in(UpdateState::Installing);
        assert_eq!(
            updates.finish_check(UpdateState::UpToDate),
            Err(UpdateState::Installing)
        );
        let updates = updates_in(UpdateState::Checking);
        assert_eq!(
            updates.finish_check(UpdateState::UpToDate),
            Ok(UpdateState::Checking)
        );
        assert_eq!(updates.current(), UpdateState::UpToDate);
    }

    #[test]
    fn install_can_only_be_started_once() {
        let updates = updates_in(UpdateState::Available {
            version: "0.2.0".into(),
        });
        let starting = UpdateState::Downloading {
            downloaded: 0,
            total: None,
        };
        assert_eq!(
            updates.transition(can_install, starting.clone()),
            Ok(UpdateState::Available {
                version: "0.2.0".into()
            })
        );
        assert_eq!(updates.current(), starting);
        // 第二次点击：已经在下载，不再开始，状态不变
        assert_eq!(
            updates.transition(can_install, starting.clone()),
            Err(starting.clone())
        );
        assert_eq!(updates.current(), starting);
        assert_eq!(updates.available_version(), None);
    }

    #[test]
    fn progress_with_known_total_is_reported_once_per_percent() {
        let mut progress = Progress::default();
        let reports: Vec<UpdateState> = (0..1000)
            .filter_map(|_| progress.advance(10, Some(10_000)))
            .collect();
        // 第一块（0%）报一次，之后 1%..100% 各一次
        assert_eq!(reports.len(), 101);
        assert_eq!(
            reports[0],
            UpdateState::Downloading {
                downloaded: 10,
                total: Some(10_000)
            }
        );
        assert_eq!(
            reports[1],
            UpdateState::Downloading {
                downloaded: 100,
                total: Some(10_000)
            }
        );
        assert_eq!(
            reports.last(),
            Some(&UpdateState::Downloading {
                downloaded: 10_000,
                total: Some(10_000)
            })
        );
    }

    #[test]
    fn progress_with_unknown_total_is_reported_every_256_kib() {
        let mut progress = Progress::default();
        let reports: Vec<UpdateState> = (0..16)
            .filter_map(|_| progress.advance(64 * 1024, None))
            .collect();
        // 第一块报一次，之后每满 256 KiB 一次（共 1 MiB）
        let downloaded: Vec<u64> = reports
            .iter()
            .map(|s| match s {
                UpdateState::Downloading {
                    downloaded,
                    total: None,
                } => *downloaded,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            downloaded,
            [64 * 1024, 256 * 1024, 512 * 1024, 768 * 1024, 1024 * 1024]
        );
        // 总大小为 0（服务器给了个怪值）按未知处理，不除以零
        let mut zero = Progress::default();
        assert_eq!(
            zero.advance(1, Some(0)),
            Some(UpdateState::Downloading {
                downloaded: 1,
                total: None
            })
        );
        assert_eq!(zero.advance(1, Some(0)), None);
    }

    #[test]
    fn state_json_matches_the_frontend_contract() {
        assert_eq!(
            serde_json::to_value(UpdateState::Downloading {
                downloaded: 5,
                total: Some(9)
            })
            .unwrap(),
            serde_json::json!({ "state": "downloading", "downloaded": 5, "total": 9 })
        );
        assert_eq!(
            serde_json::to_value(UpdateState::Unsupported {
                reason: UnsupportedReason::FromSource
            })
            .unwrap(),
            serde_json::json!({ "state": "unsupported", "reason": "fromSource" })
        );
        assert_eq!(
            release_notes_url("0.2.0"),
            "https://github.com/jiangzhuo/halokey/releases/tag/v0.2.0"
        );
    }
}
