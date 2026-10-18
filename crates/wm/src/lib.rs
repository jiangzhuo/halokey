//! 切换 / 启动应用的平台后端，实现 `halokey-host` 的 [`Launcher`]。
//!
//! Linux：X11 EWMH（`_NET_CLIENT_LIST` / `WM_CLASS` / `_NET_ACTIVE_WINDOW`）；Wayland 没有
//! 通用协议，之后按合成器适配。macOS（NSWorkspace + `open`）与 Windows（Win32）后端只经编译检查。
//!
//! Linux 上 [`ApplicationTarget`] 的字段含义：`bundle_identifier` = WM_CLASS 的 class 或 instance
//! （不区分大小写），也用作 `gtk-launch` 的桌面文件 ID；`path` = 可执行文件路径；`name` 只做显示。

#![deny(unsafe_code)]

use halokey_host::{Activation, ApplicationTarget, Launcher};
use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod x11;

/// 任何启动都立即失败、永远不在前台的启动器，用于没有窗口管理后端的环境。
#[derive(Debug, Default)]
pub struct Unavailable {
    /// 原因，用于日志。
    pub reason: String,
}

impl Launcher for Unavailable {
    fn launch(&mut self, _target: &ApplicationTarget) -> Activation {
        let a = Activation::pending();
        a.complete(false);
        a
    }
    fn is_frontmost(&mut self, _target: &ApplicationTarget) -> bool {
        false
    }
}

/// 打开当前平台的默认后端；失败时返回 [`Unavailable`] 并带原因，不让守护进程起不来。
pub fn open_default() -> Box<dyn Launcher> {
    #[cfg(target_os = "linux")]
    {
        match x11::X11Launcher::connect() {
            Ok(l) => Box::new(l),
            Err(e) => Box::new(Unavailable {
                reason: e.to_string(),
            }),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        Box::new(Unavailable {
            reason: "此平台的切换应用后端尚未实现".into(),
        })
    }
}

/// 当前打开的一个顶层窗口，供 APP 编辑器挑选。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    /// WM_CLASS 的 class（Linux 上写进 `bundleIdentifier`）。
    pub class: String,
    /// WM_CLASS 的 instance。
    pub instance: String,
    /// `_NET_WM_NAME`，退化到 `WM_NAME`；拿不到为空。
    pub title: String,
    /// `/proc/<pid>/exe`，拿不到为空。
    pub exe: String,
}

impl WindowInfo {
    /// 去掉没有 class 的，按 class 去重（保留先出现的标题），按 class 不区分大小写排序。
    pub fn dedup_sorted(items: Vec<WindowInfo>) -> Vec<WindowInfo> {
        let mut seen = std::collections::HashSet::new();
        let mut out: Vec<WindowInfo> = items
            .into_iter()
            .filter(|w| !w.class.is_empty())
            .filter(|w| seen.insert(w.class.to_ascii_lowercase()))
            .collect();
        out.sort_by_key(|w| w.class.to_ascii_lowercase());
        out
    }
}

/// 列出当前顶层窗口；没有后端时返回原因（Wayland、无 X 连接、非 Linux）。
pub fn list_windows() -> Result<Vec<WindowInfo>, String> {
    #[cfg(target_os = "linux")]
    {
        x11::list_windows().map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("此平台还没有窗口列表".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_sorted_keeps_first_title_per_class_and_sorts() {
        let w = |class: &str, title: &str| WindowInfo {
            class: class.into(),
            instance: class.to_ascii_lowercase(),
            title: title.into(),
            exe: String::new(),
        };
        let out = WindowInfo::dedup_sorted(vec![
            w("Zed", "z1"),
            w("chrome", "c1"),
            w("Zed", "z2"),
            w("", "no class"),
        ]);
        assert_eq!(out.len(), 2);
        assert_eq!((out[0].class.as_str(), out[0].title.as_str()), ("chrome", "c1"));
        assert_eq!((out[1].class.as_str(), out[1].title.as_str()), ("Zed", "z1"));
    }
}
