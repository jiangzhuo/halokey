//! 用真机证据日志做回归：olanzi `docs/evidence/2026-10-16-linux-hidraw-mouse-type.log`。

use halokey_proto::keyconfig::{self, ACCESS_READ_REPLY};
use halokey_proto::{frame, status, Control, EntryKind, KeyEntry, MouseCode, VendorKeyEvent};

fn hex(text: &str) -> Vec<u8> {
    text.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).unwrap())
        .collect()
}

#[test]
fn six_readbacks_from_linux_hidraw() {
    let samples = [
        ("81 06 50 11 00 01 01 02 09", 0, EntryKind::Keyboard, 0x09),
        ("81 06 50 11 01 01 01 02 28", 1, EntryKind::Keyboard, 0x28),
        ("81 06 50 11 02 01 01 02 2b", 2, EntryKind::Keyboard, 0x2B),
        ("81 06 50 11 03 01 01 02 2a", 3, EntryKind::Keyboard, 0x2A),
        ("81 06 50 11 04 01 01 07 06", 4, EntryKind::Mouse, 0x06),
        ("81 06 50 11 05 01 01 07 05", 5, EntryKind::Mouse, 0x05),
    ];
    for (text, index, kind, code) in samples {
        let plaintext = hex(text);
        assert!(frame::is_reply(&plaintext));
        assert_eq!(frame::cmd(&plaintext), Some(1));
        assert!(keyconfig::matches(&plaintext, ACCESS_READ_REPLY, index));
        let reply = keyconfig::parse_read_reply(&plaintext).unwrap();
        assert_eq!(reply.index, index);
        assert_eq!(
            reply.entries,
            vec![KeyEntry {
                kind,
                code,
                sign: false
            }]
        );
        assert!(Control::from_index(index).is_ok());
    }
    let knob_right = keyconfig::parse_read_reply(&hex("81 06 50 11 04 01 01 07 06")).unwrap();
    assert_eq!(
        knob_right.entries[0].mouse_code(),
        Some(MouseCode::WheelDown)
    );
}

#[test]
fn mouse_page_write_requests_and_acks() {
    assert_eq!(
        keyconfig::write_request(0, &[KeyEntry::mouse(MouseCode::Button(4))]).unwrap(),
        hex("01 06 50 04 00 01 01 07 03")
    );
    assert_eq!(
        keyconfig::write_request(0, &[KeyEntry::mouse(MouseCode::Button(5))]).unwrap(),
        hex("01 06 50 04 00 01 01 07 04")
    );
    assert_eq!(
        keyconfig::write_request(0, &[KeyEntry::keyboard(0x09)]).unwrap(),
        hex("01 06 50 04 00 01 01 02 09")
    );
    for ack in [
        "81 06 50 14 00 01 01 07 03",
        "81 06 50 14 00 01 01 07 04",
        "81 06 50 14 00 01 01 02 09",
    ] {
        assert!(keyconfig::is_write_ack(&hex(ack), 0));
        assert!(!keyconfig::is_write_ack(&hex(ack), 1));
    }
    // 按下时接口 2 的鼠标报文按钮字节：07 03 → 0x08，07 04 → 0x10。
    assert_eq!(MouseCode::Button(4).report_bit(), Some(0x08));
    assert_eq!(MouseCode::Button(5).report_bit(), Some(0x10));
}

#[test]
fn request_encodes_to_wire_and_decodes_back() {
    let request = keyconfig::read_request(0).unwrap();
    let wire = frame::encode_request(&request).unwrap();
    assert_eq!(wire.len(), 64);
    assert_eq!(wire[0], frame::REPORT_ID);
    let decoded = frame::decode_report(&wire).unwrap();
    assert_eq!(&decoded[..5], &hex("01 06 50 01 00")[..]);
    assert!(decoded[5..].iter().all(|b| *b == 0));
}

#[test]
fn vendor_events_for_all_six_controls_from_linux_monitor() {
    // docs/evidence/2026-10-16-linux-vendor-events.log
    let samples = [
        ("8b 10 6f 01 00", Control::Key1, true, 0x6f),
        ("8b 10 6f 00 00", Control::Key1, false, 0x6f),
        ("8b 10 70 01 01", Control::Key2, true, 0x70),
        ("8b 10 71 00 02", Control::Key3, false, 0x71),
        ("8b 10 6e 01 03", Control::KnobPress, true, 0x6e),
        ("8b 10 72 01 04", Control::KnobRight, true, 0x72),
        ("8b 10 73 01 05", Control::KnobLeft, true, 0x73),
    ];
    for (text, control, pressed, action) in samples {
        let plaintext = hex(text);
        assert_eq!(frame::cmd(&plaintext), Some(0x0B));
        assert_eq!(
            VendorKeyEvent::decode(&plaintext),
            Some(VendorKeyEvent {
                control,
                pressed,
                action
            })
        );
        assert!(!status::is_battery_notice(&plaintext));
    }
    // 心跳回显不是按键事件，也不是回复。
    let echo = hex("06 01 23 00 01");
    assert_eq!(VendorKeyEvent::decode(&echo), None);
    assert!(!frame::is_reply(&echo));
    assert_eq!(echo, status::HEARTBEAT);
    // 同一监听里的电量广播与 `status` 读到的 60% / 3791 mV 一致。
    let notice = status::parse_battery_notice(&hex("0b 7b d0 0e 00 3c c6 01")).unwrap();
    assert_eq!((notice.millivolts, notice.percent), (3792, Some(60)));
}
