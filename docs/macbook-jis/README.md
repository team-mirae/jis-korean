# A MacBook with a JIS keyboard: Caps Lock as カタカナ

A Mac JIS keyboard has 英数 and かな but no カタカナ, so it has no key that goes to Korean (spec
§8.2.1). This Karabiner-Elements rule makes plain Caps Lock カタカナ. Caps Lock with a modifier
(Shift or ⌥) is still Caps Lock.

Tested 2026-10-24 on macOS 27.0.1 with JIS Korean 0.3.0 (`v0.3.0-macos`, a full install from the
release zip), on a MacBook with a built-in JIS keyboard.

| Key | Goes to |
|---|---|
| Caps Lock | Korean (as カタカナ) |
| Shift / ⌥ + Caps Lock | Caps Lock |
| 英数 | English (JIS English) |
| かな | Japanese, handled by macOS |

## The rule

[caps-lock-to-katakana.json](caps-lock-to-katakana.json) maps `caps_lock` to
`japanese_pc_katakana`:

- **Plain Caps Lock only.** The rule's `from` lists no modifiers, and Karabiner then matches the key
  only when no modifier is held, so a held modifier lets Caps Lock through.
- **Apple keyboards only:** the built-in keyboard, and keyboards with Apple's vendor ids (1452 over
  USB, 76 over Bluetooth), which includes Magic Keyboards. Other keyboards keep their Caps Lock.

JIS Korean reads カタカナ itself from keyboards stored as JIS, so the remapped key goes to Korean
like a Windows JIS keyboard's カタカナ (see [mac/README.md](../../mac/README.md)).

## Setup

1. Install JIS Korean, add Mode A and JIS English, and grant Input Monitoring and Accessibility
   when it asks (input menu → **Allow カタカナ and F18 Everywhere…**).
2. Install [Karabiner-Elements](https://karabiner-elements.pqrs.org/) and approve its driver
   extension.
3. Add the rule:

   ```sh
   mkdir -p ~/.config/karabiner/assets/complex_modifications
   cp caps-lock-to-katakana.json ~/.config/karabiner/assets/complex_modifications/
   ```

   Then in Karabiner-Elements: **Complex Modifications → Add predefined rule**, and enable
   "Caps Lock → カタカナ for JIS Korean".
4. Karabiner now holds the built-in keyboard, so the keys come from Karabiner's virtual keyboard
   (`593-1452-0`), which has to be stored as JIS (42):

   ```sh
   defaults read /Library/Preferences/com.apple.keyboardtype
   ```

   If it isn't 42, see
   [macos-katakana-and-eisu.md](../troubleshooting/macos-katakana-and-eisu.md). Then restart JIS
   Korean: `pkill -x JISKorean`.
