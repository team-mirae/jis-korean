# JIS Korean: layout and behavior spec

Status: **draft**. Source: the note "2-set on JIS layout spec (draft)" and its interactive
prototype (https://claude.ai/artifact/5TpYiuCYgZk8grLq6Sp4Sa). Where the note is silent, this
spec writes down what the prototype does. Those sections are marked *(from prototype)*. Decisions
this spec had to make that neither source settles are marked **[decide]** and also appear in
[Open questions](#9-open-questions).

JIS Korean is a Korean 2-set (두벌식, KS X 5002) input method for a JIS physical keyboard. It
extends 2-set over the full area that JIS kana typing uses (1 to ¥, down to ろ) and over F6–F10.
That is **Mode A**, the original layout, and everything below describes it unless it says
otherwise. **Mode B** carries the parts that fit onto US ANSI keyboards (§3.6).

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

### 3.6 Modes

A **mode** is one keyboard's key map. Every Korean mode keeps standard 2-set on the 26 letter keys
(§1), Shift+Z as F10, F6–F10, the automaton (§4) and everything after §4. A mode only says what
the other keys type, and which stock keyboard sits under it. Mode A is the default.

| Mode | Name | Keyboard | Stock keyboard |
|---|---|---|---|
| A | JIS Korean Original Layout | JIS | JIS |
| B | JIS Korean for US ANSI | US ANSI (HHKB US, MacBook US) | US ANSI |
| C | JIS Korean for UK ISO | ISO (UK) | UK ISO (PC legends) |

**Mode C** is for ISO keyboards, whose extra key between left Shift and Z (`IntlBackslash`) neither
A nor B has. It follows Mode B, on UK stock symbols (other ISO countries can come later as their own
modes or forks; their users are expected to keep their own layout as a second input source):

| Key | Unshifted | Shift |
|---|---|---|
| Number row 1–0 | ㅃ ㅉ ㄸ ㄲ ㅆ ㅟ ㅝ ㅚ ㅢ ㅘ, as Mode A | the digit |
| [ (`BracketLeft`) | [ | 「 |
| ] (`BracketRight`) | ] | 」 |
| the extra key (`IntlBackslash`) | · (U+00B7) | — (U+2014) |
| everything else | the UK stock symbol | the UK stock symbol |

The three symbol keys ([, ], `IntlBackslash`) are optional: a setting, **Dedicated symbol keys**,
turns them off, and then they type their UK ISO characters (`[ {`, `] }`, `\ |`), the same as in
the F8 escape. They don't pass through to the OS layout, so they type the same whatever it is. The
number row is always dedicated. Default: on.
`IntlBackslash` isn't in Modes A or B, nor `IntlYen` and `IntlRo` in Mode C: they commit and pass
through (§3.5). On macOS, ISO keyboards send the extra key and the key left of 1 with each other's
key codes; the shell maps them back (§2).

**Mode B** keeps Mode A's number row (§3.1: ㅃ ㅉ ㄸ ㄲ ㅆ ㅟ ㅝ ㅚ ㅢ ㅘ, digits on Shift) and moves the
brackets to the US `[` and `]` keys:

| Key | Unshifted | Shift |
|---|---|---|
| [ (`BracketLeft`) | [ | 「 |
| ] (`BracketRight`) | ] | 」 |

Every other key types what it does on US ANSI, **by design**: - = \ ` ; ' , . / keep their
unshifted and shifted characters, so every symbol stays where a US typist expects it. `IntlYen` and
`IntlRo` aren't in Mode B: they commit and pass through (§3.5). `Backquote` isn't in Mode A (on a JIS
board it's the 半角/全角 key).

US ANSI users are encouraged to use Mode B as-is, to keep every symbol where they know it, or to
fork it into a mode of their own that fits their typing or their keyboard (docs/modes.md).

The stock keyboard drives everything that reads keys as plain characters, so nothing implies
JIS in Mode B:

- **F8** (§6.1) escapes to standard 2-set over the stock keys: in Mode B, Shift+2 = `@`.
- **English readings** (`Engine::english`) and the **standard 2-set reading** (§10.2) use the
  stock characters: in Mode B, `ㅎ ㅏ ㄴ Shift+2` (G K S Shift+2) predicts 한2 and 한@, where Mode A
  predicts 한2 and 한".

Examples (Mode B): `Shift+[` → 「; `Shift+2` → 2, read in English as @; `F8 Shift+2` → @;
`ㅎ ㅏ ㄴ Shift+Z` → gks.

#### 3.6.1 Mode ids and names

Every mode has three identifiers:

- **Id:** a number in the ranges below (0–99, and 10000–11000 for previews). Ids are saved in settings and passed over the
  C ABI (`jk_set_mode`), so an id is never renumbered or reused, even when its mode is removed.
- **Mode name:** what tools, settings and the public API (§8.2) use. Upper-case snake case: words of
  `A`–`Z` and `0`–`9` joined by `_`, starting with a letter (`A_QWERTY`,
  `SPLIT_ORTHO_LAYOUTDESIGNERNAME`). Unique, and never renamed once released.
- **Display name:** what a menu shows (`JIS English`). Free text, unique, and it can be translated.

Ids are in ranges; they don't have to be given in order:

| Ids | For |
|---|---|
| 0–14 | Mirae Research's official modes. 0–8 are taken or reserved (below); 9–14 are held for future official designations. |
| 15–30 | Official modes contributed from outside, each accepted by a majority of the contributors. |
| 31–99 | Custom layouts: any free id, chosen by whoever makes the layout. Nothing coordinates them, so two forks can pick the same one. |
| 10000–11000 | Development and preview builds: a temporary id for a mode still being made. When it's done it gets a permanent id in its range (0–30 for an official mode), and the temporary one is dropped. Never saved by a release build's settings. |

Every other id is invalid for now (0–99 may grow later). Every platform can hold ids up to
2,147,483,647 (Minecraft's Java `int` is the narrowest), so the ranges have room to grow.

**Ids are looked up, never counted.** Ids have gaps (3, then 9 or 31), and a build has only the
modes it was built with (JIS English needs `ENGLISH=1`; reserved ids have no mode yet). So the
engine keeps an explicit table of the modes it has, each with its id, and nothing assumes ids run
from 0 without gaps:

- **Listing:** `jk_mode_count` says how many modes this build has, and `jk_mode_id_at(index)` gives
  each one's id, in the table's order (`Mode::ALL` in Rust). The index is only a position in that
  list, never an id.
- **An unknown id is refused, not replaced.** `jk_set_mode` returns 1 and switches, or returns 0 and
  keeps the current mode for an id this build doesn't have. `jk_legend_in` gives no legend (0), and
  the name and mode-name calls give an empty string. A shell that loads a saved id this build
  doesn't have (say, JIS English after a build without it) falls back to Mode A itself, visibly.
- **Names:** `jk_mode_name_ptr`/`_len` give the display name, `jk_mode_code_ptr`/`_len` the mode name
  (`A`, `A_QWERTY`).

A mode name's first word is its **namespace**:

| Namespace | Whose | Rules |
|---|---|---|
| `A_` | Mirae Research's official modes only (ids 0–14) | Shipped and maintained here. |
| `B_` | Contributed variants of Mode B, for US ANSI boards (ids 15–30) | The stricter guidelines in `docs/modes.md`: review, tests and docs as for an official mode, and a majority of the contributors in favour. Minor adjustments on an ANSI board, such as Korean ANSI or a modified laptop keyboard. |
| `C_` | Contributed variants of Mode C, for ISO boards (ids 15–30) | The same guidelines. Other ISO locales than C's UK. |
| Anything else (`COMPANYNAME_QWERTZ`, `COMPANYNAME_CUSTOMLAYOUTNAME`) | Whoever makes it, in a fork or their own build (ids 31–99) | Only the naming rules and §3.6's "what every mode keeps"; not reviewed here. Use a prefix of your own, such as your company or your name. |

**The first three modes keep their one-letter names**, `A`, `B` and `C` (ids 0, 1, 2), as released:
tools and the public API already use them. A one-letter name has no `_`, so it's in no namespace,
and no new mode gets one.

**Scope.** Mode B is US ANSI and Mode C is UK ISO; their variants (`B_`, `C_`) stay on that board
type. No mode forbids any keyboard, but support covers only the board a mode is for: a `B_` variant
that doesn't work on an ISO board, for example, isn't an issue this repository will fix.

| Id | Mode name | Display name | Badge text | Status |
|---|---|---|---|---|
| 0 | `A` | JIS Korean Original Layout | (keys) | Shipped |
| 1 | `B` | JIS Korean for US ANSI | (keys) | Shipped |
| 2 | `C` | JIS Korean for UK ISO | (keys) | Shipped |
| 3 | `A_QWERTY` | JIS English | ABC | Specified (§3.6.2) |
| 4 | `A_QWERTZ` | **[decide]** | QZ | Reserved |
| 5 | `A_SWISS_QZ` | **[decide]** | CH | Reserved |
| 6 | `A_AZERTY` | **[decide]** | AY | Reserved |
| 7 | `A_DVORAK` | **[decide]** | DV | Reserved |
| 8 | `A_COLEMAK` | **[decide]** | CMK | Reserved |

A **reserved** mode has its id and name held for it, and nothing else yet: no key table, no build.
Ids 4–8 are Mirae Research's English-layout modes in the family of JIS English (§3.6.2): each its
layout's letters, with the symbols still to be decided per mode, and a badge of the taegeuk with its
badge text over the lower right, as `ABC` on JIS English's. Ids 9–14 are held for Mirae Research's
next official modes.

#### 3.6.2 `A_QWERTY`: JIS English

**JIS English** types English: the standard JIS QWERTY layout, with JIS English legends and JIS
symbols, on any keyboard (Shift+2 is `"`, the key right of P is `@`, `IntlYen` is `¥`, `IntlRo` is
`\`). It's the Mac's ABC for JIS Korean users: an English input source in the same input method as
the Korean modes, so switching between them never leaves JIS Korean (§8.2.1).

It isn't a 2-set mode (nor are ids 4–8): none of §3.1–3.5 or §4 onward applies. It behaves like
the Japanese input method's Romaji mode without its Japanese conversions: the engine consumes no key
(every key goes to the app, typed by the mode's keyboard layout, §8.1) and composes nothing; no
predictions, hanja, F6, F7 or F8. Caps Lock types capitals. It adds press and hold (§3.6.3) and
F9/F10 on the word just typed, with Shift+Z as F10 as in the Korean modes (§3.6.4). On macOS,
Shift+X goes back to Korean by default (§8.2.1); capital X is then x followed by Shift+Z, or Caps
Lock. Switching in commits whatever a Korean mode left unconfirmed. The core principle (§1) is about the Korean modes and
is unaffected. The engine still names the layout: its key table is the JIS stock table (Mode A's
stock keyboard), which the macOS keyboard layout is generated from, so the symbols can't drift from
Mode A's F8 escape.

It's **optional**: built only on request (the engine's `english` feature; `make ENGLISH=1` on macOS),
so a default build has the Korean modes only. Its id and name are reserved either way.

#### 3.6.3 Press and hold (JIS English)

Holding a letter key offers its accented and other variants, as macOS does for its English layouts
(which it does only for plain keyboard layouts, never inside an input method, so JIS English has its
own). The letter is typed as usual; its first auto-repeat (`JK_MOD_REPEAT`) opens the list instead
of repeating it. The letter already typed is taken back into the preedit (`take_back`: 1), and the
list shows its variants, none selected:

| Letter | Variants (1, 2, …) | With Shift |
|---|---|---|
| a | à á â ä æ ã å ā | À Á Â Ä Æ Ã Å Ā |
| c | ç ć č | Ç Ć Č |
| e | è é ê ë ē ė ę | È É Ê Ë Ē Ė Ę |
| i | î ï í ī į ì | Î Ï Í Ī Į Ì |
| l | ł | Ł |
| n | ñ ń | Ñ Ń |
| o | ô ö ò ó œ ø ō õ | Ô Ö Ò Ó Œ Ø Ō Õ |
| s | ß ś š | Ś Š |
| u | û ü ù ú ū | Û Ü Ù Ú Ū |
| y | ÿ | Ÿ |
| z | ž ź ż | Ž Ź Ż |

While the list is open:

- **1–9** pick that variant. A digit past the list keeps the letter and is typed after it.
- **The arrows** choose (the preedit shows the choice); from none, → and ↓ go to the first, ← and ↑
  to the last, wrapping. **Enter** commits the choice; with none chosen it keeps the letter and
  goes on to the app.
- **Esc** keeps the letter. **Backspace** removes it.
- **The held key's own repeats** do nothing more.
- **Any other key** (Space, a letter, a shortcut) keeps what shows, the choice or the letter, and
  goes on to the app.

Other keys repeat as usual: a key without variants, a repeat whose key or Shift differs from the
press before it, and anything with Ctrl, Option or Cmd. The Korean modes ignore `JK_MOD_REPEAT`.

With Caps Lock on, the letter and its variants are capitals.

Examples (key codes; `…` is the key's auto-repeat): `KeyU KeyU… Digit2` → ü; `S-KeyU S-KeyU… Digit2`
→ Ü; `KeyE KeyE… ArrowLeft Enter` → ę; `KeyA KeyA… KeyB` → ab; `KeyO KeyO… Backspace` → nothing.

#### 3.6.4 F9 / F10 in JIS English

JIS English keeps the Japanese input method's F9/F10 (zenkaku / hankaku) on the word just typed, so a
capital is a lower-case letter plus Shift+Z. The engine follows the **word just typed** from the keys
it lets through: ASCII letters and digits extend it, in the case the app typed (it's told Caps
Lock's state, `JK_MOD_CAPS`); Backspace shortens it; anything else ends it (Space, symbols, Enter,
arrows, shortcuts), and so does anything that moves the caret elsewhere (a click, a focus change, a
mode switch: flush and activate).

- **F10, or Shift+Z,** takes the word back into the preedit (`take_back`: its length) as **UPPER**,
  then cycles **Capitalized → as typed → UPPER …**. (§6.2's cycle starts at "as typed", but here
  that's what the document already has, so F10 starts one step on.)
- **F9** takes it back in **full width**, as typed, then cycles UPPER → Capitalized → as typed.
- Switching between F9 and F10 changes the width and keeps the case, as in §6.2.
- **Esc** puts back the word as typed. **Any other key** keeps the conversion and goes on to the app.
- With no word, F9, F10 and Shift+Z do nothing (and aren't typed).
- After a press-and-hold list, the letter it leaves (if it's a plain letter) is part of the word.

Examples: `KeyX S-KeyZ` → X; `KeyA KeyB KeyC F10 F10` → Abc; `KeyA Digit1 F9` → ａ１;
`KeyA KeyB KeyC F10 Escape` → abc; `KeyA Space S-KeyZ` → a and a space.

Examples: `KeyG KeyK KeyS` → `gks`; `S-Digit2` → `"`; `BracketLeft` → `@`; `IntlYen` → `¥`.

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
- **Windows** (`/windows`): a TSF text input processor. It reads scan codes, so the keys are the
  same whatever layout Windows has loaded; on a JIS keyboard, the Japanese (106/109) hardware
  keyboard setting makes ¥ and ろ arrive. Password fields and the secure desktop do not load TIPs.
  That is a known limit. Modes, 한/영 and settings: §8.4.
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
11. ~~**Non-JIS hardware.**~~ Settled: Mode B (§3.6) for US ANSI. It keeps Mode A's number row
    and brackets and leaves every other key as US ANSI has it.
12. ~~**`B_` and `C_` namespaces.**~~ Settled (§3.6.1): `B_` varies Mode B on US ANSI boards,
    `C_` varies Mode C on ISO boards; each is supported on its board type only.
13. **JIS English on other platforms (§3.6.2).** Windows keeps 한/영 (§8.4). Should Windows, the
    web playground and Minecraft offer `A_QWERTY` too, or is it macOS only?

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
| English | the same keys on QWERTY (the mode's stock symbols, §3.6) | `gksrmf` → gksrmf |
| English, Capitalized / UPPER | case variants, **only when English ranks first** | `hello` → Hello, HELLO |
| Standard 2-set | plain 2-set on the mode's stock keyboard (JIS, or US ANSI in Mode B): no implied ㅇ, no double press, Shift+vowel is just the vowel, number-row keys type digits | `bb` → ㅠㅠ (JIS Korean: 유유); `rk1` → 가1 |
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
in the browser); macOS reads `defaults write me.sjun.inputmethod.JISKorean Weight.<Scenario>`;
Windows reads `HKCU\Software\JIS Korean`, value `Weight.<Scenario>` (§8.4).

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
  - **Windows:** `HKCU\Software\JIS Korean`, values `LearnFromChoices` and `Learned`. The
    indicator's menu has "Learn from My Choices (Stays on This PC)" and "Learned Weights"
    (read-only, with "Reset What It Learned").
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

While JIS Korean is active, macOS uses a private layout bundled with the input method, one per
mode: `JISKorean.keylayout` for Mode A, `JISKoreanANSI.keylayout` for Mode B and
`JISKoreanISO.keylayout` for Mode C. The input method
names the mode's layout to the client on each activation (`overrideKeyboard(withKeyboardNamed:)`).
- **Generated at build time** by `mac/tools/make-keylayout` from the engine's own legends
  (`cargo run --example legends <mode id>`), so it can't drift from §3.
- **Its plain and Shift layers** carry the mode's legends. Shift+Z shows F10.
- **Its Cmd, Ctrl and Option layers, and unlabelled keys,** are ABC's on the mode's keyboard (JIS
  for Mode A, ANSI for Mode B, ISO for Mode C), so shortcuts and Option characters are unchanged.
- **Typing is unaffected:** the input method reads key codes, not characters.

### 8.2 Modes as input sources, and the public API (macOS)

Each mode (§3.6) is its own **input mode** of the one input method, the way Japanese has Kana and
Romaji:

| Mode | Input source ID | Shown as |
|---|---|---|
| A | `me.sjun.inputmethod.JISKorean.2Set` | JIS Korean |
| B | `me.sjun.inputmethod.JISKorean.2SetANSI` | JIS Korean for US ANSI |
| C | `me.sjun.inputmethod.JISKorean.2SetISO` | JIS Korean for UK ISO |
| `A_QWERTY` | `me.sjun.inputmethod.JISKorean.A_QWERTY` | JIS English |

Modes after C take their mode name as the last part of the input source ID (§3.6.1).

The user adds the ones they want in System Settings → Keyboard → Input Sources (Korean → JIS
Korean). Installing enables Mode A only.

**Why input sources, and not a setting.** The mode follows the keyboard, and switching apps
doesn't change which keyboard the user is typing on, so a per-app mode would mean nothing. As input
sources, the modes are visible to everything that already understands input sources: the input
menu, System Settings, and tools JIS Korean has no integration with, such as Hammerspoon and
Karabiner, which can read and select them by ID. Never rename an ID: those tools match on it.

**Public API.** The JIS Korean program also answers any other program, without an input-source
API:

| | How | Allowed |
|---|---|---|
| Read the mode | `defaults read me.sjun.inputmethod.JISKorean CurrentMode` → the mode name, `A`, `B`, `C` or `A_QWERTY` (also `CurrentModeID`, `CurrentModeInputSourceID`, `CurrentModeName`), or `JISKorean.app/Contents/MacOS/JISKorean mode` | always |
| Hear changes | distributed notification `me.sjun.inputmethod.JISKorean.ModeDidChange`, user info `mode`, `id`, `inputSourceID`, `name` | always |
| Change the mode | distributed notification `me.sjun.inputmethod.JISKorean.SetMode`, user info `mode` (a mode name, an engine id or an input source ID), or `JISKorean mode B` | only with **Let Other Apps Change the Mode** on, in the input menu with JIS Korean's other settings (off by default) |

A change request is answered with `ModeDidChange`, or with
`me.sjun.inputmethod.JISKorean.ModeChangeRefused` and a `reason`: the setting is off, the mode is
unknown, or the mode isn't added in System Settings (the API never adds one). `CurrentMode` is the
mode last used, whether or not JIS Korean is the selected input source now; the selected source
says that. Selecting the input source directly (the input menu, `TISSelectInputSource`,
Hammerspoon's `hs.keycodes.currentSourceID`) is macOS's own switching and isn't gated by the
setting.

### 8.2.1 Switching between Korean and English (macOS)

On Windows, 한/영 switches JIS Korean off and on inside one input method (§8.4). On macOS the same
idea is an input mode, JIS English (`A_QWERTY`, §3.6.2), instead of a hidden on/off flag:

- **Why a mode, not a flag.** A flag inside the controller would switch fastest, but macOS wouldn't
  know about it: the menu bar would keep the Korean icon, 英数 and Ctrl+Space would go to Apple's
  ABC, and the state would be lost whenever macOS reselects an input source.
- **Why a mode, not a separate input method.** Each input method is its own process. Switching
  between two (JIS Korean and ABC) has macOS deselect one, select the other and reconnect the app,
  asynchronously, so a key pressed right after can land in the wrong one. Switching between modes of
  one input method stays in the same process and the same session with the app, which is how
  Japanese switches between Kana and Romaji.
- **It's macOS's English input source.** It's declared with the Roman script, so 英数 selects it
  (as with Japanese input methods' own Romaji modes), Ctrl+Space switches between it and the last
  Korean mode, and the menu bar shows its badge (`docs/logo/mode-a-qwerty.svg`). Unlike Windows'
  ABC it's **remembered**, like any input source, not reset to Korean.
- **Switching back.** In JIS English, JIS Korean is still the selected input method, so it still
  sees every key. かな selects the last Korean mode used.
- **Switch Key:** one key toggles between JIS English and the last Korean mode, in both directions.
  **F18 by default**, so a key remapped to F18 (Karabiner, or a keyboard's own firmware) works
  without further setup. The input menu offers None and F13 to F19 (Switch Key (JIS English ↔
  Korean)); any other key code is set with `defaults write me.sjun.inputmethod.JISKorean SwitchKey
  -int <code>` (-1 for none).
- **Shift+X** in JIS English goes back to the last Korean mode, for users without a remapping tool
  (Hammerspoon, Karabiner) or a spare key. **On by default**, turned off in the input menu (Shift+X
  in JIS English Returns to Korean); with it on, capital X is x then Shift+Z (§3.6.4), or Caps Lock. Only in JIS
  English: in the Korean modes Shift+X is ㅌ as always (§1). 英数, かな and the Switch Key only act when
  the mode they'd select is added in System Settings; otherwise they go on as usual. The switch key is
  consumed (it never reaches the app); 英数 and かな reach JIS Korean only if macOS passes them on,
  and are left to macOS when it doesn't. Switching commits what's unconfirmed first (§5.1).
- These keys are the shell's, like Ctrl+Space: they select input modes and never reach the engine.

Open: does macOS give 英数 to a Roman mode of a third-party input method, and does it count that mode
as ASCII-capable (so a user can remove Apple's ABC from their input sources)? Both to be checked on
a real build before the mode ships; if 英数 still goes to ABC, the shell catches 英数 itself as it
does かな.

### 8.3 Resolving keys and text

Two functions answer "what does this key type" and "which keys type this", for programs that drive
or check JIS Korean from outside (`engine/src/resolve.rs`, the C ABI `jk_resolve_*`, and on macOS
the public API of §8.2). They run on a copy of a live engine, so they match it exactly: its mode,
and with predictions on, its predictor, weights and what it learned. On macOS that is the engine of
the app JIS Korean was last active in, and every answer names the app, the mode and whether
predictions were on.

A key press is a `KeyboardEvent.code` with `S-` for Shift held (`KeyG`, `S-Digit2`); the macOS API
also takes a virtual key code (`5`, `S-5`). One press is one key, with or without Shift.

- **resolveKeyCodeInput(keyCode)** → the character the key types on its own in the mode (`KeyG` →
  ㅎ, `S-BracketLeft` → 「 in Mode B). None for keys that type no character: F-keys, Space,
  Shift+Z (F10).
- **resolveKeyCodeInput(keyCode, context)** → what the field shows after `context` (unconfirmed
  text typed so far) and then the key: the context with the key's effect, including anything it
  commits (`하` + `KeyS` → 한; `한` + `Space` → `한 `; with predictions on, `hello` + `Space` →
  `hello `). The context is staged by keys whose own Korean reading is the context, as a person types it
  (국의 with its ㅇ); only when there are none, by keys that show it through the top prediction.
  None when no keys type the context.
- **resolveKeyCodeFromOutput(text, minKeypressCount = 1, maxKeypressCount = 1)** → every
  combination of key presses that types `text` from an empty field, with that many presses: `ㅃ`
  → `[Digit1]`, `[S-KeyQ]`. Fewest presses first, then fewest Shifts; at most 1000. **Never an
  error:** a minimum below 1, a maximum below the minimum or an empty text give an empty array, and
  a maximum above 16 is cut to 16.

Keys apply as the shell applies them (§10): with predictions on, the marked text shows the top
prediction unless that is the Korean reading, and Space and Enter commit what is shown. The search
tries the keys whose characters are part of the text (in the layout or its F8 escape), F6, F8,
and F7, F9, F10, Space and Enter only where the text needs them; it skips presses that change
nothing, and follows only key routes whose text heads toward the target (its jamo, its letters
before F9/F10, a hanja's reading before F7). Every combination returned is checked by typing it.

### 8.4 Windows: modes, 한/영 and settings

**One input method, the mode as a setting.** Windows lists one keyboard, **JIS Korean** (ko-KR),
and the mode (§3.6) is chosen in its menu (right-click the taskbar indicator): Mode A, B or C,
listed from the engine's modes, one setting for every app, Mode A by default. In Mode C the menu
also has Dedicated symbol keys (§3.6). This differs from macOS (§8.2) on purpose: on
Windows each input method is one more entry in the Win+Space list and in the language settings,
and none of the tools that read input sources there select them by ID the way Hammerspoon and
Karabiner do on macOS, so a mode per input method would cost more than it gives. The reason the
mode is not per app is the same as on macOS: it follows the keyboard.

**한/영** works exactly like the Microsoft Korean IME's: it switches JIS Korean off (**ABC**: every
key goes to the app untouched, typed by Windows' own layout) and back on (**한글**), committing
what's unconfirmed first (§5.1, IME switch). The state is one for the whole session, shared by every
app, and never stored: it starts on 한글 after a restart or a crash. The taskbar shows 한글 or ABC
where the Microsoft IME shows 가 / A, and a click there switches too. On a US ANSI keyboard,
Windows' Korean layout sends 한/영 from Right Alt.

It's an on/off switch, nothing more: the English typed in ABC never reaches the engine (no
predictions, F9/F10, hanja or learning), and switching mid-word splits the word. The recommended
way to mix English in is to stay in JIS Korean (Mode B on US ANSI keyboards), let the predictions
read English words (§10), and train them on the user's own English words with a training file
(§10.12); `windows/README.md` explains this to users.

**Private fields.** The list (§7, §10) shows readings of what's typed, so it's excluded from
screen capture (recordings, screenshots, screen sharing). In a field marked as a password or PIN
(input scope, disabled keyboard, a password character) every key passes through, as in ABC;
Windows already keeps TIPs out of most of them. Password boxes that aren't marked (common in
banking and certificate programs) can't be told apart from text: there the user switches to ABC,
or turns JIS Korean off for that program in the menu (`Apps\<program.exe>\Off`), which passes
every key through there only.

**Settings** are the macOS ones, stored in `HKCU\Software\JIS Korean` (`Mode` by engine id,
`DedicatedSymbols`, `Predictions`, `EnterCommits`, `LearnFromChoices`, `Learned`, `Weight.<Scenario>`, per app
`Apps\<program.exe>\Predictions`); the default-off apps (§10.11.1) are matched by program file name.
The training file is `%APPDATA%\JIS Korean\training.json`. The Minecraft mod reads `Mode` to type
the same mode in the game.

---

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
(input menu → Learned Weights → Open Training Folder), and forgets its words when it's removed;
Windows does the same with `%APPDATA%\JIS Korean\training.json` (§8.4).
