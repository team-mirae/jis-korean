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

## What VIA adds

VIA's key picker has every QMK keycode, including the ones Hansung's picker leaves out: カタカナ
(`KC_INT2`), ¥ (`KC_INT3`), ろ (`KC_INT1`), and **Any** for typing a keycode such as
`LT(1,KC_F18)` or `LGUI_T(KC_LNG2)` (tap 英数, hold ⌘). Whether the firmware acts on each of them
isn't known yet: it's Hansung's own firmware on a Nuvoton chip, not QMK, and it may store a keycode
it doesn't act on. Hansung's driver offers `LT` from firmware 4.4, so layer-tap should work;
mod-tap (`MT`, `LGUI_T`) and the International keys need testing on the board.

Tested on firmware 5.9.2 (read with Hansung's version command, 0xF0), which reports 4 layers.
