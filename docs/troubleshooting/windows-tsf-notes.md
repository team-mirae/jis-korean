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

## The engine decides which keys commit what's shown (#8)

**Symptom (reproduced).** The shell kept its own list of keys that commit a shown prediction
(shortcuts, Tab, Space, Enter, Esc, arrows, `JK_KEY_OTHER`). A key outside the mode's map that
isn't on the list (Backquote in Mode A) committed the Korean reading under the prediction:
`hello` + Backquote gave ``옫이애` ``.

**Change.** `HandleKey` asks `jk_key_commits`, as macOS does, and sends those keys through
`jk_key_after_prediction`. `HandleList` stays: ↓/Tab and ↑/Shift+Tab move through the list, Enter
commits a chosen row, Esc goes back to none chosen (spec §5.1). Esc with nothing chosen commits
what's shown and passes on (spec §5.1 / #16 default).

**Verified.** `type-test.ps1` cases: ``hello` ``, Esc commits `hello` and reaches the app, Tab Tab
Esc Esc, ↓↓↑ and Tab Tab Shift+Tab, plus the existing Tab / Shift+Enter cases.

## Apps that end or refuse the composition: loose jamo (#6)

**Measured.** Logged once per program: "composition refused hr=…" and "composition terminated N ms
after update". Two `jk_testpad` targets end every composition 0–47 ms after the shell's update:
- `cancel`, an Edit (IMM layer) completing it with `ImmNotifyIME(CPS_COMPLETE)`;
- `canceltsf`, a TSF RichEdit ending it with `ITfContextOwnerCompositionServices::TerminateComposition`.

Both typed `KeyE KeyK KeyF KeyK KeyQ KeyK KeyD` as `ㄷ아ㄹ아ㅂ아ㅇ` instead of `다라방`. No host
that refuses `StartComposition` was found: a read-only TSF RichEdit never sends the TIP a key.

**Change (#6 option A, behind `kKeepTextWithoutComposition`; the owner hasn't decided it).** When
the app ends the composition during the shell's update or less than 100 ms after it
(`kOwnTermination`), or refuses one, the text stays as plain text (`orphan_`) and the engine keeps its
segment. The next update replaces it: with a new composition over it if the app takes one, else as
plain text. It's dropped (the engine starts fresh, the text stays as typed) when the caret has left
it before a key, on a focus change, or when the text left the document. Flushes (focus, 한/영, menu)
commit into it. The edit session still never calls the engine.

**What it can't fix.** Through the IMM layer (a transitory context), the ended composition's text
leaves the TSF document as the app's result string: a range over it is empty at the next key, and
replacing it appended instead (`ㄷ다달다라…`). So transitory contexts keep the old behaviour, and
the `cancel` target still gives `ㄷ아ㄹ아ㅂ아ㅇ`. Classic Win32 controls keep the interim selection
(above), so they don't end compositions in the first place. Chromium is TSF-native, where the fix
applies, but it wasn't measured.

**Found on the way.** The TSF RichEdit ends the composition by itself when a `VK_PACKET` arrives,
31 ms after the last update, which the 100 ms rule takes for the shell's own. The kept range then
grew over the injected space and the next commit replaced it (`하하x` for `하 하x`). Kept ranges
now have inward gravity (`TF_GRAVITY_FORWARD`, `TF_GRAVITY_BACKWARD`): text typed next to them
stays out.

**Verified.** `canceltsf`: `다라방 ` with the constant on, `ㄷ아ㄹ아ㅂ아ㅇ ` with it off. Same text
as the ordinary TSF RichEdit for Backspace by jamo, F7 + Enter, Tab through predictions, ArrowLeft,
F10 ×2, Esc and `goddmf`. `type-test.ps1` `#6` cases; full run passes.

## A key with no input context (#3)

**Measured.** In a window whose input context was removed (`ImmAssociateContext(hwnd, NULL)`,
`jk_testpad --target nocontext`), TSF never called the key sink: WASD reached the window and
nothing carried into the next field, before the change too. Not reproduced, then; a host that
calls the sink with no context (`pic` null) wasn't found.

**Change (a guard).** Both key handlers resolve one target context (the key's, else the
composition's). With none, the key isn't eaten and any unconfirmed text is dropped
(`DropInvisible`, logged once per program: "a key without an input context"). 한/영 still works
there: it only writes the session's compartment. A focus change with unconfirmed text but no
composition drops it too, and after every synchronous update the engine may not hold unconfirmed
text that no composition shows ("unconfirmed text with nothing showing it: dropped"), which also
covers #6 and #7.

**Verified.** `type-test.ps1` `#3` case: WASD in `nocontext`, then `한 ` in the Edit with nothing
before it. Full run passes.

## Characters injected as Unicode (`VK_PACKET`) (#4)

**Measured.** `SendInput` with `KEYEVENTF_UNICODE` (`jk_testpad` `U+XXXX`) does reach the TIP's
key sink as `VK_PACKET`, but with lParam `0x1`: scan code 0, not the character's low byte as the
issue assumed. Scan 0 already mapped to `JK_KEY_OTHER`, so the characters came through right
before the change too (`a b,/05Vs}감`, and `하 하x` while composing).

**Change.** `EngineKey` maps `VK_PACKET` to `JK_KEY_OTHER` before looking at the scan code, so it
stays a character (commit, then pass it on) whatever lParam carries, and the scan-code-0 recovery
(#9) can't turn it into a position. The first `VK_PACKET` per program is logged with its lParam.

**Verified.** Unit test (`EngineKey(0xE7, s)` for eight low bytes) and `type-test.ps1` `#4` cases in
edit/rich/tsf, on 한글 and in ABC. Not tried: KeePass auto-type, AutoHotkey `SendText`.

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

- **#3, a key with no context.** Not reproduced (TSF didn't call the sink), guarded (above).
- **#4, `VK_PACKET`.** Not reproduced (lParam's scan code is 0), hardened (above).
- **#5, `OnTestKeyDown` twice.** Fixed (above).
- **#6, loose jamo.** Partly fixed (above): TSF-native hosts keep composing; IMM-layer hosts that
  end compositions still get loose jamo. Spec §8.4's line is the owner's call ([decide]).
- **#7, edit sessions.** Fixed (above). Still open: re-sending a key the app got before a queued
  commit ([decide]); measure the "edit session queued" log line in Chromium first.
- **#8, commits through the engine.** Fixed (above); Tab is still `JK_KEY_OTHER` on both
  platforms.
- **#9, the rest.**
  - Correctness: the stale `tested_` (see #5), `DllRelease` inside the thread-pool callback, unclamped
    `double` → `int` in training, the `Predictor()` free race, scan code 0.
  - Per-key cost: `PrivateField` and `ApplySettings`, and training on the UI thread.
  - Unbacked TSF categories: `UIELEMENTENABLED`, `INPUTMODECOMPARTMENT`.
  - JIS keys (半角/全角, 英数, カタカナ as `VK_KANA`), which need measuring and a spec line.

  With the F8 escape retired, Windows needed only its comment at `OnActivated` updated (done).
