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

The one deliberate exception is the implied ㅇ (§4.2): a vowel with no initial types a full syllable
(ㅏ → 아) instead of a bare jamo. F6 right after commits the bare jamo instead.

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
| ^ | ^ | ^ | unassigned (types ^ either way) |
| ¥ | — | — | em dash, U+2014 |

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
| Z X C V | ㅋ ㅌ ㅊ ㅍ | same | standard 2-set |
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
| F10 | 반각영숫자 | Same, half-width. | done in prototype |

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
- **Android** (`/android`) and **iOS** (`/ios`): planned, empty for now.
- **Web** (`/web`): a React playground that runs the same engine in the browser, keyed on
  `KeyboardEvent.code`. On non-JIS keyboards, `IntlYen` and `IntlRo` don't exist, so the page needs
  on-screen keys for them.

---

## 9. Open questions

From the note:

1. **Symbols with no key.** Compared with stock JIS, these are now unreachable:
   ( ) # $ % & * + = | @ ` { } < > / and ¥. Most urgent: ( ) (common in Korean text), plus @ and /
   (email addresses, URLs, dates). The F8 escape covers them for now.
2. **Free key.** ^ is the only unassigned key on the number row.
3. **~ vs -.** ~ is also used with numbers (3~5일, 10:00~12:00) and now needs Shift released. A
   second ~ on Shift+^ would cover it.
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
