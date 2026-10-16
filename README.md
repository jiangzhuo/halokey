# halokey

> 🌐 [中文](README.zh.md)

Cross-platform, open-source control center for the Ulanzi Vibe Key (AU05): remap the three keys and the knob, run host-side actions, pick from a radial on-screen menu, and manage lighting, microphone and power settings. One codebase for Linux, macOS and Windows.

**Status: pre-alpha.** On Linux the whole chain runs: protocol, transport, host model, headless daemon, tray app with a settings window and the halo overlay. macOS and Windows backends do not exist yet.

## Try it now

Requires a Rust toolchain (`rustup`). On Linux, copy `packaging/linux/70-halokey.rules` to `/etc/udev/rules.d/` and replug the receiver, or temporarily `chmod a+rw /dev/hidraw*`.

```sh
cargo run -p halokey-cli -- list                 # find the receiver's vendor channel
cargo run -p halokey-cli -- status               # online state, battery, voltage
cargo run -p halokey-cli -- keys                 # the six on-device key slots
cargo run -p halokey-cli -- monitor --heartbeat  # decode vendor events while you press keys
cargo run -p halokey-cli -- set-key 0 kb 0x68    # write key 1 = F13, then read back (persists on the device)
cargo run -p halokey-cli -- run -v               # daemon: heartbeat, gestures, uinput injection (needs /dev/uinput)
cargo run -p halokey-cli -- export-keymap > my.json   # the device table as editable host keymap JSON
cargo run -p halokey-cli -- run --config docs/examples/knob-moves-pointer.json   # knob moves the mouse pointer
cargo run -p halokey-cli -- run --config docs/examples/switch-apps-and-macro.json # keys switch apps, knob press runs a macro
cargo run -p halokey-app                         # tray app: settings window + halo overlay, reads ~/.config/halokey/keymap.json
```

`docs/examples/halo-menu.json` puts a two-level halo menu on the knob press; copy it to `~/.config/halokey/keymap.json` or paste it into the settings window.

To install on Debian/Ubuntu: `cargo tauri build --bundles deb` in `apps/halokey`, then `sudo dpkg -i target/release/bundle/deb/halokey_*.deb` and replug the receiver so the bundled udev rule applies.

`set-key` writes the device's own table. Pick a slot you can afford to break first, and note the old value it prints.

## Why another one

Ulanzi Studio supports only Windows and macOS. Every community project so far is either macOS-only or Linux-only, and none of them offers the radial menu that Studio has. halokey starts over on Rust and Tauri 2 so that the device logic is written once and only thin platform adapters differ.

## Planned features

| Feature | State |
|---|---|
| Vendor channel (TEA-encrypted HID, report ID `0x55`) | **done**: `halokey-proto` + `halokey-hid`, verified on Linux `hidraw` |
| Key and knob remapping, layers, single / double / long press | **done** in `halokey-host`, ported from olanzi with its tests |
| Host actions: keyboard, mouse buttons, wheel, pointer movement, app switching, macros, shell | all but shell commands work on Linux X11 via `halokey run`; shell commands are planned |
| Radial "halo" menu: an eight-option overlay driven by the knob | **done** on Linux: `halos` in the keymap, nested menus, verified on GNOME/X11 |
| Lighting, noise reduction, standby timeouts, battery readout | lighting read/write, battery and versions done (verified: breathing works on all four lights); noise reduction read-only; standby timeouts planned |
| Tray / menu bar icon, launch at login, auto-update | tray icon and settings window done (`apps/halokey`, Tauri 2); login item and auto-update planned |

## Platform matrix

| Component | Linux | macOS | Windows |
|---|---|---|---|
| HID transport | `hidraw` via hidapi | IOKit via hidapi | Win32 HID via hidapi |
| Input injection | uinput | CGEvent | SendInput |
| App switching | X11 EWMH (done), Wayland per compositor | NSWorkspace | SetForegroundWindow |
| Overlay window | X11 ready, Wayland needs layer-shell | non-activating panel | no-activate window |

## Repository layout

```text
crates/proto     TEA, frame codec, command table, key table – pure, heavily unit-tested
crates/hid       hidapi transport, heartbeat, hotplug
crates/host      keymap model, gesture router, macros, action runner
crates/input     uinput / CGEvent / SendInput backends
crates/wm        app switching backends
crates/runtime   device service thread shared by the CLI and the app
apps/halokey     Tauri 2 shell: tray, settings window, halo overlay
docs/            design and protocol notes, English default with `.zh.md` twins
tools/           repository checks
```

## Relationship to other projects

- [olanzi](https://github.com/MrCroxx/olanzi) holds the reverse-engineering record and a native macOS app; halokey reuses its protocol documentation and keymap format.
- [vibed](https://github.com/ihavespoons/vibed) documented the mouse-page entry types recovered from the vendor SDK, which halokey confirmed on hardware.

halokey is an independent community project. It is not affiliated with or endorsed by Ulanzi or Kehwin, and it does not include or load any vendor binaries.

## Documentation

| # | Document | Content |
|---|---|---|
| 01 | [Architecture](docs/01-architecture.md) | crates, platform adapters, overlay, known risks |
| 02 | [Protocol Notes](docs/02-protocol-notes.md) | what halokey measured beyond the olanzi record |

Every document exists in English (default filename) and Chinese (`.zh.md`); `python3 tools/check_docs.py` verifies that the pairs stay in sync.

## License

[MIT](LICENSE)
