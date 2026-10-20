# mac

The macOS input method for JIS Korean: an InputMethodKit app that wraps `/engine`. All behavior
is in the engine. This shell maps physical key codes to engine keys and applies the result as
inserted text and marked text.

Built without Xcode: the Command Line Tools are enough.

```sh
make                 # build/full/JISKorean.app, universal (arm64 + x86_64), with predictions
make PREDICT=0       # build/lite/JISKorean.app: the predictive layer stripped out entirely
make install         # copy to ~/Library/Input Methods, register it, restart it (PREDICT=0 works too)
make uninstall
make dist            # build/<variant>/JISKorean-<version>[-lite].zip
```

## Predictions (spec §10)

The full build includes the engine's predictive layer and its model (`Resources/predict/`). With it
on, the marked text shows the **top reading** of what you typed: `hello` shows as hello, not
옫이애, and `bb` as ㅠㅠ. Space, Enter, Tab, Escape, arrows, shortcuts and clicking away commit what's
shown. F6 still forces Korean and F10 English letters.

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

Mode A and Mode B are two input sources of the one input method, like Japanese – Kana and
Japanese – Romaji. `make install` enables Mode A (`me.sjun.inputmethod.JISKorean.2Set`); add Mode B
(`me.sjun.inputmethod.JISKorean.2SetANSI`, "JIS Korean for US ANSI") in System Settings → Keyboard →
Input Sources → + → Korean. If you're upgrading from a version without Mode B, log out and back in
first: until then System Settings doesn't list the new mode. The mode follows the keyboard, not the app, which is why each mode is
its own input source that any tool can see and select.

Any program can read the mode, hear it change, and, if the user turned on **Let Other Apps Change
the Mode** in the input menu, change it:

```sh
defaults read me.sjun.inputmethod.JISKorean CurrentMode       # A or B
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
- Debug logs: `/usr/bin/log stream --predicate 'process == "JISKorean"'` (in zsh, `log` alone is a shell builtin), and the input method's own `~/Library/Logs/JISKorean.log` (launches, activations, Keyboard Viewer), off by default: `defaults write me.sjun.inputmethod.JISKorean DiagnosticLog -bool true` turns it on immediately, `-bool false` off.

- Input method installed but not launching after a reinstall, or Keyboard Viewer showing ABC
  legends: see [docs/troubleshooting/macos-input-method-not-launching.md](../docs/troubleshooting/macos-input-method-not-launching.md).
