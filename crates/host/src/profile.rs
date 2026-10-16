//! 可导出 / 导入的命名配置。版本 2 内嵌 [`HostKeymap`]；版本 1 只有六个单键码，导入时迁移。

use serde::{Deserialize, Serialize};

use crate::action::{is_uuid, KeyEntry};
use crate::{Error, HostKeymap};

/// 一份命名配置。
#[derive(Debug, Clone, PartialEq)]
pub struct HostProfile {
    /// UUID 字符串。
    pub id: String,
    /// 显示名称，1–80 字。
    pub name: String,
    pub keymap: HostKeymap,
}

impl HostProfile {
    /// 当前写出的版本。
    pub const VERSION: u32 = 2;

    /// 校验名称与键位表。
    pub fn validate(&self) -> Result<(), Error> {
        if !is_uuid(&self.id) {
            return Err(Error::Json("配置标识不是 UUID".into()));
        }
        if self.name.trim().is_empty() || self.name.chars().count() > 80 {
            return Err(Error::InvalidName);
        }
        self.keymap.validate()
    }

    /// 从 JSON 导入（含版本 1 迁移），大小上限同键位表。
    pub fn from_json(bytes: &[u8]) -> Result<HostProfile, Error> {
        if bytes.len() > HostKeymap::MAX_JSON_BYTES {
            return Err(Error::TooLarge);
        }
        let wire: Wire = serde_json::from_slice(bytes)?;
        let keymap = match wire.version {
            // 旧版只记录六个单键；迁移时保留标识、名称，不擅自增加手势。
            1 => {
                let codes = wire.codes.ok_or(Error::UnsupportedVersion)?;
                let bindings: Vec<(u8, Vec<KeyEntry>)> = codes
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (i as u8, vec![KeyEntry::new(*c)]))
                    .collect();
                HostKeymap::from_device_bindings(&bindings)?
            }
            2 => wire.keymap.ok_or(Error::UnsupportedVersion)?,
            _ => return Err(Error::UnsupportedVersion),
        };
        let profile = HostProfile {
            id: wire.id,
            name: wire.name,
            keymap,
        };
        profile.validate()?;
        Ok(profile)
    }

    /// 导出 JSON，始终为版本 2。
    pub fn to_json(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        Ok(serde_json::to_vec(&Wire {
            id: self.id.clone(),
            version: Self::VERSION,
            name: self.name.clone(),
            keymap: Some(self.keymap.clone()),
            codes: None,
        })?)
    }
}

#[derive(Serialize, Deserialize)]
struct Wire {
    id: String,
    version: u32,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    keymap: Option<HostKeymap>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    codes: Option<Vec<u8>>,
}
