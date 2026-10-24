//! 应用内更新的状态与纯逻辑（检查循环、通知、安装流程在上层调用）。

use std::path::Path;
use std::time::Duration;

use serde::Serialize;
use tauri::utils::config::BundleType;

/// 更新状态；序列化后直接给前端（`state` 字段区分）。
#[allow(dead_code)] // 部分变体由下一个任务的检查/安装流程构造
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
#[allow(dead_code)] // 下一个任务的后台检查循环使用
pub const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);
/// 两次后台检查的间隔。
#[allow(dead_code)] // 下一个任务的后台检查循环使用
pub const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// 当前运行方式是否支持更新；`None` 表示支持。
/// macOS 的 `bundle_type()` 恒为 `App`，所以还要看可执行文件是否真在 `.app` 里。
#[allow(dead_code)] // 下一个任务使用
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
#[allow(dead_code)] // 下一个任务使用
pub fn should_notify(available: &str, notified: Option<&str>) -> bool {
    notified != Some(available)
}

/// 检查失败后的状态：后台检查静默回到 `Idle`，手动检查才报错。
#[allow(dead_code)] // 下一个任务使用
pub fn after_check_error(manual: bool, message: String) -> UpdateState {
    if manual {
        UpdateState::Error { message }
    } else {
        UpdateState::Idle
    }
}

/// 只有发现新版本之后才能开始安装。
#[allow(dead_code)] // 下一个任务使用
pub fn can_install(state: &UpdateState) -> bool {
    matches!(state, UpdateState::Available { .. })
}

/// 安装出错后的状态：用户取消系统授权不算错误。
#[allow(dead_code)] // 下一个任务使用
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
#[allow(dead_code)] // 下一个任务使用
pub fn release_notes_url(version: &str) -> String {
    format!("https://github.com/jiangzhuo/halokey/releases/tag/v{version}")
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
