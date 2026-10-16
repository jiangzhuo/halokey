//! Windows：用 `SendInput` 合成键盘、鼠标事件。不需要额外权限（UIPI 限制除外）。

#![allow(unsafe_code)]

use halokey_host::KeyEntry;
use halokey_proto::MouseCode;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN,
    MOUSEEVENTF_XUP, MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY,
};

use crate::hid_usage::windows_vk;
use crate::{press_order, Error, Injector, MouseButton};

/// 一格滚轮的增量。
const WHEEL_DELTA: i32 = 120;

/// SendInput 注入器。
#[derive(Debug, Default)]
pub struct SendInputInjector;

impl SendInputInjector {
    /// 无需初始化。
    pub fn open() -> Result<SendInputInjector, Error> {
        Ok(SendInputInjector)
    }

    fn send(inputs: &[INPUT]) -> Result<(), Error> {
        if inputs.is_empty() {
            return Ok(());
        }
        // SAFETY: inputs 是有效的 INPUT 数组，长度与 cbSize 一致。
        let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent as usize != inputs.len() {
            return Err(Error::Io(std::io::Error::last_os_error()));
        }
        Ok(())
    }

    fn key_input(entry: KeyEntry, down: bool) -> Result<INPUT, Error> {
        let (vk, extended) = windows_vk(entry.code).ok_or(Error::UnmappedKey(entry.code))?;
        let mut flags = KEYBD_EVENT_FLAGS(0);
        if extended {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        if !down {
            flags |= KEYEVENTF_KEYUP;
        }
        Ok(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        })
    }

    fn mouse_input(flags: MOUSE_EVENT_FLAGS, data: u32, dx: i32, dy: i32) -> INPUT {
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    mouseData: data,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    fn button_input(button: MouseButton, down: bool) -> Result<INPUT, Error> {
        let (flags, data) = match (button, down) {
            (1, true) => (MOUSEEVENTF_LEFTDOWN, 0),
            (1, false) => (MOUSEEVENTF_LEFTUP, 0),
            (2, true) => (MOUSEEVENTF_RIGHTDOWN, 0),
            (2, false) => (MOUSEEVENTF_RIGHTUP, 0),
            (3, true) => (MOUSEEVENTF_MIDDLEDOWN, 0),
            (3, false) => (MOUSEEVENTF_MIDDLEUP, 0),
            (4, true) => (MOUSEEVENTF_XDOWN, 1),
            (4, false) => (MOUSEEVENTF_XUP, 1),
            (5, true) => (MOUSEEVENTF_XDOWN, 2),
            (5, false) => (MOUSEEVENTF_XUP, 2),
            _ => return Err(Error::UnmappedKey(button)),
        };
        Ok(Self::mouse_input(flags, data, 0, 0))
    }

    fn entries(entries: &[KeyEntry], down: bool) -> Result<Vec<INPUT>, Error> {
        let mut ordered = press_order(entries);
        if !down {
            ordered.reverse();
        }
        let mut inputs = Vec::new();
        for entry in ordered {
            match entry.mouse_code() {
                Some(MouseCode::Button(n)) => inputs.push(Self::button_input(n, down)?),
                Some(MouseCode::WheelUp) if down => inputs.push(Self::mouse_input(
                    MOUSEEVENTF_WHEEL,
                    WHEEL_DELTA as u32,
                    0,
                    0,
                )),
                Some(MouseCode::WheelDown) if down => inputs.push(Self::mouse_input(
                    MOUSEEVENTF_WHEEL,
                    (-WHEEL_DELTA) as u32,
                    0,
                    0,
                )),
                Some(_) => {}
                None if entry.kind == 0x02 && entry.code != 0 => {
                    inputs.push(Self::key_input(entry, down)?)
                }
                None => {}
            }
        }
        Ok(inputs)
    }
}

impl Injector for SendInputInjector {
    fn press(&mut self, entries: &[KeyEntry]) -> Result<(), Error> {
        Self::send(&Self::entries(entries, true)?)
    }

    fn release(&mut self, entries: &[KeyEntry]) -> Result<(), Error> {
        Self::send(&Self::entries(entries, false)?)
    }

    fn mouse_button(&mut self, button: MouseButton, down: bool) -> Result<(), Error> {
        Self::send(&[Self::button_input(button, down)?])
    }

    fn wheel(&mut self, vertical: i32, horizontal: i32) -> Result<(), Error> {
        let mut inputs = Vec::new();
        if vertical != 0 {
            inputs.push(Self::mouse_input(
                MOUSEEVENTF_WHEEL,
                (vertical * WHEEL_DELTA) as u32,
                0,
                0,
            ));
        }
        if horizontal != 0 {
            inputs.push(Self::mouse_input(
                MOUSEEVENTF_HWHEEL,
                (horizontal * WHEEL_DELTA) as u32,
                0,
                0,
            ));
        }
        Self::send(&inputs)
    }

    fn pointer_move(&mut self, dx: i32, dy: i32) -> Result<(), Error> {
        Self::send(&[Self::mouse_input(MOUSEEVENTF_MOVE, 0, dx, dy)])
    }
}
