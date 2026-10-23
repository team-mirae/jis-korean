# Set up JIS Korean on macOS

You come here from [start.md](start.md) with the user's keyboards and remap answer.
[rules.md](rules.md) applies to every step: ask before each scan, and ask before each install or
change, one at a time.

Facts come from [mac/README.md](../../mac/README.md) and spec §3.6, §8.1–§8.2.1. Read them before you
explain anything, and follow them where this prompt is less precise.

## 1. Check what's there (with permission)

Each of these is one read-only check. Ask before running it, as rules.md says, or give it to the
user to run.

| To learn | Command |
|---|---|
| macOS version | `sw_vers -productVersion` |
| Command Line Tools | `xcode-select -p` |
| Rust | `rustup target list --installed` (or `cargo --version`) |
| Connected keyboards, and how they connect | `ioreg -r -c IOHIDInterface -l` (the entries with usage page 1 and usage 6: `Product`, `Transport`, `VendorID`, `ProductID`) |
| The type macOS stored for each keyboard | `defaults read /Library/Preferences/com.apple.keyboardtype` (`product-vendor-0` = 40 ANSI, 41 ISO, 42 JIS) |
| Karabiner-Elements running | `pgrep -fl Karabiner` |
| JIS Korean already installed | `ls ~/Library/Input\ Methods/` |
| Signing certificate | `security find-identity -v -p codesigning` |

If the user chose to check things themselves, the clicks are: System Settings → General → About
(version); System Settings → Keyboard → Keyboards (or System Information → Bluetooth / USB) for the
keyboards; Keyboard → Change Keyboard Type for the type.

## 2. English: JIS English, or macOS's own?

JIS Korean can come with its own English input source, **JIS English** (and US English, UK English,
JIS Dvorak and JIS Colemak), in the same input method as the Korean modes. Explain why someone might
want it, in a few lines, with whichever of these fit their answers so far:

- **On a JIS keyboard: do you type Japanese in Kana mode?** In Kana mode, A types ち, not あ, so
  English needs an input source of its own anyway. JIS English is that source, and then each key
  goes to one language: 英数 to English, かな to Japanese, カタカナ (or F18) to Korean.
- **Have you had problems with Hammerspoon or other tools that switch input sources in the
  background?** Switching between two input methods means two processes, and macOS switches
  asynchronously, so a key pressed right after can land in the wrong one. Korean and English modes
  in one input method switch inside one process (spec §8.2.1).
- **Should a US ANSI keyboard, a new one or one whose firmware can't be remapped, type JIS
  symbols?** JIS English types JIS QWERTY with JIS symbols on any keyboard (Shift+2 is `"`), with no
  remapping software.

Then ask:

> Do you want JIS English?
>
> 1. Tell me the differences between macOS's built-in English and JIS English first
> 2. Not sure: install it, and I'll try it later
> 3. No, I'll use macOS's own English input source
> 4. No, I already use a custom or third-party one for English

- **1:** summarize from [mac/README.md](../../mac/README.md) (JIS English) and spec §3.6.2–§3.6.3,
  then ask again. The main points:
  - JIS English types JIS symbols on any keyboard. ABC's symbols depend on the keyboard type macOS
    has stored for the keyboard.
  - ⌥ + a letter shows its accented variants in a numbered row (⌥U then 2 → ü). There's no press
    and hold: holding a key repeats it. In terminals, ⌥ goes to the app untouched.
  - It's the English source 英数 selects, if ABC is swapped for ABC – Extended
    ([#2](https://github.com/team-mirae/jis-korean/issues/2)). Ctrl+Space switches between it and the
    last Korean mode.
  - It comes with the `ENGLISH=1` build. That build also has the カタカナ and F18 switch keys, which
    need Input Monitoring and Accessibility (step 5).
  - macOS keeps ABC (or ABC – Extended) either way, for password fields.
- **2:** build with `ENGLISH=1`, but don't add JIS English as an input source. Tell them where it is
  for later: System Settings → Keyboard → Input Sources → Edit → + → English → JIS English.
- **3:** don't add JIS English. Still suggest `ENGLISH=1` if they want カタカナ or F18 to go to Korean:
  those keys only come with that build, and they work from ABC too. 英数 then goes to ABC.
- **4:** ask which one they use and how it's set up: the input source (a custom keyboard layout,
  Japanese Romaji, Google Japanese Input's alphanumeric mode, another input method), how they
  switch to it and back (英数, Ctrl+Space, a Karabiner rule, a Hammerspoon script, another tool), and
  which keys their tools already use. Then plan around it so nothing they have breaks:
  - don't add JIS English, and don't change their input sources, rules or scripts without asking
    (rules.md section 3);
  - check for clashes before suggesting switch keys: F18 or the Korean Key already in use, 英数
    bound elsewhere, a Karabiner rule on `japanese_pc_katakana`;
  - if their tool selects input sources by ID, JIS Korean's modes have stable IDs
    (`me.sjun.inputmethod.JISKorean.2Set` and so on, spec §8.2).

## 3. Build and install

There's no prebuilt release yet: JIS Korean is built from this repository. It needs:
1. **Command Line Tools** (`xcode-select --install`, Apple's, about 1 GB). No Xcode needed.
2. **Rust** through rustup, with the `aarch64-apple-darwin` and `x86_64-apple-darwin` targets.

Ask about each separately (rules.md section 2). Rust can come from rustup.rs or from Homebrew. If
Homebrew isn't installed, it's a third question of its own: don't install it on the way to
something else.

Pick the build:
- `make install ENGLISH=1` for JIS English, or for the switch keys (カタカナ / F18 to Korean) with any
  English source (step 2). It also installs a login agent that starts JIS Korean at login, so the
  keys work right after logging in. **Recommend this one** unless they want neither, and switch
  with Ctrl+Space or the input menu only.
- `make install` for the Korean modes only, switched with Ctrl+Space or the input menu.
- `PREDICT=0` with either one leaves out predictions (spec §10): smaller, fully deterministic.

Run it in `mac/`. It installs to `~/Library/Input Methods`, so no administrator password.

**Signing:** without an Apple Development certificate in the keychain, the build is signed ad hoc.
Then every rebuild counts as a new app, and the permissions in step 5 have to be granted again after
each one. Say this before the first build. If they have a certificate (`security find-identity`),
the Makefile uses it.

## 4. Add the input sources

These are changes: ask before each one, or walk the user through the clicks.

- System Settings → Keyboard → Input Sources → Edit → + → **Korean** → the mode for each keyboard:
  Mode A (JIS), Mode B ("JIS Korean for US ANSI") or Mode C ("JIS Korean for UK ISO"). See the mode
  table in start.md.
- If they chose JIS English in step 2: + → **English** → JIS English (or US / UK English, JIS
  Dvorak, JIS Colemak).
- If **ABC** is in the list, 英数 selects it instead of JIS English. Offer to add **ABC – Extended**
  and remove ABC ([#2](https://github.com/team-mirae/jis-korean/issues/2)). macOS keeps one plain
  layout for password fields, so ABC – Extended stays.
- If it doesn't appear in the list: log out and back in once
  ([troubleshooting](../troubleshooting/macos-input-method-not-launching.md)).

Apps that were already open use the new input method after they're reopened.

## 5. Permissions for the switch keys (`ENGLISH=1`)

JIS Korean asks for **Input Monitoring** (to tell カタカナ from かな) and **Accessibility** (to catch
F18 and カタカナ before apps do). Explain both before the prompts appear. The user grants them in
System Settings → Privacy & Security. Never script these.

## 6. Switch keys, by keyboard and remap answer

The recommended flow (spec §8.2.1) is one key per language: **Korean** with カタカナ or the Korean
Key (F18 by default), **English** with 英数, **Japanese** with かな if Japanese is installed.

**A JIS keyboard with a カタカナ key** (Windows JIS boards): nothing to remap. It works when macOS
has stored the keyboard as JIS (42). If it's stored as 40 or 41, offer the fix in
[the troubleshooting page](../troubleshooting/macos-katakana-and-eisu.md). That's a `sudo` change the
user runs.

**A Mac JIS keyboard** (英数 and かな only): Korean needs another key. Go by the remap answer below.
With Karabiner, the recommended setup is [macbook-jis-to-hhkb](https://github.com/themrsung/keyboard-troubleshooting/tree/main/layouts/macbook-jis-to-hhkb):
a tap of Caps Lock is カタカナ, a hold is the HHKB Fn layer, and Caps Lock moves to Fn+Tab.

**US ANSI or UK ISO:** there's no 英数 or かな. Ctrl+Space switches between the last two input
sources. A Korean Key and an English key need remapping: go by the remap answer below.

### a. Non-intrusive tools: Karabiner-Elements

Ask whether they want Karabiner-Elements, saying it needs Input Monitoring and a driver extension the
user approves. Then, one change at a time:
- a simple modification from a key they don't use (right ⌘, or a key they choose) to **F18**,
  the Korean Key;
- on a non-JIS board, optionally a key to **japanese_eisuu** (英数) for English;
- for an ANSI board they want to use as JIS: follow
  [the 87-key guide](https://github.com/themrsung/keyboard-troubleshooting/tree/main/layouts/us-ansi-87-key-to-jis). Have macOS recognize the board as
  JIS first, which needs no Karabiner. Karabiner's JIS virtual keyboard (`593-1452-0` stored as 42)
  is the fallback if that doesn't work.

Never remap `japanese_pc_katakana`. After changing Karabiner's device settings, restart JIS Korean
(`pkill -x JISKorean`).

### b. A custom stack

Everything in a, plus whatever they want to maintain:
- Karabiner complex modifications per device or per app;
- a different Korean Key: the input menu offers F13–F19, and any key code works with
  `defaults write me.sjun.inputmethod.JISKorean KoreanKey -int <code>`;
- their own mode, forked from Mode A, B or C ([docs/modes.md](../modes.md)), built into their own
  `make install`;
- build variants (`PREDICT=0`), the diagnostic log (`defaults write me.sjun.inputmethod.JISKorean
  DiagnosticLog -bool true`, read in `~/Library/Logs/JISKorean.log`).

Say plainly that they own it: macOS and Karabiner updates can break their rules, and each rebuild
(ad hoc) needs the permissions again.

### c. No software remapping

- **Remappable firmware** (QMK, VIA, Vial, ZMK, the maker's app): put **F18** on a key for Korean. F18
  works whatever type macOS has stored for the keyboard. Firmware can also send 英数 (LANG2) and
  かな (LANG1). The [87-key guide](https://github.com/themrsung/keyboard-troubleshooting/tree/main/layouts/us-ansi-87-key-to-jis) has a complete
  two-layer keymap. Its step 1 has macOS recognize the board as JIS with Keyboard Setup Assistant
  (Z, then Fn+ろ), with no software remapping. If that doesn't work for their board, offer F18 for
  Korean, and Mode B.
- **JIS Korean's own setting:** the Korean Key can be any key code (`KoreanKey`, above). This isn't
  a remapper, just which key JIS Korean listens for.
- **No remapping at all:** Ctrl+Space and the input menu.

Changing a keyboard's firmware is a change (rules.md section 3). Show the keymap and let the user
flash it.

## 7. Check it works

Ask the user to type, in TextEdit or any text field:
- `ㅎ ㅏ ㄴ` → 한, and `ㄱ ㅏ ㄴ ㅏ` → 가나;
- in Mode A, Shift+1 → 1 and 1 → ㅃ;
- F6 after a word commits it (spec §6);
- each switch key, from each input source.

If something fails, turn on the diagnostic log (ask first: it's a change) and read
[mac/README.md](../../mac/README.md) and [docs/troubleshooting](../troubleshooting/) before guessing.

## 8. Wrap up

List what was installed, what was changed, and how to undo each:
- `make uninstall` in `mac/` removes JIS Korean and its login agent;
- System Settings → Keyboard → Input Sources removes the modes;
- Karabiner's rules and devices are undone in Karabiner-Elements, and the app has its own uninstaller;
- a keyboard type goes back with Keyboard Setup Assistant or the same `defaults write` with the old
  value.
