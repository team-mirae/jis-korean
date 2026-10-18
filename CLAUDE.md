# JIS Korean

A Korean 2-set (두벌식) input method for JIS physical keyboards. The extra JIS keys get dedicated
double consonants, compound vowels, Korean punctuation and Japanese-IME-style F6–F10 conversions.

## Source of truth

- **`docs/spec.md`** defines all behavior. Read it before changing anything in `/engine`.
- If code and spec disagree, the spec wins, or the spec gets updated in the same change. Never let
  them drift.
- Items marked **[decide]** or listed under "Open questions" are not settled. Don't implement a guess
  silently: ask, or implement the spec's stated default and say so.
- Origin: the owner's note "2-set on JIS layout spec (draft)" and its prototype artifact
  (https://claude.ai/artifact/5TpYiuCYgZk8grLq6Sp4Sa).

## Core principle (never break)

Every valid standard 2-set key and combination still types what it always did. Dedicated keys add
shortcuts; they never remove anything. Shift+Q is still ㅃ, ㅗ+ㅏ is still ㅘ, and double finals
still compose. The one sanctioned exception is the implied ㅇ (spec §4.2): ㅏ → 아, undone by F6.

## Layout

| Path | What |
|---|---|
| `/engine` | Core engine: key map, Hangul automaton, segment/commit logic, F6–F10. Platform-independent, no OS APIs, no I/O. |
| `/mac` | macOS input method (InputMethodKit) that wraps the engine. |
| `/windows` | Windows TSF text input processor that wraps the engine. |
| `/android`, `/ios` | Planned mobile input methods. Empty for now. |
| `/web` | React playground that runs the engine in the browser so people can try it before installing. |
| `/docs` | Spec and design notes. |

## Architecture rules

- **All behavior lives in `/engine`.** Shells translate OS key events into canonical key codes
  (`KeyboardEvent.code` names, see spec §2) and apply the engine's output (commit text, preedit,
  candidates) to the host. A shell that decides what a key types is a bug.
- The engine API is **event in → actions out**: `(key code, modifiers) → { commit, preedit, consumed }`.
  No callbacks into the shell, no global state. A shell can then hold one engine instance per input
  context.
- The engine must be deterministic and never panic or crash on any input. On Windows it runs inside
  every app's process.

## Testing

- The engine gets **golden tables**: key sequences → expected commit and preedit, covering every
  automaton transition, backspace, each F-key, and every core-principle example in the spec.
- Every spec example (e.g. `한 F6 ㅏ → 한ㅏ`, the F9/F10 cycle in §6.2) must be a test.
- Run with `cargo test` in `/engine`. `tests/header.rs` fails if `include/jis_korean.h` drifts from
  the key list.
- Shells are tested by hand against the app matrix (to be written per platform).

## Conventions

- User-facing name: **JIS Korean**.
- Write spec examples with jamo and JIS legends (`ㅎ ㅏ ㄴ`, `F6`), and write code-level tests with
  canonical key codes (`KeyG KeyK KeyS`).
- Unicode code points in the spec and in comments as `U+XXXX`.
