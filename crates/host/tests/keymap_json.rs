//! 与 olanzi 导出格式的兼容性，以及 `HostKeymap` 校验。

use halokey_host::{
    ApplicationTarget, ControlActionMap, Error, HostAction, HostKeymap, HostLayer, HostProfile,
    KeyEntry, LongPressBehavior, MacroStep, NamedHostAction,
};

fn keymap() -> HostKeymap {
    HostKeymap::default_keymap()
}

#[test]
fn device_migration_preserves_combos_and_sorts_controls() {
    let mut bindings: Vec<(u8, Vec<KeyEntry>)> = halokey_proto::Control::FACTORY_CODES
        .iter()
        .enumerate()
        .map(|(i, c)| (i as u8, vec![KeyEntry::new(*c)]))
        .collect();
    bindings[1].1 = vec![KeyEntry::new(0xE3), KeyEntry::new(0x06)];
    bindings.reverse();
    let map = HostKeymap::from_device_bindings(&bindings).unwrap();
    assert_eq!(
        map.controls.iter().map(|c| c.index).collect::<Vec<_>>(),
        (0..6).collect::<Vec<u8>>()
    );
    assert_eq!(
        map.controls[1].press,
        vec![KeyEntry::new(0xE3), KeyEntry::new(0x06)]
    );
    assert!(map
        .controls
        .iter()
        .all(|c| c.double_press.is_none() && c.long_press.is_none()));
    assert_eq!(
        (
            map.double_press_window,
            map.long_press_threshold,
            map.version
        ),
        (0.25, 0.5, 1)
    );
}

#[test]
fn every_control_must_occur_exactly_once() {
    for indices in [
        vec![0, 1, 2, 3, 4],
        vec![0, 1, 2, 3, 4, 4],
        vec![0, 1, 2, 3, 4, 6],
        (0..7).collect(),
    ] {
        let map = HostKeymap::new(
            indices
                .into_iter()
                .map(|i| ControlActionMap::new(i, vec![]))
                .collect(),
        );
        assert_eq!(map.validate(), Err(Error::InvalidControls));
    }
    let mut map = keymap();
    map.version = 99;
    assert_eq!(map.validate(), Err(Error::UnsupportedVersion));
}

#[test]
fn timing_rejects_nonfinite_out_of_range_and_inverted_windows() {
    for (d, l) in [
        (0.14, 0.5),
        (0.51, 1.0),
        (0.2, 0.29),
        (0.2, 2.01),
        (0.4, 0.4),
        (f64::NAN, 0.5),
        (0.25, f64::INFINITY),
    ] {
        let mut map = keymap();
        map.double_press_window = d;
        map.long_press_threshold = l;
        assert_eq!(map.validate(), Err(Error::InvalidTiming), "{d} {l}");
    }
    for (d, l) in [(0.15, 0.3), (0.5, 2.0)] {
        let mut map = keymap();
        map.double_press_window = d;
        map.long_press_threshold = l;
        assert!(map.validate().is_ok());
    }
}

#[test]
fn rotation_has_no_gestures_and_entries_are_validated() {
    for entries in [
        vec![KeyEntry {
            kind: 3,
            code: 0x28,
        }],
        vec![KeyEntry::new(0xFF)],
        vec![KeyEntry::new(4); 25],
    ] {
        for slot in 0..3 {
            let mut map = keymap();
            match slot {
                0 => map.controls[0].press = entries.clone(),
                1 => map.controls[0].double_press = Some(entries.clone()),
                _ => map.controls[0].long_press = Some(entries.clone()),
            }
            assert!(map.validate().is_err());
        }
    }
    let mut map = keymap();
    map.controls[4].double_press = Some(vec![KeyEntry::new(4)]);
    assert_eq!(map.validate(), Err(Error::UnsupportedGesture));
    map = keymap();
    map.controls[5].long_press_behavior = LongPressBehavior::Tap;
    assert_eq!(map.validate(), Err(Error::UnsupportedGesture));
    map = keymap();
    map.controls[0].double_press_behavior = LongPressBehavior::Hold;
    // 字段直接改动后要先对齐版本号，校验才会报内容错误而不是版本错误。
    map.normalize_version();
    assert_eq!(map.validate(), Err(Error::InvalidDoublePressBehavior));
    map = keymap();
    map.controls[0].long_press_tap_count = 21;
    assert_eq!(map.validate(), Err(Error::InvalidLongPressTapCount));
    map = keymap();
    map.controls[4].press_action = Some(HostAction::MomentaryLayer(1));
    map.normalize_version();
    assert_eq!(map.validate(), Err(Error::InvalidLayerAction));
}

#[test]
fn swift_exported_json_imports_and_round_trips() {
    let json = r#"{
      "version": 5,
      "controls": [
        {"index":0,"press":[{"type":2,"code":1}],"pressAction":{"momentaryLayer":{"_0":1}},"longPress":[{"type":2,"code":41}],
         "pressBehavior":"hold","pressTapCount":2,"doublePressBehavior":"tap","doublePressTapCount":2,
         "longPressBehavior":"burst","longPressTapCount":3,"inheritedGestures":[]},
        {"index":1,"press":[{"type":2,"code":40}],"doublePress":[{"type":2,"code":43}],"inheritedGestures":[]},
        {"index":2,"press":[{"type":2,"code":41}],"pressAction":{"library":{"_0":"123E4567-E89B-12D3-A456-426614174000"}}},
        {"index":3,"press":[{"type":2,"code":70}],"pressBehavior":"burst","pressTapCount":4},
        {"index":4,"press":[{"type":2,"code":79}]},
        {"index":5,"press":[{"type":2,"code":42}]}
      ],
      "layers": [{"id":1,"controls":[{"index":1,"press":[{"type":2,"code":4}],"inheritedGestures":["doublePress"]}]}],
      "actionLibrary": [{"id":"123E4567-E89B-12D3-A456-426614174000","name":"编辑器","slot":0,
                         "action":{"macro":{"_0":[{"application":{"_0":{"bundleIdentifier":"com.example.editor","path":"","name":"Editor"}}},{"delay":{"_0":0.2}},{"keyboard":{"_0":[{"type":2,"code":227},{"type":2,"code":4}]}}]}}}],
      "doublePressWindow": 0.3,
      "longPressThreshold": 0.6
    }"#;
    let map = HostKeymap::from_json(json.as_bytes()).unwrap();
    assert_eq!(map.version, 5);
    assert_eq!(
        map.controls[0].effective_press(),
        HostAction::MomentaryLayer(1)
    );
    assert_eq!(map.controls[0].long_press_tap_count, 3);
    assert_eq!(map.controls[3].press_behavior, LongPressBehavior::Burst);
    assert_eq!(map.layers[0].controls[0].inherited_gestures.len(), 1);
    assert_eq!(
        map.resolved_control(1, &[1]).unwrap().press,
        vec![KeyEntry::new(4)]
    );
    assert_eq!(
        map.resolved_control(1, &[1]).unwrap().double_press,
        Some(vec![KeyEntry::new(43)])
    );
    let resolved = map.resolve(&map.controls[2].effective_press()).unwrap();
    assert!(matches!(resolved, HostAction::Macro(ref steps) if steps.len() == 3));
    let bytes = map.to_json().unwrap();
    let again = HostKeymap::from_json(&bytes).unwrap();
    assert_eq!(again, map);
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains(r#""momentaryLayer":{"_0":1}"#));
    assert!(text.contains(r#""inheritedGestures":["doublePress"]"#));
    assert!(!text.contains("null"));
}

#[test]
fn version_selection_follows_content() {
    let mut map = keymap();
    assert_eq!(map.version, 1);
    map.controls[0].press_action = Some(HostAction::Application(ApplicationTarget {
        bundle_identifier: "a.b".into(),
        path: String::new(),
        name: "A".into(),
    }));
    map.normalize_version();
    assert_eq!(map.version, 2);
    map.action_library.push(NamedHostAction {
        id: "123e4567-e89b-12d3-a456-426614174000".into(),
        name: "宏".into(),
        slot: 0,
        action: HostAction::Macro(vec![MacroStep::Delay(0.5)]),
    });
    map.normalize_version();
    assert_eq!(map.version, 3);
    map.layers.push(HostLayer {
        id: 2,
        controls: vec![],
    });
    map.normalize_version();
    assert_eq!(map.version, 4);
    map.controls[1].press_tap_count = 3;
    map.normalize_version();
    assert_eq!(map.version, 5);
    assert!(map.validate().is_ok());
    map.controls[1].press_tap_count = 1;
    assert_eq!(map.validate(), Err(Error::InvalidPressTapCount));
    map.controls[1].press_tap_count = 5;
    assert!(map.validate().is_ok());
    // 版本低于内容所需时拒绝。
    let mut stale = map.clone();
    stale.version = 1;
    assert_eq!(stale.validate(), Err(Error::UnsupportedVersion));
    // 未知的未来动作先报版本不兼容。
    let future = r#"{"version":9,"controls":[],"doublePressWindow":0.25,"longPressThreshold":0.5}"#;
    assert_eq!(
        HostKeymap::from_json(future.as_bytes()),
        Err(Error::UnsupportedVersion)
    );
    let future_action = r#"{"version":2,"controls":[{"index":0,"press":[],"pressAction":{"futureAction":{"_0":2}}}],"doublePressWindow":0.25,"longPressThreshold":0.5}"#;
    assert!(matches!(
        HostKeymap::from_json(future_action.as_bytes()),
        Err(Error::Json(_))
    ));
    assert_eq!(
        HostKeymap::from_json(&vec![b' '; 32 * 1024 + 1]),
        Err(Error::TooLarge)
    );
}

#[test]
fn profile_round_trip_and_legacy_codes_migration() {
    let profile = HostProfile {
        id: "123e4567-e89b-12d3-a456-426614174000".into(),
        name: "日常".into(),
        keymap: keymap(),
    };
    let bytes = profile.to_json().unwrap();
    assert_eq!(HostProfile::from_json(&bytes).unwrap(), profile);
    let legacy = r#"{"id":"123E4567-E89B-12D3-A456-426614174000","version":1,"name":"旧","codes":[1,40,41,70,79,42]}"#;
    let migrated = HostProfile::from_json(legacy.as_bytes()).unwrap();
    assert_eq!(migrated.keymap.controls[3].press, vec![KeyEntry::new(70)]);
    assert_eq!(migrated.keymap.version, 1);
    let bad = r#"{"id":"123E4567-E89B-12D3-A456-426614174000","version":3,"name":"x"}"#;
    assert_eq!(
        HostProfile::from_json(bad.as_bytes()),
        Err(Error::UnsupportedVersion)
    );
    let blank = HostProfile {
        name: "   ".into(),
        ..profile
    };
    assert_eq!(blank.validate(), Err(Error::InvalidName));
}
