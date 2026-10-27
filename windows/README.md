# windows

The Windows input method for JIS Korean: a TSF text input processor (`jiskorean.dll`) that wraps
`/engine`. All behavior is in the engine. This shell maps scan codes to engine keys (spec §2) and
applies the result to the app as a TSF composition, the hanja and prediction list, and the
taskbar's 한글 / ABC indicator.

## Build

Windows 10 or 11, Visual Studio 2022 Build Tools (C++ x64/x86), CMake 3.28+, Ninja, and Rust
(rustup) with the `x86_64-pc-windows-msvc` and `i686-pc-windows-msvc` targets.

```powershell
rustup target add i686-pc-windows-msvc
tools/build.ps1               # x64 + x86 (32-bit apps load the x86 DLL), runs the shell tests
tools/build.ps1 -Lite         # without the predictive layer (build/<arch>-lite)
tools/build.ps1 -Legacy       # predictions ranked by the rules alone, no learned ranker (build/<arch>-legacy)
tools/build.ps1 -Arch x64     # one architecture
python -I tools/make-icons.py # only after the logo changes: icons/taegeuk.ico from docs/logo
```

The DLL embeds the hanja dictionary and, in the full build, the prediction model (about 6.5 MB),
so each process maps one copy from the image. It links the static CRT: it loads into every app
and can't count on a VC++ runtime being installed. The engine is built with its `english` feature,
for ABC's English layouts (spec §8.4).

## Installer

```powershell
tools/package.ps1             # builds x64 + x86, full and lite, then build/dist/JISKorean-<version>-setup.exe
tools/package.ps1 -SkipBuild  # package what's in build/ already
```

The installer users download: one wizard (Welcome, Licence, Full or Lite, the starting mode,
progress, Finish) that does what `tools/install.ps1` does and nothing more. It asks for UAC once,
copies both DLLs to `Program Files\JIS Korean` (moving a DLL a running app holds aside, as the
script does), registers them, adds JIS Korean to the installing user's Korean keyboards (adding
Korean if it isn't there, without removing anything), and registers an uninstaller in Apps &
features. Running a newer installer over an older one, or over a development install, upgrades
it in place and keeps the user's settings. The mode page only shows for a user with no `Mode`
yet. The uninstaller keeps `%APPDATA%\JIS Korean` unless its box is ticked, and leaves
`HKCU\Software\JIS Korean` (the settings) as the script does. Setup changes the language list
as the user who ran it; the uninstaller can only do it as the account it was elevated as, so a
user who uninstalls with another administrator's password keeps a dead entry in their list.
Silent installs:
`JISKorean-<version>-setup.exe /VERYSILENT /VARIANT=lite /MODE=B` (`/VARIANT=full|lite`,
`/MODE=A|B|C`).

It's [Inno Setup 6](https://jrsoftware.org/isinfo.php) (`installer/jiskorean.iss`): free,
builds from the command line (`winget install JRSoftware.InnoSetup`), and its wizard pages, the
uninstall question and the steps that must run as the user rather than elevated (the language
list, `installer/keyboards.ps1`) are a few lines of script each, where an MSI would need custom
actions for all of them. The version comes from the engine's `Cargo.toml`, as in `CMakeLists.txt`.

**Signing** is off by default. Set `JK_SIGN_CERT` to a `.pfx` file (and `JK_SIGN_PASSWORD`, and
`JK_SIGN_TIMESTAMP` to change the timestamp server) and `package.ps1` signs the DLLs, the
installer and its uninstaller with `signtool`. Unsigned installers get a SmartScreen warning
("Windows protected your PC" → More info → Run anyway).

## Development install

```powershell
tools/install.ps1             # admin (UAC): copy to Program Files\JIS Korean, register x64 + x86,
                              # add JIS Korean to the Korean language's keyboards
tools/install.ps1 -Update     # copy a new build in, no UAC (development installs only)
tools/install.ps1 -Uninstall
tools/install.ps1 -Log        # turn the diagnostic log on and follow it
```

Then pick **JIS Korean** with Win+Space. Nothing else in the language list changes: other Korean
input methods stay where they are.

**Restart apps after installing or updating.** An app that was running keeps the old DLL, or
none at all, until it restarts (FKeyIME `docs/issues/apps-dont-load-fkeyime.md`, the same on
Windows for any TIP).

## Using it

| | |
|---|---|
| Modes A, B, C (spec §3.6) | Right-click the 한글 / ABC indicator in the taskbar → Mode A (JIS), Mode B (US ANSI) or Mode C (UK ISO). One setting for every app: the mode follows the keyboard, not the app. Mode A is the default. In Mode C the menu also has **Dedicated Symbol Keys** (on by default): off, [ ] and the ISO extra key type their UK ISO characters. |
| 한/영 | Switches JIS Korean to English (ABC, in the English layout below) and back to 한글, as the Microsoft IME does. It commits what's unconfirmed first. The state is one for the whole session, every app, and isn't stored: after a restart it's 한글. A click on the indicator does the same. On a US ANSI keyboard, Windows' Korean layout sends 한/영 from Right Alt. JIS and ISO keyboards have no 한/영 key, and JIS Korean deliberately doesn't pick one: map the key you want to 한/영 (`VK_HANGUL`) with your keyboard's software or a remapper such as PowerToys (spec §8.4). |
| English Layout (한/영) | What ABC types, a selector of its own in the menu. By default the keyboard's English: JIS English in Mode A, US English in Mode B, UK English in Mode C. Any other English layout on request (JIS Dvorak, JIS Colemak, …; your keyboard's first), or **Windows' Keyboard Layout**, which passes every key to the app untouched. Shortcuts (Ctrl, Alt, Win) always go to the app; the Alt + letter variants of macOS's English modes are deliberately left out (spec §3.6.3). |
| Predictions (spec §10) | The unconfirmed text shows the top reading (`hello` shows as hello). ↓ / Tab and ↑ move through the list under it, Space / Enter commit what's shown, a click picks a row. Off by default in terminals, remote desktop, password managers, File Explorer and launchers (spec §10.11.1). |
| Hanja (F7) | The list under the text, numbered 1–9 per page (spec §7). |

The indicator's menu (right-click) also has: Predictions, Predictions in this app, Learn from My
Choices (Stays on This PC), Learned Weights (read-only, with Reset What It Learned and Open
Training Folder), and Enter Commits Without a New Line.

### 한/영 and English: type in JIS Korean, and train it on your English words

The 한/영 key is there, and it works exactly like the Microsoft IME's, but it's a plain on/off
switch, and that is all it can be. It's very hard to make it work the way people tend to expect:

- **It doesn't know what you were typing.** Pressing it commits what's unconfirmed as it is and
  then hands every key to the app. It can't go back and turn the last word into English, and
  pressing it mid-word splits the word in two.
- **English typed in ABC is invisible to JIS Korean's composition.** No predictions, no F9 / F10,
  no hanja, and nothing learned from it: ABC only types the English layout's characters.
- **One state for everything.** It's the same in every app and every field, and starts on 한글
  after a restart, so a field that "should" be English needs the key pressed again.
- **Mixed text costs two extra presses per switch**, where JIS Korean's predictions need none.

So we recommend **typing in JIS Korean all the time, in Mode B on a US ANSI keyboard**, and
letting the predictions read the English words: `hello` shows as hello, `server를` as server를,
and Shift+Z (F10) forces the English letters of a word when the prediction is wrong. Then **train
the local model on the English words you use**: list them in a training file
(`docs/training-contract.md`) at `%APPDATA%\JIS Korean\training.json` (menu → Learned Weights →
Open Training Folder). It's applied the next time JIS Korean activates, stays on this PC, and is
forgotten again if you delete the file. Keep 한/영 for typing that isn't words: passwords, code,
commands, games.

**Example: `.co.kr`.** Out of the box `naver.com` types fine, but `naver.co.kr` comes out as
`naver.채.악`: `co` and `kr` aren't in the built-in English lists, and we keep them that way. Add
them yourself:

```json
{
  "format": "jis-korean-training",
  "version": 1,
  "vocabulary": { "english": [{ "word": "co" }, { "word": "kr" }] },
  "examples": [{ "text": "kr", "keys": "KeyK KeyR", "count": 20 }, { "text": "co.kr", "count": 20 }]
}
```

A word in `vocabulary` is enough for `co`. `kr` also needs `examples`: its keys spell 악, a
known Korean word, which a vocabulary count can't outrank (measured: still 악 at a count of
1,000,000). An example is learned as your own pick (spec §10.11), which can. Applying a file with
examples turns on Learn from My Choices.

## Settings

`HKCU\Software\JIS Korean` (read with `HKLM\Software\JIS Korean` as a fallback for apps that can't
see the user's key). The menu writes them; apps pick a change up at their next focus change.

| Value | Type | Default | |
|---|---|---|---|
| `Mode` | DWORD | 0 | 0 = Mode A (JIS), 1 = Mode B (US ANSI), 2 = Mode C (UK ISO) |
| `DedicatedSymbols` | DWORD | 1 | Mode C's [ ] and ISO extra key: 1 = JIS Korean's symbols, 0 = UK ISO's (spec §3.6) |
| `EnglishMode` | DWORD | unset | ABC's layout: an English mode's id (3 JIS English, 7 JIS Dvorak, 8 JIS Colemak, 9 US English, 10 UK English), `0xFFFFFFFF` = Windows' layout; unset = the keyboard's default (spec §8.4) |
| `Predictions` | DWORD | 1 | the predictive layer |
| `EnterCommits` | DWORD | 0 | Enter commits without a new line (spec §5.1) |
| `LearnFromChoices` | DWORD | 0 | learning from your choices (spec §10.11) |
| `Learned` | SZ | | what was learned: one line of counts, no text or keys |
| `Weight.<Scenario>` | SZ | | user tuning (spec §10.5): `Korean` `English` `Implied` `Mixed` `TwoSet` `KeepFinals` `Hanja` `ImpliedL`, e.g. `0.8` |
| `Apps\<program.exe>\Predictions` | DWORD | | predictions in one app, overriding the default-off list |
| `Apps\<program.exe>\Off` | DWORD | | 1: JIS Korean passes every key through in that program (menu: Off in <program>) |
| `DiagnosticLog` | DWORD | 0 | log activations, focus and errors (never typed text) to `%ProgramData%\JIS Korean\jiskorean.log` |
| `TrainingFileDate`, `TrainingWords.*` | SZ | | the training file last applied, and its words |

## Testing

```powershell
tools/build.ps1          # shell tests: scan codes, keys typed into the engine, the JSON reader
tools/type-test.ps1      # types through the installed TIP; don't touch the keyboard meanwhile
build/x64/jk_unloadtest.exe   # loads, activates and unloads the DLL in rounds: memory mustn't grow
```

`type-test.ps1` runs `build/x64/jk_testpad.exe`, which selects JIS Korean for itself, types key
sequences with `SendInput` by scan code and reads the result back, in three kinds of control: a
classic Edit and a RichEdit through the IMM compatibility layer, and a RichEdit on TSF itself
(`SES_USECTF`), plus targets that misbehave the way some apps do (no input context, a composition
ended at every key, a read-only field; see the top of `tests/testpad.cpp`). `jk_testpad` alone
opens the same window to type in by hand. CI builds both
architectures, full and lite, and runs the shell tests; typing through the TIP needs a desktop.

What the shell does differently from what TSF's documentation suggests, and why, is in
[docs/troubleshooting/windows-tsf-notes.md](../docs/troubleshooting/windows-tsf-notes.md).

## Files

| File | What |
|---|---|
| `src/text_service.*` | One per thread: key events in, composition out, the list, focus-loss commits, 한/영, the settings menu |
| `src/key_map.*` | Scan codes → engine keys (spec §2). No Windows headers; unit-tested |
| `src/lang_bar.*` | The taskbar indicator (한글 / ABC, drawn at run time) and its menu |
| `src/candidate_window.*` | The hanja / prediction list |
| `src/settings.*` | The registry settings and the default-off apps |
| `src/training.*`, `src/json.*` | Training files (spec §10.12) |
| `src/shared_data.*` | The dictionary and predictor, parsed once per process off the UI thread |
| `src/registration.*`, `src/dllmain.cpp` | COM and TSF registration (`regsvr32`) |
| `src/guids.h` | **Permanent** GUIDs: they end up in users' registries and language lists. Never change them |
| `tests/main.cpp`, `tests/testpad.cpp`, `tests/unload.cpp` | Shell tests, the test window, and the unload test |
| `installer/` | The installer (`jiskorean.iss`) and its per-user step (`keyboards.ps1`) |
| `tools/` | `build.ps1`, `package.ps1`, `install.ps1`, `type-test.ps1`, `dev.ps1` (a VS developer shell), `make-icons.py` |

## Passwords and screen recording

The prediction and hanja list shows readings of what's being typed, so in a password box it would
show the password in other forms. JIS Korean protects it in layers:

- **The list never appears in screen recordings, screenshots or screen sharing**
  (`WDA_EXCLUDEFROMCAPTURE`), whatever the field.
- **Password boxes that say they are one get no input method at all.** Windows itself keeps TIPs
  out of `ES_PASSWORD` edits and fields with a password input scope (measured: JIS Korean never
  receives a key there). JIS Korean also checks for them itself (input scope, disabled keyboard,
  a password character on an Edit or RichEdit) and passes keys through if one slips by *(not
  measured: Windows caught every case we could build)*.
- **Password boxes that don't say so can't be detected.** Banking and certificate programs often
  draw their own (●●● in a plain field), and the keys then compose like any text. Use **ABC**
  (한/영) there, or right-click the indicator → **Off in <program>**, which passes every key through
  in that program only, from then on. Measured: UniSign accepts its certificate password in ABC.
  JIS Korean starts off in the local agents of certificate programs found on a test PC
  (CrossCert UniSign, DreamSecurity MagicLine4NX, SignKorea; `kOffByDefault` in
  `src/settings.cpp`). Certificate windows drawn inside a web page are in the browser: use ABC.

## Known limits

- Password fields and the secure desktop (UAC, sign-in) don't use TIPs (spec §8).
- Apps that read raw keys (games) see Windows' own layout, not JIS Korean.
- Browsers take F10 for their menu: use Shift+Z (spec §3.4).
- Store apps that run sandboxed (AppContainer) may not be able to read the settings or the
  training file, and then use the defaults *(not measured yet)*.
