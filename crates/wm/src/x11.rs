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
