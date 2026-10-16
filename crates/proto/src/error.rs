use core::fmt;

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
            Error::InvalidFrame => "设备报文格式无效",
            Error::RequestTooLong => "请求超过 64 字节明文上限",
            Error::InvalidEntries => "按键配置条目不完整或格式无效",
            Error::InvalidControl => "控件编号超出可写范围",
            Error::UnsupportedKey => "此键码尚未确认可用",
        };
        f.write_str(text)
    }
}

impl std::error::Error for Error {}
