# mac

The macOS input method for JIS Korean: an InputMethodKit app that wraps `/engine`. All behavior
is in the engine. This shell maps physical key codes to engine keys and applies the result as
inserted text and marked text.

Built without Xcode: the Command Line Tools are enough.

```sh
make            # build/JISKorean.app, universal (arm64 + x86_64)
make install    # copy to ~/Library/Input Methods, register it, restart it
make uninstall
```

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
| `tools/make-icon.swift` | Draws the menu bar icon (template image). |
| `tools/register.swift` | `TISRegisterInputSource`, so a fresh install appears without logging out. |

## Notes

- Shortcuts (Ctrl, Option, Cmd) commit first, then go to the app unchanged.
- Debug logs: `log stream --predicate 'process == "JISKorean"'`.
