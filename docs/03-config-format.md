# 03 · Configuration Format
> 🌐 [中文](03-config-format.zh.md)

> 📚 Docs set: [README](../README.md) · [01 Architecture](01-architecture.md) · [02 Protocol Notes](02-protocol-notes.md) · **03 Configuration Format**

halokey keeps one JSON file, `~/.config/halokey/keymap.json` on Linux. The settings window edits it; `halokey run --config` and `halokey export-keymap` use the same shape. The format is olanzi's `HostKeymap` with two halokey extensions, so files exported from olanzi import unchanged.

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
| `version` | 1–6, chosen from the content on save. 1–5 are olanzi's; 6 means halokey extensions are present (`halos`, `halo` or `pointerMove` actions, mouse-page entries). olanzi rejects version 6 on purpose. |
| `controls` | Exactly six, one per control index: 0 key 1, 1 key 2, 2 key 3, 3 knob press, 4 knob right, 5 knob left. |
| `layers` | Optional sparse overlays 1–3, activated by `momentaryLayer` actions. |
| `actionLibrary` | Optional reusable application or macro actions referenced by `library` actions. |
| `halos` | Optional radial menus, see §4. |
| `lighting` | Optional per-layer backlight: `{ "lights": [ { "mode": "off|solid|breathe", "brightness": 0–20 } ×4 ] }` in the order key 1, key 2, key 3, knob. Also allowed inside each `layers[]` entry. The runtime writes the highest active layer's record (falling back to the top level) to the device's four indicator lights, keeping `work_time` / `breathe_level` and the global mode / brightness untouched. Omitted everywhere = never touch the lights. Requires version 6. |
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

A keyboard entry is `{ "type": 2, "code": <HID usage> }`; modifiers are `0xE0`–`0xE7`; code `0` means no action. A mouse-page entry is `{ "type": 7, "code": n }` with `0`–`4` for buttons 1–5 and `5` / `6` for wheel up / down.

Actions use olanzi's tagged form `{ "<kind>": { "_0": <value> } }`:

| Kind | Value | Notes |
|---|---|---|
| `keyboard` | entry list | Same as the plain entry list. |
| `application` | `{ "bundleIdentifier", "path", "name" }` | Linux X11: `bundleIdentifier` matches `WM_CLASS` or is a desktop-file id for `gtk-launch`; `path` an executable. macOS: bundle id / `.app` path / name. Windows: executable name / path. |
| `macro` | list of steps | Steps are `keyboard`, `application` or `delay` (`{ "delay": { "_0": 0.3 } }`, 0.05–10 s), 1–32 of them. |
| `library` | UUID string | References `actionLibrary`. |
| `momentaryLayer` | 1–3 | Keys and knob press only, single or long press. |
| `pointerMove` | `{ "dx", "dy" }` | halokey extension. Pixels per trigger, each within ±1000, not both zero. |
| `halo` | halo id | halokey extension. Opens that menu. |

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

One to eight options; labels and ids 1–80 characters; option actions may be anything except `library` and `momentaryLayer`. While a halo is open the knob rotates the selection, knob press confirms, any key cancels, and four idle seconds close it.

## 5. Examples

- [`examples/knob-moves-pointer.json`](examples/knob-moves-pointer.json): knob moves the pointer.
- [`examples/switch-apps-and-macro.json`](examples/switch-apps-and-macro.json): keys switch apps, knob press runs a macro.
- [`examples/halo-menu.json`](examples/halo-menu.json): a two-level halo on knob press.
- [`examples/layer-backlight.json`](examples/layer-backlight.json): hold key 1 to breathe all keys (per-layer backlight).
