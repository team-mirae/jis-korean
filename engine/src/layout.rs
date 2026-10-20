//! Key maps (spec §3, §3.6 and §6.1).

use crate::key::Key;

/// What a key types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Out {
    /// A Hangul jamo, fed to the composition automaton.
    Jamo(char),
    /// A character that goes into the text as-is.
    Char(char),
}

impl Out {
    pub fn char(self) -> char {
        match self {
            Out::Jamo(c) | Out::Char(c) => c,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layout {
    /// The mode's own layout.
    JisKorean,
    /// F8: standard 2-set on the stock keys (JIS, or US ANSI in Mode B).
    Escaped,
}

/// A mode: one keyboard's key map (spec §3.6). Every mode shares the letter keys, which always
/// type standard 2-set (the core principle); a mode only says what the other keys type.
///
/// To add a mode (docs/modes.md has the full contract):
/// 1. Add a variant here and to [`Mode::ALL`], with the next [`Mode::id`] that's neither used nor
///    reserved. Id 2 (Mode C) is reserved for ISO keyboards (docs/modes.md, "Reserved").
/// 2. Give it a [`Mode::name`] and its two tables in [`Mode::korean`] and [`Mode::stock`], each in
///    a function of its own (`<keyboard>_korean`, `stock_<keyboard>`) below.
/// 3. Run `cargo test`: `tests/modes.rs` checks every mode against the contract.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Mode A, the original layout, built for JIS keyboards.
    #[default]
    A,
    /// Mode B, for US ANSI keyboards (HHKB US, MacBook US): Mode A's number row and brackets,
    /// and every other key as US ANSI types it.
    B,
}

impl Mode {
    /// Every mode, in id order.
    pub const ALL: &'static [Mode] = &[Mode::A, Mode::B];

    /// The stable id over the C ABI and in saved settings. Never reuse or renumber one.
    pub fn id(self) -> u32 {
        match self {
            Mode::A => 0,
            Mode::B => 1,
        }
    }

    /// The mode with this id. Unknown ids get the default, Mode A.
    pub fn from_id(id: u32) -> Mode {
        Mode::ALL.iter().copied().find(|m| m.id() == id).unwrap_or_default()
    }

    /// The name shells show (a settings menu, the playground).
    pub fn name(self) -> &'static str {
        match self {
            Mode::A => "JIS Korean Original Layout",
            Mode::B => "JIS Korean for US ANSI",
        }
    }

    /// What the non-letter keys type: (unshifted, shifted). `None` for keys the mode leaves out:
    /// they commit and pass through to the app.
    fn korean(self, key: Key) -> Option<(Out, Out)> {
        match self {
            Mode::A => jis_korean(key),
            Mode::B => ansi_korean(key),
        }
    }

    /// The stock keyboard's characters on the non-letter keys: the F8 escape (standard 2-set on
    /// the stock keys) and the English reading of every key (predictions, F9/F10's symbols).
    /// It must cover every key [`Mode::korean`] maps.
    fn stock(self, key: Key) -> Option<(char, char)> {
        match self {
            Mode::A => stock_jis(key),
            Mode::B => stock_us(key),
        }
    }
}

/// What `key` types in `layout` in Mode A. `None` for keys outside the map (F-keys, Space, …).
pub fn output(layout: Layout, key: Key, shift: bool) -> Option<Out> {
    output_in(Mode::A, layout, key, shift)
}

/// What `key` types in `layout` in `mode`.
pub fn output_in(mode: Mode, layout: Layout, key: Key, shift: bool) -> Option<Out> {
    if let Some((base, shifted)) = two_set(key) {
        return Some(Out::Jamo(if shift { shifted } else { base }));
    }
    let (base, shifted) = match layout {
        Layout::JisKorean => mode.korean(key)?,
        Layout::Escaped => {
            let (base, shifted) = mode.stock(key)?;
            (Out::Char(base), Out::Char(shifted))
        }
    };
    Some(if shift { shifted } else { base })
}

/// Standard 2-set (KS X 5002) on the 26 letter keys: (unshifted, shifted).
fn two_set(key: Key) -> Option<(char, char)> {
    use Key::*;
    let pair = match key {
        KeyQ => ('ㅂ', 'ㅃ'),
        KeyW => ('ㅈ', 'ㅉ'),
        KeyE => ('ㄷ', 'ㄸ'),
        KeyR => ('ㄱ', 'ㄲ'),
        KeyT => ('ㅅ', 'ㅆ'),
        KeyO => ('ㅐ', 'ㅒ'),
        KeyP => ('ㅔ', 'ㅖ'),
        _ => {
            let j = match key {
                KeyY => 'ㅛ',
                KeyU => 'ㅕ',
                KeyI => 'ㅑ',
                KeyA => 'ㅁ',
                KeyS => 'ㄴ',
                KeyD => 'ㅇ',
                KeyF => 'ㄹ',
                KeyG => 'ㅎ',
                KeyH => 'ㅗ',
                KeyJ => 'ㅓ',
                KeyK => 'ㅏ',
                KeyL => 'ㅣ',
                KeyZ => 'ㅋ',
                KeyX => 'ㅌ',
                KeyC => 'ㅊ',
                KeyV => 'ㅍ',
                KeyB => 'ㅠ',
                KeyN => 'ㅜ',
                KeyM => 'ㅡ',
                _ => return None,
            };
            (j, j)
        }
    };
    Some(pair)
}

/// Mode A's non-letter keys (spec §3.1–3.4): (unshifted, shifted).
fn jis_korean(key: Key) -> Option<(Out, Out)> {
    use Key::*;
    use Out::{Char as C, Jamo as J};
    let pair = match key {
        Digit1 => (J('ㅃ'), C('1')),
        Digit2 => (J('ㅉ'), C('2')),
        Digit3 => (J('ㄸ'), C('3')),
        Digit4 => (J('ㄲ'), C('4')),
        Digit5 => (J('ㅆ'), C('5')),
        Digit6 => (J('ㅟ'), C('6')),
        Digit7 => (J('ㅝ'), C('7')),
        Digit8 => (J('ㅚ'), C('8')),
        Digit9 => (J('ㅢ'), C('9')),
        Digit0 => (J('ㅘ'), C('0')),
        Minus => (C('!'), C('?')),
        Equal => (C('('), C(')')),
        IntlYen => (C('\u{2014}'), C('^')),
        BracketLeft => (C('\''), C('"')),
        BracketRight => (C('['), C('\u{300C}')),
        Semicolon => (C('~'), C('-')),
        Quote => (C(':'), C(';')),
        Backslash => (C(']'), C('\u{300D}')),
        Comma => (C(','), C(',')),
        Period => (C('.'), C('.')),
        Slash => (C('\u{00B7}'), C('\u{00B7}')),
        IntlRo => (C('\\'), C('_')),
        _ => return None,
    };
    Some(pair)
}

/// Mode B's non-letter keys (spec §3.6): Mode A's number row, the brackets with 「 」 on Shift,
/// and stock US ANSI everywhere else.
fn ansi_korean(key: Key) -> Option<(Out, Out)> {
    use Key::*;
    use Out::Char as C;
    match key {
        Digit1 | Digit2 | Digit3 | Digit4 | Digit5 | Digit6 | Digit7 | Digit8 | Digit9 | Digit0 => jis_korean(key),
        BracketLeft => Some((C('['), C('\u{300C}'))),
        BracketRight => Some((C(']'), C('\u{300D}'))),
        _ => stock_us(key).map(|(base, shifted)| (C(base), C(shifted))),
    }
}

/// The stock US ANSI characters on the non-letter keys: Mode B's F8 escape and English readings.
fn stock_us(key: Key) -> Option<(char, char)> {
    use Key::*;
    let pair = match key {
        Backquote => ('`', '~'),
        Digit1 => ('1', '!'),
        Digit2 => ('2', '@'),
        Digit3 => ('3', '#'),
        Digit4 => ('4', '$'),
        Digit5 => ('5', '%'),
        Digit6 => ('6', '^'),
        Digit7 => ('7', '&'),
        Digit8 => ('8', '*'),
        Digit9 => ('9', '('),
        Digit0 => ('0', ')'),
        Minus => ('-', '_'),
        Equal => ('=', '+'),
        BracketLeft => ('[', '{'),
        BracketRight => (']', '}'),
        Backslash => ('\\', '|'),
        Semicolon => (';', ':'),
        Quote => ('\'', '"'),
        Comma => (',', '<'),
        Period => ('.', '>'),
        Slash => ('/', '?'),
        _ => return None,
    };
    Some(pair)
}

/// The stock JIS characters on the non-letter keys: Mode A's F8 escape and English readings.
fn stock_jis(key: Key) -> Option<(char, char)> {
    use Key::*;
    let pair = match key {
        Digit1 => ('1', '!'),
        Digit2 => ('2', '"'),
        Digit3 => ('3', '#'),
        Digit4 => ('4', '$'),
        Digit5 => ('5', '%'),
        Digit6 => ('6', '&'),
        Digit7 => ('7', '\''),
        Digit8 => ('8', '('),
        Digit9 => ('9', ')'),
        Digit0 => ('0', '0'),
        Minus => ('-', '='),
        Equal => ('^', '~'),
        IntlYen => ('¥', '|'),
        BracketLeft => ('@', '`'),
        BracketRight => ('[', '{'),
        Semicolon => (';', '+'),
        Quote => (':', '*'),
        Backslash => (']', '}'),
        Comma => (',', '<'),
        Period => ('.', '>'),
        Slash => ('/', '?'),
        IntlRo => ('\\', '_'),
        _ => return None,
    };
    Some(pair)
}
