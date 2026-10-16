# 01 · Architecture
> 🌐 [中文](01-architecture.zh.md)

> 📚 Docs set: [README](../README.md) · **01 Architecture** · [02 Protocol Notes](02-protocol-notes.md)

Design notes for halokey. Everything here is a plan unless marked **[CONFIRMED]**, which means it was measured on hardware.

---

## 1. Goals

1. One codebase for Linux, macOS and Windows; platform-specific code lives only in adapter crates.
2. No vendor binaries: the TEA protocol is implemented from the public reverse-engineering record.
3. Host-side execution: while halokey runs, the device sends raw control events and the host decides what happens, which is what makes layers, gestures, the halo menu and pointer movement possible.
4. The device keeps working as a plain keyboard when halokey is not running, using whatever six-slot table was last written to it.

## 2. Crates

```text
crates/proto     TEA (32 rounds, ECB, little-endian halves), 64-byte frames, command table,
                 key-config codec, entry types 0x02 / 0x03 / 0x07. No I/O, no platform code.
crates/hid       hidapi transport. Opens interface 3 (usage page 0xFFFC), writes report ID 0x55
                 + 63 bytes, reads 64-byte replies, runs the heartbeat, polls for hotplug.
crates/host      HostKeymap model (JSON-compatible with olanzi), gesture router
                 (single / double / long press, knob ticks), four layers, QMK-style macros,
                 action runner. Pure logic, deterministic, unit-tested with a fake clock.
crates/input     trait Injector { key_down, key_up, mouse_button, wheel, pointer_move }
                 Linux: uinput · macOS: CGEvent · Windows: SendInput
crates/wm        trait WindowManager { activate(app), frontmost() }
                 Linux: X11 EWMH now, Wayland per compositor later · macOS: NSWorkspace · Windows: Win32
apps/halokey     Tauri 2: tray, settings window (webview), halo overlay window, CLI subcommands
```

Dependency direction is strictly downward: `apps` → `host` → `proto`; `hid`, `input`, `wm` are leaves that `apps` wires together.

## 3. Data flow

```text
device ──vendor event──▶ hid ──▶ host (gesture router, layers) ──▶ action
                                                                   ├─▶ input (keys, mouse)
                                                                   ├─▶ wm (switch app)
                                                                   ├─▶ halo overlay (show / rotate / pick)
                                                                   └─▶ shell command
```

The heartbeat keeps the device in vendor-event mode. When halokey stops, suspends, or the machine sleeps, it hands control back so the firmware types its stored table again.

## 4. The halo overlay

- A second Tauri window: transparent, undecorated, always on top, skipped in the taskbar, **not focusable**. It must never steal focus, because the chosen action is injected into whatever application was active when the knob was first turned.
- Knob ticks rotate the selection, knob press confirms, a timeout or key press cancels.
- Eight options per halo; each option is any host action, so a halo can also open another halo.
- Platform notes: Linux X11 works as-is, Wayland needs the layer-shell protocol and GNOME does not support it; macOS needs a non-activating panel; Windows needs a no-activate window style.

## 5. Protocol facts carried over

| Fact | Source |
|---|---|
| **[CONFIRMED]** TEA key, delta `0x9E3779B9`, 32 rounds, `TEA_Enc(00×8) == 38 90 c4 99 a3 60 aa ad` | olanzi docs 02 |
| **[CONFIRMED]** Plaintext is 64 bytes, only the first 63 go on the wire after the report ID; replies decrypt only the first 56 bytes | olanzi docs 02 |
| **[CONFIRMED]** Key table read `01 06 50 01 <index>`, write `01 06 50 04 <index> 01 <num> <type> <code>`, ack `81 06 50 14` | olanzi docs 02 §6 |
| **[CONFIRMED]** Entry type `0x07` is the mouse page: code N = button N+1, `0x05` / `0x06` = wheel up / down, no pointer movement | olanzi docs 05 §10, measured on Linux 2026-10-16 |
| **[CONFIRMED]** Linux exposes interface 3 as `/dev/hidraw`; reads work without a heartbeat | olanzi docs 05 §10 |

## 6. Known risks

| Risk | Plan |
|---|---|
| Windows HID requires the output report to be exactly the descriptor length | pad to 64 bytes including the report ID, verify on hardware before release |
| macOS: `IOHIDManagerOpen` is all-or-nothing | hidapi opens each device with `IOHIDDeviceOpen`; verify, never regress to the manager call |
| Wayland has no always-on-top | layer-shell where available, documented degradation elsewhere |
| `/dev/hidraw` and `/dev/uinput` are root-only by default | ship a udev rule and a user-group instruction |
| Vendor event mode leaves the device silent if the host crashes | heartbeat watchdog, firmware timeout recovery documented from community reports |

## 7. Milestones

1. `proto` with unit tests from the evidence logs; `hid` reads the six-slot table on all three platforms. **Linux done 2026-10-16**; macOS and Windows still to be run on hardware.
2. Headless daemon: heartbeat, vendor events, keyboard and mouse injection, olanzi keymap import. **Linux: `halokey run` verified on hardware 2026-10-16** (key 1 typed `f`, key 2 sent Enter, knob scrolled a page, and with `docs/examples/knob-moves-pointer.json` the knob moved the pointer); macro and app-switch execution still pending.
3. Tray and settings window.
4. Halo overlay.
5. Lighting, microphone, power settings; packaging, signing, auto-update.
