# 02 · Protocol Notes
> 🌐 [中文](02-protocol-notes.zh.md)

> 📚 Docs set: [README](../README.md) · [01 Architecture](01-architecture.md) · **02 Protocol Notes**

Facts about the AU05 vendor channel that halokey established itself, on top of the olanzi record. Everything here was measured on Linux through `/dev/hidraw` with the `halokey` CLI unless stated otherwise. Raw logs live in [evidence/](evidence/).

---

## 1. Linux transport

[CONFIRMED] hidapi's pure-Rust hidraw backend enumerates the receiver with usage pages: interface 2 exposes usage pages `0x000c`, `0x0001` (usages 1, 2, 6), interface 3 exposes `0xfffc` usage 1. halokey selects the vendor channel by usage page and falls back to interface number 3 only when a backend reports no usage page.

[CONFIRMED] Reads need no heartbeat: `status` and `keys` answered within milliseconds on a device that had been idle for minutes.

## 2. Heartbeat echo

[CONFIRMED] Every heartbeat `06 01 23 00 01` sent by the host comes back verbatim from the receiver about 1 ms later, with neither the reply bit nor the access nibble set. olanzi listed the heartbeat as having no verified acknowledgement; this echo is that acknowledgement. 119 echoes for 119 heartbeats in [2026-10-16-linux-vendor-events.log](evidence/2026-10-16-linux-vendor-events.log). A missing echo is therefore a usable receiver-health signal, which the daemon should watch.

## 3. Vendor key events on all six controls

[CONFIRMED] With the heartbeat running, each control produced `8b 10 <action> <pressed> <index>`:

| Control | index | action | Frames per gesture |
|---|---|---|---|
| Key 1 | `00` | `6f` | press + release |
| Key 2 | `01` | `70` | press + release |
| Key 3 | `02` | `71` | press + release |
| Knob press | `03` | `6e` | press + release |
| Knob → right | `04` | `72` | **one `pressed = 1` frame per detent, no release** |
| Knob ← left | `05` | `73` | **one `pressed = 1` frame per detent, no release** |

The action byte is stable per control but is not a key code; hosts route by `index`. Rotation produced two frames per felt detent in this capture (6 frames for 3 clicks, 8 for 4), so the host must treat each frame as one step and never wait for a release.

## 4. Battery broadcast (notice `0x7b`)

[INFERRED] The device broadcasts `0b 7b <v_lo> <v_hi> 00 <percent> <?> 01` every few seconds without being asked. In four samples the little-endian voltage at bytes 2–3 and the percentage at byte 5 matched the polled `01 01 02 01` reading exactly (`cf 0e` / `d0 0e` = 3791 / 3792 mV, `3c` = 60%; earlier `15 0f` = 3861 mV, `46` = 70%). Byte 6 (`c6`, `c1`, `c0`, `bb`) and byte 7 (always `01`) are unexplained, so halokey does not derive a charging flag from the notice. `halokey-proto` exposes this as `status::parse_battery_notice`.

## 5. Mouse page (type `0x07`)

[CONFIRMED] See olanzi docs 02 §6 and 05 §10: key code N is mouse button N+1, `0x05` / `0x06` are wheel up / down, and there is no pointer-movement entry. `halokey keys` renders these as `鼠标按键 N` / `滚轮↑` / `滚轮↓`.

## 6. Open questions

- Whether the standard HID interface stays silent while the heartbeat runs on Linux (it does on macOS). Not yet captured on this platform.
- The meaning of bytes 6–7 of the battery notice.
- Whether the receiver keeps echoing heartbeats while the key unit is powered off (would distinguish receiver health from key-unit presence).
