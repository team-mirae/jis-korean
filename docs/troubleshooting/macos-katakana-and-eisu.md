# macOS: カタカナ doesn't switch to Korean, 英数 selects ABC

Seen 2026-10-22 on macOS 27.0.1, `main` at `cd71e0d` built with `make ENGLISH=1`, an HHKB Studio JIS
over Bluetooth through Karabiner-Elements 16.3.0 (remapping only Pause → Play/Pause), on a MacBook
with a US ANSI built-in keyboard. Both problems are about the switch keys of spec §8.2.1.

Turn on the input method's own log first; every step below reads it:

```sh
defaults write me.sjun.inputmethod.JISKorean DiagnosticLog -bool true   # off again: -bool false
tail -f ~/Library/Logs/JISKorean.log
```

## カタカナ acts as かな (or does nothing)

### Symptoms

- カタカナ selects Japanese, or nothing happens. The Korean Key (F18) works.
- The log has only the key tap's lines for it, never a hardware line:

  ```
  tap: key code 104 down, カタカナ held false
  ```

- At launch, the log says:

  ```
  hardware keys: no keyboard is configured as JIS; カタカナ isn't listened for
  ```

### Cause

macOS turns カタカナ into かな (key code 104), so JIS Korean tells them apart by reading the
keyboard itself (HID usage 0x88). It opens only the keyboards macOS has configured as JIS:
`/Library/Preferences/com.apple.keyboardtype`, one entry per keyboard (`product-vendor-country`),
with the type Keyboard Setup Assistant chose. **Type 40 is ANSI, 41 ISO, 42 JIS.** On this Mac every
keyboard was 40 (ANSI), the HHKB Studio included, although it types JIS correctly through
Karabiner, so the listener had nothing to open.

With Karabiner-Elements remapping a keyboard, Karabiner holds that keyboard, and its keys come out
of Karabiner's virtual keyboard (vendor 1452, product 593, entry `593-1452-0`). That entry needs to
be JIS too. Karabiner's own "Virtual Keyboard: JIS" setting doesn't write it.

Check what macOS has:

```sh
defaults read /Library/Preferences/com.apple.keyboardtype
karabiner_cli --list-connected-devices    # in /Library/Application Support/org.pqrs/Karabiner-Elements/bin
```

### Fix

Set the keyboard and Karabiner's virtual keyboard to JIS (42), then restart JIS Korean:

```sh
sudo defaults write /Library/Preferences/com.apple.keyboardtype keyboardtype \
  -dict-add "593-1452-0" -int 42 "21-1278-0" -int 42    # Karabiner's virtual keyboard, HHKB Studio
pkill -x JISKorean                                         # relaunches on the next keystroke in a text field
```

Use your keyboard's own `product-vendor-0` key in place of `21-1278-0`. Or use Keyboard Setup Assistant
(System Settings → Keyboard → Change Keyboard Type…) with the keyboard, and choose JIS.

After the relaunch, the log says `hardware keys: on for 2 JIS keyboard(s), some held by another
program` ("held" is Karabiner holding the HHKB itself, expected), and a press gives:

```
hardware key 0x88 down
Korean Key (tap)
```

Restarting Karabiner wasn't needed.

## 英数 selects ABC instead of JIS English

[#2](https://github.com/team-mirae/jis-korean/issues/2). With Apple's **ABC** enabled, 英数 goes
to ABC, although JIS English (`me.sjun.inputmethod.JISKorean.A_QWERTY`) is added and JIS Korean's
key tap sees the key (`tap: key code 102 down`). This answers the open question of spec §8.2.1:
with ABC present, macOS picks ABC.

**Workaround:** replace ABC with **ABC – Extended**. In System Settings → Keyboard → Input Sources →
Edit…, first add English → ABC – Extended, then remove ABC (macOS keeps at least one plain keyboard
layout, so ABC can't be removed while it's the only one). 英数 then goes to JIS English, and
ABC – Extended still types in password fields.

## Scripting input sources on macOS 27

`TISEnableInputSource` and `TISDisableInputSource` return `noErr` on macOS 27.0.1 but the change
doesn't persist (the source stays as it was, also in `com.apple.HIToolbox`). Add and remove input
sources in System Settings. `TISSelectInputSource` works.
