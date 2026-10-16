//! AU05 厂商通道的 hidapi 传输层。
//!
//! 一个 [`Transport`] 对应接收器的接口 3（usage page `0xFFFC`）。所有方法都是同步阻塞的，
//! 设计上由一个专门的设备线程串行调用；不在回调里写设备。
//!
//! 平台后端：Linux 走纯 Rust 的 hidraw 实现（不需要 libudev），macOS 走 IOKit
//! （hidapi 按设备逐个 `IOHIDDeviceOpen`，不是全有全无的 `IOHIDManagerOpen`），
//! Windows 走原生 HID。Windows 要求输出报告与描述符等长，所以线上永远发满 64 字节。

#![forbid(unsafe_code)]

use std::ffi::CString;
use std::fmt;
use std::time::{Duration, Instant};

use halokey_proto::frame::{self, Plaintext};
use halokey_proto::{keyconfig, status, Battery, KeyConfigReply, KeyEntry};

pub use hidapi::HidApi;

/// 传输层错误。
#[derive(Debug)]
pub enum Error {
    /// hidapi 报告的错误。
    Hid(hidapi::HidError),
    /// 没有找到 AU05 接收器的厂商通道接口。
    NoDevice,
    /// 指定时间内没有收到匹配的回复。
    Timeout,
    /// 回复格式不合法。
    Protocol(halokey_proto::Error),
    /// 写入后读回的内容与写入不一致。
    ReadbackMismatch {
        /// 期望写入的条目。
        expected: Vec<KeyEntry>,
        /// 设备实际保存的条目。
        actual: Vec<KeyEntry>,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Hid(e) => write!(f, "HID 错误：{e}"),
            Error::NoDevice => f.write_str("没有找到 Vibe Key 接收器的厂商通道接口"),
            Error::Timeout => f.write_str("设备没有在限定时间内回复"),
            Error::Protocol(e) => write!(f, "协议错误：{e}"),
            Error::ReadbackMismatch { expected, actual } => {
                write!(f, "写入后读回不一致：期望 {expected:?}，实际 {actual:?}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<hidapi::HidError> for Error {
    fn from(e: hidapi::HidError) -> Self {
        Error::Hid(e)
    }
}

impl From<halokey_proto::Error> for Error {
    fn from(e: halokey_proto::Error) -> Self {
        Error::Protocol(e)
    }
}

/// 一个可打开的厂商通道接口。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceId {
    /// hidapi 设备路径（平台相关的不透明字符串）。
    pub path: CString,
    /// USB 序列号；接收器有多个时用它区分。
    pub serial: Option<String>,
    /// USB 接口号；厂商通道是 3。
    pub interface: i32,
    /// usage page；厂商通道是 `0xFFFC`。某些后端枚举不到时为 0。
    pub usage_page: u16,
}

impl DeviceId {
    fn from_info(info: &hidapi::DeviceInfo) -> DeviceId {
        DeviceId {
            path: info.path().to_owned(),
            serial: info.serial_number().map(str::to_owned),
            interface: info.interface_number(),
            usage_page: info.usage_page(),
        }
    }

    /// 是否为厂商通道：优先看 usage page，枚举不到 usage page 的后端退回接口号。
    fn is_vendor_channel(info: &hidapi::DeviceInfo) -> bool {
        info.vendor_id() == halokey_proto::VENDOR_ID
            && info.product_id() == halokey_proto::PRODUCT_ID
            && (info.usage_page() == halokey_proto::VENDOR_USAGE_PAGE
                || (info.usage_page() == 0 && info.interface_number() == 3))
    }
}

/// 列出所有 AU05 厂商通道接口。
pub fn enumerate(api: &HidApi) -> Vec<DeviceId> {
    api.device_list()
        .filter(|info| DeviceId::is_vendor_channel(info))
        .map(DeviceId::from_info)
        .collect()
}

/// 未匹配到当前请求的帧交给这个回调（厂商按键事件、通知、迟到的回复）。
pub type FrameSink = Box<dyn FnMut(&Plaintext) + Send>;

/// 打开的厂商通道。
pub struct Transport {
    device: hidapi::HidDevice,
    id: DeviceId,
    sink: Option<FrameSink>,
    heartbeats_sent: u64,
}

impl fmt::Debug for Transport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Transport")
            .field("id", &self.id)
            .field("heartbeats_sent", &self.heartbeats_sent)
            .finish_non_exhaustive()
    }
}

impl Transport {
    /// 打开第一个找到的厂商通道。
    pub fn open_first(api: &HidApi) -> Result<Transport, Error> {
        let id = enumerate(api).into_iter().next().ok_or(Error::NoDevice)?;
        Transport::open(api, &id)
    }

    /// 打开指定接口。
    pub fn open(api: &HidApi, id: &DeviceId) -> Result<Transport, Error> {
        let device = api.open_path(&id.path)?;
        device.set_blocking_mode(false)?;
        Ok(Transport {
            device,
            id: id.clone(),
            sink: None,
            heartbeats_sent: 0,
        })
    }

    /// 接口信息。
    pub fn id(&self) -> &DeviceId {
        &self.id
    }

    /// 已发送的心跳数。
    pub fn heartbeats_sent(&self) -> u64 {
        self.heartbeats_sent
    }

    /// 设置未匹配帧的接收回调。
    pub fn set_sink(&mut self, sink: Option<FrameSink>) {
        self.sink = sink;
    }

    /// 发送一条请求明文（至多 64 字节）。
    pub fn send(&mut self, request: &[u8]) -> Result<(), Error> {
        let wire = frame::encode_request(request)?;
        let written = self.device.write(&wire)?;
        if written != wire.len() {
            return Err(Error::Hid(hidapi::HidError::IncompleteSendError {
                sent: written,
                all: wire.len(),
            }));
        }
        Ok(())
    }

    /// 读取一帧并解密；`timeout` 内没有数据返回 `None`。
    pub fn recv(&mut self, timeout: Duration) -> Result<Option<Plaintext>, Error> {
        let mut buf = [0u8; 64];
        let millis = timeout.as_millis().min(i32::MAX as u128) as i32;
        let n = self.device.read_timeout(&mut buf, millis)?;
        if n == 0 {
            return Ok(None);
        }
        match frame::decode_report(&buf[..n]) {
            Ok(plaintext) => Ok(Some(plaintext)),
            Err(e) => Err(Error::Protocol(e)),
        }
    }

    /// 把 `timeout` 内收到的所有帧交给 sink；用于监听模式。
    pub fn pump(&mut self, timeout: Duration) -> Result<usize, Error> {
        let deadline = Instant::now() + timeout;
        let mut count = 0;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(count);
            }
            match self.recv(remaining)? {
                Some(plaintext) => {
                    count += 1;
                    if let Some(sink) = self.sink.as_mut() {
                        sink(&plaintext);
                    }
                }
                None => return Ok(count),
            }
        }
    }

    /// 发送请求并等待第一帧满足 `is_match` 的回复；其他帧交给 sink。
    pub fn transact(
        &mut self,
        request: &[u8],
        timeout: Duration,
        mut is_match: impl FnMut(&Plaintext) -> bool,
    ) -> Result<Plaintext, Error> {
        self.send(request)?;
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(Error::Timeout);
            }
            match self.recv(remaining)? {
                Some(plaintext) if is_match(&plaintext) => return Ok(plaintext),
                Some(plaintext) => {
                    if let Some(sink) = self.sink.as_mut() {
                        sink(&plaintext);
                    }
                }
                None => return Err(Error::Timeout),
            }
        }
    }

    /// 发送一次 Studio 专用心跳。没有已验证的应答约定，所以不等待回复。
    pub fn heartbeat(&mut self) -> Result<(), Error> {
        self.send(&status::HEARTBEAT)?;
        self.heartbeats_sent += 1;
        Ok(())
    }

    /// 主机接管（`true`）或交还（`false`）。只发送，不等待。
    pub fn set_software_online(&mut self, online: bool) -> Result<(), Error> {
        self.send(&status::software_online(online))
    }

    /// 查询本体是否在线。接收器永远应答，看的是 status 字节。
    pub fn query_online(&mut self, timeout: Duration) -> Result<bool, Error> {
        let reply = self.transact(&status::ONLINE_REQUEST, timeout, |f| {
            status::is_online_reply(f)
        })?;
        Ok(status::parse_online(&reply)?)
    }

    /// 查询电量。本体关机时没有回复。
    pub fn query_battery(&mut self, timeout: Duration) -> Result<Battery, Error> {
        let reply = self.transact(&status::BATTERY_REQUEST, timeout, |f| {
            status::is_battery_reply(f)
        })?;
        Ok(status::parse_battery(&reply)?)
    }

    /// 读一个控件的按键配置。
    pub fn read_key(&mut self, index: u8, timeout: Duration) -> Result<KeyConfigReply, Error> {
        let request = keyconfig::read_request(index)?;
        let reply = self.transact(&request, timeout, |f| {
            keyconfig::matches(f, keyconfig::ACCESS_READ_REPLY, index)
        })?;
        Ok(keyconfig::parse_read_reply(&reply)?)
    }

    /// 写一个控件的按键配置：等写确认，再读回核对；不一致视为失败。
    pub fn write_key(
        &mut self,
        index: u8,
        entries: &[KeyEntry],
        timeout: Duration,
    ) -> Result<KeyConfigReply, Error> {
        let request = keyconfig::write_request(index, entries)?;
        self.transact(&request, timeout, |f| keyconfig::is_write_ack(f, index))?;
        let readback = self.read_key(index, timeout)?;
        if readback.entries != entries {
            return Err(Error::ReadbackMismatch {
                expected: entries.to_vec(),
                actual: readback.entries,
            });
        }
        Ok(readback)
    }
}
