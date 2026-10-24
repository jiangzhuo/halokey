//! macOS：用 CGEvent 合成键盘、鼠标事件。需要"辅助功能"权限。
//!
//! 修饰键按住期间，后续普通键的事件要带上对应的 flags，否则系统不把它们当组合键。
//! 没有辅助功能权限时 CGEvent 会被系统静默丢弃，所以 [`CgEventInjector::open`] 先检查权限，
//! 没有就弹出系统授权提示并报错，而不是装作成功。

// 只有调用 AXIsProcessTrustedWithOptions 这一处 FFI 需要 unsafe。
#![allow(unsafe_code)]

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGMouseButton, ScrollEventUnit,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use halokey_host::KeyEntry;
use halokey_proto::MouseCode;

use crate::hid_usage::macos_keycode;
use crate::{press_order, Error, Injector, MouseButton};

/// CGEvent 注入器。
pub struct CgEventInjector {
    source: CGEventSource,
    held_flags: CGEventFlags,
    buttons: u8,
}

impl std::fmt::Debug for CgEventInjector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CgEventInjector")
    }
}

fn io_err(what: &str) -> Error {
    Error::Io(std::io::Error::other(what.to_string()))
}

fn modifier_flag(usage: u8) -> CGEventFlags {
    match usage {
        0xE0 | 0xE4 => CGEventFlags::CGEventFlagControl,
        0xE1 | 0xE5 => CGEventFlags::CGEventFlagShift,
        0xE2 | 0xE6 => CGEventFlags::CGEventFlagAlternate,
        0xE3 | 0xE7 => CGEventFlags::CGEventFlagCommand,
        0x01 => CGEventFlags::CGEventFlagSecondaryFn,
        _ => CGEventFlags::CGEventFlagNull,
    }
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    static kAXTrustedCheckOptionPrompt: CFStringRef;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
}

/// 当前进程是否已有辅助功能权限；`prompt` 为真且没有权限时，系统会弹出授权提示。
pub fn accessibility_trusted(prompt: bool) -> bool {
    // SAFETY：kAXTrustedCheckOptionPrompt 是框架导出的常量 CFString，按 get rule 包装不转移所有权。
    let key = unsafe { CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt) };
    let options = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::from(prompt))]);
    // SAFETY：传入的字典在调用期间一直存活。
    unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) }
}

impl CgEventInjector {
    /// 检查辅助功能权限并创建事件源。没有权限时弹出系统提示并返回错误。
    pub fn open() -> Result<CgEventInjector, Error> {
        if !accessibility_trusted(true) {
            return Err(io_err(
                "没有辅助功能权限：请在「系统设置 → 隐私与安全性 → 辅助功能」里允许 halokey，然后重启 halokey",
            ));
        }
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|()| io_err("无法创建 CGEventSource"))?;
        Ok(CgEventInjector {
            source,
            held_flags: CGEventFlags::CGEventFlagNull,
            buttons: 0,
        })
    }

    fn key(&mut self, entry: KeyEntry, down: bool) -> Result<(), Error> {
        let code = macos_keycode(entry.code).ok_or(Error::UnmappedKey(entry.code))?;
        let flag = modifier_flag(entry.code);
        if down {
            self.held_flags |= flag;
        } else {
            self.held_flags &= !flag;
        }
        let event = CGEvent::new_keyboard_event(self.source.clone(), code, down)
            .map_err(|()| io_err("无法创建键盘事件"))?;
        // F 键要带 function 标志；修饰键本身用 flagsChanged 语义由系统处理。
        let mut flags = self.held_flags;
        if matches!(entry.code, 0x3A..=0x45 | 0x68..=0x73) {
            flags |= CGEventFlags::CGEventFlagSecondaryFn;
        }
        event.set_flags(flags);
        event.post(CGEventTapLocation::HID);
        Ok(())
    }

    fn location(&self) -> Result<CGPoint, Error> {
        let probe = CGEvent::new(self.source.clone()).map_err(|()| io_err("无法读取指针位置"))?;
        Ok(probe.location())
    }

    fn mouse(&mut self, button: MouseButton, down: bool) -> Result<(), Error> {
        let (kind, cg_button) = match (button, down) {
            (1, true) => (CGEventType::LeftMouseDown, CGMouseButton::Left),
            (1, false) => (CGEventType::LeftMouseUp, CGMouseButton::Left),
            (2, true) => (CGEventType::RightMouseDown, CGMouseButton::Right),
            (2, false) => (CGEventType::RightMouseUp, CGMouseButton::Right),
            (3..=5, true) => (CGEventType::OtherMouseDown, CGMouseButton::Center),
            (3..=5, false) => (CGEventType::OtherMouseUp, CGMouseButton::Center),
            _ => return Err(Error::UnmappedKey(button)),
        };
        let point = self.location()?;
        let event = CGEvent::new_mouse_event(self.source.clone(), kind, point, cg_button)
            .map_err(|()| io_err("无法创建鼠标事件"))?;
        if button >= 3 {
            // 按钮 3–5 走 OtherMouse，按钮编号字段从 2 起（0 左、1 右、2 中）。
            event.set_integer_value_field(
                core_graphics::event::EventField::MOUSE_EVENT_BUTTON_NUMBER,
                i64::from(button) - 1,
            );
        }
        event.post(CGEventTapLocation::HID);
        if down {
            self.buttons |= 1 << (button - 1);
        } else {
            self.buttons &= !(1 << (button - 1));
        }
        Ok(())
    }
}

impl Injector for CgEventInjector {
    fn press(&mut self, entries: &[KeyEntry]) -> Result<(), Error> {
        for entry in press_order(entries) {
            match entry.mouse_code() {
                Some(MouseCode::Button(n)) => self.mouse(n, true)?,
                Some(MouseCode::WheelUp) => self.wheel(1, 0)?,
                Some(MouseCode::WheelDown) => self.wheel(-1, 0)?,
                None if entry.kind == 0x02 && entry.code != 0 => self.key(entry, true)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn release(&mut self, entries: &[KeyEntry]) -> Result<(), Error> {
        for entry in press_order(entries).into_iter().rev() {
            match entry.mouse_code() {
                Some(MouseCode::Button(n)) => self.mouse(n, false)?,
                None if entry.kind == 0x02 && entry.code != 0 => self.key(entry, false)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn mouse_button(&mut self, button: MouseButton, down: bool) -> Result<(), Error> {
        self.mouse(button, down)
    }

    fn wheel(&mut self, vertical: i32, horizontal: i32) -> Result<(), Error> {
        let event = CGEvent::new_scroll_event(
            self.source.clone(),
            ScrollEventUnit::LINE,
            2,
            vertical,
            horizontal,
            0,
        )
        .map_err(|()| io_err("无法创建滚轮事件"))?;
        event.post(CGEventTapLocation::HID);
        Ok(())
    }

    fn pointer_move(&mut self, dx: i32, dy: i32) -> Result<(), Error> {
        let current = self.location()?;
        let point = CGPoint::new(current.x + f64::from(dx), current.y + f64::from(dy));
        let kind = if self.buttons & 1 != 0 {
            CGEventType::LeftMouseDragged
        } else {
            CGEventType::MouseMoved
        };
        let event = CGEvent::new_mouse_event(self.source.clone(), kind, point, CGMouseButton::Left)
            .map_err(|()| io_err("无法创建移动事件"))?;
        event.post(CGEventTapLocation::HID);
        Ok(())
    }
}
