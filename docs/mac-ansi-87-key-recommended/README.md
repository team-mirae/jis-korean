# macOS: an ANSI 87-key keyboard set up as a JIS board (recommended)

You can set up a remappable ANSI tenkeyless (87-key) keyboard, such as a QMK/VIA board, to work
like a JIS keyboard with JIS Korean on macOS. It then runs Mode A (`2Set`) and switches languages
the way spec §8.2.1 recommends for JIS keyboards, with one key per language: カタカナ goes to
Korean, 英数 to English and かな to Japanese. The ANSI board has no ¥ or ろ key, and JIS Korean's
backup Korean Key is F18. Both are on the Fn layer.

It needs Karabiner-Elements, which turns the ANSI board into a JIS keyboard for macOS (step 1).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="layout-dark.png">
  <img alt="Mode A on an ANSI tenkeyless board. Layer 0: カタカナ, 英数 and かな on the bottom row, holding to Fn and ⌘; Mode A's number row, letters and punctuation, with the ANSI [ ] \ keys typing JIS's @ [ ] keys; 한글, 한자, 두벌식, 전각 and 반각 on F6–F10. Layer 2: F13–F24 on F1–F12, with F18 as the Korean Key, and ¥, / and ろ right of Space." src="layout-light.png">
</picture>

In this folder:
- `layout.html`, the drawing above. To redraw `layout-light.png` and `layout-dark.png`, render it at 3×
  with headless Chrome, the same way as `docs/layout`.
- `mongdol_87.layout.json`, the reference board's VIA keymap. Import it in VIA, or read it for the
  keycodes. Layers 0 and 2 are the ones described here.

It takes two layers: **layer 0**, the base layer, and **layer 2**, the Fn layer. The reference board
is a MONGDOL 87 BT-1 over Bluetooth, remapped in VIA. On that board, Windows mode sets the default
layer to 0. The other layers (1 and 3, its Mac mode) aren't used here. On another board, use
whichever layer is its default and any momentary layer for Fn.

If you'd rather keep the board ANSI as it is, use Mode B (`2SetANSI`, spec §3.6) instead. It needs
no remapping, but it has no JIS switch keys.

## 1. Keep the board ANSI and send it through Karabiner's JIS virtual keyboard

JIS Korean reads カタカナ only from keyboards macOS has stored as JIS (spec §8.2.1). Don't store the
board itself as JIS. Leave it ANSI (type 40), which is what Keyboard Setup Assistant usually picks
for it, and let Karabiner-Elements send its keys through its virtual keyboard, stored as JIS:

1. In Karabiner-Elements → Devices, find the board and turn on **Modify events** and **Ignore vendor
   events**.
2. Store Karabiner's virtual keyboard (`593-1452-0`) as JIS (42). Karabiner's own "Virtual Keyboard:
   JIS" setting doesn't do this. Check that the board itself is still 40, then restart JIS Korean:

```sh
defaults read /Library/Preferences/com.apple.keyboardtype        # the board (product-vendor-0) stays 40
sudo defaults write /Library/Preferences/com.apple.keyboardtype keyboardtype -dict-add "593-1452-0" -int 42
pkill -x JISKorean                                                 # relaunches on the next key in a text field
```

The ANSI board's keys then reach macOS from a JIS keyboard. JIS Korean reads カタカナ there
([#18](https://github.com/team-mirae/jis-korean/issues/18)), and ¥ and ろ type as JIS keys. Don't
add Karabiner rules that remap `japanese_pc_katakana`, `international1` or `international3`.

The reference board was tried stored as JIS too, without going through Karabiner, and it didn't
work. Neither カタカナ nor ¥ and ろ worked, although Karabiner-EventViewer showed the board sending
the right usages (0x88, 0x89 and 0x87). [Troubleshooting](../troubleshooting/macos-katakana-and-eisu.md)
has the log lines to check. Fn+F6 (F18, below) works however the board is set up.

## 2. Layer 0 (base)

Only the keys that differ from a stock ANSI TKL are listed. Keycodes are QMK/VIA names.

| Position | Keycode | Tap | Hold |
|---|---|---|---|
| Bottom-left (stock Ctrl) | `LT(2,KC_KANA)` | カタカナ (HID 0x88) → Korean | Fn (layer 2) |
| Second bottom-left key | `KC_LALT` | ⌥ | |
| Left of Space | `MT(MOD_LGUI,KC_HANJ)` | 英数 (HID LANG2) → English | ⌘ |
| Right of Space | `KC_HAEN` | かな (HID LANG1) → Japanese, handled by macOS | |
| Next two keys | `KC_RGUI`, `KC_RALT` | ⌘, ⌥ | |
| Rightmost bottom key | `MO(2)` | | Fn (layer 2) |
| Caps Lock | `KC_LCTL` | | Ctrl |

Ctrl to the left of A is HHKB-style. If you'd rather have Ctrl at the bottom left, swap the two
keys: Caps Lock's position gets `LT(2,KC_KANA)` (tap カタカナ, hold Fn) and the bottom-left key gets
`KC_LCTL`. Nothing else depends on where they are.

QMK's names for the language keys are Korean-centric. On macOS, `KC_HANJ` (LANG2) arrives as 英数
and `KC_HAEN` (LANG1) arrives as かな. `KC_KANA` is the Windows JIS カタカナ/ひらがな key, which
macOS doesn't have. JIS Korean reads that key at the hardware level, so Input Monitoring and
Accessibility must be granted (`mac/README.md`).

With a tap-hold key, the tap is sent when you release the key, so カタカナ and 英数 switch on
release.

The reference board also has a few extras that JIS Korean doesn't need:
- Print Screen sends `LSG(KC_4)` (⌘⇧4).
- Scroll Lock is `KC_CAPS`.
- Pause is Play/Pause.

## 3. Layer 2 (Fn)

| Position | Keycode | What it's for |
|---|---|---|
| F1 … F12 | `KC_F13` … `KC_F24` | **Fn+F6 = F18**, the default Korean Key (spec §8.2.1). It works without the JIS keyboard type. The others are free for shortcuts. |
| Right of Space | `KC_JYEN` | ¥ (`IntlYen`), for Mode A's ¥ key and the ー of Japanese Kana |
| Next key | `KC_SLSH` | / (`Slash`), the same as the plain / key |
| Next key | `KC_RO` | ろ (`IntlRo`), for Mode A's ろ key and Japanese Kana's ろ |
| Rightmost bottom key | `MO(2)` | stays Fn |

Everything else on the reference board's layer 2 (media, mouse, RGB keys and arrows on the right
hand) is the board's own and can be anything.

## 4. What the ANSI caps type as JIS

A JIS board sends the same key positions as ANSI wherever both have a key (spec §2). So once the
board's keys come through the JIS virtual keyboard, a few caps type the JIS key in their place:

| ANSI cap | JIS key | Canonical code |
|---|---|---|
| `[` | @ | `BracketLeft` |
| `]` | [ | `BracketRight` |
| `\` (above Enter) | ] | `Backslash` |
| `=` | ^ | `Equal` |
| `'` | : | `Quote` |
| `` ` `` | none (半角/全角's position) | `Backquote`, which isn't in Mode A (spec §3.6) |

The tables in spec §3 give what each of these types in Mode A. In JIS English the symbols are JIS
ones, not the ones on the caps: Shift+2 is `"`, for example.

## 5. Known issues

- With Apple's ABC enabled, 英数 selects ABC instead of JIS English. Use ABC – Extended as the plain
  layout instead ([#2](https://github.com/team-mirae/jis-korean/issues/2)).
- If you change Karabiner's device settings, restart JIS Korean (`pkill -x JISKorean`) so it reads
  the keyboards again.
