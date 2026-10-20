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
tools/build.ps1 -Arch x64     # one architecture
python -I tools/make-icons.py # only after the logo changes: icons/taegeuk.ico from docs/logo
```

The DLL embeds the hanja dictionary and, in the full build, the prediction model (about 6.5 MB),
so each process maps one copy from the image. It links the static CRT: it loads into every app
and can't count on a VC++ runtime being installed.

## Install

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
| Mode A / Mode B (spec §3.6) | Right-click the 한글 / ABC indicator in the taskbar → Mode A (JIS) or Mode B (US ANSI). One setting for every app: the mode follows the keyboard, not the app. Mode A is the default. |
| 한/영 | Switches JIS Korean off (ABC: every key goes to the app untouched, as Windows' US layout types it) and on again, as the Microsoft IME does. It commits what's unconfirmed first. The state is one for the whole session, every app, and isn't stored: after a restart it's 한글. A click on the indicator does the same. On a US ANSI keyboard, Windows' Korean layout sends 한/영 from Right Alt. |
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
- **English typed in ABC is invisible to JIS Korean.** No predictions, no F9 / F10, no hanja,
  and nothing learned from it: the keys never reach the engine.
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
| `Mode` | DWORD | 0 | 0 = Mode A (JIS), 1 = Mode B (US ANSI) |
| `Predictions` | DWORD | 1 | the predictive layer |
| `EnterCommits` | DWORD | 0 | Enter commits without a new line (spec §5.1) |
| `LearnFromChoices` | DWORD | 0 | learning from your choices (spec §10.11) |
| `Learned` | SZ | | what was learned: one line of counts, no text or keys |
| `Weight.<Scenario>` | SZ | | user tuning (spec §10.5): `Korean` `English` `Implied` `Mixed` `TwoSet` `KeepFinals` `Hanja` `ImpliedL`, e.g. `0.8` |
| `Apps\<program.exe>\Predictions` | DWORD | | predictions in one app, overriding the default-off list |
| `DiagnosticLog` | DWORD | 0 | log activations, focus and errors (never typed text) to `%ProgramData%\JIS Korean\jiskorean.log` |
| `TrainingFileDate`, `TrainingWords.*` | SZ | | the training file last applied, and its words |

## Testing

```powershell
tools/build.ps1          # shell tests: scan codes, keys typed into the engine, the JSON reader
tools/type-test.ps1      # types through the installed TIP; don't touch the keyboard meanwhile
```

`type-test.ps1` runs `build/x64/jk_testpad.exe`, which selects JIS Korean for itself, types key
sequences with `SendInput` by scan code and reads the result back, in three kinds of control: a
classic Edit and a RichEdit through the IMM compatibility layer, and a RichEdit on TSF itself
(`SES_USECTF`). `jk_testpad` alone opens the same window to type in by hand. CI builds both
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
| `tests/main.cpp`, `tests/testpad.cpp` | Shell tests, and the test window |
| `tools/` | `build.ps1`, `install.ps1`, `type-test.ps1`, `dev.ps1` (a VS developer shell), `make-icons.py` |

## Known limits

- Password fields and the secure desktop (UAC, sign-in) don't use TIPs (spec §8).
- Apps that read raw keys (games) see Windows' own layout, not JIS Korean.
- Browsers take F10 for their menu: use Shift+Z (spec §3.4).
- Store apps that run sandboxed (AppContainer) may not be able to read the settings or the
  training file, and then use the defaults *(not measured yet)*.
