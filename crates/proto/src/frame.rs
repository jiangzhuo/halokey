//! 厂商通道的帧封装。
//!
//! [CONFIRMED] 明文 64 字节；线上报文为 report ID `0x55` 加密文的前 63 字节，共 64 字节。
//! 回复同样是 64 字节带 `0x55` 前缀（Linux hidraw）或 63 字节不带前缀（macOS IOKit 回调）。
//! 回复只解密前 7 个完整分组（56 字节），末 7 字节是不可解的填充，绝不当作协议字段。

use crate::Error;

/// 厂商通道 report ID。
pub const REPORT_ID: u8 = 0x55;
/// 明文长度。
pub const PLAINTEXT_LEN: usize = 64;
/// 线上报文总长（含 report ID）。
pub const WIRE_LEN: usize = 64;
/// 线上报文中的密文字节数。
pub const CIPHERTEXT_ON_WIRE: usize = 63;
/// 回复中可解密的字节数。
pub const DECRYPTED_LEN: usize = 56;

/// 解码后的回复明文。
pub type Plaintext = [u8; DECRYPTED_LEN];

/// 把请求明文（至多 64 字节，自动补零）编码成线上的 64 字节报文。
pub fn encode_request(request: &[u8]) -> Result<[u8; WIRE_LEN], Error> {
    if request.len() > PLAINTEXT_LEN {
        return Err(Error::RequestTooLong);
    }
    let mut plaintext = [0u8; PLAINTEXT_LEN];
    plaintext[..request.len()].copy_from_slice(request);
    crate::tea::encrypt_in_place(&mut plaintext);
    let mut wire = [0u8; WIRE_LEN];
    wire[0] = REPORT_ID;
    wire[1..].copy_from_slice(&plaintext[..CIPHERTEXT_ON_WIRE]);
    Ok(wire)
}

/// 解码设备回复：接受 64 字节带 `0x55` 前缀或 63 字节不带前缀，返回解密后的前 56 字节。
pub fn decode_report(bytes: &[u8]) -> Result<Plaintext, Error> {
    let payload = match bytes.len() {
        WIRE_LEN if bytes[0] == REPORT_ID => &bytes[1..],
        CIPHERTEXT_ON_WIRE => bytes,
        _ => return Err(Error::InvalidFrame),
    };
    let mut plaintext = [0u8; DECRYPTED_LEN];
    plaintext.copy_from_slice(&payload[..DECRYPTED_LEN]);
    crate::tea::decrypt_in_place(&mut plaintext);
    Ok(plaintext)
}

/// 帧头命令号：`frame[0] & 0x1F`。`0x06` 由接收器处理，`0x01` 由 Vibe Key 本体处理。
pub fn cmd(frame: &[u8]) -> Option<u8> {
    frame.first().map(|b| b & 0x1F)
}

/// 是否为回复：`frame[0] & 0x80` 或 `frame[3] & 0x10`。
pub fn is_reply(frame: &[u8]) -> bool {
    frame.first().is_some_and(|b| b & 0x80 != 0) || frame.get(3).is_some_and(|b| b & 0x10 != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_framing_and_only_complete_blocks_decoded() {
        let request = [1, 6, 0x50, 4, 0, 1, 1, 2, 0x68];
        let report = encode_request(&request).unwrap();
        assert_eq!(report[0], REPORT_ID);
        let decoded = decode_report(&report).unwrap();
        assert_eq!(&decoded[..request.len()], &request);
        assert_eq!(decode_report(&report[1..]).unwrap(), decoded);
        assert_eq!(decode_report(&report[..62]), Err(Error::InvalidFrame));
        let mut wrong_id = report;
        wrong_id[0] = 0;
        assert_eq!(decode_report(&wrong_id), Err(Error::InvalidFrame));
        assert_eq!(encode_request(&[0; 65]), Err(Error::RequestTooLong));
        assert!(encode_request(&[0; 64]).is_ok());
    }

    #[test]
    fn header_helpers() {
        assert_eq!(cmd(&[0x81, 6, 0x50, 0x11]), Some(1));
        assert_eq!(cmd(&[0x06, 3, 0x0A, 0x11]), Some(6));
        assert_eq!(cmd(&[]), None);
        assert!(is_reply(&[0x81, 6, 0x50, 0x11]));
        assert!(is_reply(&[0x06, 3, 0x0A, 0x11]));
        assert!(!is_reply(&[0x01, 6, 0x50, 0x01]));
        assert!(!is_reply(&[]));
    }
}
