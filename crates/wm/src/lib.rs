//! 切换 / 启动应用的平台后端，实现 `halokey-host` 的 [`Launcher`]。
//!
//! Linux：X11 EWMH（`_NET_CLIENT_LIST` / `WM_CLASS` / `_NET_ACTIVE_WINDOW`）；Wayland 没有
//! 通用协议，之后按合成器适配。macOS（NSWorkspace）与 Windows（Win32）后端尚未实现。
//!
//! Linux 上 [`ApplicationTarget`] 的字段含义：`bundle_identifier` = WM_CLASS 的 class 或 instance
//! （不区分大小写），也用作 `gtk-launch` 的桌面文件 ID；`path` = 可执行文件路径；`name` 只做显示。

#![forbid(unsafe_code)]

use halokey_host::{Activation, ApplicationTarget, Launcher};

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
