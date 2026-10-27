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
- `OnTestKeyDown` itself can be called twice for one key-down: see #5 below.

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

## The prediction list showed a certificate password

**Symptom (owner, 2026-10-21).** In a certificate program's password box, the prediction list
showed readings of the password: a screen recording would have captured it.

**What Windows does.** Fields marked as passwords get no TIP at all: an `ES_PASSWORD` Edit and a
TSF RichEdit with `IS_PASSWORD` never sent JIS Korean a key (`type-test.ps1 -Filter password`). The
certificate program's box isn't marked, so to Windows and to JIS Korean it's ordinary text.

**Solution.** The list window is excluded from capture (`SetWindowDisplayAffinity`,
`WDA_EXCLUDEFROMCAPTURE`). Fields that are marked are passed through by JIS Korean too, in case
one reaches it. Unmarked ones can't be detected: ABC (한/영) or the menu's "Off in <program>". The
log names the program on every line, so programs that need to be off by default can be added to
`kOffByDefault` in `windows/src/settings.cpp` once measured. A notice while the screen is recorded
was considered and left out: Windows has no API saying the screen is captured, guessing from
program names misfires, and the list is already out of captures.

## Loading the dictionary blocked the first focus

Parsing the embedded hanja dictionary takes 80–110 ms and the predictor 16–31 ms per process
(measured). Both now load on a thread-pool thread when JIS Korean activates; the first key waits
only if they aren't ready yet.

## A key-down tested twice ran the engine twice (#5)

**Symptom (reproduced).** `jk_testpad --peek 1 --probe 1` (a message loop that peeks each key and
calls `ITfKeystrokeMgr::TestKeyDown` on it before taking it) turned `KeyE KeyK KeyE KeyK Space` into
`ㄷ다앋다아  ` in the TSF RichEdit. The Edit and the IMM RichEdit were unaffected: there the peeked
key is already `VK_PROCESSKEY`.

**Change.** `OnTestKeyDown` remembers the key-down it ran the engine for and answers a second test
of it from that (`SameKeyDown` in `key_map.cpp`, unit-tested). The message time alone can't tell: a
test before the message was taken reads the *previous* message's time. So a first press (lParam bit
30 clear) is the same one when its lParam matches within 500 ms, since it can't come twice without
a key-up. An auto-repeat must match the message time too. The memory ends on that key's key-up
(`OnTestKeyUp`/`OnKeyUp`), on a focus change, or at the next key tested; the 2 s window is gone.
`OnKeyDown` still takes the result once, by virtual key, scan code and extended flag. The first
repeat test per program is logged ("a key-down was tested twice").

**Verified.** `type-test.ps1` `#5` cases: `다다 ` with peek+probe in edit/rich/tsf; held `KeyG`
(`D-KeyG` ×4) gives four ㅎ, with and without probing. Not measured: Chrome/Edge, Word, Windows
Terminal (look for the log line).

## Edit sessions hold the service, and never call the engine (#7)

**Symptom (read, not reproduced).** A session TSF queued (`TF_E_SYNCHRONOUS` → `TF_ES_ASYNCDONTCARE`)
held a raw `this`, so it could run after `Deactivate` freed the engine; it called `jk_flush` itself,
and dereferenced `ITfComposition::GetRange` without checking it.

**Change.** The session is `EditText`, holding a `ComPtr<TextService>`, the context and the
composition it was meant for. It writes only the text it was given:
- After `Deactivate` (`active_` false) it ends that composition with its commit and starts nothing.
- A composition that ended before a queued session ran is left as shown (nothing is inserted next to
  it); a composition without a range is dropped and what follows starts a new one.
- On commit the caret goes after the text only if it's still inside it, so a key the app already
  handled (arrow, Tab) isn't undone.
- A refused composition sets `refused_`; `Settle` lets the engine go of the segment after the
  session (or before the next key, when it was queued), committing what was shown
  (`jk_flush_prediction` when a prediction was).
- `RunEditSession` reports a queued session; it's logged once per program ("edit session queued").

**Verified.** `jk_testpad` switching to the Microsoft IME and back (`@off @on`) mid-composition, 15
times in one process, in edit/rich/tsf: no crash, every syllable committed once. Full
`type-test.ps1` passes. Not run: Application Verifier (needs elevation); a host that queues
sessions (Chromium) wasn't measured, so the async log line hasn't been seen yet.

## Open issues: root causes

Read from the code on macOS (2026-10-27, at `b5ed9dd`), not yet built or measured on Windows. Each
issue's body has the full analysis and the steps to verify it with `type-test.ps1` and the testpad.

- **#3, a key with no context.** With no `ITfContext` and nothing composed, both key handlers still
  run the engine and eat the key. `UpdateText` drops the text, so the jamo stay invisible in the
  engine, and `OnSetFocus` only flushes a composition it can see, so they surface in the next field.
  Fix: no target context → the key isn't eaten, and an invisible segment is cleared, also on focus change.
- **#4, `VK_PACKET`.** Characters injected with `KEYEVENTF_UNICODE` carry the character, not a scan
  code, in lParam. `EngineKey` reads it as a position (Space → ㅇ). Fix: `VK_PACKET` →
  `JK_KEY_OTHER`, which commits first and passes the character on.
- **#5, `OnTestKeyDown` twice.** Fixed (above).
- **#6, loose jamo.** A refused `StartComposition` flushes the engine at every key, and a
  composition the app ends right after the update (no interim selection kept) clears it. With a
  prediction shown, the refused path also inserts the prediction but flushes the Korean reading.
  Fix: log both per program first, then keep the range on screen and keep the engine's segment.
  Which host behaviour the spec wants is the owner's call ([decide]).
- **#7, edit sessions.** Fixed (above). Still open: re-sending a key the app got before a queued
  commit ([decide]); measure the "edit session queued" log line in Chromium first.
- **#8, commits through the engine.** The three bugs were fixed in a24d710. The shell's own list of
  committing keys remains, and goes when the engine decides that per key. Tab/↑/↓ with predictions
  already move through the list as on macOS (`HandleList`, decided). Tab is `JK_KEY_OTHER` on both
  platforms.
- **#9, the rest.**
  - Correctness: the stale `tested_` (see #5), `DllRelease` inside the thread-pool callback, unclamped
    `double` → `int` in training, the `Predictor()` free race, scan code 0.
  - Per-key cost: `PrivateField` and `ApplySettings`, and training on the UI thread.
  - Unbacked TSF categories: `UIELEMENTENABLED`, `INPUTMODECOMPARTMENT`.
  - JIS keys (半角/全角, 英数, カタカナ as `VK_KANA`), which need measuring and a spec line.

  With the F8 escape retired, Windows needed only its comment at `OnActivated` updated (done).
