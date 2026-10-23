# macOS: an HHKB Professional BT (US) as a JIS keyboard

The HHKB Professional BT (US layout) has 60 keys and can't be remapped beyond its DIP switches.
Karabiner-Elements gives it every key a JIS board has, mostly as tap vs hold on the modifiers. JIS
Korean then runs Mode A (`2Set`) on it, with one key per language as spec §8.2.1 recommends for JIS
keyboards: カタカナ to Korean, 英数 to English, かな to Japanese.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="layout-dark.png">
  <img alt="Mode A on an HHKB US remapped as JIS. Left ◇, left ◆ and right ◆ tap to カタカナ, 英数 and かな and hold to ⌥ and ⌘; right Shift taps to ろ; \ types ¥, Delete types JIS's ], and ` is Backspace. The Fn layer has F1–F12 on 1 to =, with 한글, 한자, 두벌식, 전각 and 반각 on F6–F10, and Delete on Fn+`." src="layout-light.png">
</picture>

In this folder:
- `layout.html`, the drawing above. To redraw `layout-light.png` and `layout-dark.png`, render it at 3×
  with headless Chrome, the same way as `docs/layout`.
- `hhkb-pro-bt-as-jis.json`, the Karabiner-Elements rules.

For a keyboard whose firmware can be remapped, see
[the 87-key guide](../mac-ansi-87-key-recommended/README.md) instead: it needs no software.

## The keys

| HHKB key | Tap | Hold |
|---|---|---|
| Left ◇ (⌥) | カタカナ (`japanese_pc_katakana`) → Korean | ⌥ |
| Left ◆ (⌘) | 英数 (`japanese_eisuu`) → English | ⌘ |
| Right ◆ (⌘) | かな (`japanese_kana`) → Japanese, handled by macOS | ⌘ |
| Right ◇ (⌥) | | Fn, a second one, where an HHKB JIS has it |
| Right Shift | ろ (`international1`), where JIS has it: right of / | Shift |
| `\` (top row, left of `` ` ``) | ¥ (`international3`), where JIS has it: left of Backspace | |
| Delete (right of `]`) | JIS ] (`backslash`): ] in Mode A, む in Kana | |
| `` ` `` (top right) | Backspace, where an HHKB JIS has it | |

A tap is a press and release with no other key in between. F6–F10 are Fn+6 to Fn+0, with either Fn.

**Backspace and む swap places.** An HHKB US has no key for JIS's ] (む in Kana), so it takes
Delete's place, and Backspace moves to the top-right key. That's where both are on an HHKB JIS, and
standard ANSI keyboards have Backspace top right too (with \ where the HHKB has Delete). Forward
Delete stays on Fn+`` ` ``, which the HHKB sends as Delete itself, so the top-right key is Backspace
alone and Delete with Fn, as on an HHKB JIS.

Only touch typists with years of HHKB US Backspace habits will feel it. For them the rule file has
an alternative that keeps Delete as Backspace and puts JIS ] on `` ` `` instead: enable that instead
of the recommended one. It's easy to switch back and forth.

The other caps type the JIS key at their position, as on any ANSI board seen as JIS: the HHKB's
`[` `]` type JIS's @ [, `=` types ^, and `'` types :. See section 4 of
[the 87-key guide](../mac-ansi-87-key-recommended/README.md#4-what-the-ansi-caps-type-as-jis).

## The Fn layer

The HHKB's Fn layer is in its firmware: Fn never reaches the computer, and Fn+6 arrives as a plain
F6. Right ◇ becomes a second Fn key, where an HHKB JIS has its Fn. Karabiner can't use the HHKB's
firmware layer, so the rule sends each of the HHKB US Fn layer's keys itself:

| Fn + | Sends |
|---|---|
| 1 … 0, -, = | F1 … F12 (F6–F10: 한글, 한자, 두벌식, 전각, 반각) |
| `\` / `` ` `` | Insert / forward Delete |
| Tab | Caps Lock |
| I / O / P | Print Screen / Scroll Lock / Pause, or with the Mac extras ⌘⇧4 / Scroll Lock / Play/Pause |
| [ ; ' / | ↑ ← → ↓ |
| K L , . | Home, Page Up, End, Page Down |
| H J N M | keypad * / + − |
| Return | keypad Enter |
| A S D F | volume down, volume up, mute, eject |

The rule file has two versions: **[Fn: standard]**, the HHKB's own keys, and **[Fn: Mac extras]**,
with ⌘⇧4 (a screenshot of a selection) on I and Play/Pause on P, since macOS mostly ignores Print
Screen and Pause. Enable one, not both. Fn+Esc (power) is left out.

## Setup

1. **DIP switches:** Mac mode (SW1 off, SW2 on), so ◇ is ⌥ and ◆ is ⌘, and SW6 on (wake up
   enable). All the others off.
2. **Karabiner's virtual keyboard as JIS.** Karabiner sends the HHKB's keys through its virtual
   keyboard, and macOS has to take that keyboard for JIS (42). Karabiner's own "Virtual Keyboard:
   JIS" setting doesn't do this:

   ```sh
   defaults read /Library/Preferences/com.apple.keyboardtype          # 593-1452-0 should be 42
   sudo defaults write /Library/Preferences/com.apple.keyboardtype keyboardtype -dict-add "593-1452-0" -int 42
   ```

3. **The rules.** Copy [hhkb-pro-bt-as-jis.json](hhkb-pro-bt-as-jis.json) to
   `~/.config/karabiner/assets/complex_modifications/`, then Karabiner-Elements → Complex
   Modifications → Add predefined rule. Enable:
   - the **[required]** rule;
   - **[recommended]** (JIS ] on Delete, Backspace on `` ` ``) or **[alternative]** (JIS ] on
     `` ` ``, Delete stays Backspace), not both;
   - **[Fn: standard]** or **[Fn: Mac extras]**, not both, **above** the others in the list.
     Karabiner uses the first rule that matches, so below them Fn+`` ` `` and Fn+`\` would type
     Backspace and ¥.

   They only act on the HHKB Pro BT (vendor 1278, product 514).
4. **Karabiner → Devices:** turn on **Modify events** for the HHKB Pro BT.
5. Restart JIS Korean so it reads the keyboards again: `pkill -x JISKorean`.

JIS Korean needs Input Monitoring and Accessibility for カタカナ (`mac/README.md`). Don't add other
Karabiner rules that remap `japanese_pc_katakana`, `international1` or `international3`.

## Tested

On an HHKB Professional BT (US, `HHKB-BT`) over Bluetooth, macOS 27, Karabiner-Elements 16.3:
- Japanese Kana: むろん → 無論, むろまち → 室町, こころ → 心, ろく → 六, ろーま → ローマ, ろーぷ → ロープ, むーど
  → ムード.
- Mode A: Korean types as on a JIS board.
- The added keys type JIS's symbols: ] } 」 on the ] key (む in Kana), \ _ ろ on ろ, and \ | ー on ¥.

Switching between this and a real JIS board takes some getting used to, since the modifiers double
as language keys here. On its own, it's a good way to try kana typing on an HHKB US before
deciding on an HHKB JIS.
