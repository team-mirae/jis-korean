# mac

The macOS input method for JIS Korean: an InputMethodKit app that wraps `/engine`. All behavior
is in the engine. This shell maps physical key codes to engine keys and applies the result as
inserted text and marked text.

Built without Xcode: the Command Line Tools are enough.

```sh
make                 # build/full/JISKorean.app, universal (arm64 + x86_64), with predictions
make PREDICT=0       # build/lite/JISKorean.app: the predictive layer stripped out entirely
make LEGACY_PREDICT=1  # build/full-legacy/JISKorean.app: predictions ranked by the rules only, no learned ranker
make ENGLISH=1       # build/full-english/JISKorean.app: adds JIS English (works with PREDICT=0 too)
make install         # copy to ~/Library/Input Methods, register it, restart it (PREDICT=0 and ENGLISH=1 work too)
make uninstall
make dist            # build/<variant>/JISKorean-<version>[-lite|-legacy].zip
```

## Predictions (spec §10)

The full build includes the engine's predictive layer and its model (`Resources/predict/`). With it
on, the marked text shows the **top reading** of what you typed: `hello` shows as hello, not
옫이애, and `bb` as ㅠㅠ. When more than one reading is listed, ↓ and Tab move down the suggestion
list and ↑ and Shift+Tab up (the chosen row replaces the marked text), Enter commits the chosen row,
and Escape goes back to the top reading. Otherwise Space, Enter, Tab, Escape, arrows, shortcuts and
clicking away commit what's shown, and then act as they would on it (Shift+Enter and "Enter Commits
Without a New Line" only commit). F6 still forces Korean and F10 English letters. The suggestion
list never appears in screen sharing, recordings or screenshots.

Tune it per scenario (spec §10.5) with multipliers on each reading's learned probability, applied
when the input method activates:

```sh
defaults write me.sjun.inputmethod.JISKorean Weight.English -float 0.8   # English readings 0.8× as likely
defaults write me.sjun.inputmethod.JISKorean Weight.Korean -float 1.2
# Also Weight.Implied, Weight.Mixed, Weight.TwoSet, Weight.KeepFinals, Weight.Hanja (default 1.0)
```

The full build ranks the readings with the **learned ranker** (spec §10.13): a linear model over
features of each reading, `Resources/predict/learned.bin` (about 2 MB), with the rules' hard
guarantees kept (hanja never first, a capital at the start of a word is English, …).
`make LEGACY_PREDICT=1` builds the lighter legacy version, ranked by the hand-written rules and the
key n-gram model alone (engine feature `legacy-predict`), without `learned.bin`.

Turn it off with **Predictions** in the input menu (on by default): the input method then behaves
exactly like the lite build. `make PREDICT=0` builds that lite version, with no prediction code in
the Rust library or the Swift binary and no model in the bundle (about 140 KB smaller).

Requirements: Rust with the `aarch64-apple-darwin` and `x86_64-apple-darwin` targets.

After the first `make install`, add it in **System Settings → Keyboard → Input Sources → Edit →
+ → Korean → JIS Korean**. After later installs, newly opened apps use the new build at once, but
apps that were already running keep talking to the old one until you remove JIS Korean there and
add it back, quit and reopen the app, or log out and back in
([why](../docs/troubleshooting/macos-input-method-not-launching.md)). If it doesn't show up in the
list, log out and back in once.

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
`requestID`; the answer comes as the same name + `Result`, with the same `requestID`. Requests are
answered one at a time, with pauses so typing isn't held up; with more than 8 waiting, the oldest
is answered with an `error` instead.

## JIS English (spec §3.6.2, §8.2.1)

`make ENGLISH=1` adds the English modes: **JIS English** (`A_QWERTY`), and **US English**
(`B_QWERTY`) and **UK English** (`CJ_QWERTY`) with US ANSI and UK ISO symbols, and **JIS Dvorak**
(`A_DVORAK`) and **JIS Colemak** (`A_COLEMAK`) with Dvorak's and Colemak's letters and JIS symbols
(spec §3.6.4: ⌘ and ⌥ shortcuts stay on QWERTY), which all work the same way.
JIS English is an English input mode of JIS Korean, listed
under English in System Settings → Keyboard → Input Sources (log out and back in after the first
install, as for any new mode). It types JIS QWERTY with JIS symbols on any keyboard, like the
Japanese Romaji mode without its conversions, and ⌥ + a letter offers its accented variants in a
row (⌥U then 2 → ü; 1–9 and 0 pick, Esc cancels; spec §3.6.3). There's no press and hold: holding a
key repeats it. In terminals ⌥ goes to the app untouched.

Switching stays inside JIS Korean, from any app or input source: **英数** always goes to English,
**カタカナ** (the Windows JIS key right of the space bar) and **F18** always to Korean. The JIS keys
go to the JIS modes: 英数 to JIS English (or the JIS Dvorak or Colemak mode, if that's the English
mode used last), カタカナ to JIS Korean (Mode A); F18 goes to the Korean mode used last. When the
mode a key goes to isn't added, 英数 goes to the English mode used last, and カタカナ to the Korean
mode used last; with no mode to go to, the key goes on as usual.

To catch these keys everywhere, JIS Korean needs **Input Monitoring** (macOS turns カタカナ into
かな, so it reads the keyboard itself) and **Accessibility** (to catch the keys before apps such as
iTerm, which keeps F-keys, and before macOS sends 英数 to ABC). It never asks at launch: choose
**Allow カタカナ and F18 Everywhere…** in the input menu, which asks for both. With them granted,
the keys are caught only while an English mode is added or a keyboard is configured as JIS, and
each only when the mode it goes to is added (otherwise F18 reaches the app, for a Hyper key say).
Without the permissions, 英数 and F18 work only in apps that pass them on to the input method, and
カタカナ acts as かな. The keys are caught inside JIS Korean's own process, so
`make install ENGLISH=1` also installs a login agent
(`~/Library/LaunchAgents/me.sjun.inputmethod.JISKorean.login.plist`) that starts it in the
background at login; without it they'd do nothing after a login until JIS Korean was selected once.
The input
menu's **Korean Key (Back to Korean)** picks F13–F19 or none, or any key code with
`defaults write me.sjun.inputmethod.JISKorean KoreanKey -int <code>`. Every other key types as on
ABC, Shift+Z, Shift+X, F9, F10 and Caps Lock included. かな is left to macOS, which selects Japanese.
JIS Korean reads カタカナ only from keyboards macOS has configured as JIS (one plugged in or
configured later counts at once, without a restart). With
**Karabiner-Elements**, a keyboard Karabiner remaps is held by Karabiner, and its keys come out of
Karabiner's virtual keyboard (it reports itself as an Apple keyboard, vendor 1452, product 593):
keep that configured as JIS and don't remap `japanese_pc_katakana`, and カタカナ works with
Karabiner on or off (tested with an HHKB Hybrid Type-S JIS and an HHKB Studio). If カタカナ acts as
かな, the keyboard (or Karabiner's virtual keyboard) is probably stored as ANSI: see
[docs/troubleshooting/macos-katakana-and-eisu.md](../docs/troubleshooting/macos-katakana-and-eisu.md).
A Mac JIS keyboard (a MacBook's built-in one) has no カタカナ: the recommended setup,
[macbook-jis-to-hhkb](https://github.com/themrsung/keyboard-troubleshooting/tree/main/layouts/macbook-jis-to-hhkb), makes a tap of
Caps Lock カタカナ and a hold the HHKB Fn layer with Karabiner, with Caps Lock on Fn+Tab.
To set up a remappable ANSI 87-key keyboard as a JIS board (カタカナ, 英数, かな, ¥ and ろ on two
layers), see
[keyboard-troubleshooting](https://github.com/themrsung/keyboard-troubleshooting)'s
[us-ansi-87-key-to-jis](https://github.com/themrsung/keyboard-troubleshooting/tree/main/layouts/us-ansi-87-key-to-jis).

The permissions follow the build's signature: signed with an Apple Development certificate (the
Makefile picks one from the keychain), they survive rebuilds; ad hoc, every build has to be granted
again in System Settings → Privacy & Security.
`tools/add-english-mode.sh` adds the mode to the built bundle's Info.plist; the checked-in
Info.plist has the Korean modes only.

Recommended for a JIS keyboard with Japanese also installed (spec §8.2.1):

| From | To | Key |
|---|---|---|
| any | Korean (JIS Korean) | カタカナ (Windows JIS keyboards) |
| any | Korean (the last Korean mode) | F18 (remap a key to F18, e.g. with Karabiner) |
| any | English (JIS English) | 英数 (JIS Romaji) |
| any | Japanese (Kana) | かな (JIS Kana), handled by macOS |
| Japanese | Korean | カタカナ or F18 work here too |

Apple's ABC can't be removed from the input sources: macOS always keeps one plain keyboard layout
for password fields, and input method modes don't count (spec §8.2.1). With **ABC** itself
enabled, macOS sends 英数 to ABC instead of JIS English. With both permissions granted, JIS Korean
catches 英数 before macOS does and goes to JIS English itself, which is meant to make this
unnecessary (not yet confirmed on a real setup); without them, or if 英数 still selects ABC, add
**ABC – Extended**, then remove ABC
([#2](https://github.com/team-mirae/jis-korean/issues/2),
[troubleshooting](../docs/troubleshooting/macos-katakana-and-eisu.md)). System Settings lists the
English modes under English.

## Files

| File | What |
|---|---|
| `Package.swift` | The shell as a Swift package, for CodeQL only (code scanning traces Swift through `swift build`); it makes no app. Build with `make`. |
| `Sources/main.swift` | Starts the `IMKServer`. |
| `Sources/InputController.swift` | One controller and engine per text client: key events in, insert/marked text out, focus-loss commit, the "Enter commits" menu item. |
| `Sources/KeyMap.swift` | macOS virtual key codes → engine key ids (spec §2). |
| `Sources/InputModes.swift` | The modes as input modes, and the public API (spec §8.2). |
| `Sources/KoreanKeyTap.swift`, `Sources/HardwareKeys.swift` | `ENGLISH=1` only: the system-wide key tap for カタカナ, F18 and 英数, and the hardware listener that tells カタカナ from かな (spec §8.2.1). |
| `Sources/Training.swift` | The training file (docs/training-contract.md); its examples are learned in the background. |
| `Info.plist` | Bundle and input source metadata, with one input mode per engine mode. Bundle id `me.sjun.inputmethod.JISKorean`. |
| `Resources/icons/` | The logo and one badge per mode (16 px and 32 px PNGs), drawn in `docs/logo/` (MIT). Each pair ships as one `.tiff`. |
| `tools/register.swift` | `TISRegisterInputSource`, so a fresh install appears without logging out. Re-enables only the sources that were enabled. |
| `tools/make-keylayout` | One Keyboard Viewer layout per mode (spec §8.1). |

## Notes

- Shortcuts (Ctrl, Option, Cmd) commit first, then go to the app unchanged.
- Debug logs: `/usr/bin/log stream --predicate 'process == "JISKorean"'` (in zsh, `log` alone is a shell builtin), and the input method's own `~/Library/Logs/JISKorean.log`, off by default: `defaults write me.sjun.inputmethod.JISKorean DiagnosticLog -bool true` turns it on immediately, `-bool false` off. It logs launches, activations and deactivations (with the input source macOS has selected and the mode JIS Korean is in), mode switches and the fields that resync their engine or layout to them, Keyboard Viewer layouts named, one line per key without the key (whether the engine took it, the field's engine mode and layout, and the keyboard type as the event and macOS's global report it), keys that took over 100 ms, commits the system or an app forced (their length only), and each unmapped key code once per app. Never any text.
- Bare jamo instead of syllables (ㅂㅏㄷㅏ for 바다) in one field means the app typed the keys through the mode's keyboard layout without asking the input method: password fields (macOS bypasses input methods there), and in Chrome 152–156 any field that once looked like a password (Chromium issue 567321058, fixed in 157). Apple's Korean input method types bare jamo there too. In an ordinary field, turn the log on and type a few keys there: the `key in …` lines show whether the keys reached the input method, and what the field's engine and layout were. Full diagnosis: [docs/troubleshooting/macos-bare-jamo-in-chrome.md](../docs/troubleshooting/macos-bare-jamo-in-chrome.md).

- Input method installed but not launching after a reinstall, or Keyboard Viewer showing ABC
  legends: see [docs/troubleshooting/macos-input-method-not-launching.md](../docs/troubleshooting/macos-input-method-not-launching.md).
