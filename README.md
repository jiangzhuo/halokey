# halokey

> 🌐 [中文](README.zh.md)

Cross-platform, open-source control center for the Ulanzi Vibe Key (AU05): remap the three keys and the knob, run host-side actions, pick from a radial on-screen menu, and manage lighting, microphone and power settings. One codebase for Linux, macOS and Windows.

**Status: pre-alpha, design phase.** The protocol is verified; no application code exists yet.

## Why another one

Ulanzi Studio supports only Windows and macOS. Every community project so far is either macOS-only or Linux-only, and none of them offers the radial menu that Studio has. halokey starts over on Rust and Tauri 2 so that the device logic is written once and only thin platform adapters differ.

## Planned features

| Feature | State |
|---|---|
| Vendor channel (TEA-encrypted HID, report ID `0x55`) | protocol verified on Linux `hidraw` and macOS IOKit |
| Key and knob remapping, layers, single / double / long press | design carried over from olanzi |
| Host actions: keyboard, mouse buttons, wheel, pointer movement, app switching, macros, shell | planned |
| Radial "halo" menu: an eight-option overlay driven by the knob | planned |
| Lighting, noise reduction, standby timeouts, battery readout | protocol known, planned |
| Tray / menu bar icon, launch at login, auto-update | planned |

## Platform matrix

| Component | Linux | macOS | Windows |
|---|---|---|---|
| HID transport | `hidraw` via hidapi | IOKit via hidapi | Win32 HID via hidapi |
| Input injection | uinput | CGEvent | SendInput |
| App switching | X11 EWMH, Wayland per compositor | NSWorkspace | SetForegroundWindow |
| Overlay window | X11 ready, Wayland needs layer-shell | non-activating panel | no-activate window |

## Repository layout (planned)

```text
crates/proto     TEA, frame codec, command table, key table – pure, heavily unit-tested
crates/hid       hidapi transport, heartbeat, hotplug
crates/host      keymap model, gesture router, macros, action runner
crates/input     uinput / CGEvent / SendInput backends
crates/wm        app switching backends
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

Every document exists in English (default filename) and Chinese (`.zh.md`); `python3 tools/check_docs.py` verifies that the pairs stay in sync.

## License

[MIT](LICENSE)
