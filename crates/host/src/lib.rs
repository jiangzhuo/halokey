//! halokey 主机侧的纯逻辑：动作模型、带层的键位表、手势状态机。
//!
//! 这里不碰设备、不读时钟、不发事件。时间由调用者注入，输出是一串 [`Transition`]，
//! 由平台适配层翻译成真实的键盘 / 鼠标事件或动作执行。
//! JSON 格式与 olanzi 的 `HostKeymap` / `HostProfile` 兼容（Swift Codable 的
//! `{"variant": {"_0": …}}` 枚举编码），olanzi 导出的配置可以直接导入。

#![forbid(unsafe_code)]

pub mod action;
pub mod error;
pub mod gesture;
pub mod keymap;
pub mod lighting;
pub mod profile;
pub mod runner;

pub use action::{
    ApplicationTarget, Halo, HaloOption, HostAction, KeyEntry, MacroStep, NamedHostAction,
    PointerDelta, ScrollDelta,
};
pub use error::Error;
pub use gesture::{GestureRouter, Transition};
pub use keymap::{ControlActionMap, ControlGesture, HostKeymap, HostLayer, LongPressBehavior};
pub use lighting::{LayerLighting, LightEffect, LightMode};
pub use profile::HostProfile;
pub use runner::{ActionRunner, Activation, ActivationState, Launcher, MacroKeys, RunError};
