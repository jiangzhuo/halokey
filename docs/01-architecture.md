# 01 · Architecture
> 🌐 [中文](01-architecture.zh.md)

> 📚 Docs set: [README](../README.md) · **01 Architecture** · [02 Protocol Notes](02-protocol-notes.md) · [03 Configuration Format](03-config-format.md)

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
crates/host      HostKeymap model (JSON shape derived from olanzi, version 6 only), gesture router
                 (single / double / long press, knob ticks), four layers, QMK-style macros,
                 action runner. Pure logic, deterministic, unit-tested with a fake clock.
crates/input     trait Injector { key_down, key_up, mouse_button, wheel, pointer_move }
                 Linux: uinput · macOS: CGEvent · Windows: SendInput
crates/wm        trait Launcher { launch(app) -> Activation, is_frontmost(app), poll() }
                 Linux: X11 EWMH now, Wayland per compositor later · macOS: NSWorkspace · Windows: Win32
crates/runtime   the device service: one thread owning transport, heartbeat, router, injector,
                 runner; commands in, snapshots and events out. Shared by the CLI and the app.
apps/halokey     Tauri 2: tray, settings window (webview), halo overlay window, CLI subcommands
ui/              Svelte 5 + Vite front end for both windows (`index.html`, `halo.html` → `ui/dist`)
```

Dependency direction is strictly downward: `apps` → `runtime` → `host` → `proto`; `hid`, `input`, `wm` are leaves that `runtime` wires together.

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
- Platform notes: Linux X11 works as-is. **Wayland, tested 2026-10-20 in a nested GNOME Shell (mutter 46) session**: the settings window and the halo overlay both work (the user saw no visible problem on knob press); GTK cannot pin or position windows on Wayland, so the overlay sits where the compositor puts it. Window listing and app switching still go through XWayland (X11 clients only). macOS needs a non-activating panel; Windows needs a no-activate window style.

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
| WebKitGTK renders a blank window with the DMA-BUF renderer on NVIDIA | the app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` at startup unless already set |
| `/dev/hidraw` and `/dev/uinput` are root-only by default | ship a udev rule and a user-group instruction |
| Vendor event mode leaves the device silent if the host crashes | heartbeat watchdog, firmware timeout recovery documented from community reports |

## 7. Milestones

1. `proto` with unit tests from the evidence logs; `hid` reads the six-slot table on all three platforms. **Linux done 2026-10-16**; macOS and Windows still to be run on hardware.
2. Headless daemon: heartbeat, vendor events, keyboard and mouse injection, olanzi keymap import. **Linux: `halokey run` verified on hardware 2026-10-16** (key 1 typed `f`, key 2 sent Enter, knob scrolled a page, and with `docs/examples/knob-moves-pointer.json` the knob moved the pointer)). App switching and macros verified on GNOME/X11 2026-10-16 with `docs/examples/switch-apps-and-macro.json`: keys raised Text Editor and Chrome, knob press switched to the editor and typed `hi`. **Milestone 2 complete on Linux.**
3. Tray and settings window. **Done on Linux 2026-10-16** (`apps/halokey`, Tauri 2.12, vanilla HTML/JS, JSON editor plus a form-based action and halo editor, device panel, autostart toggle, write-to-device backup). **Front end rebuilt 2026-10-17** (phase 1 of the redesign): Svelte 5 + Vite + TypeScript in `ui/`, olanzi-style keymap page (layer bar, product render, gesture cards, categorised palette, save/undo with Rust-side validation) and a read-only settings page; per-layer backlight, halo editor and the remaining settings writes follow in later phases.
4. Halo overlay. **Done on Linux 2026-10-16**: non-focusable transparent window driven by runtime events; rotation selects, press confirms, keys cancel, 4 s idle closes; nested halos verified. **Wheel redesign + halo editor 2026-10-18** (phase 3): Surface-Dial style wedge wheel with size / position / timeout settings; full-page halo editor. **Combo recording, APP and macro editors 2026-10-18** (phase 4). **Japanese UI, English by default 2026-10-18** (phase 5).
5. Lighting, microphone, power settings; packaging, signing, auto-update. **Lighting read/write and device info done 2026-10-16**; `cargo tauri build` produces a `.deb` with the udev rule. Microphone and standby writes done; signing and auto-update remain.
