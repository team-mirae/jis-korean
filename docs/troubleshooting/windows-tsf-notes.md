# Windows: how the TSF shell talks to apps, and why

Measured on Windows 11 Pro 25H2 (build 26200) with `windows/tools/type-test.ps1`, in three kinds of
control: a classic Edit and a RichEdit through the IMM compatibility layer, and a RichEdit on TSF
itself (`SES_USECTF`). Read this before changing `windows/src/text_service.cpp`: each of these was
"fixed" the obvious way first, and the obvious way broke typing.

## A key reported as eaten in OnTestKeyDown never reaches the app

**Symptom.** Enter, the arrows and Tab after Korean text: the text was committed, but the key
itself was lost (no new line, the caret didn't move), in all three controls.

**Cause.** The shell said "mine" for every key while composing in `OnTestKeyDown`, so that
`OnKeyDown` could commit first, then handed keys the engine didn't consume back with
`*eaten = FALSE`. By then Windows has already turned the key into a processed IME key: the app
never sees it, and no key-up follows. FKeyIME's "commit, then pass" uses the same pattern.

**Solution.** The engine runs in `OnTestKeyDown`. A key it doesn't consume is committed there and
reported as not eaten, so the app gets it as usual; the result is kept for the `OnKeyDown` that
may follow. Two more measured facts shape that cache:

- `OnKeyDown` gets the lParam with the repeat count cleared (`0x220000` after `0x220001`), so the
  cache compares the virtual key, scan code and extended flag only. Comparing the whole lParam
  made every key go through the engine twice.
- Through the IMM layer, `OnKeyDown` follows even for a key `OnTestKeyDown` didn't eat (Enter), so
  the cache keeps not-eaten results too.

## Classic apps need the composition selected as an interim character

**Symptom (FKeyIME, and confirmed here).** With a plain caret after the composition, apps that
reach TSF through the IMM layer end a Korean composition at once: jamo come out one by one.

**What works.** Selecting the composition as an interim character (a block cursor), as the
Microsoft Korean IME does. But JIS Korean's composition is a whole segment, not one syllable:

| Tried | Result |
|---|---|
| Interim selection over the whole composition | `SetSelection` fails with E_INVALIDARG once it's longer than one character |
| A normal (non-interim) selection over it | The app ends the composition: `한ㄱ을` for 한글 |
| Interim on the last character only | The IMM layer commits everything before it, at every key: `한한한한글` |
| Interim on the first character, then leave it | **Works.** TSF keeps the interim selection as the composition grows: after `SetText` it covers the whole composition, interim style included (read back with `GetSelection`) |

So the shell sets the interim selection when it can, accepts the one TSF kept when it can't, and
only then falls back to a caret. Contexts know whether they go through the IMM layer:
`TF_SS_TRANSITORY` in `ITfContext::GetStatus`. TSF-native contexts get a plain caret.

## Windows closes the Korean open/close compartment on activation

**Symptom.** The first build switched every app to ABC (passthrough) right after activating.

**Cause.** Windows sets `GUID_COMPARTMENT_KEYBOARD_OPENCLOSE` to closed when a Korean profile
activates, and the shell took that change for the user pressing 한/영.

**Solution.** 한/영's state lives in a TSF global compartment of JIS Korean's own (shared by every
app in the session, never stored). The open/close compartment only mirrors it, for the input
indicator: a change from anywhere else is put back. 한/영, the indicator's button and the menu are
the only ways to switch.

## Loading the dictionary blocked the first focus

Parsing the embedded hanja dictionary takes 80–110 ms and the predictor 16–31 ms per process
(measured). Both now load on a thread-pool thread when JIS Korean activates; the first key waits
only if they aren't ready yet.
