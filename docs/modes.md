# Modes: the contract for adding one

A **mode** is one keyboard's key map (spec §3.6). JIS Korean ships two:

| Id | Mode | Name | Keyboard |
|---|---|---|---|
| 0 | A | JIS Korean Original Layout | JIS |
| 1 | B | JIS Korean for US ANSI | US ANSI (HHKB US, MacBook US) |

You can fork JIS Korean and add a mode for your own typing or your own keyboard (a UK ISO board,
a split board with its own symbol keys, a layout with more symbols on Shift). This page is what a
mode must define and what it must keep.

## What every mode keeps

These are shared by every mode, and a mode can't change them:

- **Standard 2-set on the 26 letter keys**, Shift layer included (the core principle, spec §1).
- **Shift+Z is F10**, and **F6–F10** work as spec §6 says.
- **The automaton and everything after it** (spec §4 onward): the implied ㅇ, the double press,
  backspace by jamo, commit rules, hanja and predictions.
- **Space, Enter, Backspace, Tab, Esc, the arrows and the modifiers** keep their spec §3.5 behavior.

## What a mode defines

All of it lives in `engine/src/layout.rs`, on the `Mode` enum:

| Part | Where | Rule |
|---|---|---|
| Variant | `enum Mode`, `Mode::ALL` | Name it after its letter (`C`, `D`, …). Add it to `ALL` in id order. |
| Id | `Mode::id` | The next unused number. Ids are saved in settings and passed over the C ABI (`jk_set_mode`), so never renumber or reuse one. |
| Name | `Mode::name` | What a settings menu shows, e.g. "JIS Korean for UK ISO". Unique. |
| Key table | `Mode::korean` → `<keyboard>_korean(key)` | What each non-letter key types: `(unshifted, shifted)`, each `Out::Jamo` (goes into the automaton) or `Out::Char` (goes in as-is). Return `None` for keys the mode leaves out: they commit and pass through to the app. |
| Stock table | `Mode::stock` → `stock_<keyboard>(key)` | The characters the keyboard's own layout prints on those keys. It drives the F8 escape, the English reading and the standard 2-set prediction, so a mode never suggests a symbol from another keyboard. It must cover every key the key table maps to a `Char`. |

Keep the key table's comments in the style of `jis_korean` and `ansi_korean`: one line on what
the mode changes and why.

If the keyboard has a key the engine doesn't know yet (Mode B added `Backquote`), add it to the
`keys!` list in `engine/src/key.rs`, just before `Modifier`, and to `include/jis_korean.h` with the
matching id. `tests/header.rs` fails until the two agree.

## What the tests check

`cargo test` in `/engine` runs `tests/modes.rs` over every mode in `Mode::ALL`:

- ids are unique, round-trip through `Mode::from_id`, and unknown ids fall back to Mode A;
- names are unique and non-empty;
- the letter keys type standard 2-set, in the mode's layout and in its F8 escape;
- the stock table has a character for every key the key table maps to a character;
- the control keys (Space, Enter, Backspace, F-keys, Esc) aren't in either table.

Add your mode's own examples next to Mode B's in `tests/modes.rs` (what a few keys type, what they
read as in English, and one F8 example), and one prediction example in `tests/predict.rs` if the
stock symbols differ from an existing mode's.

## Docs

- `docs/spec.md` §3.6: a row in the modes table and the mode's key table.
- `docs/layout/jis-korean-layouts.html`: a section with the mode's board, built from its tables in
  the page's script (`A`, `B_HHKB`, `B_MAC` show the pattern: Mode A's keys are reused, the
  untouched keys use `stock(…)`). Re-render the PNGs (light and dark, 3×) as the README shows them.

## Shells

The engine API is the same for every mode: a shell calls `jk_set_mode(engine, id)` (Rust:
`Engine::set_mode`) and draws legends with `jk_legend_in(key, shift, escaped, mode)`.
`jk_mode_count` and `jk_mode_name_ptr`/`_len` list the modes and their names.

### macOS: one input source per mode

On macOS each mode is its own input mode of the one input method (spec §8.2), the way Japanese has
Kana and Romaji. That's deliberate: the mode follows the keyboard, and apps don't change which
keyboard the user is typing on, so a per-app setting would mean nothing. As input sources, the
modes can be seen and selected by tools JIS Korean doesn't integrate with (Hammerspoon,
Karabiner), and JIS Korean's public API reports the mode to any program.

To add a mode on macOS:

1. **Input mode:** a `tsInputModeListKey` entry in `mac/Info.plist`, listed in
   `tsVisibleInputModeOrderedArrayKey`. Its ID is `me.sjun.inputmethod.JISKorean.2Set<Keyboard>`
   (`.2Set` is Mode A, `.2SetANSI` Mode B), and it never changes once released: users' tools
   select it by that ID.
2. **Its name** in `mac/Resources/*.lproj/InfoPlist.strings`, keyed by the ID.
3. **`InputModes.all`** in `mac/Sources/InputModes.swift`: the engine id, letter, input source ID
   and Keyboard Viewer layout name.
4. **Keyboard Viewer layout:** a `make-keylayout` rule in `mac/Makefile` (name, `JIS` or `ANSI`,
   and a unique negative id), its file in `KEYLAYOUTS`, and a `KLInfo_<name>` entry in
   `mac/Resources/JISKoreanKeyboardLayouts-Info.plist`.
5. **Keys:** any key code the keyboard adds, in `mac/Sources/KeyMap.swift` (Mode B added
   `kVK_ANSI_Grave` → `Backquote`).

`make install` doesn't enable a new mode: users add it in System Settings.

### Other shells

Windows, Minecraft and the web playground run Mode A for now. Each needs a way to pick the mode
and, for keys only another keyboard has, their canonical codes.
