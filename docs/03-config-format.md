# 03 · Configuration Format
> 🌐 [中文](03-config-format.zh.md)

> 📚 Docs set: [README](../README.md) · [01 Architecture](01-architecture.md) · [02 Protocol Notes](02-protocol-notes.md) · **03 Configuration Format**

halokey keeps one JSON file, `~/.config/halokey/keymap.json` on Linux. The settings window edits it; `halokey run --config` and `halokey export-keymap` use the same shape. The shape is derived from olanzi's `HostKeymap` but the file is halokey's own: version is always 6 and olanzi files are not accepted.

---

## 1. Top level

```json
{
  "version": 6,
  "controls": [ … six entries, index 0–5 … ],
  "layers": [ { "id": 1, "controls": [ … ] } ],
  "actionLibrary": [ { "id": "UUID", "name": "…", "slot": 0, "action": { … } } ],
  "halos": [ { "id": "main", "name": "主菜单", "options": [ … ] } ],
  "doublePressWindow": 0.25,
  "longPressThreshold": 0.5
}
```

| Field | Meaning |
|---|---|
| `version` | Always `6`. Any other value is rejected (halokey has not shipped, so there is no older format to keep reading). |
| `controls` | Exactly six, one per control index: 0 key 1, 1 key 2, 2 key 3, 3 knob press, 4 knob right, 5 knob left. |
| `layers` | Optional sparse overlays 1–3, activated by the layer actions in §3 (`momentaryLayer`, `toggleLayer`, `toLayer`, `oneShotLayer`). |
| `actionLibrary` | Optional reusable application or macro actions referenced by `library` actions. |
| `halos` | Optional radial menus, see §4. |
| `lighting` | Optional per-layer backlight: `{ "lights": [ { "mode": "off|solid|breathe", "brightness": 0–20 } ×4 ] }` in the order key 1, key 2, key 3, knob. Also allowed inside each `layers[]` entry. The runtime writes the highest active layer's record (falling back to the top level) to the device's four indicator lights, setting each lit light's `work_time` to 255 (in work mode a light shows its effect for `work_time` seconds; 255 keeps it on), keeping `breathe_level` and the global brightness untouched. Layer backlight is applied only while the device's global light mode is "work"; "all off" and "all on" (chosen on the Settings page) are a master switch the runtime never overrides. Key 1's LED is not visible on the AU05, so only keys 2, 3 and the knob show anything. Brightness 2 vs 20 is clearly visible on firmware 4.4.2 (verified 2026-10-18). Every layer write re-sends `kind` and `work_time` for each lit light, because the firmware only (re)starts the lights written in that batch and interrupts the others. Omitted everywhere = never touch the lights. Requires version 6. |
| `doublePressWindow` | 0.15–0.5 s. |
| `longPressThreshold` | 0.3–2 s, must exceed the double-press window. |

## 2. A control

```json
{
  "index": 3,
  "press": [ { "type": 2, "code": 42 } ],
  "pressAction": { "halo": { "_0": "main" } },
  "doublePress": [ { "type": 2, "code": 40 } ],
  "longPress": null,
  "pressBehavior": "hold",
  "pressTapCount": 2,
  "doublePressBehavior": "tap",
  "doublePressTapCount": 2,
  "longPressBehavior": "hold",
  "longPressTapCount": 2,
  "inheritedGestures": []
}
```

- `press` is the keyboard entry list for a single press; `pressAction` overrides it when present. The same pair exists for `doublePress` / `longPress`; a missing pair disables that gesture.
- Knob rotation (index 4, 5) only has a press action and no behaviors other than `hold`.
- Behaviors: `hold` follows the physical press, `tap` sends one pulse, `burst` sends `…TapCount` pulses (2–20). Double press cannot be `hold`.
- `inheritedGestures` is only valid inside `layers`; listed gestures fall through to lower layers.

## 3. Entries and actions

A keyboard entry is `{ "type": 2, "code": <HID usage> }`; modifiers are `0xE0`–`0xE7`; code `0` means no action. A mouse-page entry is `{ "type": 7, "code": n }` with `0`–`4` for buttons 1–5 and `5` / `6` for wheel up / down, one notch per trigger (use the `scroll` action for more).

Actions use olanzi's tagged form `{ "<kind>": { "_0": <value> } }`:

| Kind | Value | Notes |
|---|---|---|
| `keyboard` | entry list | Same as the plain entry list. |
| `application` | `{ "bundleIdentifier", "path", "name" }` | Linux X11: `bundleIdentifier` matches `WM_CLASS` or is a desktop-file id for `gtk-launch`; `path` an executable. macOS: bundle id / `.app` path / name. Windows: executable name / path. |
| `macro` | list of steps | Steps are `keyboard`, `application` or `delay` (`{ "delay": { "_0": 0.3 } }`, 0.05–10 s), 1–32 of them. |
| `library` | UUID string | References `actionLibrary`. |
| `momentaryLayer` | 1–3 | QMK `MO`. Active while the control is held. Keys and knob press only, single or long press. |
| `toggleLayer` | 1–3 | halokey extension, QMK `TG`. Turns the layer on, or off if it is already on; it stays after release. Keys and knob press, any gesture. |
| `toLayer` | 0–3 | halokey extension, QMK `TO`. Leaves only this layer locked (`0` = back to Layer 0) and drops a pending one-shot layer; layers held by `momentaryLayer` keys stay until those keys are released. Keys and knob press, any gesture. |
| `oneShotLayer` | 1–3 | halokey extension, QMK `OSL`. The next key or knob press uses the layer, then it drops. If the knob turns first, the whole continuous turn uses it; the layer drops 1 s after the last tick or at the next press. Keys and knob press, any gesture. |
| `pointerMove` | `{ "dx", "dy" }` | halokey extension. Pixels per trigger, each within ±1000, not both zero. |
| `scroll` | `{ "dx", "dy" }` | halokey extension. Wheel notches per trigger in screen directions (`dy` > 0 scrolls down, `dx` > 0 right), each within ±10, not both zero. A knob tick sends them as one event, so fast turns do not queue up the way `burst` does; how far one notch scrolls is still up to the system and each app. |
| `halo` | halo id | halokey extension. Opens that menu. |

The active layers are the union of layers held by `momentaryLayer`, layers locked by `toggleLayer` / `toLayer`, and the pending one-shot layer; where several define the same gesture, the higher-numbered layer wins. Locked layers survive the idle hand-back and reconnects and are cleared when a new keymap is saved; the one-shot layer is dropped on idle, disconnect and save. The AU05 has no screen, so give each locked layer its own `lighting` to see which one is on, and give Layer 0 a record too, or the lights keep the last layer's effect after returning to it.

The settings window edits `actionLibrary` through the APP and Macro editors (the APP editor can pick WM_CLASS / executable from the current X11 windows via `list_windows`), and records key combinations from the real keyboard (modifiers `0xE0`–`0xE7` first, then the main key). Macro steps copy the application inline; they never reference the library.

## 4. Halos

```json
{
  "id": "main",
  "name": "主菜单",
  "options": [
    { "label": "Chrome", "action": { "application": { "_0": { "bundleIdentifier": "google-chrome", "path": "", "name": "Chrome" } } } },
    { "label": "更多…", "action": { "halo": { "_0": "more" } } }
  ]
}
```

One to eight options; labels and ids 1–80 characters; option actions may be anything except `library` and the layer actions (`momentaryLayer`, `toggleLayer`, `toLayer`, `oneShotLayer`). While a halo is open the knob rotates the selection, knob press confirms, any key cancels, and idle seconds close it (4 by default, set in the settings window).

The settings window's halo editor writes only slots that have an action, in slot order. The on-screen wheel's position, timeout and size live in `settings.json` (`halo` section), not in the keymap.

## 5. Examples

- [`examples/knob-moves-pointer.json`](examples/knob-moves-pointer.json): knob moves the pointer.
- [`examples/switch-apps-and-macro.json`](examples/switch-apps-and-macro.json): keys switch apps, knob press runs a macro.
- [`examples/halo-menu.json`](examples/halo-menu.json): a two-level halo on knob press.
- [`examples/layer-backlight.json`](examples/layer-backlight.json): hold key 1 to breathe all keys (per-layer backlight).
- [`examples/knob-modes.json`](examples/knob-modes.json): knob press cycles the knob through scroll, arrow keys and undo / redo with `toLayer` (the knob light shows the mode); key 1 is a `oneShotLayer` whose next knob turn switches browser tabs.

## 6. settings.json

Application preferences live next to the keymap in `settings.json` and never enter the keymap file:

| Key | Value | Default |
|---|---|---|
| `idleMinutes` | minutes of inactivity before halokey hands the device back to its own firmware; `0` = never | `5` |
| `halo.position` | `pointer` or `center` | `pointer` |
| `halo.timeoutSeconds` | seconds before the halo wheel closes | `4` |
| `halo.size` | `small`, `medium`, `large` | `medium` |
| `closeToTray` | closing the settings window hides it to the tray (`true`) or quits (`false`) | `true` |

Language and appearance are per-machine browser preferences, not part of this file.
