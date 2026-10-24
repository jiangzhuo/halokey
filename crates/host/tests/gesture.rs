//! 移植自 olanzi `GestureRouterTests`，行为必须一致。

use halokey_host::gesture::event;
use halokey_host::{
    ControlActionMap, GestureRouter, HostKeymap, KeyEntry, LongPressBehavior, Transition,
};

fn single() -> Vec<KeyEntry> {
    vec![KeyEntry::new(1)]
}
fn double() -> Vec<KeyEntry> {
    vec![KeyEntry::new(0x28)]
}
fn long() -> Vec<KeyEntry> {
    vec![KeyEntry::new(0x29)]
}

fn map(with_double: bool, with_long: bool, behavior: LongPressBehavior) -> HostKeymap {
    HostKeymap::new(
        (0..6u8)
            .map(|i| {
                let mut c = ControlActionMap::new(i, single());
                if i < 4 && with_double {
                    c.double_press = Some(double());
                }
                if i < 4 && with_long {
                    c.long_press = Some(long());
                }
                c.long_press_behavior = if i < 4 {
                    behavior
                } else {
                    LongPressBehavior::Hold
                };
                c
            })
            .collect(),
    )
}

fn down(i: u8) -> halokey_proto::VendorKeyEvent {
    event(i, true)
}
fn up(i: u8) -> halokey_proto::VendorKeyEvent {
    event(i, false)
}
fn begin(i: u8, entries: Vec<KeyEntry>) -> Transition {
    Transition::Begin { index: i, entries }
}
fn end(i: u8) -> Transition {
    Transition::End { index: i }
}
fn pulse(entries: Vec<KeyEntry>, i: u8) -> Vec<Transition> {
    vec![begin(i, entries), end(i)]
}
fn router(m: HostKeymap) -> GestureRouter {
    let mut r = GestureRouter::new();
    assert!(r.configure(Some(m)).is_empty());
    r
}
const HOLD: LongPressBehavior = LongPressBehavior::Hold;
const TAP: LongPressBehavior = LongPressBehavior::Tap;

#[test]
fn default_hold_begins_immediately_and_stays_held_without_timeout() {
    let mut r = router(map(false, false, HOLD));
    assert_eq!(r.receive(down(0), 0.0), vec![begin(0, single())]);
    assert_eq!(r.advance(100.0), vec![]);
    assert_eq!(r.receive(up(0), 101.0), vec![end(0)]);
}

#[test]
fn duplicate_down_and_up_are_ignored() {
    let mut r = router(map(false, false, HOLD));
    r.receive(down(0), 0.0);
    assert_eq!(r.receive(down(0), 0.1), vec![]);
    assert_eq!(r.receive(up(0), 0.2), vec![end(0)]);
    assert_eq!(r.receive(up(0), 0.3), vec![]);
}

#[test]
fn rapid_clicks_without_double_action_each_begin_immediately() {
    for index in 0..4 {
        let mut r = router(map(false, false, HOLD));
        for click in 0..5 {
            let t = click as f64 * 0.08;
            assert_eq!(r.receive(down(index), t), vec![begin(index, single())]);
            assert_eq!(r.receive(up(index), t + 0.03), vec![end(index)]);
        }
        assert_eq!(r.advance(2.0), vec![]);
    }
}

#[test]
fn rapid_clicks_with_only_long_action_each_fire_single_then_long_still_works() {
    for index in 0..4 {
        let mut r = router(map(false, true, HOLD));
        for click in 0..5 {
            let t = click as f64 * 0.08;
            assert_eq!(r.receive(down(index), t), vec![]);
            assert_eq!(r.receive(up(index), t + 0.03), pulse(single(), index));
        }
        assert_eq!(r.receive(down(index), 0.4), vec![]);
        assert_eq!(r.advance(1.0), vec![begin(index, long())]);
        assert_eq!(r.receive(up(index), 1.1), vec![end(index)]);
        assert_eq!(r.advance(2.0), vec![]);
    }
}

#[test]
fn single_waits_from_first_release_and_fires_at_exact_deadline() {
    let mut r = router(map(true, false, HOLD));
    assert_eq!(r.receive(down(0), 0.0), vec![]);
    assert_eq!(r.receive(up(0), 0.1), vec![]);
    assert_eq!(r.advance(0.349), vec![]);
    assert_eq!(r.advance(0.35), pulse(single(), 0));
    assert_eq!(r.advance(10.0), vec![]);
}

#[test]
fn double_only_fires_on_second_release_and_suppresses_single() {
    let mut r = router(map(true, false, HOLD));
    r.receive(down(0), 0.0);
    r.receive(up(0), 0.1);
    assert_eq!(r.receive(down(0), 0.2), vec![]);
    assert_eq!(r.advance(2.0), vec![]);
    assert_eq!(r.receive(up(0), 3.0), pulse(double(), 0));
    assert_eq!(r.advance(4.0), vec![]);
}

#[test]
fn second_down_at_deadline_starts_new_sequence_after_first_single() {
    let mut r = router(map(true, false, HOLD));
    r.receive(down(0), 0.0);
    r.receive(up(0), 0.25);
    assert_eq!(r.receive(down(0), 0.5), pulse(single(), 0));
    assert_eq!(r.receive(up(0), 0.75), vec![]);
    assert_eq!(r.advance(1.0), pulse(single(), 0));
}

#[test]
fn long_begins_at_threshold_and_stays_held_until_release() {
    let mut r = router(map(true, true, HOLD));
    r.receive(down(0), 0.0);
    assert_eq!(r.advance(0.499), vec![]);
    assert_eq!(r.advance(0.5), vec![begin(0, long())]);
    assert_eq!(r.advance(10.0), vec![]);
    assert_eq!(r.receive(up(0), 11.0), vec![end(0)]);
    assert_eq!(r.advance(12.0), vec![]);
}

#[test]
fn release_exactly_at_long_threshold_is_long_pulse() {
    let mut r = router(map(false, true, HOLD));
    r.receive(down(0), 0.0);
    assert_eq!(r.receive(up(0), 0.5), pulse(long(), 0));
}

#[test]
fn short_press_with_only_long_alternative_fires_single_at_release() {
    let mut r = router(map(false, true, HOLD));
    r.receive(down(0), 0.0);
    assert_eq!(r.receive(up(0), 0.499), pulse(single(), 0));
    assert_eq!(r.advance(2.0), vec![]);
}

#[test]
fn second_long_wins_and_cancels_first_single_and_double() {
    let mut r = router(map(true, true, HOLD));
    r.receive(down(0), 0.0);
    r.receive(up(0), 0.125);
    r.receive(down(0), 0.25);
    assert_eq!(r.advance(0.75), vec![begin(0, long())]);
    assert_eq!(r.receive(up(0), 0.8), vec![end(0)]);
    assert_eq!(r.advance(2.0), vec![]);
}

#[test]
fn duplicate_second_down_does_not_restart_long_clock() {
    let mut r = router(map(true, true, HOLD));
    r.receive(down(0), 0.0);
    r.receive(up(0), 0.125);
    r.receive(down(0), 0.25);
    assert_eq!(r.receive(down(0), 0.7), vec![]);
    assert_eq!(r.advance(0.75), vec![begin(0, long())]);
}

#[test]
fn rotation_always_pulses_and_does_not_wait_for_up() {
    let mut r = router(map(true, true, HOLD));
    assert_eq!(r.receive(down(4), 0.0), pulse(single(), 4));
    assert_eq!(r.receive(down(4), 0.001), pulse(single(), 4));
    assert_eq!(r.receive(up(4), 0.002), vec![]);
    assert_eq!(r.receive(down(5), 0.003), pulse(single(), 5));
}

#[test]
fn cancellation_never_flushes_expired_single_or_starts_pending_long() {
    let mut r = router(map(true, true, HOLD));
    r.receive(down(0), 0.0);
    r.receive(up(0), 0.1);
    r.receive(down(1), 0.1);
    assert_eq!(r.cancel(true), vec![]);
    assert_eq!(r.advance(100.0), vec![]);
}

#[test]
fn configuration_change_releases_held_and_quarantines_until_up() {
    let mut r = router(map(false, false, HOLD));
    r.receive(down(0), 0.0);
    let mut updated = map(false, false, HOLD);
    updated.controls[0].press = double();
    assert_eq!(r.configure(Some(updated)), vec![end(0)]);
    assert_eq!(r.receive(down(0), 1.0), vec![]);
    assert_eq!(r.receive(up(0), 2.0), vec![]);
    assert_eq!(r.receive(down(0), 3.0), vec![begin(0, double())]);
}

#[test]
fn suspended_physical_down_requires_up_before_reactivation() {
    let mut r = router(map(false, false, HOLD));
    r.observe_while_suspended(down(0));
    assert_eq!(r.receive(down(0), 1.0), vec![]);
    assert_eq!(r.receive(up(0), 2.0), vec![]);
    assert_eq!(r.receive(down(0), 3.0), vec![begin(0, single())]);
}

#[test]
fn disconnect_resets_old_physical_generation_and_rotations_are_never_quarantined() {
    let mut r = router(map(false, false, HOLD));
    r.receive(down(0), 0.0);
    r.cancel(false);
    assert_eq!(r.receive(down(0), 1.0), vec![begin(0, single())]);
    r.cancel(true);
    assert_eq!(r.receive(down(4), 2.0), pulse(single(), 4));
}

#[test]
fn controls_have_independent_timers_and_clock_never_reopens_window() {
    let mut r = router(map(true, false, HOLD));
    r.receive(down(0), 0.0);
    r.receive(up(0), 0.1);
    r.receive(down(1), 0.2);
    r.receive(up(1), 0.3);
    assert_eq!(r.advance(0.35), pulse(single(), 0));
    assert_eq!(r.advance(0.2), vec![]);
    assert_eq!(r.advance(0.55), pulse(single(), 1));
}

#[test]
fn tap_long_press_pulses_once_and_ignores_timers_duplicates_and_release() {
    for index in 0..4 {
        let mut r = router(map(true, true, TAP));
        assert_eq!(r.receive(down(index), 0.0), vec![]);
        assert_eq!(r.advance(0.499), vec![]);
        assert_eq!(r.advance(0.5), pulse(long(), index));
        assert_eq!(r.receive(down(index), 0.6), vec![]);
        assert_eq!(r.advance(10.0), vec![]);
        assert_eq!(r.receive(up(index), 11.0), vec![]);
        assert_eq!(r.receive(up(index), 12.0), vec![]);
        assert_eq!(r.advance(13.0), vec![]);
        assert_eq!(r.receive(down(index), 14.0), vec![]);
        assert_eq!(r.receive(up(index), 14.5), pulse(long(), index));
    }
}

#[test]
fn second_tap_long_press_suppresses_both_pending_single_and_double() {
    let mut r = router(map(true, true, TAP));
    r.receive(down(0), 0.0);
    r.receive(up(0), 0.125);
    r.receive(down(0), 0.25);
    assert_eq!(r.advance(0.75), pulse(long(), 0));
    assert_eq!(r.receive(up(0), 0.8), vec![]);
    assert_eq!(r.advance(2.0), vec![]);
}

#[test]
fn tap_long_press_still_allows_short_primary_and_double_actions() {
    let mut r = router(map(false, true, TAP));
    r.receive(down(0), 0.0);
    assert_eq!(r.receive(up(0), 0.499), pulse(single(), 0));
    r.configure(Some(map(true, true, TAP)));
    r.receive(down(0), 1.0);
    r.receive(up(0), 1.125);
    r.receive(down(0), 1.25);
    assert_eq!(r.receive(up(0), 1.3), pulse(double(), 0));
    assert_eq!(r.advance(3.0), vec![]);
}

#[test]
fn cancel_and_reconfigure_never_replay_completed_or_pending_tap_long_press() {
    for fire_before_cancel in [false, true] {
        let mut r = router(map(true, true, TAP));
        r.receive(down(0), 0.0);
        if fire_before_cancel {
            assert_eq!(r.advance(0.5), pulse(long(), 0));
        }
        assert_eq!(r.cancel(true), vec![]);
        assert_eq!(r.advance(1.0), vec![]);
        assert_eq!(r.configure(Some(map(true, true, HOLD))), vec![]);
        assert_eq!(r.receive(down(0), 2.0), vec![]);
        assert_eq!(r.receive(up(0), 3.0), vec![]);
        assert_eq!(r.receive(down(0), 4.0), vec![]);
        assert_eq!(r.advance(4.5), vec![begin(0, long())]);
        assert_eq!(r.configure(Some(map(true, true, TAP))), vec![end(0)]);
        assert_eq!(r.receive(up(0), 5.0), vec![]);
    }
}

#[test]
fn burst_starts_at_threshold_holds_forty_milliseconds_and_waits_eighty_after_up() {
    // 移植自 LongPressBurstTests：长按连按 2 次 = down/up, 80 ms, down/up。
    let mut m = map(false, true, LongPressBehavior::Burst);
    m.controls[0].long_press_tap_count = 2;
    let mut r = router(m);
    r.receive(down(0), 0.0);
    assert_eq!(r.advance(0.5), vec![begin(0, long())]);
    assert_eq!(r.advance(0.53), vec![]);
    assert_eq!(r.advance(0.54), vec![end(0)]);
    assert_eq!(r.advance(0.61), vec![]);
    assert_eq!(r.advance(0.62), vec![begin(0, long())]);
    assert_eq!(r.advance(0.66), vec![end(0)]);
    assert_eq!(r.advance(10.0), vec![]);
    assert_eq!(r.receive(up(0), 11.0), vec![]);
}

#[test]
fn momentary_layer_is_immediate_and_release_restores_base() {
    use halokey_host::{HostAction, HostLayer};
    let mut m = map(false, false, HOLD);
    m.controls[0].press_action = Some(HostAction::MomentaryLayer(1));
    m.layers.push(HostLayer {
        id: 1,
        controls: vec![ControlActionMap::new(1, double())],
        lighting: None,
    });
    assert!(m.validate().is_ok());
    let mut r = router(m);
    assert_eq!(r.receive(down(0), 0.0), vec![]);
    assert_eq!(r.active_layers(), vec![1]);
    assert_eq!(r.receive(down(1), 0.1), vec![begin(1, double())]);
    assert_eq!(r.receive(up(1), 0.2), vec![end(1)]);
    assert_eq!(r.receive(up(0), 0.3), vec![]);
    assert_eq!(r.active_layers(), Vec::<u8>::new());
    assert_eq!(r.receive(down(1), 0.4), vec![begin(1, single())]);
    // 旋钮在层激活时也走该层。
    r.receive(up(1), 0.5);
    let mut m2 = map(false, false, HOLD);
    m2.controls[0].press_action = Some(HostAction::MomentaryLayer(2));
    m2.layers.push(HostLayer {
        id: 2,
        controls: vec![ControlActionMap::new(4, long())],
        lighting: None,
    });
    r.configure(Some(m2));
    r.receive(down(0), 1.0);
    assert_eq!(r.receive(down(4), 1.1), pulse(long(), 4));
    r.receive(up(0), 1.2);
    assert_eq!(r.receive(down(4), 1.3), pulse(single(), 4));
}

#[test]
fn scroll_on_the_knob_executes_once_per_tick_even_with_burst() {
    use halokey_host::{HostAction, ScrollDelta};
    let fast = HostAction::Scroll(ScrollDelta { dx: 0, dy: 4 });
    let mut m = map(false, false, HOLD);
    m.controls[4].press_action = Some(fast.clone());
    m.controls[4].press_behavior = LongPressBehavior::Burst;
    m.controls[4].press_tap_count = 5;
    let mut r = router(m);
    let execute = vec![Transition::Execute {
        index: 4,
        action: fast,
    }];
    assert_eq!(r.receive(down(4), 0.0), execute);
    assert_eq!(r.receive(down(4), 0.005), execute);
    assert_eq!(r.advance(1.0), vec![]);
}

// ---- 锁定层（TG / TO）与一次性层（OSL）：halokey 扩展，olanzi 没有 ----

fn with_layer(mut m: HostKeymap, id: u8, controls: Vec<ControlActionMap>) -> HostKeymap {
    m.layers.push(halokey_host::HostLayer {
        id,
        controls,
        lighting: None,
    });
    m
}

#[test]
fn toggle_layer_flips_on_each_press_and_survives_release() {
    use halokey_host::HostAction;
    let mut m = map(false, false, HOLD);
    m.controls[0].press_action = Some(HostAction::ToggleLayer(1));
    let m = with_layer(m, 1, vec![ControlActionMap::new(1, double())]);
    assert!(m.validate().is_ok());
    let mut r = router(m);
    assert_eq!(r.receive(down(0), 0.0), vec![]);
    assert_eq!(r.active_layers(), vec![1]);
    assert_eq!(r.receive(up(0), 0.1), vec![]);
    assert_eq!(r.active_layers(), vec![1]);
    assert_eq!(r.receive(down(1), 0.2), vec![begin(1, double())]);
    assert_eq!(r.receive(up(1), 0.3), vec![end(1)]);
    // 第二次按下关掉；Layer 1 没覆盖键 1，所以它继承基础层的 TG(1)。
    r.receive(down(0), 0.4);
    r.receive(up(0), 0.5);
    assert_eq!(r.active_layers(), Vec::<u8>::new());
    assert_eq!(r.receive(down(1), 0.6), vec![begin(1, single())]);
}

#[test]
fn to_layer_replaces_locked_layers_keeps_held_momentary_and_to_zero_returns_to_base() {
    use halokey_host::HostAction;
    let mut m = map(false, false, HOLD);
    m.controls[0].press_action = Some(HostAction::ToggleLayer(1));
    m.controls[1].press_action = Some(HostAction::ToLayer(2));
    m.controls[2].press_action = Some(HostAction::MomentaryLayer(3));
    m.controls[3].press_action = Some(HostAction::ToLayer(0));
    let mut r = router(m);
    r.receive(down(0), 0.0);
    r.receive(up(0), 0.1);
    assert_eq!(r.active_layers(), vec![1]);
    r.receive(down(1), 0.2);
    r.receive(up(1), 0.3);
    assert_eq!(r.active_layers(), vec![2]);
    // 按住中的 MO 归按住的键所有，TO 不收回它。
    r.receive(down(2), 0.4);
    r.receive(down(0), 0.5);
    assert_eq!(r.active_layers(), vec![1, 2, 3]);
    r.receive(up(0), 0.6);
    r.receive(down(1), 0.7);
    assert_eq!(r.active_layers(), vec![2, 3]);
    r.receive(up(1), 0.8);
    r.receive(up(2), 0.9);
    assert_eq!(r.active_layers(), vec![2]);
    r.receive(down(3), 1.0);
    r.receive(up(3), 1.1);
    assert_eq!(r.active_layers(), Vec::<u8>::new());
}

#[test]
fn one_shot_layer_applies_to_the_next_press_only() {
    use halokey_host::HostAction;
    let mut m = map(false, false, HOLD);
    m.controls[0].press_action = Some(HostAction::OneShotLayer(1));
    let m = with_layer(m, 1, vec![ControlActionMap::new(1, double())]);
    let mut r = router(m);
    r.receive(down(0), 0.0);
    assert_eq!(r.receive(up(0), 0.1), vec![]);
    assert_eq!(r.active_layers(), vec![1]);
    assert_eq!(r.receive(down(1), 5.0), vec![begin(1, double())]);
    assert_eq!(r.active_layers(), Vec::<u8>::new());
    assert_eq!(r.receive(up(1), 5.1), vec![end(1)]);
    assert_eq!(r.receive(down(1), 5.2), vec![begin(1, single())]);
}

#[test]
fn one_shot_layer_covers_a_continuous_knob_turn_then_drops_after_a_pause() {
    use halokey_host::HostAction;
    let mut m = map(false, false, HOLD);
    m.controls[0].press_action = Some(HostAction::OneShotLayer(2));
    let m = with_layer(m, 2, vec![ControlActionMap::new(4, long())]);
    let mut r = router(m);
    r.receive(down(0), 0.0);
    r.receive(up(0), 0.1);
    assert_eq!(r.receive(down(4), 1.0), pulse(long(), 4));
    assert_eq!(r.receive(down(4), 1.9), pulse(long(), 4));
    assert_eq!(r.advance(2.899), vec![]);
    assert_eq!(r.active_layers(), vec![2]);
    assert_eq!(r.advance(2.9), vec![]);
    assert_eq!(r.active_layers(), Vec::<u8>::new());
    assert_eq!(r.receive(down(4), 3.0), pulse(single(), 4));
    // 转动中按键：一次性层已被这段转动用掉，按键走基础层。
    r.receive(down(0), 4.0);
    r.receive(up(0), 4.1);
    assert_eq!(r.receive(down(4), 4.2), pulse(long(), 4));
    assert_eq!(r.receive(down(1), 4.3), vec![begin(1, single())]);
    assert_eq!(r.active_layers(), Vec::<u8>::new());
}

#[test]
fn layer_switches_on_double_and_long_press_fire_when_the_gesture_resolves() {
    use halokey_host::HostAction;
    let mut m = map(false, false, HOLD);
    m.controls[0].double_press_action = Some(HostAction::ToggleLayer(1));
    m.controls[0].long_press_action = Some(HostAction::ToLayer(3));
    let mut r = router(m);
    r.receive(down(0), 0.0);
    r.receive(up(0), 0.1);
    r.receive(down(0), 0.2);
    assert_eq!(r.active_layers(), Vec::<u8>::new());
    assert_eq!(r.receive(up(0), 0.3), vec![]);
    assert_eq!(r.active_layers(), vec![1]);
    r.receive(down(0), 1.0);
    assert_eq!(r.advance(1.5), vec![]);
    assert_eq!(r.active_layers(), vec![3]);
    assert_eq!(r.receive(up(0), 2.0), vec![]);
    assert_eq!(r.active_layers(), vec![3]);
}

#[test]
fn cancel_keeps_locked_layers_drops_one_shot_and_new_configuration_resets_both() {
    use halokey_host::HostAction;
    let mut m = map(false, false, HOLD);
    m.controls[0].press_action = Some(HostAction::ToggleLayer(1));
    m.controls[1].press_action = Some(HostAction::OneShotLayer(2));
    let mut r = router(m.clone());
    r.receive(down(0), 0.0);
    r.receive(up(0), 0.1);
    r.receive(down(1), 0.2);
    r.receive(up(1), 0.3);
    assert_eq!(r.active_layers(), vec![1, 2]);
    r.cancel(false);
    assert_eq!(r.active_layers(), vec![1]);
    // 同一份配置不算换配置。
    assert_eq!(r.configure(Some(m.clone())), vec![]);
    assert_eq!(r.active_layers(), vec![1]);
    let mut changed = m;
    changed.controls[2].press = double();
    r.configure(Some(changed));
    assert_eq!(r.active_layers(), Vec::<u8>::new());
}

#[test]
fn example_knob_modes_cycles_with_knob_press_and_one_shot_tabs() {
    use halokey_host::LightMode;
    let m =
        HostKeymap::from_json(include_bytes!("../../../docs/examples/knob-modes.json")).unwrap();
    let wheel_down = vec![KeyEntry::mouse(halokey_proto::MouseCode::WheelDown)];
    let right = vec![KeyEntry::new(0x4F)];
    let redo = vec![
        KeyEntry::new(0xE0),
        KeyEntry::new(0xE1),
        KeyEntry::new(0x1D),
    ];
    let next_tab = vec![KeyEntry::new(0xE0), KeyEntry::new(0x2B)];
    let mut r = router(m.clone());
    assert_eq!(r.receive(down(4), 0.0), pulse(wheel_down.clone(), 4));
    for (t, layers, turn) in [
        (1.0, vec![1], right.clone()),
        (2.0, vec![2], redo),
        (3.0, vec![], wheel_down),
    ] {
        r.receive(down(3), t);
        r.receive(up(3), t + 0.1);
        assert_eq!(r.active_layers(), layers);
        assert_eq!(r.receive(down(4), t + 0.2), pulse(turn, 4));
    }
    // Layer 1 上按键 1：一次性 Layer 3 盖在上面，这段转动切标签，停转后回到 Layer 1。
    r.receive(down(3), 10.0);
    r.receive(up(3), 10.1);
    r.receive(down(0), 11.0);
    r.receive(up(0), 11.1);
    assert_eq!(r.active_layers(), vec![1, 3]);
    assert_eq!(r.receive(down(4), 12.0), pulse(next_tab.clone(), 4));
    assert_eq!(r.receive(down(4), 12.5), pulse(next_tab, 4));
    r.advance(13.5);
    assert_eq!(r.active_layers(), vec![1]);
    assert_eq!(r.receive(down(4), 14.0), pulse(right, 4));
    assert_eq!(
        m.effective_lighting(&[]).unwrap().lights[3].mode,
        LightMode::Off
    );
    assert_eq!(
        m.effective_lighting(&[1, 3]).unwrap().lights[1].mode,
        LightMode::Solid
    );
}
