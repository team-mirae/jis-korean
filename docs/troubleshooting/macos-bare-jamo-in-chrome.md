# macOS: typing commits bare jamo instead of syllables (Chrome and other apps)

Investigated and fixed 2026-10-23 on macOS 27.0 (Darwin 27.0.0). Fix committed in `d8d9577`
("re-sync the app's keyboard layout, not just the engine, to the current mode"). The behavior it
documents for password fields and old Chrome is in spec §8.1; this file is the diagnosis and the
fix, and how to tell the fixable case from the two that aren't.

## Symptom

With JIS Korean selected and a Korean mode current, some fields type the jamo apart instead of
composing them: `바다사자` comes out as `ㅂㅏㄷㅏㅅㅏㅈㅏ`. It is per field, not per app, and it
stuck: one Chrome tab (or the omnibox) would compose normally while another, or the same field
later, committed bare jamo and stayed that way. It was most visible in Chrome but not unique to it.

## The one diagnostic that localizes it

The implied-ㅇ rule (spec §1, §4.2): a lone `ㅏ` composes to `아`. So a bare `ㅏ` on screen is not a
syllable the engine built and committed early — it is a jamo the **engine never saw**. If the
engine had received the key and flushed per keystroke you would get `아`, not `ㅏ`. Bare jamo means
the key reached the app without going through the engine.

## How a key becomes bare jamo (the mechanism, spec §8.1)

The engine reads **key codes**, not characters. When it does not consume a key (an English mode
passing a letter on, a shortcut, or any key for a field that never asked the input method), macOS
types that key through the **active keyboard layout**. JIS Korean's Korean-mode layouts
(`JISKorean.keylayout` and the ANSI/ISO variants) carry the mode's **jamo legends** on their plain
and Shift layers, because that is what Keyboard Viewer shows while a Korean mode is active. So an
unconsumed key, in a Korean mode, is typed by the layout as a bare jamo. Apple's own Korean input
method has the same exposure through its `2SetHangul` layout.

So "bare jamo" always means: *the key did not reach the engine, and the Korean layout typed it.*
There are three ways that happens. One was our bug; two are not fixable from our side.

## Root cause of the stuck case (ours): the layout and the engine disagreed per app

`overrideKeyboard(withKeyboardNamed:)` selects the layout **per client process, not per field**, and
macOS drops the override when the input method is deactivated in that app. An earlier fix
(`0d51744`) re-synced each field's **engine mode** to the current mode, but not the layout. Chrome's
omnibox and its page run as **separate IMK controllers that activate out of order**, so this could
happen inside one Chrome process:

1. A Korean field activates → names the Korean layout for the whole Chrome process.
2. Another field activates whose engine is on an English mode → it passes its letters on,
   **unconsumed**, expecting ABC underneath.
3. The letters are typed by the Korean layout still named on the process → bare jamo. Permanent,
   because nothing re-named the layout for the second field.

Three selection races made the mode itself drift, which fed the same mechanism:

- An out-of-order activation could revert the input method's idea of the `current` mode to the
  source a field was *leaving* rather than the one being selected.
- The input method is never the **active** app, and AppKit **suspends** a background app's
  distributed-notification delivery; under the default *coalesce* behavior macOS's own
  "selected input source changed" notice could be held back, so the input method missed the switch.
- `current` started at **Mode A** at launch regardless of the source actually selected — and an
  install restarts the input method, so right after an update it could be wrong.

## The fix (`d8d9577`)

- **`syncMode` re-names the field's mode's layout** when the app's last-named layout differs, engine
  mode or not. `KeyboardViewerLayout` now tracks the named layout **per app** and forgets it on
  deactivate (macOS has dropped the override by then). This is the core fix: the layout can no
  longer be left on Korean for a field whose engine is English.
- A selection JIS Korean **asks** macOS for counts as made until macOS reports it (2 s cap), so a
  field that activates in that window reads the target mode, not the source being left.
- `current` initializes from the **selected source at launch**, not Mode A.
- The selected-source notification is observed with **`.deliverImmediately`**, so suspension can't
  hold it back.
- The language switch key acts on the field in the **frontmost** app (the one being typed in), not
  merely the last controller to receive a key.
- Unrelated but in the same commit: the suggestion list's caret round trip
  (`attributes(forCharacterIndex:lineHeightRectangle:)`) moved **off the key path**. It was a
  blocking IPC into Chrome's page process on every keystroke.

## The two cases that are not fixable from our side

Both are the same mechanism — the key never reaches the input method — so both show bare jamo in a
Korean mode, and neither is our bug:

- **Password fields.** macOS bypasses input methods for secure text entry. Every key is typed by the
  layout. Expected; Apple's Korean does the same. Type Korean in a normal field.
- **Chrome 152–156, a field that was ever a password.** Chromium returns a **nil
  `NSTextInputContext`** for a field once it has had `TEXT_INPUT_FLAG_HAS_BEEN_PASSWORD`
  (Chromium issue 567321058), so keys bypass the input method even after the field turns back into a
  normal one. Fixed in Chrome 157. Update Chrome, or reload the page so the field is recreated.

## How to tell which case you are in

Turn the diagnostic log on (it logs no text — only key codes' consumed state, modes and layouts):

```sh
defaults write me.sjun.inputmethod.JISKorean DiagnosticLog -bool true   # off: -bool false
```

Type a few keys in the failing field and read `~/Library/Logs/JISKorean.log`:

- A line **`key in <app>: consumed true, engine mode 0, layout JISKorean, current A`** for each key
  means the engine is receiving and consuming them: the fix is working and that field composes. If a
  field still shows bare jamo while these lines appear with `consumed false` and an English
  `engine mode`/`layout`, that is the old bug — capture the surrounding `activated`/`layout resync`
  lines.
- **No `key in …` line at all** for the field while bare jamo appears means the key never reached
  the input method: a password field, or Chrome 152–156's ex-password field. Not our side; see
  above.
- `activated in <app>: selected <X>, current <Y>, using <Z>` and `layout resync <old> -> <new> in
  <app>` show the mode/layout decisions as fields activate; `a key took <ms> ms in <app>` flags a
  slow key path.

## Honest limit of this investigation

The fix is from code analysis plus the implied-ㅇ diagnostic above and the layout/launch mechanics
confirmed in `macos-input-method-not-launching.md`. The live end-user symptom was **not reproduced
in this session**: driving a real foreground GUI app autonomously isn't possible here (a
terminal-spawned test app never becomes frontmost, so the input method never activates for it, the
same caveat that doc records). Confirm by typing `바다사자` in a normal Chrome page field, in the
omnibox, and across two tabs, with the log on.
