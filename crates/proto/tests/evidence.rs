//! 用真机证据日志做回归：olanzi `docs/evidence/2026-10-16-linux-hidraw-mouse-type.log`。

use halokey_proto::keyconfig::{self, ACCESS_READ_REPLY};
use halokey_proto::{frame, Control, EntryKind, KeyEntry, MouseCode};

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
