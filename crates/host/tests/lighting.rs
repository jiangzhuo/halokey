//! 分层背光：字段往返、版本、校验、有效背光解析、到设备记录的映射。
use halokey_host::{
    Error, HostAction, HostKeymap, HostLayer, LayerLighting, LightEffect, LightMode,
};
use halokey_proto::{IndicatorLight, Lighting};

fn effects(mode: LightMode, brightness: u8) -> LayerLighting {
    LayerLighting {
        lights: [LightEffect { mode, brightness }; 4],
    }
}

#[test]
fn files_without_lighting_round_trip_unchanged() {
    let map = HostKeymap::default_keymap();
    let json = String::from_utf8(map.to_json().unwrap()).unwrap();
    assert!(!json.contains("lighting"));
    let back = HostKeymap::from_json(json.as_bytes()).unwrap();
    assert_eq!(back.lighting, None);
    assert_eq!(back.version, 1);
}

#[test]
fn lighting_round_trips_and_requires_version_6() {
    let mut map = HostKeymap::default_keymap();
    map.lighting = Some(effects(LightMode::Solid, 14));
    map.controls[0].press_action = Some(HostAction::MomentaryLayer(1));
    map.layers.push(HostLayer {
        id: 1,
        controls: Vec::new(),
        lighting: Some(effects(LightMode::Breathe, 20)),
    });
    map.normalize_version();
    assert_eq!(map.version, 6);
    let json = map.to_json().unwrap();
    let text = String::from_utf8(json.clone()).unwrap();
    assert!(text.contains("\"mode\": \"breathe\"") || text.contains("\"mode\":\"breathe\""));
    let back = HostKeymap::from_json(&json).unwrap();
    assert_eq!(back.lighting, map.lighting);
    assert_eq!(back.layers[0].lighting, map.layers[0].lighting);
}

#[test]
fn brightness_above_20_is_rejected() {
    let mut map = HostKeymap::default_keymap();
    map.lighting = Some(effects(LightMode::Solid, 21));
    map.normalize_version();
    assert!(matches!(map.validate(), Err(Error::InvalidLighting)));
    let json = br#"{"version":6,"controls":[],"lighting":{"lights":[{"mode":"solid","brightness":3}]},"doublePressWindow":0.25,"longPressThreshold":0.5}"#;
    assert!(
        matches!(
            HostKeymap::from_json(json),
            Err(Error::InvalidLightingCount)
        ),
        "三盏灯的记录必须被拒绝，并给中文错误"
    );
    assert_eq!(
        Error::InvalidLightingCount.to_string(),
        "背光记录必须有四盏灯。"
    );
}

#[test]
fn effective_lighting_picks_highest_active_layer_with_a_record() {
    let mut map = HostKeymap::default_keymap();
    map.controls[0].press_action = Some(HostAction::MomentaryLayer(1));
    map.controls[1].press_action = Some(HostAction::MomentaryLayer(3));
    map.layers.push(HostLayer {
        id: 1,
        controls: Vec::new(),
        lighting: Some(effects(LightMode::Breathe, 5)),
    });
    map.layers.push(HostLayer {
        id: 3,
        controls: Vec::new(),
        lighting: None,
    });
    assert_eq!(map.effective_lighting(&[]), None, "Layer 0 没记录 → 不管灯");
    assert_eq!(
        map.effective_lighting(&[3]),
        None,
        "Layer 3 没记录且 Layer 0 没记录"
    );
    assert_eq!(
        map.effective_lighting(&[1, 3]).unwrap().lights[0].brightness,
        5
    );
    map.lighting = Some(effects(LightMode::Solid, 9));
    assert_eq!(
        map.effective_lighting(&[3]).unwrap().lights[0].brightness,
        9,
        "Layer 3 没记录 → 回到 Layer 0"
    );
    assert_eq!(
        map.effective_lighting(&[1]).unwrap().lights[0].brightness,
        5
    );
}

#[test]
fn device_mapping_sets_work_time_255_for_lit_lights_and_keeps_breathe_level() {
    let current = Lighting {
        mode: 2,
        brightness: 14,
        lights: [IndicatorLight {
            kind: 1,
            work_time: 7,
            breathe_level: 3,
            breathe_brightness: 1,
            always_on_brightness: 2,
        }; 4],
    };
    let mut want = effects(LightMode::Breathe, 20);
    want.lights[3] = LightEffect {
        mode: LightMode::Off,
        brightness: 0,
    };
    let out = want.device_lighting(&current);
    assert_eq!(out.mode, 2);
    assert_eq!(out.brightness, 14);
    assert_eq!(
        out.lights[0],
        IndicatorLight {
            kind: 2,
            work_time: 255,
            breathe_level: 3,
            breathe_brightness: 20,
            always_on_brightness: 20
        }
    );
    assert_eq!(out.lights[3].kind, 0);
    assert_eq!(out.lights[3].work_time, 7, "关着的灯不改工作时长");
    assert_eq!(out.lights[3].always_on_brightness, 0);
    // 全局模式 0（全灭）或 1（全亮）都会压住单灯设置：有亮的灯就切到 2（工作）。
    let dark = Lighting {
        mode: 1,
        ..current.clone()
    };
    assert_eq!(want.device_lighting(&dark).mode, 2);
    let all_off = LayerLighting {
        lights: [LightEffect {
            mode: LightMode::Off,
            brightness: 0,
        }; 4],
    };
    assert_eq!(
        all_off.device_lighting(&dark).mode,
        2,
        "全关的记录也切到工作模式，四盏 kind=0 才真的灭"
    );
}

#[test]
fn example_layer_backlight_parses() {
    let bytes = include_bytes!("../../../docs/examples/layer-backlight.json");
    let map = HostKeymap::from_json(bytes).unwrap();
    assert_eq!(map.version, 6);
    assert_eq!(
        map.effective_lighting(&[1]).unwrap().lights[0].mode,
        LightMode::Breathe
    );
    assert_eq!(
        map.effective_lighting(&[]).unwrap().lights[0].mode,
        LightMode::Solid
    );
}
