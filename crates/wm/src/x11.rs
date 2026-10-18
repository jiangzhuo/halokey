//! X11 EWMH 后端。

use std::process::Command;
use std::time::Instant;

use halokey_host::{Activation, ApplicationTarget, Launcher};
use x11rb::connection::Connection;
use x11rb::properties::WmClass;
use x11rb::protocol::xproto::{AtomEnum, ClientMessageEvent, ConnectionExt, EventMask, Window};
use x11rb::rust_connection::RustConnection;

/// X11 启动器。
pub struct X11Launcher {
    conn: RustConnection,
    root: Window,
    net_active_window: u32,
    net_client_list: u32,
    net_wm_pid: u32,
    pending: Vec<(ApplicationTarget, Activation, Instant)>,
}

impl std::fmt::Debug for X11Launcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("X11Launcher")
            .field("pending", &self.pending.len())
            .finish()
    }
}

impl X11Launcher {
    /// 连接 `$DISPLAY`。
    pub fn connect() -> Result<X11Launcher, Box<dyn std::error::Error>> {
        let (conn, screen) = x11rb::connect(None)?;
        let root = conn.setup().roots[screen].root;
        let atom = |name: &[u8]| -> Result<u32, Box<dyn std::error::Error>> {
            Ok(conn.intern_atom(false, name)?.reply()?.atom)
        };
        let net_active_window = atom(b"_NET_ACTIVE_WINDOW")?;
        let net_client_list = atom(b"_NET_CLIENT_LIST")?;
        let net_wm_pid = atom(b"_NET_WM_PID")?;
        Ok(X11Launcher {
            conn,
            root,
            net_active_window,
            net_client_list,
            net_wm_pid,
            pending: Vec::new(),
        })
    }

    fn active_window(&self) -> Option<Window> {
        let reply = self
            .conn
            .get_property(
                false,
                self.root,
                self.net_active_window,
                AtomEnum::WINDOW,
                0,
                1,
            )
            .ok()?
            .reply()
            .ok()?;
        let window = reply.value32().and_then(|mut v| v.next())?;
        (window != 0).then_some(window)
    }

    fn client_windows(&self) -> Vec<Window> {
        self.conn
            .get_property(
                false,
                self.root,
                self.net_client_list,
                AtomEnum::WINDOW,
                0,
                4096,
            )
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().map(|v| v.collect()))
            .unwrap_or_default()
    }

    fn window_matches(&self, window: Window, target: &ApplicationTarget) -> bool {
        if !target.path.is_empty() {
            let pid = self
                .conn
                .get_property(false, window, self.net_wm_pid, AtomEnum::CARDINAL, 0, 1)
                .ok()
                .and_then(|c| c.reply().ok())
                .and_then(|r| r.value32().and_then(|mut v| v.next()));
            if let Some(pid) = pid {
                if let Ok(exe) = std::fs::read_link(format!("/proc/{pid}/exe")) {
                    if exe == std::path::Path::new(&target.path) {
                        return true;
                    }
                }
            }
        }
        let class = match WmClass::get(&self.conn, window) {
            Ok(cookie) => match cookie.reply() {
                Ok(Some(class)) => class,
                _ => return false,
            },
            Err(_) => return false,
        };
        let wanted = if target.bundle_identifier.is_empty() {
            &target.name
        } else {
            &target.bundle_identifier
        };
        let wanted = wanted.to_ascii_lowercase();
        let same = |bytes: &[u8]| String::from_utf8_lossy(bytes).to_ascii_lowercase() == wanted;
        same(class.class()) || same(class.instance())
    }

    fn find_window(&self, target: &ApplicationTarget) -> Option<Window> {
        self.client_windows()
            .into_iter()
            .rev()
            .find(|w| self.window_matches(*w, target))
    }

    fn activate(&self, window: Window) -> bool {
        // source indication 2 = 应用请求；窗口管理器据此允许抢焦点。
        let event = ClientMessageEvent::new(32, window, self.net_active_window, [2, 0, 0, 0, 0]);
        let mask = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY;
        self.conn.send_event(false, self.root, mask, event).is_ok() && self.conn.flush().is_ok()
    }

    fn spawn(target: &ApplicationTarget) -> bool {
        if !target.path.is_empty() {
            return Command::new(&target.path).spawn().is_ok();
        }
        if !target.bundle_identifier.is_empty() && !target.bundle_identifier.contains('/') {
            return Command::new("gtk-launch")
                .arg(&target.bundle_identifier)
                .spawn()
                .is_ok();
        }
        false
    }
}

impl Launcher for X11Launcher {
    fn launch(&mut self, target: &ApplicationTarget) -> Activation {
        let activation = Activation::pending();
        if let Some(window) = self.find_window(target) {
            activation.complete(self.activate(window));
            return activation;
        }
        if Self::spawn(target) {
            self.pending
                .push((target.clone(), activation.clone(), Instant::now()));
        } else {
            activation.complete(false);
        }
        activation
    }

    fn is_frontmost(&mut self, target: &ApplicationTarget) -> bool {
        self.active_window()
            .is_some_and(|w| self.window_matches(w, target))
    }

    /// 为刚启动的应用寻找窗口并激活；超过执行器的等待上限后放弃。
    fn poll(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        for (target, activation, started) in pending {
            if activation.state() != halokey_host::ActivationState::Pending {
                continue;
            }
            if let Some(window) = self.find_window(&target) {
                activation.complete(self.activate(window));
            } else if started.elapsed().as_secs_f64() > halokey_host::runner::ACTIVATION_TIMEOUT {
                activation.complete(false);
            } else {
                self.pending.push((target, activation, started));
            }
        }
    }
}

/// 枚举 `_NET_CLIENT_LIST` 里的窗口：WM_CLASS、标题、可执行文件。独立短连接，不碰守护线程里的启动器。
pub fn list_windows() -> Result<Vec<crate::WindowInfo>, Box<dyn std::error::Error>> {
    let (conn, screen) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen].root;
    let atom = |name: &[u8]| -> Result<u32, Box<dyn std::error::Error>> {
        Ok(conn.intern_atom(false, name)?.reply()?.atom)
    };
    let net_client_list = atom(b"_NET_CLIENT_LIST")?;
    let net_wm_name = atom(b"_NET_WM_NAME")?;
    let utf8_string = atom(b"UTF8_STRING")?;
    let net_wm_pid = atom(b"_NET_WM_PID")?;
    let windows: Vec<Window> = conn
        .get_property(false, root, net_client_list, AtomEnum::WINDOW, 0, 4096)?
        .reply()?
        .value32()
        .map(|v| v.collect())
        .unwrap_or_default();
    let text = |window: Window, property: u32, kind: u32| -> Option<String> {
        conn.get_property(false, window, property, kind, 0, 1024)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| String::from_utf8_lossy(&r.value).into_owned())
            .filter(|s| !s.is_empty())
    };
    let mut out = Vec::new();
    for w in windows {
        let Some(class) = WmClass::get(&conn, w).ok().and_then(|c| c.reply().ok()).flatten() else {
            continue;
        };
        let title = text(w, net_wm_name, utf8_string)
            .or_else(|| text(w, AtomEnum::WM_NAME.into(), AtomEnum::STRING.into()))
            .unwrap_or_default();
        let exe = conn
            .get_property(false, w, net_wm_pid, AtomEnum::CARDINAL, 0, 1)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().and_then(|mut v| v.next()))
            .and_then(|pid| std::fs::read_link(format!("/proc/{pid}/exe")).ok())
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        out.push(crate::WindowInfo {
            class: String::from_utf8_lossy(class.class()).into_owned(),
            instance: String::from_utf8_lossy(class.instance()).into_owned(),
            title,
            exe,
        });
    }
    Ok(crate::WindowInfo::dedup_sorted(out))
}

#[cfg(test)]
mod tests {
    /// 需要真实 X 连接；`cargo test -p halokey-wm -- --ignored --nocapture` 手动跑。
    #[test]
    #[ignore]
    fn lists_real_windows() {
        let windows = super::list_windows().expect("X11 connection");
        for w in &windows {
            eprintln!("{:<32} {:<40} {}", w.class, w.title, w.exe);
        }
        assert!(windows.iter().all(|w| !w.class.is_empty()));
    }
}
