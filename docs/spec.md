# JIS Korean: layout and behavior spec

Status: **draft**. Source: the note "2-set on JIS layout spec (draft)" and its interactive
prototype (https://claude.ai/artifact/5TpYiuCYgZk8grLq6Sp4Sa). Where the note is silent, this
spec writes down what the prototype does. Those sections are marked *(from prototype)*. Decisions
this spec had to make that neither source settles are marked **[decide]** and also appear in
[Open questions](#9-open-questions).

JIS Korean is a Korean 2-set (두벌식, KS X 5002) input method for a JIS physical keyboard. It
extends 2-set over the full area that JIS kana typing uses (1 to ¥, down to ろ) and over F6–F10.

---

## 1. Core principle

Every valid standard 2-set key and combination still types what it always did. A dedicated key
adds a shortcut; it never takes anything away.

- Shift+Q is still ㅃ, even though 1 also types ㅃ.
- ㅗ then ㅏ is still ㅘ, even though 0 also types ㅘ.
- Double finals (ㄹ+ㄱ → ㄺ etc.) compose as usual.

Three deliberate exceptions (§4.2 and §6):

- **Implied ㅇ:** a vowel with no initial types a full syllable (ㅏ → 아) instead of a bare jamo. F6
  right after commits the bare jamo instead.
- **Shift+vowel** (vowel keys whose Shift types nothing else: all but O and P) after a syllable
  starts a new syllable on ㅇ instead of acting like the plain vowel (마ㄹ+⇧ㅣ → 말이, not 마리).
- **Shift+Z is F10** (§6), not ㅋ: plain Z still types ㅋ.

Any change that breaks this principle is a spec change, not an implementation detail.

---

## 2. Physical keys

Keys are identified by **physical position**, never by the character the OS layout would produce.
This spec uses W3C `KeyboardEvent.code` names as the canonical IDs. Every platform shell maps its
native key codes to these.

| JIS legend | Canonical code | macOS virtual key | Windows scan code |
|---|---|---|---|
| 1 … 0 | `Digit1` … `Digit0` | ANSI positions | 0x02 … 0x0B |
| − | `Minus` | `kVK_ANSI_Minus` | 0x0C |
| ^ | `Equal` | `kVK_ANSI_Equal` | 0x0D |
| ¥ (left of Backspace) | `IntlYen` | `kVK_JIS_Yen` (0x5D) | 0x7D |
| Q … P | `KeyQ` … `KeyP` | ANSI positions | |
| @ | `BracketLeft` | `kVK_ANSI_LeftBracket` | 0x1A |
| [ | `BracketRight` | `kVK_ANSI_RightBracket` | 0x1B |
| A … L | `KeyA` … `KeyL` | ANSI positions | |
| ; | `Semicolon` | `kVK_ANSI_Semicolon` | 0x27 |
| : | `Quote` | `kVK_ANSI_Quote` | 0x28 |
| ] | `Backslash` | `kVK_ANSI_Backslash` | 0x2B |
| Z … M | `KeyZ` … `KeyM` | ANSI positions | |
| , . / | `Comma` `Period` `Slash` | ANSI positions | 0x33 0x34 0x35 |
| ろ | `IntlRo` | `kVK_JIS_Underscore` (0x5E) | 0x73 |
| F6 … F10 | `F6` … `F10` | `kVK_F6` … `kVK_F10` | 0x40 … 0x44 |

The platform columns are a starting point. **Verify** them on real hardware during each shell's
first spike.

---

## 3. Key map

Unless stated otherwise, every output below is a **character** (it goes into the text as-is),
except Hangul jamo, which go into the composition automaton (§4).

### 3.1 Number row

| Key | Unshifted | Shift | Notes |
|---|---|---|---|
| 1 | ㅃ | 1 | |
| 2 | ㅉ | 2 | |
| 3 | ㄸ | 3 | |
| 4 | ㄲ | 4 | |
| 5 | ㅆ | 5 | |
| 6 | ㅟ | 6 | rarest of the five; on the key some typists reach with the left hand |
| 7 | ㅝ | 7 | |
| 8 | ㅚ | 8 | most frequent of ㅟ ㅝ ㅚ; middle finger |
| 9 | ㅢ | 9 | placed for ergonomics |
| 0 | ㅘ | 0 | JIS kana わ is on this key |
| − | ! | ? | |
| ^ | ( | ) | parentheses, the most-missed symbols (were F8 only) |
| ¥ | — | ^ | em dash (U+2014); ^ moved here from the ^ key (^^ in chat) |

Digits are on Shift, so numbers type with Shift held.

### 3.2 Q row

| Key | Unshifted | Shift | Notes |
|---|---|---|---|
| Q W E R T | ㅂ ㅈ ㄷ ㄱ ㅅ | ㅃ ㅉ ㄸ ㄲ ㅆ | standard 2-set |
| Y U I | ㅛ ㅕ ㅑ | ㅛ ㅕ ㅑ | standard 2-set (Shift ignored) |
| O P | ㅐ ㅔ | ㅒ ㅖ | standard 2-set |
| @ | ' | " | straight quotes, U+0027 / U+0022 |
| [ | [ | 「 | U+300C |

### 3.3 A row

| Key | Unshifted | Shift | Notes |
|---|---|---|---|
| A S D F G | ㅁ ㄴ ㅇ ㄹ ㅎ | same | standard 2-set |
| H J K L | ㅗ ㅓ ㅏ ㅣ | same | standard 2-set |
| ; | ~ | - | - is on Shift so phone numbers and dates (010-1234-5678) type without releasing Shift |
| : | : | ; | * is dropped |
| ] | ] | 」 | U+300D |

### 3.4 Z row

| Key | Unshifted | Shift | Notes |
|---|---|---|---|
| Z | ㅋ | **F10** (half-width letters, §6) | Shift+Z isn't ㅋ: F10 isn't reachable everywhere (Windows browsers take it for their menu) |
| X C V | ㅌ ㅊ ㅍ | same | standard 2-set |
| B N M | ㅠ ㅜ ㅡ | same | standard 2-set |
| , | , | , | Shift ignored |
| . | . | . | Shift ignored |
| / | · | · | Korean middle dot, U+00B7; Shift ignored |
| ろ | \ | _ | unchanged JIS |

### 3.5 Keys outside this map

Keys not listed (Space, Enter, Backspace, Tab, arrows, modifiers, other F-keys, keypad) keep their
normal behavior, except as §5 and §6 define. Any keystroke with Ctrl, Alt/Option or Cmd/Win held
**commits the composition first** and then passes through to the app unchanged **[decide]**.

---

## 4. Hangul composition

### 4.1 Automaton

The standard 2-set automaton, with initial (초성), medial (중성) and final (종성) slots:

- **Initials:** ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ
- **Medials:** ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ
- **Finals:** ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ (ㄸ ㅃ ㅉ are never finals)
- **Compound medials by sequence:** ㅗㅏ→ㅘ, ㅗㅐ→ㅙ, ㅗㅣ→ㅚ, ㅜㅓ→ㅝ, ㅜㅔ→ㅞ, ㅜㅣ→ㅟ, ㅡㅣ→ㅢ
- **Double finals by sequence:** ㄱㅅ→ㄳ, ㄴㅈ→ㄵ, ㄴㅎ→ㄶ, ㄹㄱ→ㄺ, ㄹㅁ→ㄻ, ㄹㅂ→ㄼ, ㄹㅅ→ㄽ,
  ㄹㅌ→ㄾ, ㄹㅍ→ㄿ, ㄹㅎ→ㅀ, ㅂㅅ→ㅄ

Transitions:

| State | Input | Result |
|---|---|---|
| empty | consonant | start syllable with it as initial |
| empty | vowel | new syllable on an implied ㅇ (§4.2): ㅏ → 아 |
| initial only | vowel | set medial |
| initial only | consonant | flush the lone initial, start a new syllable |
| initial + medial | vowel that combines | compound medial |
| initial + medial | vowel that doesn't combine | new syllable on an implied ㅇ (§4.2): 가+ㅓ → 가어 |
| initial + medial | consonant that can be a final | set final |
| initial + medial | consonant that cannot be a final (ㄸ ㅃ ㅉ) | flush, start new syllable |
| with final | consonant that forms a double final | double final |
| with final | vowel | **move the (last) final to the next syllable:** 닭+ㅏ → 달가, 한+ㅏ → 하나 |
| with final | other consonant | flush, start new syllable |
| any | non-jamo character | flush, then emit the character |

A dedicated compound key (6–0) inputs that compound medial **directly**. It behaves exactly like
the same medial built by sequence, except that nothing combines further with it: ㅘ+ㅣ does not
become ㅙ (ㅙ is reached as ㅗ+ㅐ, as usual) *(from prototype)*, so a dedicated compound key after a
vowel always starts a new syllable (고+0 → 고와). A dedicated double-consonant key
(1–5) is identical to Shift+Q/W/E/R/T.

Output is precomposed Hangul syllables (U+AC00–U+D7A3) when an initial and a medial are present,
and compatibility jamo (U+3131–U+318E) otherwise.

### 4.2 Implied ㅇ

A vowel that can't attach to an initial gets an implied ㅇ and starts a syllable: at the start of
the segment, after a symbol, or after a syllable whose vowel it can't combine with.

| Keys | Result |
|---|---|
| ㅏ | 아 |
| ㅗ ㅏ | 와 (the vowels still combine) |
| ㄱ ㅏ ㅓ | 가어 |
| ㄱ ㅗ 0 | 고와 |
| ㅏ ㄴ ㅏ | 아나 (the implied ㅇ behaves like a typed one) |

**F6 right after** commits with the implied ㅇ removed: ㅏ F6 → ㅏ, 가어 F6 → 가ㅓ,
와 F6 → ㅘ. This works while the syllable has no final yet. Once it has one (안), F6 commits it as
it is. Being a commit, F6 ends everything: the next vowel gets an implied ㅇ again
(ㅏ F6 ㅏ → ㅏ, then 아 unconfirmed).

**Same vowel again keeps the final.** A vowel after a final takes it (말 ㅣ → 마리), but the
writer may have meant 말이. Pressing the **same vowel key again right away** puts the final back
and starts the vowel's own syllable on an implied ㅇ:

| Keys | Result |
|---|---|
| ㅁ ㅏ ㄹ ㅣ | 마리 |
| ㅁ ㅏ ㄹ ㅣ ㅣ | 말이 |
| ㄷ ㅏ ㄹ ㄱ ㅏ ㅏ | 닭아 (a double final goes back whole) |
| ㅁ ㅏ ㄹ 0 0 | 말와 |
| ㅁ ㅏ ㄹ ㅣ ㅣ ㅣ | 말이이 (only the first repeat toggles) |
| ㅁ ㅏ ㄹ ㅣ ㅏ | 마리아 (a different vowel doesn't toggle) |
| ㅁ ㅏ ㄹ ㅣ F6 ㅣ | 마리 committed, then 이 (F6 commits in between) |

The ㅇ from a double press was asked for, so F6 commits it as it is (말이 F6 → 말이), unlike a
plain implied ㅇ. Backspace removes the
repeat and gives the first reading back (말이 → 마리).

**Shift+vowel starts a syllable on ㅇ.** Shift on a vowel key whose Shift types nothing else (Y U I
H J K L B N M; not O and P, which type ㅒ ㅖ) finishes the syllable before it as it is, final
included, and starts a new syllable with ㅇ and that vowel. It's the single-keystroke alternative
to the double press, and it also keeps two vowels apart that would otherwise combine.

| Keys | Result |
|---|---|
| ㅁ ㅏ ㄹ ⇧ㅣ | 말이 (plain ㅣ: 마리) |
| ㄷ ㅏ ㄹ ㄱ ⇧ㅏ | 닭아 (a double final stays whole) |
| ㅗ ⇧ㅣ | 오이 (plain ㅣ: 외) |
| ㄱ ㅏ ㄴ ⇧ㅗ ㅏ | 간와 (the new vowel can still combine with the next one) |
| ⇧ㅏ | 아 (same as the implied ㅇ) |
| ⇧ㄲ ⇧ㅏ | 까: an initial waiting for its vowel takes it as usual, so Shift held over from a double is harmless |
| ㄱ ⇧ㅒ, 간 ⇧ㅖ | 걔, 가녜: O and P are unchanged |

The ㅇ was asked for, so F6 commits it as it is (unlike an implied ㅇ). Backspace removes the
whole Shift+vowel in one press. In the F8 escape, Shift behaves as in standard 2-set. The English
reading keeps the shifted letter (⇧L → `L`).

### 4.3 Backspace

While composing, Backspace removes the **last jamo keystroke** and recomposes:
간 → 가 → ㄱ → (empty). A compound medial typed as a sequence comes apart one key at a time
(과 → 고), and one typed with a dedicated key goes in one press (과 via 0 → ㄱ) *(from prototype)*.
An implied ㅇ isn't a keystroke, so it goes with its vowel (가어 → 가).
With nothing unconfirmed, Backspace passes through to the app.

---

## 5. Unconfirmed text and commit

The engine keeps an **unconfirmed segment**: the text typed since the last commit, shown in the
app as preedit (marked text / composition string). It holds composed Hangul, the syllable still
being composed, and pending symbols (§5.3). This is closer to the Japanese IME than to a per-syllable
Korean IME, and it is what F9/F10 need **[decide]**.

### 5.1 Commit triggers

| Input | Effect |
|---|---|
| Space | Commit the segment, then type a space. Space **never converts** (unlike the Japanese kana IME). |
| F6 | Commit the segment (§6). Nothing else is typed. |
| Shift+Enter | Commit the segment. Nothing else is typed. *(planned)* |
| Enter | Default: commit the segment, then pass Enter to the app. Setting "Enter commits": commit only, as Shift+Enter does. *(planned)* **[decide]** |
| Focus loss, app switch, IME switch | Commit the segment. |
| Ctrl/Alt/Cmd/Win shortcuts | Commit, then pass through (§3.5). |
| Arrow keys, Tab, Esc | Commit, then pass through **[decide]**. |

### 5.2 Words

A **word**, for F9/F10, is the trailing run of jamo keystrokes in the segment that each came from a
letter or digit key. A symbol, a digit typed with Shift or a conversion ends the
run *(from prototype)*. Each keystroke keeps its **raw character**: the Latin letter of a letter
key (uppercase when typed with Shift), or the digit of a number-row key (so key 1 = ㅃ has raw
character `1`).

### 5.3 Symbols and digits with F9/F10

Typed symbols and Shift+digits stay in the segment like everything else. When the segment **ends in
a run of ASCII symbols or digits**, F9/F10 convert that run instead of a word: `2026` F9 →
`２０２６`, 한! F9 → 한！. The run and a word never mix: a word ends at the first symbol, and a
symbol run ends at the first jamo. Case cycling and F6 (restore and commit) work the same way.
The full-width form of U+0021–U+007E is the code point + 0xFEE0; space becomes U+3000.

---

## 6. Function keys

| Key | Label | Behavior | Status |
|---|---|---|---|
| F6 | 한글 | **Right after F9/F10:** restore the Hangul of the converted word. **On a plain implied ㅇ:** drop it (§4.2). Then **commit** everything unconfirmed, as switching to English and back does (한 F6 ㅏ → 한 committed, 아 unconfirmed; one more F6 → 한ㅏ). F6 changes nothing else. No-op if nothing is unconfirmed. Also the return path from 한자 (F7). | done in prototype |
| F7 | 한자 | Hanja conversion of the word at the end of the unconfirmed text, with a candidate list (§7). Pairs with F6 (native / borrowed script), mirroring JIS F6 hiragana / F7 katakana. | done |
| F8 | 두벌식 | Transient escape to standard 2-set on JIS (§6.1). | done in prototype (IME switching not simulated) |
| F9 | 전각영숫자 | Convert the current word to full-width alphanumerics, from the raw characters (§5.2). | done in prototype |
| F10 | 반각영숫자 | Same, half-width. **Shift+Z does the same**, on every platform, since F10 isn't always reachable (Windows browsers take it for their menu). Not in the F8 escape, where Shift+Z is ㅋ. With predictions on (§10), when the top prediction is already byte-identical to what F10 would give, the first Shift+Z presses F10 twice (hello → HELLO); later presses step the cycle as F10 does. F10 itself never skips. | done |

### 6.1 F8 escape

- F8 switches to **standard 2-set over the stock JIS keys**: the letter keys keep 2-set, and every
  other key types its stock JIS character (1 = `1`, Shift+2 = `"`, @ = `@`, etc.).
- F8 while already escaped stays escaped (one-way key) *(from prototype)*.
- Shift+F8, or switching to another IME and back, returns to the JIS Korean layout.
- The prototype carries a syllable being composed across the switch. Whether F8 should commit first
  is open.

### 6.2 F9 / F10 cycling *(from prototype)*

- The first press converts the word as typed (raw case kept).
- Repeated presses of the same key cycle **as typed → UPPER → Capitalized → as typed …**
- Pressing the other key switches width and keeps the current case step.
- F6 right after either key restores the original Hangul and commits it.
- Any other input ends the conversion state. The converted text stays in the segment as plain
  characters.
- With no word to convert, F9/F10 are no-ops.

Example: ㅎ ㅏ ㄴ (keys G K S) → 한. F10 → `gks`. F10 → `GKS`. F10 → `Gks`. F9 → `Ｇｋｓ`. F6 → 한, committed.

---

## 7. Hanja (F7)

Basic support: dictionary lookup with a candidate list. No learning, no reconversion of committed
text.

### 7.1 Dictionary

`engine/dict/hanja.tsv`, built by `engine/dict/build_dict.py` from:

- **libhangul `data/hanja/hanja.txt`** (BSD-3-Clause; the data file is BSD even though libhangul's
  code is LGPL): every single-syllable entry with its meaning (學: 배울 학), and the first two
  forms of each word up to four syllables.
- **Unicode Unihan `kHangul`** (Unicode License v3): syllables in KS X 1001 or the education set
  that `hanja.txt` lacks.

Candidates keep source order, which is roughly by frequency. The engine does no I/O: each shell
loads the file and hands it to the engine (`jk_dict_new`), and engines share one parsed copy. The
license files sit next to the dictionary and ship with it.

### 7.2 Keys

| State | Key | Effect |
|---|---|---|
| no list | F7 | Open a list for the **longest dictionary word at the end of the unconfirmed text**: 나는학교 → 학교. Only whole syllables count. No match, or no dictionary: nothing happens. |
| list open | F7, ↓ | Next candidate (wraps) |
| | Shift+F7, ↑ | Previous candidate (wraps) |
| | → / ← | Next / previous page of 9 |
| | 1–9 (number row, no Shift) | Pick that candidate on the current page |
| | Enter | Pick the selected candidate. No new line. |
| | Esc, Backspace | Close the list; the Hangul stays unconfirmed and editable |
| | F6 | Close the list and commit the Hangul (back to Hangul, spec §6) |
| | anything else | Pick the selected candidate, then handle the key as usual (가 F7 Space → 可 committed + space) |
| | focus loss | Commit the selected candidate |

While the list is open, the preedit shows the **selected candidate in place** of the reading. A
pick stays unconfirmed like any other text. Earlier syllables keep their jamo, so Backspace still
works on them by jamo. When the reading doesn't start on a keystroke boundary (말이 typed with
the double press, then F7 on 이), the text before it becomes plain characters first.

Shells show the page that holds the selected candidate, numbered 1–9, with comments. The web
playground draws it under the caret; the macOS shell for now shows only the inline preview.

---

## 8. Platforms

All behavior above lives in `/engine`. Platform shells only translate between OS events and the
engine:

- **macOS** (`/mac`): an InputMethodKit input method.
- **Windows** (`/windows`): a TSF text input processor. It runs on the Japanese (106/109) hardware
  keyboard setting so JIS scan codes arrive intact. Password fields and the secure desktop do not load
  TIPs. That is a known limit.
- **Minecraft** (`/minecraft`): a Fabric mod running the engine (as wasm) in-game. It adds its own
  layer on top of the engine: each staging area is read as Korean and as English (`Engine::english`),
  Space/Enter/Tab commit the more likely one, F6 forces Korean and **F7 forces English instead of
  hanja**. See `minecraft/README.md`.
- **iPadOS** (`/ipados`): planned, empty for now. Android and iPhone are out of scope.
- **Web** (`/web`): a React playground that runs the same engine in the browser, keyed on
  `KeyboardEvent.code`. On non-JIS keyboards, `IntlYen` and `IntlRo` don't exist, so the page needs
  on-screen keys for them.

---

## 9. Open questions

From the note:

1. **Symbols with no key.** Compared with stock JIS, these are now unreachable:
   # $ % & * + = | @ ` { } < > / and ¥. Most urgent: @ and / (email addresses, URLs, dates). The
   F8 escape covers them for now. ~~( )~~ Settled: on the ^ key (( unshifted, ) with Shift).
2. ~~**Free key.**~~ Settled: the ^ key types ( and ); ^ moved to Shift+¥.
3. ~~**~ vs -.**~~ Settled: ~ stays on the ; key unshifted; the ^ key went to ( ) instead.
4. **F8 and composition.** The prototype carries the syllable being composed across the F8 switch.
   Should F8 commit first, like F6 does?
5. ~~**F6 vs Shift+Enter.**~~ Settled: F6 commits everything unconfirmed (after dropping a plain
   implied ㅇ or undoing F9/F10). Shift+Enter commits as-is.
6. **Shift on the jamo number keys.** Shift on 1–0 types digits, and the jamo on those keys exist
   only unshifted. Confirm this is final.

Raised by this spec:

7. **Segment model (§5).** Keep the whole segment unconfirmed (Japanese style, needed for F9 over
   whole words and for pending symbols), or commit each finished syllable (Korean style, what apps
   and users of Korean IMEs expect)?
8. **Plain Enter default (§5.1).** Commit + newline by default, with "Enter commits" as the opt-in.
   Is that the intended reading?
9. **Digits inside a word (§5.2).** Shift+digit ends the F9 word, so `abc123` converts as `abc`
   then `123` (each with its own F9/F10, §5.3). Is that enough?
10. **Arrows, Tab, Esc (§5.1).** Outside the hanja list they commit then pass through. Should Esc
    cancel the segment instead?
11. **Non-JIS hardware.** Supported at all on desktop (ANSI has no ¥ or ろ key), or JIS only?

---

## 10. Prediction *(draft)*

An optional layer on top of the deterministic engine that ranks **readings of the staged,
uncommitted text**. It never changes what the keys do; it only offers alternatives to commit.

### 10.1 Lite and full

| | Lite | Full |
|---|---|---|
| Build | `cargo build --no-default-features` | default (`predict` feature) |
| Run time | — | prediction is on only after the host loads a predictor (`jk_predictor_new`, `jk_set_predictor`); with none set, the full build behaves exactly like lite |
| What the user gets | §1–§7 only: Space commits the JIS Korean reading | ranked readings to choose from, in addition |

Hosts choose per user setting. Turning it off is always a complete fallback: no state, no model,
no behavior change.

### 10.2 Readings (candidates)

| Kind | Reading | Example (keys → text) |
|---|---|---|
| Korean | JIS Korean, as composed | `gksrmf` → 한글 |
| English | the same keys on JIS QWERTY (stock JIS symbols) | `gksrmf` → gksrmf |
| English, Capitalized / UPPER | case variants, **only when English ranks first** | `hello` → Hello, HELLO |
| Standard 2-set | plain 2-set on stock JIS: no implied ㅇ, no double press, Shift+vowel is just the vowel, number-row keys type digits | `bb` → ㅠㅠ (JIS Korean: 유유); `rk1` → 가1 |
| Korean, finals kept | each vowel after a final starts its own syllable on ㅇ | `akfl` → 말이 (JIS Korean: 마리) |
| Bare vowels | JIS Korean without the implied ㅇ (§10.3) | `bb` → ㅠㅠ, `nn` → ㅜㅜ |
| English + particle | an English start with a Korean particle or ending (§10.3) | Anthropic에서, report에, RLHF에, server를 |
| Hanja | the top dictionary form of the **whole** Korean reading, 2+ syllables only; plus the everyday exceptions (§10.4) | 학교 → 學校, 2026년 → 2026年 |

Duplicates are removed (most readings coincide for most words), and **at most 4** are offered.

### 10.3 Ranking

Scores are in bits per key, positive leaning Korean:

1. **Model:** the same keys under two key-trigram models, English spelling vs JIS Korean typing
   (`engine/predict/ranker.bin`, built by `minecraft/tools/build_ranker.py`; held-out accuracy
   about 99% on words, 96% on real Korean text). English nouns train at 3× weight.
   **Nouns weigh more than verbs:** an English reading that is a noun in its normalized form
   (Princeton WordNet 3.0 lemmas, 3+ letters: report, server, model) gets a bonus; verbs,
   adjectives and inflected or other forms (published, reports) don't. In mixed Korean text English
   words are mostly nouns. Cost on 8,774 real Korean words: 0.73% → 0.76% ranked as English (0.81% after the corpus retrain, §10.9).
2. **Rules:** a capital at the start of a word on a key Korean never shifts means English. On a
   vowel key it may be Shift+vowel starting the word (있습니다 as `L T t m q s l e k`): unless the
   keys spell a known English word, it then leans English by 2 bits instead of settling it, and a
   known Korean word cancels the lean. Two capitals in a row (an acronym: KPI와) still settle it.
   After any key of the word, a number-row key included (있 as `l 5`), a capital on a vowel key is
   Shift+vowel. Exact
   matches in `predict/common-ko.txt` / `common-en.txt` lean their way; leftover bare jamo count
   against Korean; a command name (after `/`) is English; the previous committed word's language
   tips close calls. A known Korean word against keys that spell no known English word or noun is
   Korean (몇 over `auc`, 어디야 over `jeli`), unless the Shift rule settled it.
3. **Alternatives** rank one step below the reading they vary, unless they're a known word (so ㅠㅠ
   beats 유유). When both are words, their counts decide (§10.9): an alternative needs to be more
   than twice as common to pass the reading the keys spell (저녁 stays 저녁, not 전역).
4. **Implied ㅇ:** each implied ㅇ is weighed against its bare vowel. It's likely in real words
   (아이, 이거) and unlikely in runs of one vowel that chat doubles as jamo (유유 → ㅠㅠ: the
   lexicon counts ㅠㅠ above 유유; 워워 and 오오 stay syllables) or in known chat words, which the
   bare reading then outranks. **A final kept before a vowel** (한글 맞춤법 제14·15·19항: a final is spelled where it is
   when a particle or ending follows the word it ends, and moves within one morpheme): the engine
   offers one variant per place the keys moved a final, with that one kept and the vowel on ㅇ
   (기준입니다 for 기주닙니다 typed `rlwnslqslek`, 당신이, 어린이). Each kept final scores:
   - at least +1.5 bits when the reading up to it is a known word of two syllables or more (not a
     하다/되다 modifier such as 반영한) and the rest begins with a particle or ending on ㅇ (이 은 을
     의 에 에서 으로 입니다 이에요 아 어 았 었 으면 음 …);
   - else the corpus odds learned for ㄴ and ㄹ (`#!implied-after` lines in common-ko.txt, by the
     next syllable's vowel: 입 counts as 이; 의 strongly kept, 아 and 와 mostly moved), or −2 bits
     for other finals;
   - a double final (값이, 많이) +2 bits when its stem is a known word, −2 otherwise (지난주 typed
     `wlskswn` isn't 지낝우);
   - a ㅇ final followed by a particle that only follows consonants, as the rest of the word (은 을
     과 으로 이랑 이나; not 이, which ends many words: 아이), +2 bits: a ㅇ final that moves leaves
     the next syllable as it is, and that particle can't follow a vowel (소비량은 typed
     `thqlfidms`, not 소비랴은).
   Only what a variant keeps beyond the typed reading counts, and word counts decide between two
   known words (아니 over 안이). A test types 31 edge cases from the 국립국어원 rules the lazy way
   (드리겠습니다, 지난주, 사람 move; 기준입니다, 웃음, 많이, 소비량은 keep; 산이/사니, 너머/넘어
   both offered), and 6 more for the ㅇ final: 35 of 37 pass, the misses being stems the corpora
   don't have (값, 바둑). When the bare reading is also the plain 2-set one (ㅏㅣ for `kl`), it's
   one row weighed by both. A single vowel key is its syllable, never a letter: ㅗ alone is 오, not
   `h`, since a lone lowercase letter is hardly ever an English word.
5. **English + particle:** the keys split into an English start and a Korean ending that is a known
   particle (에, 에서, 를, 이라는, 부터는, …) or a known word made of 하다 / 되다 forms (push했어). The split counts when the start reads clearly more English than
   the whole keys (0.5 bits per key; a capitalized start such as Anthropic or RLHF always
   qualifies) and the whole isn't a known English word or noun, so `small` never becomes smal이 and
   Goodwill never Goodwil이. Among capitalized starts the longer ending wins (API를, not APIf을).
   The start must also read as English on its own (1 bit per key), and when the whole keys make a
   Korean word (in the lexicon or the hanja dictionary: `kanxms` is 아무튼, not kanx은) the split is
   only an alternative below it; junk Korean (slack에 as 님차에) doesn't hold it back. Otherwise it
   ranks above the whole-word English reading. A regression test types 8,700+ real Korean words the
   JIS Korean way, with the hanja dictionary: 0.13% rank as mixed (mostly fragments such as 년까지).
6. **Hanja are lean:** one dictionary candidate at most, whole words of 2+ syllables only, and a
   heavy penalty that keeps it last. It shows only when there's room. The exceptions (§10.4) aren't
   penalized.

### 10.4 Everyday hanja (not penalized)

These tie with the Korean reading and follow it right after:

- **Country names in the news, for OECD members** (38 as of 2026-10): 美 미, 日 일, 韓 한, 英 영,
  佛 불, 獨 독, 伊 이 (Italy), 以 이 (Israel), 加 가, 濠 호, 土 토. Alone or in runs (韓美). The
  other members' traditional forms (丁 諾 芬 匈 波 白 墺 葡 希 墨 西) aren't in everyday news use and
  are left out; 中 is everyday but China isn't an OECD member.
- **Dates:** 年 (년, 연), 月, 日, and digits before them (2026年, 10月, 19日), also with a particle
  after them (1日까지, 3日부터); 曰 after a dictionary
  stem (孔子曰).
- **Weekdays:** 月火水木金土日, alone or in runs.

### 10.5 User tuning

Each scenario has a weight, a multiplier on its learned probability: 1.0 is as learned, English
0.8 makes English readings 0.8 times as likely, and Korean 1.2 makes Korean ones 1.2 times as
likely. A weight adds log2(weight) bits to that reading's score (per key for Korean and English,
per implied ㅇ for the implied ㅇ). Scenarios: Korean, English, implied ㅇ, English + particle,
plain 2-set, finals kept, hanja, implied ㄹ after ㄴ. Weights are clamped to 0.1–10; 0 turns an
alternative scenario off entirely (its readings aren't offered), but not Korean or English, which
always show. They never override the hard rules (a capital at the start of a word, command names).

**Implied ㄹ after ㄴ:** ㄴ then ㄹ sounds ㄹㄹ (전력 [절력]), so a typist going by sound may leave the
ㄹ out: the keys of 저녁 (`wjsur`). After a ㄴ final, each vowel the composition put a moved ㄴ on
also gets a reading with an ㄹ before it (전력). It's an alternative: one step below the typed
reading unless it's a known word, with word counts deciding between two known words. Both are
often valid (저녁, 전력), so it's a slider (default 1.0; 0.00 turns it off; macOS
`Weight.ImpliedL`).

Hosts store them per user: the web playground has sliders under "Tune predictions" (remembered
in the browser); macOS reads `defaults write me.sjun.inputmethod.JISKorean Weight.<Scenario>`.

### 10.6 Open questions

- Should the hosts' Space commit the top prediction (as the Minecraft mod does) or keep committing
  the JIS Korean reading, with predictions as a picker only?
- The Minecraft mod still has its own Java copy of the ranker; it should move to this layer
  (through the wasm engine) so there's one implementation.
- Hosts so far: the web playground (predictions on by default, picker under the caret, Space
  commits the top reading) and macOS (marked text shows the top reading, committing keys take it;
  "Predictions" menu item; `make PREDICT=0` strips the layer). macOS has no picker yet: the other
  readings need an IMK candidate window.
- Learning from the user's picks (session or persistent) isn't in the draft.

### 10.7 Travel ranker (experimental, side by side)

A second way to rank the implied-ㅇ alternatives (bare vowels: ㅠㅠ against 유유; a kept final: 말이
against 마리), chosen per engine (`jk_set_ranker`: `JK_RANKER_RULES` by default, `JK_RANKER_TRAVEL`,
`JK_RANKER_ROUTE` for §10.8). Everything else ranks the same.

- **Finger travel** (`engine/src/travel.rs`): key centers on a JIS 109 board in key units, each key
  pressed by its touch-typing finger (F6–F10 by the finger under them). A press moves its finger to
  the key and every other finger back home. Shift and other modifiers move nothing; thumbs rest on
  Space. The planner (`plan.rs`) can minimize travel instead of presses (`Metric::Travel`) and
  reports it either way. Consequence: ㅇ (`d`) is a home key, so the implied ㅇ saves a press but no
  travel; a dedicated number-row key saves Shift but costs travel.
- **Ranking:** an alternative scores the Korean reading's score plus the lexicon's evidence
  (§10.9), less the effort a typist who meant it skipped: the presses and travel of the cheapest way
  to type it (planned for least travel) beyond those of the staged keys, at `press` bits per press
  and `travel` bits per key unit. No one-step-below penalty, no run-of-one-vowel rule. Retrained
  (2026-10-19), both costs are best at 0: the effort term adds nothing, and only the lexicon
  decides.

### 10.8 Route-preference ranker (experimental, side by side)

The question: given typing each jamo against JIS Korean's optimized route, how strongly would a
typist prefer the optimized one? Each reading R has an **each-jamo route** (plain 2-set, one jamo at
a time: ㅇ as `d`, Shift for doubles, two keys for compound vowels and double finals) and an
**optimized route** (the cheapest JIS Korean route: implied ㅇ, dedicated keys, Shift+vowel, double
press). A typist takes the optimized one with odds p = σ(press × presses saved + travel × travel
saved − bias), clamped to 2–98%.

- **The Korean reading:** if the keys are its each-jamo route, they were taken with 1 − p (or for
  sure when no cheaper route exists, as for 마리); otherwise they're a route with shortcuts, taken
  with p for what they save over each jamo.
- **An alternative:** if the keys are its each-jamo route (ㅠㅠ as `b b`), likewise; otherwise the
  keys leave out (or add) presses it needs, at `omission` bits per edit to the nearer of its two
  routes (말이 typed `akfl` is one press short).
- **Score:** Korean score + lexicon evidence (§10.9) + a prior against bare jamo (one step per
  jamo inside a word, ㅗㅗ백년; one step for a word of mixed jamo, ㅓㅏㅂ; none for a run of one
  jamo, the chat genre ㅠㅠㅠ) + log2 P(keys | alternative) − log2 P(keys | Korean).
- **Parameters** (near the best of the grid): press 0, travel −0.5, bias −1, omission 3. Presses
  saved don't move the preference, travel saved lowers it slightly, and a typist takes JIS Korean's
  route about 3 times in 4.

### 10.9 Lexicon counts and the side-by-side evaluation

- **Counts:** `common-*.txt` hold a hand-written list, then (below a marker) words generated from
  colloquial corpora with their counts (`word<TAB>count`), and business terms, each counted the same
  (20, or more if chat uses them more) whatever its frequency. Generated Korean words need 8 uses
  (one-syllable words 20, and a lone jamo never counts); English ones 30 and 3+ letters. Sources and notices:
  `engine/predict/LICENSE-corpora.txt`; fetch with `minecraft/tools/fetch_corpora.py`, train with
  `build_ranker.py --corpora corpora`. 1 in 10 sentences is held out. Lexicon evidence for an
  alternative against the Korean reading: +4 bits if only the alternative is a word, −2 if only the
  Korean one is, and when both are, 2 bits plus log2 of their count ratio, within ±4. All three
  rankers share it (the rules ranker otherwise unchanged).
- **Evaluation:** `cargo run --release --example sxs` scores the three on words typed properly with
  an implied ㅇ (A), ㅇ after a final typed lazily (B), held-out bare-vowel chat words (C), held-out
  chat eojeols (D), held-out English words (E) and Korean business terms (F), with grids over the
  travel and route parameters. 2026-10-19, errors on A + C + D′ (implied ㅇ in chat): rules 82,
  travel 86, route 77 (75 at omission 5, at the cost of B). E and F are the same for all. Per
  prediction (native, release): rules ≈5 µs, travel ≈14 µs, route ≈17 µs.
- `examples/abc.rs` types texts into all three behind blind labels for rater evaluations.
- The web playground can show all three lists (Tune predictions → Implied ㅇ ranking → Side by
  side).

### 10.10 Punctuation inside a word, and the trusting typist

- **Typed punctuation stays as typed** in the English reading (a `;` typed is `;`; F9/F10 still
  convert keys to their stock JIS characters).
- **Pieces:** punctuation typed inside a staged word (anything that isn't a letter or digit) splits
  it into pieces, each ranked on its own. The pieces' top readings, joined with their punctuation,
  form one more reading (Research(이하, AI)를, 기여(contribute)하는, SaaS(Software-as-a-Service)에),
  ranked above the whole-word readings.
- **The trusting typist** (`engine/src/trust.rs`, `tests/corpora.rs`) types a text with the fewest
  presses and lets the predictions do the rest: Hangul by JIS Korean's optimized route, Latin
  letter by letter (a capital Z as z, since Shift+Z is F10), hanja by their reading, symbols by their
  key or the F8 escape (a flat 3 presses, as in the planner); Space or Enter commits the top
  prediction. Hanja are the exception: never the top prediction, they're picked with ↓ (each a
  press) and fail only when not listed. A test asserts no hanja is ever on top. Measures: failed
  words, characters wrong, and **effort**: presses plus the cost of fixing failures (↓ to the word
  when listed, else Backspace and the word again without predictions), against the planner's
  presses without predictions.
- **Typing styles:** real typists don't always take the cheapest way. The planner (`plan::Style`)
  can type each syllable the cheapest way, every jamo (ㅇ typed, Shift for doubles, two keys for
  compound vowels and double finals), or a deterministic mix of all the valid ways (의 as `9`, `d9`
  or `dml`; 당신이 as `ekdtlsL`, `ekdtlsll` or `ekdtlsdl`). The tests type the owner's texts in all
  of them (0 failures each) and the AI-extended ones with per-style ratchets. Shift+vowel is a
  deliberate signal that the ㅇ was wanted, so with one in the keys the plain 2-set reading gets no
  lexicon advantage (열어 typed `d u f ⇧J` isn't 여러), and a word started with one holds back an
  English + particle split like a known Korean word does (오픈 typed `⇧H v m s` isn't Hv은).
- **Training on mixed routes:** the key model learns each Korean word typed every jamo (25%), the
  cheapest way (35%) and two random mixes of the valid ways (20% each). Number-row keys are read as
  their 2-set keys, as the predictor maps them. Held-out accuracy on mixed routes: 99.0%.
- **Texts:** `engine/tests/corpora-en-ko/` holds the owner's texts verbatim (ratchet: 0 failures)
  and `extended/` their continuations by AI agents, marked "human corpus extended by AI" on the
  first line (not typed). They weigh ¼ in the weighted failure rate and have their own ratchet.
  2026-10-19: owner's texts 0/115 failed (effort 2.7% below no predictions on the mixed one),
  AI-extended 3/2008, weighted 0.12%. `examples/trust.rs` runs any texts with all three rankers.

### 10.11 Learning from the user's choices (opt-in, on the device)

Off by default. When on, gentle multipliers on the §10.5 scenario weights follow what the user
picks. All logic is in the engine (`engine/src/learn.rs`, pure and deterministic); hosts only store
one line of counts and show the setting.

- **Signals:** the engine records them itself when a prediction is committed.
  - **Override:** a lower prediction chosen (↓ or a click) over the top one.
  - **Accept:** the top one committed while other kinds were shown.
  - **Regret:** Backspace right after an accept, counted as half a demotion.
  - Only kinds that were shown count, so rarely shown kinds don't look unpopular. A choice between
    two readings of the same kind says nothing.
- **Attribution:** a choice between the Korean reading and one of its alternatives (bare vowels,
  kept finals, 2-set, English + particle, hanja) moves only that alternative's scenario. Only
  Korean against English moves those two. The implied-ㅇ scenario's counts are about the
  bare-vowel reading, so its learned weight is the inverse (picking ㅠㅠ makes the implied ㅇ less
  likely).
- **Update:** per scenario, the rate at which it's chosen when shown below the top, and the rate at
  which it's passed over when on top, each with 100 pseudo-counts at a 5% prior. The weight is
  exp(0.35 × (log-odds up − log-odds down)), clamped to 0.5–2× (a user's own weights may go
  0.1–10). One choice moves a weight about 7%; ten about 45%; around thirty reach the clamp. Counts
  halve every 400 events.
- **Precedence:** a weight the user set by hand wins over the learned one until cleared
  (`jk_clear_prediction_weight`).
- **Hard guards that learning can't cross:**
  - A hanja reading is never the top prediction, whatever any weight says; the first other reading
    moves up.
  - The Shift rule and the chat-run rule are not weights and don't learn.
- **Stored:** no text, no keys, no times, no app names, only the counts (`jk_learner_export`, one
  line). Nothing leaves the device.
  - **macOS:** the app's preferences, keys `LearnFromChoices` and `Learned`. The input menu has
    "Learn from My Choices (Stays on This Mac)" and "Learned Weights" (read-only, with "Reset What
    It Learned").
  - **Web:** the browser's local storage only, forgotten 24 hours after learning started, whether or
    not it's the same session. The age is checked when the page loads, on every save, and once a
    minute.
  - **Minecraft:** not yet; it would need the mod's config.
- **Validation** (`tests/learn.rs`): simulated users with a habit the defaults don't match.
  - A bare-vowel chatter: half the habit is on top within three rounds. ㅗㅗ and ㅏㅏ stay a ↓ away,
    because the lexicon knows 오오 and 아아 and the clamp holds.
  - A Korean-first typist: all on top by round two.
  - A hanja lover: the hanja weight reaches the 2× clamp, and hanja still never come first.
  - Learning off learns nothing, and a trained profile leaves ordinary words alone.
  - The owner's texts, typed attentively with learning on five times over, stay at 0 failures with
    no extra ↓.

### 10.11.1 Per app (macOS)

- **Disable / Resume Learning from <app>** (input menu): choices made in that app aren't recorded
  (`jk_set_recording`); what was learned still applies there.
- **Disable / Resume Predicting in <app>**: no predictions there (the deterministic engine).
- **Off by default:** predictions start off, until the user turns them on, in apps where typing is
  commands, paths, passwords or a game's chat:
  - terminals;
  - file transfer and SSH (Transmit, Cyberduck, FileZilla, ForkLift, Termius…);
  - containers and VMs (Docker, OrbStack, Parallels, UTM…);
  - remote desktop;
  - password managers;
  - Finder and launchers;
  - Minecraft Java Edition, where the JIS Korean mod is the dedicated integration;
  - any bundle id with "terminal", "ssh", "sftp" or ".ftp".
  It's detected when JIS Korean is first used there: nothing is added to the settings until the
  user changes it. The first time, a small notice under the input menu's icon gives the reason
  (sensitive app, or dedicated mod recommended).
- **Manage Disabled Apps…**: a list of the apps with a setting, each with its canonical full path,
  fetched when the list opens. An uninstalled app stays listed, marked with an ⓘ saying it was
  deleted, until the user removes it (with a confirmation).

### 8.1 Keyboard Viewer (macOS)

While JIS Korean is active, macOS uses a private layout bundled with the input method
(`JISKorean.keylayout`, selected with `TISSetInputMethodKeyboardLayoutOverride` on each
activation).
- **Generated at build time** by `mac/tools/make-keylayout` from the engine's own legends
  (`cargo run --example legends`), so it can't drift from §3.
- **Its plain and Shift layers** carry JIS Korean's legends. Shift+Z shows F10.
- **Its Cmd, Ctrl and Option layers, and unlabelled keys,** are ABC's on a JIS keyboard, so
  shortcuts and Option characters are unchanged.
- **Typing is unaffected:** the input method reads key codes, not characters.

### 10.12 Training files (the training contract)

A training file tunes the predictions to one person from their own data, prepared by any tool or an
AI agent: `docs/training-contract.md` (the contract, with instructions and questions for agents),
`docs/training-contract.schema.json`, `docs/training-example.json`. A JSON object with `format`
`"jis-korean-training"`, `version` 1, and optionally:

- `weights`: the §10.5 sliders, applied as set by hand (0 turns an alternative off);
- `vocabulary`: the user's Korean and English words with counts, added to the word lists
  (`jk_predictor_with_words`: a new predictor, the base unchanged);
- `examples`: words as meant, optionally with the keys typed: each is typed into a scratch engine
  and, where it's listed, learned as a choice (`jk_learn_example`; §10.11's learner, so it counts
  while learning is on, and applying a file with examples turns learning on);
- `typos` and `preferences`: kept, not acted on in version 1.

Applied on the device only: the web playground has a drop zone under "Tune predictions" (words kept
in the browser until "Forget training words"; learned examples expire with the 24-hour rule);
macOS reads `~/Library/Application Support/JIS Korean/training.json` on activation when it changed
(input menu → Learned Weights → Open Training Folder), and forgets its words when it's removed.
