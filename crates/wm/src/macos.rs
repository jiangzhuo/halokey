//! macOS：NSWorkspace 查前台应用；启动 / 激活走 `open` 命令，避免在设备线程里处理 AppKit 回调。
//!
//! `ApplicationTarget` 字段含义：`bundle_identifier` = bundle ID（如 `com.apple.Safari`），
//! `path` = `.app` 路径，`name` = 显示名（`open -a` 的参数）。只经编译检查，未在真机验证。

use std::process::Command;
use std::time::Instant;

use halokey_host::{Activation, ApplicationTarget, Launcher};
use objc2_app_kit::NSWorkspace;

/// NSWorkspace 启动器。
#[derive(Debug, Default)]
pub struct WorkspaceLauncher {
    pending: Vec<(ApplicationTarget, Activation, Instant)>,
}

impl WorkspaceLauncher {
    /// 新建。
    pub fn new() -> WorkspaceLauncher {
        WorkspaceLauncher::default()
    }

    fn frontmost_matches(target: &ApplicationTarget) -> bool {
        let workspace = NSWorkspace::sharedWorkspace();
        let Some(app) = workspace.frontmostApplication() else {
            return false;
        };
        if !target.bundle_identifier.is_empty() {
            return app.bundleIdentifier().is_some_and(|id| {
                id.to_string()
                    .eq_ignore_ascii_case(&target.bundle_identifier)
            });
        }
        if !target.path.is_empty() {
            return app.bundleURL().and_then(|url| url.path()).is_some_and(|p| {
                p.to_string().trim_end_matches('/') == target.path.trim_end_matches('/')
            });
        }
        app.localizedName()
            .is_some_and(|n| n.to_string().eq_ignore_ascii_case(&target.name))
    }

    fn spawn(target: &ApplicationTarget) -> bool {
        let mut command = Command::new("open");
        if !target.path.is_empty() {
            command.arg(&target.path);
        } else if !target.bundle_identifier.is_empty() {
            command.arg("-b").arg(&target.bundle_identifier);
        } else if !target.name.is_empty() {
            command.arg("-a").arg(&target.name);
        } else {
            return false;
        }
        command.spawn().is_ok()
    }
}

impl Launcher for WorkspaceLauncher {
    fn launch(&mut self, target: &ApplicationTarget) -> Activation {
        let activation = Activation::pending();
        if Self::spawn(target) {
            self.pending
                .push((target.clone(), activation.clone(), Instant::now()));
        } else {
            activation.complete(false);
        }
        activation
    }

    fn is_frontmost(&mut self, target: &ApplicationTarget) -> bool {
        Self::frontmost_matches(target)
    }

    /// `open` 本身会把应用带到前台；这里等它真的到前台后才算完成。
    fn poll(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        for (target, activation, started) in pending {
            if activation.state() != halokey_host::ActivationState::Pending {
                continue;
            }
            if Self::frontmost_matches(&target) {
                activation.complete(true);
            } else if started.elapsed().as_secs_f64() > halokey_host::runner::ACTIVATION_TIMEOUT {
                activation.complete(false);
            } else {
                self.pending.push((target, activation, started));
            }
        }
    }
}
