//! Windows：枚举顶层窗口按进程可执行文件匹配，`SetForegroundWindow` 激活。
//!
//! `ApplicationTarget` 字段含义：`bundle_identifier` = 可执行文件名（不含 `.exe`，不区分大小写），
//! `path` = 完整路径，`name` = 显示名。只经编译检查，未在真机验证。

#![allow(unsafe_code)]

use std::path::Path;
use std::process::Command;
use std::time::Instant;

use halokey_host::{Activation, ApplicationTarget, Launcher};
use windows::core::BOOL;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    SetForegroundWindow, ShowWindow, SW_RESTORE,
};

/// Win32 启动器。
#[derive(Debug, Default)]
pub struct Win32Launcher {
    pending: Vec<(ApplicationTarget, Activation, Instant)>,
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: lparam 是我们传入的 Vec<HWND> 指针，枚举期间一直有效。
    let list = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
    if unsafe { IsWindowVisible(hwnd) }.as_bool() {
        list.push(hwnd);
    }
    BOOL(1)
}

fn exe_path(hwnd: HWND) -> Option<String> {
    let mut pid = 0u32;
    // SAFETY: 标准 Win32 调用；hwnd 来自 EnumWindows。
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 {
        return None;
    }
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut buffer = [0u16; 1024];
    let mut len = buffer.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut len,
        )
    };
    let _ = unsafe { CloseHandle(handle) };
    result.ok()?;
    Some(String::from_utf16_lossy(&buffer[..len as usize]))
}

fn matches(exe: &str, target: &ApplicationTarget) -> bool {
    if !target.path.is_empty() && exe.eq_ignore_ascii_case(&target.path) {
        return true;
    }
    let stem = Path::new(exe)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let wanted = if target.bundle_identifier.is_empty() {
        &target.name
    } else {
        &target.bundle_identifier
    };
    !wanted.is_empty() && stem.eq_ignore_ascii_case(wanted.trim_end_matches(".exe"))
}

impl Win32Launcher {
    /// 新建。
    pub fn new() -> Win32Launcher {
        Win32Launcher::default()
    }

    fn windows() -> Vec<HWND> {
        let mut list: Vec<HWND> = Vec::new();
        // SAFETY: 回调只在 EnumWindows 返回前使用 list。
        let _ = unsafe { EnumWindows(Some(collect), LPARAM(&mut list as *mut Vec<HWND> as isize)) };
        list
    }

    fn find_window(target: &ApplicationTarget) -> Option<HWND> {
        Self::windows()
            .into_iter()
            .find(|hwnd| exe_path(*hwnd).is_some_and(|exe| matches(&exe, target)))
    }

    fn activate(hwnd: HWND) -> bool {
        // SAFETY: 标准 Win32 调用。
        unsafe {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            SetForegroundWindow(hwnd).as_bool()
        }
    }

    fn spawn(target: &ApplicationTarget) -> bool {
        if !target.path.is_empty() {
            return Command::new(&target.path).spawn().is_ok();
        }
        if !target.bundle_identifier.is_empty() {
            return Command::new("cmd")
                .args(["/C", "start", "", &target.bundle_identifier])
                .spawn()
                .is_ok();
        }
        false
    }
}

impl Launcher for Win32Launcher {
    fn launch(&mut self, target: &ApplicationTarget) -> Activation {
        let activation = Activation::pending();
        if let Some(hwnd) = Self::find_window(target) {
            activation.complete(Self::activate(hwnd));
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
        // SAFETY: 标准 Win32 调用。
        let hwnd = unsafe { GetForegroundWindow() };
        exe_path(hwnd).is_some_and(|exe| matches(&exe, target))
    }

    fn poll(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        for (target, activation, started) in pending {
            if activation.state() != halokey_host::ActivationState::Pending {
                continue;
            }
            if let Some(hwnd) = Self::find_window(&target) {
                activation.complete(Self::activate(hwnd));
            } else if started.elapsed().as_secs_f64() > halokey_host::runner::ACTIVATION_TIMEOUT {
                activation.complete(false);
            } else {
                self.pending.push((target, activation, started));
            }
        }
    }
}
