//! 回归：从设备表 / 出厂表得到的 version 1 键位，加上新版动作后仍必须能解析，
//! 版本由内容决定而不是由调用方预先改好。
use halokey_host::HostKeymap;

#[test]
fn v1_keymap_with_newer_actions_parses_and_upgrades_version() {
    let base = HostKeymap::default_keymap();
    let mut value: serde_json::Value = serde_json::from_slice(&base.to_json().unwrap()).unwrap();
    assert_eq!(value["version"], 1, "出厂表应归一为 version 1");
    value["controls"][0]["pressAction"] = serde_json::json!({ "momentaryLayer": { "_0": 1 } });
    value["controls"][4]["pressAction"] =
        serde_json::json!({ "pointerMove": { "_0": { "dx": 20, "dy": 0 } } });
    let map = HostKeymap::from_json(&serde_json::to_vec(&value).unwrap())
        .expect("version 1 + 新动作必须能解析");
    assert!(map.version > 1);
    assert_eq!(map.version, map.required_version());
}
