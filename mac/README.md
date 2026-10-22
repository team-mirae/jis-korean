# mac

The macOS input method for JIS Korean: an InputMethodKit app that wraps `/engine`. All behavior
is in the engine. This shell maps physical key codes to engine keys and applies the result as
inserted text and marked text.

Built without Xcode: the Command Line Tools are enough.

```sh
make                 # build/full/JISKorean.app, universal (arm64 + x86_64), with predictions
make PREDICT=0       # build/lite/JISKorean.app: the predictive layer stripped out entirely
make ENGLISH=1       # build/full-english/JISKorean.app: adds JIS English (works with PREDICT=0 too)
make install         # copy to ~/Library/Input Methods, register it, restart it (PREDICT=0 and ENGLISH=1 work too)
make uninstall
make dist            # build/<variant>/JISKorean-<version>[-lite].zip
```

## Predictions (spec §10)

The full build includes the engine's predictive layer and its model (`Resources/predict/`). With it
on, the marked text shows the **top reading** of what you typed: `hello` shows as hello, not
옫이애, and `bb` as ㅠㅠ. Space, Enter, Tab, Escape, arrows, shortcuts and clicking away commit what's
shown, and then act as they would on it (Shift+Enter and "Enter Commits Without a New Line" only
commit). F6 still forces Korean and F10 English letters. The suggestion list never appears in screen
sharing, recordings or screenshots.

Tune it per scenario (spec §10.5) with multipliers on each reading's learned probability, applied
when the input method activates:

```sh
defaults write me.sjun.inputmethod.JISKorean Weight.English -float 0.8   # English readings 0.8× as likely
defaults write me.sjun.inputmethod.JISKorean Weight.Korean -float 1.2
# Also Weight.Implied, Weight.Mixed, Weight.TwoSet, Weight.KeepFinals, Weight.Hanja (default 1.0)
```

Turn it off with **Predictions** in the input menu (on by default): the input method then behaves
exactly like the lite build. `make PREDICT=0` builds that lite version, with no prediction code in
the Rust library or the Swift binary and no model in the bundle (about 140 KB smaller).

Requirements: Rust with the `aarch64-apple-darwin` and `x86_64-apple-darwin` targets.

After the first `make install`, add it in **System Settings → Keyboard → Input Sources → Edit →
+ → Korean → JIS Korean**. After later installs it reloads by itself, since `make install`
quits the running copy. If it doesn't show up in the list, log out and back in once.

## Modes and the public API (spec §3.6, §8.2)

Modes A, B and C are three input sources of the one input method, like Japanese – Kana and
Japanese – Romaji. `make install` enables Mode A (`me.sjun.inputmethod.JISKorean.2Set`); add Mode B
(`me.sjun.inputmethod.JISKorean.2SetANSI`, "JIS Korean for US ANSI") or Mode C
(`me.sjun.inputmethod.JISKorean.2SetISO`, "JIS Korean for UK ISO") in System Settings → Keyboard →
Input Sources → + → Korean. If you're upgrading from a version without that mode, log out and back
in first: until then System Settings doesn't list the new mode. In Mode C, the input menu has
**Dedicated Symbol Keys**: turn it off and [ ] and the key left of Z type their UK ISO characters
(`[ {`, `] }`, `\ |`). The mode follows the keyboard, not the app, which is why each mode is
its own input source that any tool can see and select.

Any program can read the mode, hear it change, and, if the user turned on **Let Other Apps Change
the Mode** in the input menu, change it:

```sh
defaults read me.sjun.inputmethod.JISKorean CurrentMode       # A, B or C
J=~/Library/"Input Methods"/JISKorean.app/Contents/MacOS/JISKorean
"$J" mode                                                     # A  JIS Korean Original Layout  me.sjun…2Set
"$J" mode B                                                   # needs the setting; prints the answer
```

```lua
-- Hammerspoon
hs.distributednotifications.new(function(_, _, info) print("JIS Korean mode", info.mode) end,
  "me.sjun.inputmethod.JISKorean.ModeDidChange"):start()
hs.distributednotifications.post("me.sjun.inputmethod.JISKorean.SetMode", nil, { mode = "B" })
```

A refused change is answered with `me.sjun.inputmethod.JISKorean.ModeChangeRefused` and a `reason`.

It also resolves keys and text (spec §8.3) on the engine of the app JIS Korean was last active in,
with that app's mode and predictions:

```sh
"$J" resolveKeyCodeInput KeyG              # {"result":"ㅎ", "app":…, "mode":"A", "predictions":true, …}
"$J" resolveKeyCodeInput KeyS 하           # the field after context 하 and the key: "한"
"$J" resolveKeyCodeInput 5 하              # macOS virtual key codes work too (5 is G)
"$J" resolveKeyCodeFromOutput ㅃ           # {"combinations":[["Digit1"],["S-KeyQ"]], …}
"$J" resolveKeyCodeFromOutput 한 3 3       # min and max key presses; a nonsensical range gives []
```

As notifications: post `me.sjun.inputmethod.JISKorean.ResolveKeyCodeInput` (`keyCode`, optional
`context`) or `…ResolveKeyCodeFromOutput` (`text`, `minKeypressCount`, `maxKeypressCount`) with a
`requestID`; the answer comes as the same name + `Result`, with the same `requestID`.

## JIS English (spec §3.6.2, §8.2.1)

`make ENGLISH=1` adds the English modes: **JIS English** (`A_QWERTY`), and **US English**
(`B_QWERTY`) and **UK English** (`CJ_QWERTY`) with US ANSI and UK ISO symbols, which work the same way.
Below, JIS English stands for the English mode used last. JIS English is an English input mode of JIS Korean, listed
under English in System Settings → Keyboard → Input Sources (log out and back in after the first
install, as for any new mode). It types JIS QWERTY with JIS symbols on any keyboard, like the
Japanese Romaji mode without its conversions, and holding a letter offers its accented variants
(u → ü, spec §3.6.3).

Switching stays inside JIS Korean: 英数 always goes to English; **カタカナ** (the Windows JIS key
right of the space bar) and **F18** always go to Korean, from any app or input source. JIS Korean
asks for **Input Monitoring** (macOS turns カタカナ into かな, so it reads the keyboard itself) and
**Accessibility** (to catch the keys before apps such as iTerm, which keeps F-keys). The input
menu's **Korean Key (Back to Korean)** picks F13–F19 or none, or any key code with
`defaults write me.sjun.inputmethod.JISKorean KoreanKey -int <code>`. Every other key types as on
ABC, Shift+Z, Shift+X, F9, F10 and Caps Lock included. かな is left to macOS, which selects Japanese.
JIS Korean reads カタカナ only from keyboards macOS has configured as JIS. With
**Karabiner-Elements**, a keyboard Karabiner remaps is held by Karabiner, and its keys come out of
Karabiner's virtual keyboard (it reports itself as an Apple keyboard, vendor 1452, product 593):
keep that configured as JIS and don't remap `japanese_pc_katakana`, and カタカナ works with
Karabiner on or off (tested with an HHKB Hybrid Type-S JIS and an HHKB Studio). If カタカナ acts as
かな, the keyboard (or Karabiner's virtual keyboard) is probably stored as ANSI: see
[docs/troubleshooting/macos-katakana-and-eisu.md](../docs/troubleshooting/macos-katakana-and-eisu.md).

The permissions follow the build's signature: signed with an Apple Development certificate (the
Makefile picks one from the keychain), they survive rebuilds; ad hoc, every build has to be granted
again in System Settings → Privacy & Security.
`tools/add-english-mode.sh` adds the mode to the built bundle's Info.plist; the checked-in
Info.plist has the Korean modes only.

Recommended for a JIS keyboard with Japanese also installed (spec §8.2.1):

| From | To | Key |
|---|---|---|
| any | Korean | カタカナ (Windows JIS keyboards), or F18 (remap a key to F18, e.g. with Karabiner) |
| any | English (JIS English) | 英数 (JIS Romaji) |
| any | Japanese (Kana) | かな (JIS Kana), handled by macOS |
| Japanese | Korean | カタカナ or F18 work here too |

Apple's ABC can't be removed from the input sources: macOS always keeps one plain keyboard layout
for password fields, and input method modes don't count (spec §8.2.1). But with **ABC** itself
enabled, macOS sends 英数 to ABC instead of JIS English: add **ABC – Extended**, then remove ABC
([#2](https://github.com/team-mirae/jis-korean/issues/2),
[troubleshooting](../docs/troubleshooting/macos-katakana-and-eisu.md)). System Settings lists the
English modes under English.

## Files

| File | What |
|---|---|
| `Sources/main.swift` | Starts the `IMKServer`. |
| `Sources/InputController.swift` | One controller and engine per text client: key events in, insert/marked text out, focus-loss commit, the "Enter commits" menu item. |
| `Sources/KeyMap.swift` | macOS virtual key codes → engine key ids (spec §2). |
| `Sources/InputModes.swift` | The modes as input modes, and the public API (spec §8.2). |
| `Info.plist` | Bundle and input source metadata, with one input mode per engine mode. Bundle id `me.sjun.inputmethod.JISKorean`. |
| `Resources/icons/` | The logo and one badge per mode (16 px and 32 px PNGs), drawn in `docs/logo/` (MIT). Each pair ships as one `.tiff`. |
| `tools/register.swift` | `TISRegisterInputSource`, so a fresh install appears without logging out. Re-enables only the sources that were enabled. |
| `tools/make-keylayout` | One Keyboard Viewer layout per mode (spec §8.1). |

## Notes

- Shortcuts (Ctrl, Option, Cmd) commit first, then go to the app unchanged.
- Debug logs: `/usr/bin/log stream --predicate 'process == "JISKorean"'` (in zsh, `log` alone is a shell builtin), and the input method's own `~/Library/Logs/JISKorean.log` (launches, activations, mode switches, Keyboard Viewer, commits the
  system or an app forced, with their length only, and each unmapped key code once per app), off by default: `defaults write me.sjun.inputmethod.JISKorean DiagnosticLog -bool true` turns it on immediately, `-bool false` off.

- Input method installed but not launching after a reinstall, or Keyboard Viewer showing ABC
  legends: see [docs/troubleshooting/macos-input-method-not-launching.md](../docs/troubleshooting/macos-input-method-not-launching.md).
