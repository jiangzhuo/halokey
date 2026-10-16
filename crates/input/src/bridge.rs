//! 把手势状态机的 [`Transition`] 翻译成注入调用，并记住每个控件按住了什么。
//!
//! 与 olanzi 的 `VendorKeyBridge` 一致：同一个键被多个控件同时持有时，只在第一次按下
//! 时真正按下、最后一次松开时真正松开；某控件注入失败只取消该控件，补 up 会重试。

use std::collections::BTreeMap;

use halokey_host::{HostAction, KeyEntry, Transition};

use crate::{Error, Injector};

/// 一次 `apply` 的结果。
#[derive(Debug, Default)]
pub struct Applied {
    /// 需要由上层执行的非键盘动作（切换应用、宏）。
    pub execute: Vec<(u8, HostAction)>,
    /// 注入失败的控件与错误；状态机应对这些控件调用 `cancel_index`。
    pub failures: Vec<(u8, Error)>,
}

/// 键盘持有状态。
#[derive(Debug, Default)]
pub struct KeyBridge {
    held: BTreeMap<u8, Vec<KeyEntry>>,
    pending_release: Vec<u8>,
    /// 成功注入的边沿计数。
    pub events: u64,
}

impl KeyBridge {
    /// 新建。
    pub fn new() -> KeyBridge {
        KeyBridge::default()
    }

    /// 当前是否有控件持有键。
    pub fn is_holding(&self) -> bool {
        !self.held.is_empty()
    }

    /// 应用一批转换。
    pub fn apply(&mut self, injector: &mut dyn Injector, transitions: Vec<Transition>) -> Applied {
        let mut applied = Applied::default();
        self.retry_releases(injector, &mut applied);
        for transition in transitions {
            let result = match &transition {
                Transition::Execute { index, action } => {
                    applied.execute.push((*index, action.clone()));
                    Ok(())
                }
                Transition::Begin { index, entries } => self.press(injector, *index, entries),
                Transition::End { index } => self.release(injector, *index),
            };
            if let Err(e) = result {
                let index = match transition {
                    Transition::Begin { index, .. }
                    | Transition::End { index }
                    | Transition::Execute { index, .. } => index,
                };
                applied.failures.push((index, e));
            }
        }
        applied
    }

    fn others(&self, index: u8) -> Vec<KeyEntry> {
        self.held
            .iter()
            .filter(|(i, _)| **i != index)
            .flat_map(|(_, e)| e.iter().copied())
            .collect()
    }

    /// 为控件按下一组条目；已被其他控件持有的键不再重复按下。
    pub fn press(
        &mut self,
        injector: &mut dyn Injector,
        index: u8,
        entries: &[KeyEntry],
    ) -> Result<(), Error> {
        if self.held.contains_key(&index) {
            return Ok(());
        }
        let mut unique: Vec<KeyEntry> = Vec::new();
        for entry in entries.iter().filter(|e| e.code != 0) {
            if !unique.contains(entry) {
                unique.push(*entry);
            }
        }
        let others = self.others(index);
        let fresh: Vec<KeyEntry> = unique
            .iter()
            .copied()
            .filter(|e| !others.contains(e))
            .collect();
        // 即使注入器在部分提交后失败，也保留所有权并补 up，不能遗失已经按下的键。
        self.held.insert(index, unique);
        match injector.press(&fresh) {
            Ok(()) => {
                self.events += 1;
                Ok(())
            }
            Err(e) => {
                let _ = self.release(injector, index);
                Err(e)
            }
        }
    }

    /// 松开控件持有的条目；仍被其他控件持有的键保持按下。
    pub fn release(&mut self, injector: &mut dyn Injector, index: u8) -> Result<(), Error> {
        let Some(entries) = self.held.get(&index).cloned() else {
            return Ok(());
        };
        let others = self.others(index);
        let fresh: Vec<KeyEntry> = entries
            .iter()
            .copied()
            .filter(|e| !others.contains(e))
            .collect();
        if let Err(e) = injector.release(&fresh) {
            if !self.pending_release.contains(&index) {
                self.pending_release.push(index);
            }
            return Err(e);
        }
        self.pending_release.retain(|i| *i != index);
        self.held.remove(&index);
        self.events += 1;
        Ok(())
    }

    fn retry_releases(&mut self, injector: &mut dyn Injector, applied: &mut Applied) {
        for index in self.pending_release.clone() {
            if let Err(e) = self.release(injector, index) {
                applied.failures.push((index, e));
            }
        }
    }

    /// 松开一切（退出、断开、配置切换时），从高 index 到低。
    pub fn release_all(&mut self, injector: &mut dyn Injector) -> Vec<(u8, Error)> {
        let mut failures = Vec::new();
        for index in self.held.keys().copied().rev().collect::<Vec<_>>() {
            if let Err(e) = self.release(injector, index) {
                failures.push((index, e));
            }
        }
        failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecordingInjector;

    fn k(code: u8) -> KeyEntry {
        KeyEntry::new(code)
    }

    #[test]
    fn shared_keys_are_pressed_once_and_released_last() {
        let mut rec = RecordingInjector::default();
        let mut bridge = KeyBridge::new();
        bridge.press(&mut rec, 0, &[k(0xE3), k(0x04)]).unwrap();
        bridge.press(&mut rec, 1, &[k(0xE3), k(0x05)]).unwrap();
        assert_eq!(
            rec.keys
                .iter()
                .map(|(e, d)| (e.code, *d))
                .collect::<Vec<_>>(),
            vec![(0xE3, true), (0x04, true), (0x05, true)]
        );
        rec.keys.clear();
        bridge.release(&mut rec, 0).unwrap();
        assert_eq!(
            rec.keys
                .iter()
                .map(|(e, d)| (e.code, *d))
                .collect::<Vec<_>>(),
            vec![(0x04, false)]
        );
        rec.keys.clear();
        bridge.release(&mut rec, 1).unwrap();
        assert_eq!(
            rec.keys
                .iter()
                .map(|(e, d)| (e.code, *d))
                .collect::<Vec<_>>(),
            vec![(0x05, false), (0xE3, false)]
        );
        assert!(!bridge.is_holding());
        assert_eq!(bridge.events, 4);
    }

    #[test]
    fn apply_routes_execute_and_ignores_no_action_zero_codes() {
        let mut rec = RecordingInjector::default();
        let mut bridge = KeyBridge::new();
        let applied = bridge.apply(
            &mut rec,
            vec![
                Transition::Begin {
                    index: 2,
                    entries: vec![k(0)],
                },
                Transition::Execute {
                    index: 3,
                    action: HostAction::MomentaryLayer(1),
                },
                Transition::End { index: 2 },
                Transition::End { index: 5 },
            ],
        );
        assert!(rec.keys.is_empty());
        assert_eq!(applied.execute.len(), 1);
        assert!(applied.failures.is_empty());
        assert!(bridge.release_all(&mut rec).is_empty());
    }
}
