use core::fmt;

use halokey_i18n::tr;

/// 协议层错误。只描述"帧不合法"这一类事实，不包含传输错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// 报文长度或 report ID 不符合厂商通道格式。
    InvalidFrame,
    /// 请求超过 64 字节明文上限。
    RequestTooLong,
    /// 按键配置的条目数量或布局不合法。
    InvalidEntries,
    /// 控件编号超出可写范围。
    InvalidControl,
    /// 键码不在已确认可用的范围内。
    UnsupportedKey,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Error::InvalidFrame => tr!(
                "设备报文格式无效",
                "Invalid device frame",
                "デバイスのフレーム形式が不正です"
            ),
            Error::RequestTooLong => tr!(
                "请求超过 64 字节明文上限",
                "Request exceeds the 64-byte plaintext limit",
                "リクエストが平文 64 バイトの上限を超えています"
            ),
            Error::InvalidEntries => tr!(
                "按键配置条目不完整或格式无效",
                "Key table entries are incomplete or invalid",
                "キー設定のエントリが不完全か不正です"
            ),
            Error::InvalidControl => tr!(
                "控件编号超出可写范围",
                "Control index is outside the writable range",
                "コントロール番号が書き込み可能な範囲外です"
            ),
            Error::UnsupportedKey => tr!(
                "此键码尚未确认可用",
                "This keycode has not been confirmed to work",
                "このキーコードは動作確認されていません"
            ),
        };
        f.write_str(&text)
    }
}

impl std::error::Error for Error {}
