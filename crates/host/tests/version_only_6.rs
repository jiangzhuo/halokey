use halokey_host::{Error, HostKeymap};

#[test]
fn only_version_6_is_accepted_and_written() {
    assert_eq!(HostKeymap::VERSION, 6);
    assert_eq!(HostKeymap::default_keymap().version, 6);
    let six = HostKeymap::default_keymap().to_json().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&six).unwrap();
    assert_eq!(v["version"], 6);
    let mut five = v.clone();
    five["version"] = serde_json::json!(5);
    assert!(matches!(
        HostKeymap::from_json(&serde_json::to_vec(&five).unwrap()),
        Err(Error::UnsupportedVersion)
    ));
    let mut seven = v;
    seven["version"] = serde_json::json!(7);
    assert!(matches!(
        HostKeymap::from_json(&serde_json::to_vec(&seven).unwrap()),
        Err(Error::UnsupportedVersion)
    ));
}
