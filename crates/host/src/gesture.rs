//! 手势状态机：把六个控件的按下 / 松开事件变成动作。
//!
//! 单调时间由调用者注入，不读系统时钟、不访问设备、不发事件。
//! 逐行对应 olanzi 的 `GestureRouter`，行为由同一套测试保证。

use std::collections::{BTreeMap, BTreeSet};

use halokey_proto::{Control, VendorKeyEvent};

use crate::action::{HostAction, KeyEntry};
use crate::keymap::{ControlActionMap, HostKeymap, LongPressBehavior};

/// 状态机的输出；真实的按键发布、权限与共享修饰键由适配层负责。
#[derive(Debug, Clone, PartialEq)]
pub enum Transition {
    /// 按下一组键盘条目。
    Begin { index: u8, entries: Vec<KeyEntry> },
    /// 松开该控件此前按下的条目。
    End { index: u8 },
    /// 执行一次非键盘动作（切换应用、宏）。
    Execute { index: u8, action: HostAction },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Pressing { start: f64, second: bool },
    Waiting { deadline: f64 },
    Holding(u64),
    Layer(u8),
    Burst,
    Completed,
}

#[derive(Debug, Clone, PartialEq)]
struct KeyboardJob {
    id: u64,
    entries: Vec<KeyEntry>,
    count: u32,
    hold: bool,
    long_burst: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct Playback {
    job: KeyboardJob,
    down: bool,
    remaining: u32,
    deadline: f64,
}

/// 连按的按住时长与间隔（秒）。
const BURST_DOWN: f64 = 0.04;
const BURST_GAP: f64 = 0.08;

/// 手势路由器。
#[derive(Debug, Default)]
pub struct GestureRouter {
    /// 每控件最多一个执行中动作和十六个待执行动作；满队列丢弃新触发。
    configuration: Option<HostKeymap>,
    // 每轮手势固定动作快照，层变化不改写按住、双击等待或连按队列。
    snapshots: BTreeMap<u8, ControlActionMap>,
    layer_owners: BTreeMap<u8, u8>,
    phases: BTreeMap<u8, Phase>,
    playback: BTreeMap<u8, Playback>,
    queues: BTreeMap<u8, Vec<KeyboardJob>>,
    next_job_id: u64,
    physical_down: BTreeSet<u8>,
    quarantined: BTreeSet<u8>,
    last_time: f64,
}

impl GestureRouter {
    /// 每控件待执行队列上限。
    pub const MAX_QUEUED_ACTIONS: usize = 16;

    /// 新建一个没有配置的路由器；没有配置时所有事件都被忽略。
    pub fn new() -> GestureRouter {
        GestureRouter::default()
    }

    /// 当前活动的层（按编号升序）。
    pub fn active_layers(&self) -> Vec<u8> {
        let mut layers: Vec<u8> = self.layer_owners.values().copied().collect();
        layers.sort_unstable();
        layers.dedup();
        layers
    }

    /// 切换配置：相同配置不动；否则取消所有进行中的动作并隔离仍按住的控件。
    pub fn configure(&mut self, configuration: Option<HostKeymap>) -> Vec<Transition> {
        if self.configuration == configuration {
            return Vec::new();
        }
        let releases = self.cancel(true);
        self.configuration = configuration;
        releases
    }

    /// 取消一切进行中的动作；`quarantine` 为真时，仍按住的物理键必须先松开才接受新动作。
    pub fn cancel(&mut self, quarantine: bool) -> Vec<Transition> {
        let releases: Vec<Transition> = self
            .playback
            .iter()
            .filter(|(_, p)| p.down)
            .map(|(index, _)| Transition::End { index: *index })
            .collect();
        self.phases.clear();
        self.snapshots.clear();
        self.layer_owners.clear();
        self.playback.clear();
        self.queues.clear();
        if quarantine {
            self.quarantined.extend(self.physical_down.iter().copied());
        } else {
            self.physical_down.clear();
            self.quarantined.clear();
        }
        releases
    }

    /// 发布失败只取消该控件的后续调度，其它控件的真实按住所有权继续保留。
    pub fn cancel_index(&mut self, index: u8) {
        self.finish(index);
        self.layer_owners.remove(&index);
        self.playback.remove(&index);
        self.queues.remove(&index);
        if self.physical_down.contains(&index) {
            self.quarantined.insert(index);
        }
    }

    /// 挂起期间只记录物理状态：按住的键在恢复后必须先松开。
    pub fn observe_while_suspended(&mut self, event: VendorKeyEvent) {
        let index = event.control.index();
        if index >= 4 {
            return;
        }
        if event.pressed {
            self.physical_down.insert(index);
            self.quarantined.insert(index);
        } else {
            self.physical_down.remove(&index);
            self.quarantined.remove(&index);
        }
    }

    /// 推进到时刻 `now`，发出到期的单击、长按与连按边沿。
    pub fn advance(&mut self, now: f64) -> Vec<Transition> {
        let time = self.monotonic(now);
        let Some(configuration) = self.configuration.clone() else {
            return Vec::new();
        };
        let mut transitions = Vec::new();
        let indices: Vec<u8> = self.phases.keys().copied().collect();
        for index in indices {
            let Some(control) = self.snapshots.get(&index).cloned() else {
                continue;
            };
            match self.phases.get(&index).copied() {
                Some(Phase::Waiting { deadline }) if time >= deadline => {
                    self.finish(index);
                    transitions.extend(self.trigger(
                        index,
                        control.effective_press(),
                        control.press_behavior,
                        control.press_tap_count,
                    ));
                }
                Some(Phase::Pressing { start, .. })
                    if time >= start + configuration.long_press_threshold =>
                {
                    let Some(action) = control.effective_long_press() else {
                        continue;
                    };
                    match action {
                        HostAction::MomentaryLayer(target) => {
                            self.layer_owners.insert(index, target);
                            self.phases.insert(index, Phase::Layer(target));
                        }
                        HostAction::Keyboard(entries) => {
                            if control.long_press_behavior == LongPressBehavior::Hold {
                                let (id, t) = self.enqueue(index, entries, 1, true, false);
                                self.phases
                                    .insert(index, id.map_or(Phase::Completed, Phase::Holding));
                                transitions.extend(t);
                            } else {
                                let burst = control.long_press_behavior == LongPressBehavior::Burst;
                                self.phases.insert(
                                    index,
                                    if burst {
                                        Phase::Burst
                                    } else {
                                        Phase::Completed
                                    },
                                );
                                let count = if burst {
                                    control.long_press_tap_count
                                } else {
                                    1
                                };
                                let (id, t) = self.enqueue(index, entries, count, false, burst);
                                if id.is_none() {
                                    self.phases.insert(index, Phase::Completed);
                                }
                                transitions.extend(t);
                            }
                        }
                        other => {
                            self.phases.insert(index, Phase::Completed);
                            transitions.push(Transition::Execute {
                                index,
                                action: other,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        // 每次推进最多走一条连按边沿；下一期限相对实际执行时间计算，不追赶积压脉冲。
        let indices: Vec<u8> = self.playback.keys().copied().collect();
        for index in indices {
            let Some(mut current) = self.playback.get(&index).cloned() else {
                continue;
            };
            if time < current.deadline {
                continue;
            }
            if current.down {
                transitions.push(Transition::End { index });
                if current.remaining == 0 {
                    self.complete(&current.job, index);
                    self.schedule_next(index, time + BURST_GAP);
                } else {
                    current.down = false;
                    current.deadline = time + BURST_GAP;
                    self.playback.insert(index, current);
                }
            } else {
                transitions.push(Transition::Begin {
                    index,
                    entries: current.job.entries.clone(),
                });
                if current.job.hold {
                    current.down = true;
                    current.deadline = f64::INFINITY;
                    self.playback.insert(index, current);
                } else if current.job.count == 1 {
                    // 排队的单次短按仍是完整的一组 down/up。
                    transitions.push(Transition::End { index });
                    self.complete(&current.job, index);
                    self.schedule_next(index, time + BURST_GAP);
                } else {
                    current.down = true;
                    current.remaining -= 1;
                    current.deadline = time + BURST_DOWN;
                    self.playback.insert(index, current);
                }
            }
        }
        transitions
    }

    /// 接收一个物理事件。
    pub fn receive(&mut self, event: VendorKeyEvent, now: f64) -> Vec<Transition> {
        // 先处理到期动作：恰好在双击截止时的 down 属于下一轮，不能依赖回调顺序。
        let mut transitions = self.advance(now);
        let time = self.last_time;
        let index = event.control.index();
        if self.configuration.is_none() {
            return transitions;
        }
        if event.control.is_rotation() {
            if event.pressed {
                if let Some(control) = self.selected_control(index) {
                    transitions.extend(self.trigger(
                        index,
                        control.effective_press(),
                        control.press_behavior,
                        control.press_tap_count,
                    ));
                }
            }
            return transitions;
        }
        if event.pressed {
            if !self.physical_down.insert(index) || self.quarantined.contains(&index) {
                return transitions;
            }
            if matches!(self.phases.get(&index), Some(Phase::Burst)) {
                // 长按连按不能被重按开启第二轮；切层仍跟随新的物理按住状态。
                if let Some(HostAction::MomentaryLayer(target)) =
                    self.snapshots.get(&index).map(|c| c.effective_press())
                {
                    self.layer_owners.insert(index, target);
                }
                return transitions;
            }
            let control = if let (Some(Phase::Waiting { .. }), Some(snapshot)) =
                (self.phases.get(&index), self.snapshots.get(&index))
            {
                snapshot.clone()
            } else {
                let Some(selected) = self.selected_control(index) else {
                    return transitions;
                };
                self.snapshots.insert(index, selected.clone());
                selected
            };
            let press = control.effective_press();
            let double = control.effective_double_press();
            let long = control.effective_long_press();
            if let HostAction::MomentaryLayer(target) = press {
                self.layer_owners.insert(index, target);
                let phase = if long.is_none() {
                    Phase::Layer(target)
                } else {
                    Phase::Pressing {
                        start: time,
                        second: false,
                    }
                };
                self.phases.insert(index, phase);
            } else if double.is_none() && long.is_none() {
                match press {
                    HostAction::Keyboard(entries)
                        if control.press_behavior == LongPressBehavior::Hold =>
                    {
                        let (id, t) = self.enqueue(index, entries, 1, true, false);
                        self.phases
                            .insert(index, id.map_or(Phase::Completed, Phase::Holding));
                        transitions.extend(t);
                    }
                    press => {
                        self.phases.insert(index, Phase::Completed);
                        transitions.extend(self.trigger(
                            index,
                            press,
                            control.press_behavior,
                            control.press_tap_count,
                        ));
                    }
                }
            } else {
                let second = matches!(self.phases.get(&index), Some(Phase::Waiting { .. }));
                self.phases.insert(
                    index,
                    Phase::Pressing {
                        start: time,
                        second,
                    },
                );
            }
        } else {
            let was_down = self.physical_down.remove(&index);
            self.layer_owners.remove(&index);
            if self.quarantined.remove(&index) {
                return transitions;
            }
            let Some(control) = self.snapshots.get(&index).cloned() else {
                return transitions;
            };
            if !was_down {
                return transitions;
            }
            match self.phases.get(&index).copied() {
                Some(Phase::Layer(_)) | Some(Phase::Completed) => self.finish(index),
                Some(Phase::Holding(id)) => {
                    self.finish(index);
                    transitions.extend(self.release_hold(index, id));
                }
                Some(Phase::Pressing { second, .. }) => {
                    let double = control.effective_double_press();
                    if let (true, Some(action)) = (second, double.clone()) {
                        self.finish(index);
                        transitions.extend(self.trigger(
                            index,
                            action,
                            control.double_press_behavior,
                            control.double_press_tap_count,
                        ));
                    } else if double.is_some() {
                        let deadline = time
                            + self
                                .configuration
                                .as_ref()
                                .map_or(0.0, |c| c.double_press_window);
                        self.phases.insert(index, Phase::Waiting { deadline });
                    } else {
                        self.finish(index);
                        transitions.extend(self.trigger(
                            index,
                            control.effective_press(),
                            control.press_behavior,
                            control.press_tap_count,
                        ));
                    }
                }
                _ => {}
            }
        }
        transitions
    }

    fn trigger(
        &mut self,
        index: u8,
        action: HostAction,
        behavior: LongPressBehavior,
        count: u32,
    ) -> Vec<Transition> {
        match action {
            HostAction::MomentaryLayer(_) => Vec::new(),
            // 已松开的单/双击和旋转无法持续保持；旧 hold 设置继续解释为一次短按。
            HostAction::Keyboard(entries) => {
                let count = if behavior == LongPressBehavior::Burst {
                    count
                } else {
                    1
                };
                self.enqueue(index, entries, count, false, false).1
            }
            other => vec![Transition::Execute {
                index,
                action: other,
            }],
        }
    }

    fn enqueue(
        &mut self,
        index: u8,
        entries: Vec<KeyEntry>,
        count: u32,
        hold: bool,
        long_burst: bool,
    ) -> (Option<u64>, Vec<Transition>) {
        let queued = self.queues.get(&index).map_or(0, Vec::len);
        if self.playback.contains_key(&index) && queued >= Self::MAX_QUEUED_ACTIONS {
            return (None, Vec::new());
        }
        self.next_job_id = self.next_job_id.wrapping_add(1);
        let job = KeyboardJob {
            id: self.next_job_id,
            entries: entries.clone(),
            count,
            hold,
            long_burst,
        };
        if self.playback.contains_key(&index) {
            self.queues.entry(index).or_default().push(job);
            return (Some(self.next_job_id), Vec::new());
        }
        if !hold && count == 1 {
            return (
                Some(job.id),
                vec![
                    Transition::Begin { index, entries },
                    Transition::End { index },
                ],
            );
        }
        let id = job.id;
        self.playback.insert(
            index,
            Playback {
                job,
                down: true,
                remaining: count - 1,
                deadline: if hold {
                    f64::INFINITY
                } else {
                    self.last_time + BURST_DOWN
                },
            },
        );
        (Some(id), vec![Transition::Begin { index, entries }])
    }

    fn release_hold(&mut self, index: u8, id: u64) -> Vec<Transition> {
        // 排队期间已经松开的 hold 不再启动，不能释放其它动作持有的同一控件。
        if let Some(current) = self.playback.get(&index) {
            if current.job.id == id {
                let down = current.down;
                self.schedule_next(index, self.last_time + BURST_GAP);
                return if down {
                    vec![Transition::End { index }]
                } else {
                    Vec::new()
                };
            }
        }
        if let Some(queue) = self.queues.get_mut(&index) {
            queue.retain(|job| job.id != id);
        }
        Vec::new()
    }

    fn complete(&mut self, job: &KeyboardJob, index: u8) {
        if !job.long_burst {
            return;
        }
        if self.physical_down.contains(&index) {
            self.phases.insert(index, Phase::Completed);
        } else {
            self.finish(index);
        }
    }

    fn schedule_next(&mut self, index: u8, deadline: f64) {
        let next = self.queues.get_mut(&index).and_then(|q| {
            if q.is_empty() {
                None
            } else {
                Some(q.remove(0))
            }
        });
        match next {
            Some(job) => {
                let count = job.count;
                self.playback.insert(
                    index,
                    Playback {
                        job,
                        down: false,
                        remaining: count,
                        deadline,
                    },
                );
            }
            None => {
                self.playback.remove(&index);
                self.queues.remove(&index);
            }
        }
    }

    fn finish(&mut self, index: u8) {
        self.phases.remove(&index);
        self.snapshots.remove(&index);
    }

    fn selected_control(&self, index: u8) -> Option<ControlActionMap> {
        let configuration = self.configuration.as_ref()?;
        let active: Vec<u8> = self.layer_owners.values().copied().collect();
        let mut resolved = configuration.resolved_control(index, &active)?;
        let press = resolved.effective_press();
        resolved.press_action = Some(configuration.resolve(&press).unwrap_or(press));
        resolved.double_press_action = resolved
            .effective_double_press()
            .map(|a| configuration.resolve(&a).unwrap_or(a));
        resolved.long_press_action = resolved
            .effective_long_press()
            .map(|a| configuration.resolve(&a).unwrap_or(a));
        Some(resolved)
    }

    fn monotonic(&mut self, now: f64) -> f64 {
        if now.is_finite() {
            self.last_time = self.last_time.max(now);
        }
        self.last_time
    }
}

/// 构造一个事件，测试与适配层共用。
pub fn event(index: u8, pressed: bool) -> VendorKeyEvent {
    VendorKeyEvent {
        control: Control::from_index(index).expect("index 0–5"),
        pressed,
        action: 0,
    }
}
