//! Linux：通过 `/dev/uinput` 创建一个虚拟键盘 + 鼠标，X11 与 Wayland 都认。
//!
//! 需要 `/dev/uinput` 的读写权限，见 `packaging/linux/70-halokey.rules`。

use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, EventType, InputEvent, KeyCode, RelativeAxisCode};
use halokey_host::KeyEntry;
use halokey_proto::MouseCode;

use crate::hid_usage::linux_keycode;
use crate::{press_order, Error, Injector, MouseButton};

/// uinput 注入器。
pub struct UinputInjector {
    device: VirtualDevice,
}

impl std::fmt::Debug for UinputInjector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UinputInjector")
    }
}

fn button_code(button: MouseButton) -> Option<KeyCode> {
    Some(match button {
        1 => KeyCode::BTN_LEFT,
        2 => KeyCode::BTN_RIGHT,
        3 => KeyCode::BTN_MIDDLE,
        4 => KeyCode::BTN_SIDE,
        5 => KeyCode::BTN_EXTRA,
        _ => return None,
    })
}

impl UinputInjector {
    /// 创建名为 `halokey` 的虚拟设备，注册全部可映射按键、五个鼠标键与四个相对轴。
    pub fn open() -> Result<UinputInjector, Error> {
        let mut keys = AttributeSet::<KeyCode>::new();
        for usage in 0u8..=255 {
            if let Some(code) = linux_keycode(usage) {
                keys.insert(KeyCode::new(code));
            }
        }
        for button in 1..=5 {
            keys.insert(button_code(button).expect("1–5 都有定义"));
        }
        let mut axes = AttributeSet::<RelativeAxisCode>::new();
        for axis in [
            RelativeAxisCode::REL_X,
            RelativeAxisCode::REL_Y,
            RelativeAxisCode::REL_WHEEL,
            RelativeAxisCode::REL_HWHEEL,
        ] {
            axes.insert(axis);
        }
        let device = VirtualDevice::builder()?
            .name("halokey")
            .with_keys(&keys)?
            .with_relative_axes(&axes)?
            .build()?;
        Ok(UinputInjector { device })
    }

    /// 键盘条目变成 KEY 事件；鼠标页按键变成 BTN 事件，滚轮只在按下时产生一格 REL_WHEEL。
    fn key_events(entries: &[KeyEntry], down: bool) -> Result<Vec<InputEvent>, Error> {
        let mut ordered = press_order(entries);
        if !down {
            ordered.reverse();
        }
        let mut events = Vec::new();
        for entry in ordered.iter().filter(|e| e.kind != 0x02 || e.code != 0) {
            match entry.mouse_code() {
                Some(MouseCode::Button(n)) => {
                    let code = button_code(n).ok_or(Error::UnmappedKey(entry.code))?;
                    events.push(InputEvent::new(EventType::KEY.0, code.0, i32::from(down)));
                }
                Some(MouseCode::WheelUp) if down => {
                    events.push(InputEvent::new(
                        EventType::RELATIVE.0,
                        RelativeAxisCode::REL_WHEEL.0,
                        1,
                    ));
                }
                Some(MouseCode::WheelDown) if down => {
                    events.push(InputEvent::new(
                        EventType::RELATIVE.0,
                        RelativeAxisCode::REL_WHEEL.0,
                        -1,
                    ));
                }
                Some(_) => {}
                None => {
                    let code = linux_keycode(entry.code).ok_or(Error::UnmappedKey(entry.code))?;
                    events.push(InputEvent::new(EventType::KEY.0, code, i32::from(down)));
                }
            }
        }
        Ok(events)
    }
}

impl Injector for UinputInjector {
    fn press(&mut self, entries: &[KeyEntry]) -> Result<(), Error> {
        let events = Self::key_events(entries, true)?;
        if events.is_empty() {
            return Ok(());
        }
        Ok(self.device.emit(&events)?)
    }

    fn release(&mut self, entries: &[KeyEntry]) -> Result<(), Error> {
        let events = Self::key_events(entries, false)?;
        if events.is_empty() {
            return Ok(());
        }
        Ok(self.device.emit(&events)?)
    }

    fn mouse_button(&mut self, button: MouseButton, down: bool) -> Result<(), Error> {
        let code = button_code(button).ok_or(Error::UnmappedKey(button))?;
        Ok(self
            .device
            .emit(&[InputEvent::new(EventType::KEY.0, code.0, i32::from(down))])?)
    }

    fn wheel(&mut self, vertical: i32, horizontal: i32) -> Result<(), Error> {
        let mut events = Vec::new();
        if vertical != 0 {
            events.push(InputEvent::new(
                EventType::RELATIVE.0,
                RelativeAxisCode::REL_WHEEL.0,
                vertical,
            ));
        }
        if horizontal != 0 {
            events.push(InputEvent::new(
                EventType::RELATIVE.0,
                RelativeAxisCode::REL_HWHEEL.0,
                horizontal,
            ));
        }
        Ok(self.device.emit(&events)?)
    }

    fn pointer_move(&mut self, dx: i32, dy: i32) -> Result<(), Error> {
        let events = [
            InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_X.0, dx),
            InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_Y.0, dy),
        ];
        Ok(self.device.emit(&events)?)
    }
}
