//! 分层背光写入调度：100 ms 防抖，只在有效背光变化时写设备。
use std::time::{Duration, Instant};

use halokey_host::LayerLighting;

// 接线在运行时主循环里（见 lib.rs 的 refresh_layers / 连接循环）。
pub(crate) const DEBOUNCE: Duration = Duration::from_millis(100);

/// 把"活动层变化"变成"该写什么、什么时候写"。
pub(crate) struct LightingScheduler {
    /// 当前应有的有效背光；`None` = 不管灯。
    desired: Option<LayerLighting>,
    /// 最近一次 `note` 改变值的时刻；防抖从这里起算。
    changed_at: Option<Instant>,
    /// 最近一次成功写入的值；失败后清空。
    last_written: Option<LayerLighting>,
}

impl LightingScheduler {
    pub(crate) fn new() -> Self {
        LightingScheduler {
            desired: None,
            changed_at: None,
            last_written: None,
        }
    }

    /// 记录"现在应该是什么"。值变了就重新起算防抖；值没变但还没成功写上（例如上次失败），
    /// 也重新起算——这样失败后的重试跟着下一次手势过渡走，而不是空转重试。
    pub(crate) fn note(&mut self, desired: Option<LayerLighting>, now: Instant) {
        if desired != self.desired {
            self.desired = desired;
            self.changed_at = Some(now);
        } else if desired.is_some() && desired != self.last_written && self.changed_at.is_none() {
            self.changed_at = Some(now);
        }
    }

    pub(crate) fn due(&mut self, now: Instant) -> Option<LayerLighting> {
        let changed_at = self.changed_at?;
        if now.duration_since(changed_at) < DEBOUNCE {
            return None;
        }
        self.changed_at = None;
        let desired = self.desired?;
        if Some(desired) == self.last_written {
            return None;
        }
        Some(desired)
    }

    pub(crate) fn written(&mut self, value: LayerLighting) {
        self.last_written = Some(value);
    }

    pub(crate) fn failed(&mut self) {
        self.last_written = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halokey_host::{LightEffect, LightMode};

    fn l(b: u8) -> LayerLighting {
        LayerLighting {
            lights: [LightEffect {
                mode: LightMode::Solid,
                brightness: b,
            }; 4],
        }
    }

    #[test]
    fn nothing_is_written_without_a_record() {
        let t0 = Instant::now();
        let mut s = LightingScheduler::new();
        s.note(None, t0);
        assert_eq!(s.due(t0 + Duration::from_secs(1)), None);
    }

    #[test]
    fn writes_once_after_debounce_and_not_again_for_the_same_value() {
        let t0 = Instant::now();
        let mut s = LightingScheduler::new();
        s.note(Some(l(5)), t0);
        assert_eq!(s.due(t0 + Duration::from_millis(50)), None, "防抖未到");
        assert_eq!(s.due(t0 + DEBOUNCE), Some(l(5)));
        s.written(l(5));
        s.note(Some(l(5)), t0 + Duration::from_millis(300));
        assert_eq!(s.due(t0 + Duration::from_millis(500)), None, "值没变不重写");
    }

    #[test]
    fn press_and_quick_release_collapse_into_one_write() {
        let t0 = Instant::now();
        let mut s = LightingScheduler::new();
        s.note(Some(l(1)), t0);
        s.written(l(1));
        s.note(Some(l(2)), t0 + Duration::from_millis(10)); // 按下 MO
        s.note(Some(l(1)), t0 + Duration::from_millis(40)); // 很快松开
        assert_eq!(
            s.due(t0 + Duration::from_millis(200)),
            None,
            "回到原值，一次都不用写"
        );
    }

    #[test]
    fn release_writes_layer0_back() {
        let t0 = Instant::now();
        let mut s = LightingScheduler::new();
        s.note(Some(l(1)), t0);
        assert_eq!(s.due(t0 + DEBOUNCE), Some(l(1)));
        s.written(l(1));
        s.note(Some(l(2)), t0 + Duration::from_secs(1));
        assert_eq!(s.due(t0 + Duration::from_secs(1) + DEBOUNCE), Some(l(2)));
        s.written(l(2));
        s.note(Some(l(1)), t0 + Duration::from_secs(2));
        assert_eq!(s.due(t0 + Duration::from_secs(2) + DEBOUNCE), Some(l(1)));
    }

    #[test]
    fn failure_forgets_last_written_so_the_next_change_retries() {
        let t0 = Instant::now();
        let mut s = LightingScheduler::new();
        s.note(Some(l(3)), t0);
        assert_eq!(s.due(t0 + DEBOUNCE), Some(l(3)));
        s.failed();
        assert_eq!(
            s.due(t0 + Duration::from_secs(1)),
            None,
            "失败后不自动重试风暴"
        );
        s.note(Some(l(3)), t0 + Duration::from_secs(2));
        assert_eq!(
            s.due(t0 + Duration::from_secs(2) + DEBOUNCE),
            Some(l(3)),
            "下一次变化再试"
        );
    }
}
