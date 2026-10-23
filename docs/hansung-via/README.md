# Hansung keyboards in VIA

Hansung's magnetic and custom keyboards (TFG Magnetox, GK… PRO V2, Archon, CloudMax) are set up
with Hansung's own web driver, [app.superstore.co.kr](https://app.superstore.co.kr). Underneath, it
talks to the keyboard with VIA's raw-HID protocol (usage page 0xFF60, protocol version 12) and
QMK's keycodes, plus Hansung's own commands for the magnetic-switch features. So
[VIA](https://usevia.app) can edit the keymap too, given a definition for the keyboard.

[tfg-magnetox-xf.json](tfg-magnetox-xf.json) is that definition for the **TFG Magnetox XF**
(vendor 0x0416, product 0x8144): the 104 keys, the 7 × 16 matrix and the lighting menu, converted
from Hansung's own definition (`app.superstore.co.kr/definitions/b/68583748.json`) to VIA's v3
format. The positions round-trip exactly, and reading the keyboard's keymap through VIA's protocol
put every key where the layout draws it.

## Using it

1. Open [usevia.app](https://usevia.app) in Chrome or Edge (it needs WebHID).
2. Settings → turn on **Show Design tab**.
3. Design → **Load Draft Definition** → choose `tfg-magnetox-xf.json`.
4. Configure → **Authorize device** → the TFG Magnetox XF.

Loading a definition writes nothing to the keyboard. Keymap changes made in VIA use VIA's standard
keymap commands, the same ones Hansung's driver uses, so both keep working on the same keymap.

## What stays in Hansung's driver

Actuation, Rapid Trigger, DKS, calibration, ModTap/Toggle keys, polling rate, and firmware
updates. VIA doesn't know Hansung's commands for them.

## What VIA adds, and what the firmware does with it

VIA's key picker has every QMK keycode, including the ones Hansung's picker leaves out, and **Any**
for typing a keycode. The keyboard stores any of them. Hansung's firmware isn't QMK, though (it runs
on a Nuvoton chip), and acts on some of them differently. Tested on the XF:

| Keycode | Result |
|---|---|
| Plain keys, `LGUI(LSFT(KC_4))` (⌘⇧4) and other modifier combos | Work |
| `LT(1,KC_KANA)` (tap カタカナ, hold Fn) | Works |
| `MO(1)` | Works |
| `KC_INT1` ろ, `KC_INT2` カタカナ, `KC_INT3` ¥ | Work (with macOS seeing the board as JIS) |
| `KC_LNG1` (かな), `KC_LNG2` (英数) | Work |
| `KC_INT4` 変換, `KC_INT5` 無変換, `KC_LNG3`–`KC_LNG5` | Sent, but macOS turns none of them into 英数 |
| **`MT()` / `LGUI_T()` and the other mod-taps** | **Broken.** Held, they add right Shift to the modifier (`LGUI_T` + Tab is ⌘⇧Tab, `LCTL_T` + Tab is ⌃⇧Tab). Tapped, the tap key goes out with the modifier still held (tapping `LCTL_T(KC_Z)` sends ⌃Z). |

The mod-tap behaviour fits the firmware reading the keycode's upper byte (0x20–0x3F) as a raw USB
modifier byte, where 0x20 is right Shift, and pressing the modifier as soon as the key goes down.
Making macOS ignore right Shift from the keyboard (`hidutil`, with the keyboard's own right Shift
set to left Shift) fixes the hold, but not the tap, so there's no workaround in firmware. Use
layer-tap for tap vs hold, or plain keys; for a key that taps 英数 and holds ⌘, use Karabiner, as
in [the HHKB](../legacy-hhkb-us-as-jis/README.md) and [MacBook](../macbook-us-as-jis/README.md)
guides.

All of this was on firmware 5.9.2 (read with Hansung's version command, 0xF0), with 4 layers.
