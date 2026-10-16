//! TEA / ECB / 8 字节分组 / 32 轮（不是 XTEA）。
//!
//! [CONFIRMED] 密钥、delta 与自检向量 `TEA_Enc(00×8) == 38 90 c4 99 a3 60 aa ad`
//! 来自 Studio 的 `kwdm.dylib`，并在 macOS 与 Linux 真机上验证。分组内两个 32 位半字
//! 按小端读写，所有加减都是环绕语义。

/// 密钥的原始字节，按小端拆成四个 32 位字即为 [`KEY`]。
pub const KEY_BYTES: [u8; 16] = [
    0xca, 0xba, 0xa5, 0xca, 0x6d, 0x8a, 0x2a, 0xbc, 0xba, 0x9e, 0x5a, 0xca, 0xca, 0x8b, 0xb8, 0x9b,
];
/// 四个密钥字。
pub const KEY: [u32; 4] = [0xCAA5_BACA, 0xBC2A_8A6D, 0xCA5A_9EBA, 0x9BB8_8BCA];
/// 标准 TEA delta。
pub const DELTA: u32 = 0x9E37_79B9;
/// 轮数。
pub const ROUNDS: u32 = 32;
/// 解密时 sum 的起始值，等于 `DELTA * ROUNDS`（环绕）。
pub const DECRYPT_SUM: u32 = 0xC6EF_3720;

fn split(block: &[u8; 8]) -> (u32, u32) {
    (
        u32::from_le_bytes([block[0], block[1], block[2], block[3]]),
        u32::from_le_bytes([block[4], block[5], block[6], block[7]]),
    )
}

fn join(block: &mut [u8; 8], v0: u32, v1: u32) {
    block[..4].copy_from_slice(&v0.to_le_bytes());
    block[4..].copy_from_slice(&v1.to_le_bytes());
}

/// 原地加密一个 8 字节分组。
pub fn encrypt_block(block: &mut [u8; 8]) {
    let (mut v0, mut v1) = split(block);
    let mut sum: u32 = 0;
    for _ in 0..ROUNDS {
        sum = sum.wrapping_add(DELTA);
        v0 = v0.wrapping_add(
            (v1 << 4).wrapping_add(KEY[0]) ^ v1.wrapping_add(sum) ^ (v1 >> 5).wrapping_add(KEY[1]),
        );
        v1 = v1.wrapping_add(
            (v0 << 4).wrapping_add(KEY[2]) ^ v0.wrapping_add(sum) ^ (v0 >> 5).wrapping_add(KEY[3]),
        );
    }
    join(block, v0, v1);
}

/// 原地解密一个 8 字节分组。
pub fn decrypt_block(block: &mut [u8; 8]) {
    let (mut v0, mut v1) = split(block);
    let mut sum: u32 = DECRYPT_SUM;
    for _ in 0..ROUNDS {
        v1 = v1.wrapping_sub(
            (v0 << 4).wrapping_add(KEY[2]) ^ v0.wrapping_add(sum) ^ (v0 >> 5).wrapping_add(KEY[3]),
        );
        v0 = v0.wrapping_sub(
            (v1 << 4).wrapping_add(KEY[0]) ^ v1.wrapping_add(sum) ^ (v1 >> 5).wrapping_add(KEY[1]),
        );
        sum = sum.wrapping_sub(DELTA);
    }
    join(block, v0, v1);
}

/// 原地加密所有完整分组；不足 8 字节的尾部原样保留。
pub fn encrypt_in_place(data: &mut [u8]) {
    for block in data.as_chunks_mut::<8>().0 {
        encrypt_block(block);
    }
}

/// 原地解密所有完整分组；不足 8 字节的尾部原样保留。
pub fn decrypt_in_place(data: &mut [u8]) {
    for block in data.as_chunks_mut::<8>().0 {
        decrypt_block(block);
    }
}

/// 返回加密后的副本，见 [`encrypt_in_place`]。
pub fn encrypt(data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    encrypt_in_place(&mut out);
    out
}

/// 返回解密后的副本，见 [`decrypt_in_place`]。
pub fn decrypt(data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    decrypt_in_place(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_words_are_little_endian_view_of_key_bytes() {
        for (i, word) in KEY.iter().enumerate() {
            let bytes: [u8; 4] = KEY_BYTES[i * 4..i * 4 + 4].try_into().unwrap();
            assert_eq!(*word, u32::from_le_bytes(bytes));
        }
        assert_eq!(DECRYPT_SUM, DELTA.wrapping_mul(ROUNDS));
    }

    #[test]
    fn confirmed_zero_vector() {
        let ciphertext = [0x38, 0x90, 0xC4, 0x99, 0xA3, 0x60, 0xAA, 0xAD];
        assert_eq!(encrypt(&[0; 8]), ciphertext);
        assert_eq!(decrypt(&ciphertext), [0; 8]);
    }

    #[test]
    fn round_trip_keeps_partial_tail_untouched() {
        let input: Vec<u8> = (0..67).collect();
        let encrypted = encrypt(&input);
        assert_eq!(&encrypted[64..], &[64, 65, 66]);
        assert_ne!(&encrypted[..64], &input[..64]);
        assert_eq!(decrypt(&encrypted), input);
    }
}
