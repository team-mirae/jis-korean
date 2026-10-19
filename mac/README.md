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

## Files

| File | What |
|---|---|
| `Sources/main.swift` | Starts the `IMKServer`. |
| `Sources/InputController.swift` | One controller and engine per text client: key events in, insert/marked text out, focus-loss commit, the "Enter commits" menu item. |
| `Sources/KeyMap.swift` | macOS virtual key codes → engine key ids (spec §2). |
| `Info.plist` | Bundle and input source metadata. Bundle id `me.sjun.inputmethod.JISKorean`. |
| `tools/make-icon.swift` | Draws the 한 badge (`icon.tiff`). `Info.plist` names it `icon.png` on one input mode, as Apple's Korean input method does; macOS still draws it slightly differently from Apple's badges in System Settings (see docs/troubleshooting/macos-input-method-not-launching.md). |
| `tools/register.swift` | `TISRegisterInputSource`, so a fresh install appears without logging out. |

## Notes

- Shortcuts (Ctrl, Option, Cmd) commit first, then go to the app unchanged.
- Debug logs: `/usr/bin/log stream --predicate 'process == "JISKorean"'` (in zsh, `log` alone is a shell builtin), and the input method's own `~/Library/Logs/JISKorean.log`.

- Input method installed but not launching after a reinstall, or Keyboard Viewer showing ABC
  legends: see [docs/troubleshooting/macos-input-method-not-launching.md](../docs/troubleshooting/macos-input-method-not-launching.md).
