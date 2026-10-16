//! 非键盘动作的执行器：切换应用与顺序宏。
//!
//! 所有步骤在设备线程分次推进；同一控件最多一个动作，全局最多八个动作。
//! 时间由调用者注入；应用启动通过 [`Launcher`] 抽象，平台后端在 `halokey-wm`。
//! 逐行对应 olanzi 的 `HostActionRunner`。

use std::sync::{Arc, Mutex};

use crate::action::{ApplicationTarget, HostAction, KeyEntry, MacroStep};
use crate::Error;

/// 宏按键使用的保留"控件"编号，避免与六个真实控件争抢同一个键。
pub const KEYBOARD_OWNER: u8 = 0xFF;
/// 全局最多排队的动作数。
pub const MAX_QUEUED: usize = 8;
/// 等待应用进入前台的上限（秒）。
pub const ACTIVATION_TIMEOUT: f64 = 5.0;
/// 宏按键的按住时长（秒）。
pub const KEY_PULSE: f64 = 0.04;

/// 一次应用激活的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationState {
    Pending,
    Launched,
    Failed,
    Cancelled,
}

/// 启动结果的句柄：后端完成时调用 [`complete`](Self::complete)，执行器取消时调用 [`cancel`](Self::cancel)。
#[derive(Debug, Clone, Default)]
pub struct Activation(Arc<Mutex<Option<ActivationState>>>);

impl Activation {
    /// 新的待定激活。
    pub fn pending() -> Activation {
        Activation(Arc::new(Mutex::new(Some(ActivationState::Pending))))
    }

    /// 当前状态。
    pub fn state(&self) -> ActivationState {
        self.0
            .lock()
            .map(|s| s.unwrap_or(ActivationState::Pending))
            .unwrap_or(ActivationState::Failed)
    }

    /// 后端报告成功或失败；只在待定时生效。
    pub fn complete(&self, success: bool) {
        if let Ok(mut s) = self.0.lock() {
            if *s == Some(ActivationState::Pending) {
                *s = Some(if success {
                    ActivationState::Launched
                } else {
                    ActivationState::Failed
                });
            }
        }
    }

    /// 执行器取消。
    pub fn cancel(&self) {
        if let Ok(mut s) = self.0.lock() {
            *s = Some(ActivationState::Cancelled);
        }
    }
}

/// 平台的应用启动与前台查询。
pub trait Launcher {
    /// 开始把目标应用带到前台；立即返回句柄。
    fn launch(&mut self, target: &ApplicationTarget) -> Activation;
    /// 目标是否为当前前台应用。
    fn is_frontmost(&mut self, target: &ApplicationTarget) -> bool;
    /// 推进后台工作（轮询窗口等）；默认什么都不做。
    fn poll(&mut self) {}
}

impl<L: Launcher + ?Sized> Launcher for Box<L> {
    fn launch(&mut self, target: &ApplicationTarget) -> Activation {
        (**self).launch(target)
    }
    fn is_frontmost(&mut self, target: &ApplicationTarget) -> bool {
        (**self).is_frontmost(target)
    }
    fn poll(&mut self) {
        (**self).poll()
    }
}

/// 宏按键的发布接口：按下一组条目、松开上次按下的条目。
pub trait MacroKeys {
    /// 回调错误类型。
    type Error;
    /// 按下。
    fn press(&mut self, entries: &[KeyEntry]) -> Result<(), Self::Error>;
    /// 松开上次按下的条目；没有按下时也应成功。
    fn release(&mut self) -> Result<(), Self::Error>;
}

/// 执行器错误；`E` 是按键回调的错误类型。
#[derive(Debug)]
pub enum RunError<E> {
    /// 应用没能在限时内到前台。
    ActivationFailed,
    /// 前台应用改变，已停止当前宏以避免发送到其他窗口。
    ForegroundChanged,
    /// 按键回调失败。
    Callback(E),
}

impl<E: std::fmt::Display> std::fmt::Display for RunError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::ActivationFailed => f.write_str("应用程序未能切换到前台，已停止当前宏。"),
            RunError::ForegroundChanged => {
                f.write_str("前台应用已改变，已停止当前宏以避免发送到其他窗口。")
            }
            RunError::Callback(e) => write!(f, "{e}"),
        }
    }
}

impl<E: std::fmt::Debug + std::fmt::Display> std::error::Error for RunError<E> {}

#[derive(Debug)]
struct Work {
    index: u8,
    steps: Vec<MacroStep>,
}

#[derive(Debug)]
enum Wait {
    Delay(f64),
    Release(f64),
    Application(ApplicationTarget, Activation, f64),
}

/// 动作执行器。
#[derive(Debug)]
pub struct ActionRunner<L: Launcher> {
    queue: Vec<Work>,
    current: Option<Work>,
    waiting: Option<Wait>,
    expected_foreground: Option<ApplicationTarget>,
    launcher: L,
}

impl<L: Launcher> ActionRunner<L> {
    /// 新建。
    pub fn new(launcher: L) -> ActionRunner<L> {
        ActionRunner {
            queue: Vec::new(),
            current: None,
            waiting: None,
            expected_foreground: None,
            launcher,
        }
    }

    /// 访问启动器。
    pub fn launcher_mut(&mut self) -> &mut L {
        &mut self.launcher
    }

    /// 是否有动作在执行或排队。
    pub fn is_busy(&self) -> bool {
        self.current.is_some() || !self.queue.is_empty()
    }

    /// 排队一个动作；同一控件已有动作或队列已满时返回 `Ok(false)`。
    pub fn enqueue(&mut self, index: u8, action: HostAction) -> Result<bool, Error> {
        action.validate()?;
        let total = self.queue.len() + usize::from(self.current.is_some());
        if self.current.as_ref().is_some_and(|w| w.index == index)
            || self.queue.iter().any(|w| w.index == index)
            || total >= MAX_QUEUED
        {
            return Ok(false);
        }
        let steps = match action {
            HostAction::Keyboard(entries) => vec![MacroStep::Keyboard(entries)],
            HostAction::Application(target) => vec![MacroStep::Application(target)],
            HostAction::Macro(steps) => steps,
            HostAction::Library(_) => return Err(Error::MissingLibraryAction),
            HostAction::MomentaryLayer(_) | HostAction::PointerMove(_) => {
                return Err(Error::NotQueueable)
            }
        };
        self.queue.push(Work { index, steps });
        Ok(true)
    }

    /// 取消一切，包括等待中的应用激活。
    pub fn cancel(&mut self) {
        if let Some(Wait::Application(_, activation, _)) = &self.waiting {
            activation.cancel();
        }
        self.waiting = None;
        self.current = None;
        self.queue.clear();
        self.expected_foreground = None;
    }

    /// 推进一步。失败时取消全部并尝试松开按键，再把错误返回。
    pub fn pump<K: MacroKeys>(&mut self, now: f64, keys: &mut K) -> Result<(), RunError<K::Error>> {
        self.launcher.poll();
        match self.advance(now, keys) {
            Ok(()) => Ok(()),
            Err(e) => {
                // 发键失败或用户切走前台后，不能继续运行积压动作。
                self.cancel();
                let _ = keys.release();
                Err(e)
            }
        }
    }

    fn advance<K: MacroKeys>(&mut self, now: f64, keys: &mut K) -> Result<(), RunError<K::Error>> {
        // 等待期间也持续检查；用户切走又切回，不应复活已经失去目标的宏。
        if let Some(expected) = self.expected_foreground.clone() {
            if !self.launcher.is_frontmost(&expected) {
                return Err(RunError::ForegroundChanged);
            }
        }
        if let Some(waiting) = &self.waiting {
            match waiting {
                Wait::Delay(deadline) => {
                    if now < *deadline {
                        return Ok(());
                    }
                }
                Wait::Release(deadline) => {
                    if now < *deadline {
                        return Ok(());
                    }
                    keys.release().map_err(RunError::Callback)?;
                }
                Wait::Application(target, activation, deadline) => match activation.state() {
                    ActivationState::Failed | ActivationState::Cancelled => {
                        return Err(RunError::ActivationFailed)
                    }
                    ActivationState::Pending => {
                        if now >= *deadline {
                            return Err(RunError::ActivationFailed);
                        }
                        return Ok(());
                    }
                    ActivationState::Launched => {
                        let target = target.clone();
                        if !self.launcher.is_frontmost(&target) {
                            if now >= *deadline {
                                return Err(RunError::ActivationFailed);
                            }
                            return Ok(());
                        }
                        self.expected_foreground = Some(target);
                    }
                },
            }
            self.waiting = None;
        }
        if self.current.as_ref().is_some_and(|w| w.steps.is_empty()) {
            self.current = None;
            self.expected_foreground = None;
        }
        if self.current.is_none() {
            if self.queue.is_empty() {
                return Ok(());
            }
            self.current = Some(self.queue.remove(0));
        }
        let Some(current) = self.current.as_mut() else {
            return Ok(());
        };
        if current.steps.is_empty() {
            return Ok(());
        }
        let step = current.steps.remove(0);
        match step {
            MacroStep::Delay(duration) => self.waiting = Some(Wait::Delay(now + duration)),
            MacroStep::Application(target) => {
                self.expected_foreground = None;
                let activation = self.launcher.launch(&target);
                self.waiting = Some(Wait::Application(
                    target,
                    activation,
                    now + ACTIVATION_TIMEOUT,
                ));
            }
            MacroStep::Keyboard(entries) => {
                if let Some(expected) = self.expected_foreground.clone() {
                    if !self.launcher.is_frontmost(&expected) {
                        return Err(RunError::ForegroundChanged);
                    }
                }
                keys.press(&entries).map_err(RunError::Callback)?;
                self.waiting = Some(Wait::Release(now + KEY_PULSE));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct FakeState {
        activations: Vec<Activation>,
        frontmost: Option<ApplicationTarget>,
        launches: usize,
    }

    #[derive(Clone, Default)]
    struct FakeLauncher(Rc<RefCell<FakeState>>);

    impl Launcher for FakeLauncher {
        fn launch(&mut self, _target: &ApplicationTarget) -> Activation {
            let a = Activation::pending();
            let mut s = self.0.borrow_mut();
            s.activations.push(a.clone());
            s.launches += 1;
            a
        }
        fn is_frontmost(&mut self, target: &ApplicationTarget) -> bool {
            self.0.borrow().frontmost.as_ref() == Some(target)
        }
    }

    fn target(name: &str) -> ApplicationTarget {
        ApplicationTarget {
            bundle_identifier: format!("com.example.{name}"),
            path: String::new(),
            name: name.into(),
        }
    }
    fn key() -> Vec<KeyEntry> {
        vec![KeyEntry::new(0x04)]
    }

    struct Log(Rc<RefCell<Vec<String>>>);

    impl MacroKeys for Log {
        type Error = String;
        fn press(&mut self, entries: &[KeyEntry]) -> Result<(), String> {
            self.0.borrow_mut().push(format!(
                "press {:?}",
                entries.iter().map(|k| k.code).collect::<Vec<_>>()
            ));
            Ok(())
        }
        fn release(&mut self) -> Result<(), String> {
            self.0.borrow_mut().push("release".into());
            Ok(())
        }
    }

    fn pump(
        runner: &mut ActionRunner<FakeLauncher>,
        now: f64,
        log: &Log,
    ) -> Result<(), RunError<String>> {
        let mut keys = Log(log.0.clone());
        runner.pump(now, &mut keys)
    }

    #[test]
    fn macro_waits_for_activation_then_delay_and_pulses_in_order() {
        let launcher = FakeLauncher::default();
        let state = launcher.0.clone();
        let mut runner = ActionRunner::new(launcher);
        let log = Log(Rc::new(RefCell::new(Vec::new())));
        let editor = target("editor");
        assert!(runner
            .enqueue(
                0,
                HostAction::Macro(vec![
                    MacroStep::Application(editor.clone()),
                    MacroStep::Delay(0.2),
                    MacroStep::Keyboard(key())
                ])
            )
            .unwrap());
        pump(&mut runner, 0.0, &log).unwrap();
        assert_eq!(state.borrow().launches, 1);
        pump(&mut runner, 1.0, &log).unwrap();
        assert!(log.0.borrow().is_empty());
        state.borrow().activations[0].complete(true);
        pump(&mut runner, 1.1, &log).unwrap();
        assert!(log.0.borrow().is_empty(), "已启动但还没到前台时不发键");
        state.borrow_mut().frontmost = Some(editor.clone());
        pump(&mut runner, 1.2, &log).unwrap(); // 激活完成，进入 delay
        pump(&mut runner, 1.3, &log).unwrap();
        assert!(log.0.borrow().is_empty());
        pump(&mut runner, 1.4, &log).unwrap(); // delay 到期，按下
        assert_eq!(*log.0.borrow(), vec!["press [4]"]);
        pump(&mut runner, 1.43, &log).unwrap();
        assert_eq!(log.0.borrow().len(), 1);
        pump(&mut runner, 1.44, &log).unwrap();
        assert_eq!(*log.0.borrow(), vec!["press [4]", "release"]);
        pump(&mut runner, 2.0, &log).unwrap();
        assert!(!runner.is_busy());
    }

    #[test]
    fn failed_or_timed_out_activation_never_sends_macro_keys() {
        for fail_explicitly in [true, false] {
            let launcher = FakeLauncher::default();
            let state = launcher.0.clone();
            let mut runner = ActionRunner::new(launcher);
            let log = Log(Rc::new(RefCell::new(Vec::new())));
            runner
                .enqueue(
                    1,
                    HostAction::Macro(vec![
                        MacroStep::Application(target("x")),
                        MacroStep::Keyboard(key()),
                    ]),
                )
                .unwrap();
            pump(&mut runner, 0.0, &log).unwrap();
            let result = if fail_explicitly {
                state.borrow().activations[0].complete(false);
                pump(&mut runner, 0.5, &log)
            } else {
                pump(&mut runner, 4.9, &log).unwrap();
                pump(&mut runner, 5.0, &log)
            };
            assert!(matches!(result, Err(RunError::ActivationFailed)));
            assert_eq!(
                *log.0.borrow(),
                vec!["release"],
                "失败后只补一次松开，不发键"
            );
            assert!(!runner.is_busy());
        }
    }

    #[test]
    fn foreground_change_aborts_before_later_key_and_cancel_ignores_late_activation() {
        let launcher = FakeLauncher::default();
        let state = launcher.0.clone();
        let mut runner = ActionRunner::new(launcher);
        let log = Log(Rc::new(RefCell::new(Vec::new())));
        let editor = target("editor");
        runner
            .enqueue(
                0,
                HostAction::Macro(vec![
                    MacroStep::Application(editor.clone()),
                    MacroStep::Keyboard(key()),
                    MacroStep::Delay(0.1),
                    MacroStep::Keyboard(key()),
                ]),
            )
            .unwrap();
        pump(&mut runner, 0.0, &log).unwrap();
        state.borrow().activations[0].complete(true);
        state.borrow_mut().frontmost = Some(editor.clone());
        pump(&mut runner, 0.1, &log).unwrap(); // 到前台，第一个键立刻按下
        assert_eq!(*log.0.borrow(), vec!["press [4]"]);
        pump(&mut runner, 0.2, &log).unwrap(); // 松开，进入 delay
        assert_eq!(*log.0.borrow(), vec!["press [4]", "release"]);
        state.borrow_mut().frontmost = Some(target("other"));
        let result = pump(&mut runner, 0.5, &log);
        assert!(matches!(result, Err(RunError::ForegroundChanged)));
        assert_eq!(
            log.0
                .borrow()
                .iter()
                .filter(|l| l.starts_with("press"))
                .count(),
            1
        );
        // 取消后迟到的激活不再推进。
        runner
            .enqueue(2, HostAction::Application(editor.clone()))
            .unwrap();
        pump(&mut runner, 1.0, &log).unwrap();
        runner.cancel();
        assert_eq!(
            state.borrow().activations[1].state(),
            ActivationState::Cancelled
        );
        state.borrow().activations[1].complete(true);
        assert_eq!(
            state.borrow().activations[1].state(),
            ActivationState::Cancelled
        );
        pump(&mut runner, 2.0, &log).unwrap();
        assert!(!runner.is_busy());
    }

    #[test]
    fn queue_is_bounded_and_duplicate_control_does_not_accumulate() {
        let mut runner = ActionRunner::new(FakeLauncher::default());
        assert!(matches!(
            runner.enqueue(9, HostAction::Library("id".into())),
            Err(Error::MissingLibraryAction)
        ));
        assert!(matches!(
            runner.enqueue(9, HostAction::MomentaryLayer(1)),
            Err(Error::NotQueueable)
        ));
        for index in 0..8u8 {
            assert!(
                runner
                    .enqueue(index, HostAction::Application(target("x")))
                    .unwrap(),
                "{index}"
            );
        }
        assert!(!runner
            .enqueue(8, HostAction::Application(target("x")))
            .unwrap());
        assert!(!runner
            .enqueue(3, HostAction::Application(target("x")))
            .unwrap());
        runner.cancel();
        assert!(!runner.is_busy());
        assert!(runner
            .enqueue(3, HostAction::Application(target("x")))
            .unwrap());
    }
}
