//! Key maps (spec §3 and §6.1).

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
    /// The JIS Korean layout.
    JisKorean,
    /// F8: standard 2-set on the stock JIS keys.
    Escaped,
}

/// What `key` types in `layout`. `None` for keys outside the map (F-keys, Space, …).
pub fn output(layout: Layout, key: Key, shift: bool) -> Option<Out> {
    if let Some((base, shifted)) = two_set(key) {
        return Some(Out::Jamo(if shift { shifted } else { base }));
    }
    let (base, shifted) = match layout {
        Layout::JisKorean => jis_korean(key)?,
        Layout::Escaped => {
            let (base, shifted) = stock_jis(key)?;
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

/// The JIS Korean non-letter keys (spec §3.1–3.4): (unshifted, shifted).
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
        Equal => (C('^'), C('^')),
        IntlYen => (C('\u{2014}'), C('\u{2014}')),
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

/// The stock JIS characters on the non-letter keys, for the F8 escape.
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
