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
    /// F8: standard 2-set on the stock keys (JIS, US ANSI in Mode B, UK ISO in Mode C).
    Escaped,
}

/// A mode: one keyboard's key map (spec §3.6). Every mode shares the letter keys, which always
/// type standard 2-set (the core principle); a mode only says what the other keys type.
///
/// Each variant carries its id as its discriminant (spec §3.6.1). Ids have gaps and a build has
/// only some modes, so nothing may assume they run from 0: look them up with [`Mode::from_id`] and
/// list them with [`Mode::ALL`].
///
/// To add a mode (docs/modes.md has the full contract):
/// 1. Add a variant here with its id (a free one in its range, spec §3.6.1), and add it to
///    [`Mode::ALL`] and [`Mode::from_id`].
/// 2. Give it a [`Mode::code`], a [`Mode::name`] and its two tables in [`Mode::korean`] and [`Mode::stock`], each in
///    a function of its own (`<keyboard>_korean`, `stock_<keyboard>`) below.
/// 3. Run `cargo test`: `tests/modes.rs` checks every mode against the contract.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Mode {
    /// Mode A, the original layout, built for JIS keyboards.
    #[default]
    A = 0,
    /// Mode B, for US ANSI keyboards (HHKB US, MacBook US): Mode A's number row and brackets,
    /// and every other key as US ANSI types it.
    B = 1,
    /// Mode C, for UK ISO keyboards: Mode B's pattern on UK symbols, plus the ISO extra key
    /// between left Shift and Z. Its symbol keys are optional ([`Mode::optional_keys`]).
    C = 2,
    /// `A_QWERTY`, JIS English (spec §3.6.2): not 2-set. Standard JIS QWERTY with JIS symbols on
    /// any keyboard; every key goes to the app, and ⌥ + a letter offers its accented variants
    /// (spec §3.6.3). Built with the `english` feature.
    #[cfg(feature = "english")]
    AQwerty = 3,
    /// `B_QWERTY`: JIS English's US ANSI sibling, US symbols (spec §3.6.2). `english` feature.
    #[cfg(feature = "english")]
    BQwerty = 9,
    /// `CJ_QWERTY`: JIS English's UK ISO sibling, UK symbols; designed for ISO, and on a JIS board
    /// ¥ and ろ type the ISO keys JIS lacks (spec §3.6.2). `english` feature.
    #[cfg(feature = "english")]
    CjQwerty = 10,
    /// `A_DVORAK`, JIS Dvorak (spec §3.6.4): JIS English with Dvorak's letters. Every symbol keeps its
    /// JIS key pair; the four a letter displaces go to Dvorak's punctuation slots. `english` feature.
    #[cfg(feature = "english")]
    ADvorak = 7,
    /// `A_COLEMAK`, JIS Colemak (spec §3.6.4): JIS English with Colemak's letters; `; +` moves to P.
    /// `english` feature.
    #[cfg(feature = "english")]
    AColemak = 8,
}

impl Mode {
    /// Every mode this build has, in the order menus list them. Not indexed by id.
    #[cfg(not(feature = "english"))]
    pub const ALL: &'static [Mode] = &[Mode::A, Mode::B, Mode::C];
    /// Every mode this build has, in the order menus list them. Not indexed by id.
    #[cfg(feature = "english")]
    pub const ALL: &'static [Mode] = &[
        Mode::A,
        Mode::B,
        Mode::C,
        Mode::AQwerty,
        Mode::BQwerty,
        Mode::CjQwerty,
        Mode::ADvorak,
        Mode::AColemak,
    ];

    /// The stable id over the C ABI and in saved settings (spec §3.6.1). Never reuse or renumber
    /// one.
    pub fn id(self) -> u32 {
        self as u32
    }

    /// The mode with this id, or `None` for an id this build doesn't have (reserved, a mode left
    /// out of the build, or no mode at all). Callers decide what to fall back to.
    pub fn from_id(id: u32) -> Option<Mode> {
        match id {
            0 => Some(Mode::A),
            1 => Some(Mode::B),
            2 => Some(Mode::C),
            #[cfg(feature = "english")]
            3 => Some(Mode::AQwerty),
            #[cfg(feature = "english")]
            7 => Some(Mode::ADvorak),
            #[cfg(feature = "english")]
            8 => Some(Mode::AColemak),
            #[cfg(feature = "english")]
            9 => Some(Mode::BQwerty),
            #[cfg(feature = "english")]
            10 => Some(Mode::CjQwerty),
            _ => None,
        }
    }

    /// The mode name (spec §3.6.1): what tools, settings and the public API use. Upper-case snake
    /// case; the first three keep their one-letter names.
    pub fn code(self) -> &'static str {
        match self {
            Mode::A => "A",
            Mode::B => "B",
            Mode::C => "C",
            #[cfg(feature = "english")]
            Mode::AQwerty => "A_QWERTY",
            #[cfg(feature = "english")]
            Mode::BQwerty => "B_QWERTY",
            #[cfg(feature = "english")]
            Mode::CjQwerty => "CJ_QWERTY",
            #[cfg(feature = "english")]
            Mode::ADvorak => "A_DVORAK",
            #[cfg(feature = "english")]
            Mode::AColemak => "A_COLEMAK",
        }
    }

    /// Whether the mode types Korean (2-set). Everything about Korean, the core principle included,
    /// is about these modes; JIS English isn't one (spec §3.6.2).
    pub fn is_korean(self) -> bool {
        match self {
            Mode::A | Mode::B | Mode::C => true,
            #[cfg(feature = "english")]
            Mode::AQwerty | Mode::BQwerty | Mode::CjQwerty | Mode::ADvorak | Mode::AColemak => false,
        }
    }

    /// The display name shells show (a settings menu, the playground).
    pub fn name(self) -> &'static str {
        match self {
            Mode::A => "JIS Korean Original Layout",
            Mode::B => "JIS Korean for US ANSI",
            Mode::C => "JIS Korean for UK ISO",
            #[cfg(feature = "english")]
            Mode::AQwerty => "JIS English",
            #[cfg(feature = "english")]
            Mode::BQwerty => "US English",
            #[cfg(feature = "english")]
            Mode::CjQwerty => "UK English",
            #[cfg(feature = "english")]
            Mode::ADvorak => "JIS Dvorak",
            #[cfg(feature = "english")]
            Mode::AColemak => "JIS Colemak",
        }
    }

    /// The symbol keys the Dedicated symbol keys setting turns off (spec §3.6): with it off they
    /// type the stock keyboard's characters ([`Mode::stock`]), as F8 would. Empty for modes without
    /// the setting.
    pub fn optional_keys(self) -> &'static [Key] {
        match self {
            Mode::A | Mode::B => &[],
            #[cfg(feature = "english")]
            Mode::AQwerty | Mode::BQwerty | Mode::CjQwerty | Mode::ADvorak | Mode::AColemak => &[],
            Mode::C => &[Key::BracketLeft, Key::BracketRight, Key::IntlBackslash],
        }
    }

    /// What the non-letter keys type: (unshifted, shifted). `None` for keys the mode leaves out:
    /// they commit and pass through to the app. `symbols` is the Dedicated symbol keys setting.
    fn korean(self, key: Key, symbols: bool) -> Option<(Out, Out)> {
        if !symbols && self.optional_keys().contains(&key) {
            return self.stock(key).map(|(base, shifted)| (Out::Char(base), Out::Char(shifted)));
        }
        match self {
            Mode::A => jis_korean(key),
            Mode::B => ansi_korean(key),
            Mode::C => iso_korean(key),
            // Not Korean: the JIS symbols as they are.
            #[cfg(feature = "english")]
            Mode::AQwerty | Mode::BQwerty | Mode::CjQwerty | Mode::ADvorak | Mode::AColemak => {
                self.stock(key).map(|(base, shifted)| (Out::Char(base), Out::Char(shifted)))
            }
        }
    }

    /// The stock keyboard's characters on the non-letter keys: the F8 escape (standard 2-set on
    /// the stock keys) and the English reading of every key (predictions, F9/F10's symbols).
    /// It must cover every key [`Mode::korean`] maps.
    fn stock(self, key: Key) -> Option<(char, char)> {
        match self {
            Mode::A => stock_jis(key),
            Mode::B => stock_us(key),
            Mode::C => stock_uk(key),
            #[cfg(feature = "english")]
            Mode::AQwerty => stock_jis(key),
            #[cfg(feature = "english")]
            Mode::BQwerty => stock_us(key),
            #[cfg(feature = "english")]
            Mode::CjQwerty => stock_uk_on_jis(key),
            #[cfg(feature = "english")]
            Mode::ADvorak | Mode::AColemak => stock_jis(self.qwerty_key(key)),
        }
    }

    /// The JIS English key whose characters `key` types in this mode: itself, except in JIS Dvorak
    /// and JIS Colemak, which move keys around (spec §3.6.4). ⌘ and ⌥ still use `key` as it is.
    pub fn qwerty_key(self, key: Key) -> Key {
        match self {
            #[cfg(feature = "english")]
            Mode::ADvorak => dvorak(key),
            #[cfg(feature = "english")]
            Mode::AColemak => colemak(key),
            _ => key,
        }
    }

    /// The Latin letter `key` types in an English mode (lower case), or `None` for a key that types
    /// something else there. Korean modes have none.
    pub fn letter(self, key: Key) -> Option<char> {
        if self.is_korean() { None } else { self.qwerty_key(key).letter() }
    }
}

/// What `key` types in `layout` in Mode A. `None` for keys outside the map (F-keys, Space, …).
pub fn output(layout: Layout, key: Key, shift: bool) -> Option<Out> {
    output_in(Mode::A, layout, key, shift)
}

/// What `key` types in `layout` in `mode`, with the mode's symbol keys on (the default).
pub fn output_in(mode: Mode, layout: Layout, key: Key, shift: bool) -> Option<Out> {
    output_with(mode, true, layout, key, shift)
}

/// What `key` types in `layout` in `mode`, with the Dedicated symbol keys setting `symbols`
/// (spec §3.6; only modes with [`Mode::optional_keys`] read it).
pub fn output_with(mode: Mode, symbols: bool, layout: Layout, key: Key, shift: bool) -> Option<Out> {
    if !mode.is_korean() {
        // JIS English: the Latin letters, upper-case with Shift.
        if let Some(letter) = mode.letter(key) {
            return Some(Out::Char(if shift { letter.to_ascii_uppercase() } else { letter }));
        }
    } else if let Some((base, shifted)) = two_set(key) {
        return Some(Out::Jamo(if shift { shifted } else { base }));
    }
    let (base, shifted) = match layout {
        Layout::JisKorean => mode.korean(key, symbols)?,
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

/// Mode C's non-letter keys (spec §3.6): Mode B's pattern on UK ISO, and the extra key between
/// left Shift and Z for the two Korean punctuation marks UK ISO has no key for.
fn iso_korean(key: Key) -> Option<(Out, Out)> {
    use Key::*;
    use Out::Char as C;
    match key {
        Digit1 | Digit2 | Digit3 | Digit4 | Digit5 | Digit6 | Digit7 | Digit8 | Digit9 | Digit0 => jis_korean(key),
        BracketLeft => Some((C('['), C('\u{300C}'))),
        BracketRight => Some((C(']'), C('\u{300D}'))),
        IntlBackslash => Some((C('\u{00B7}'), C('\u{2014}'))),
        _ => stock_uk(key).map(|(base, shifted)| (C(base), C(shifted))),
    }
}

/// JIS Dvorak's keys (spec §3.6.4): Dvorak's letters, each key typing the JIS English key named.
/// The JIS symbol keys a letter displaces go where Dvorak has punctuation: `, <` to W, `. >` to E,
/// `; +` to Z, and `/ ?` to Q (Dvorak's `' "`, which JIS has on Shift+7 and Shift+2).
#[cfg(feature = "english")]
fn dvorak(key: Key) -> Key {
    use Key::*;
    match key {
        KeyQ => Slash,
        KeyW => Comma,
        KeyE => Period,
        KeyR => KeyP,
        KeyT => KeyY,
        KeyY => KeyF,
        KeyU => KeyG,
        KeyI => KeyC,
        KeyO => KeyR,
        KeyP => KeyL,
        KeyS => KeyO,
        KeyD => KeyE,
        KeyF => KeyU,
        KeyG => KeyI,
        KeyH => KeyD,
        KeyJ => KeyH,
        KeyK => KeyT,
        KeyL => KeyN,
        Semicolon => KeyS,
        KeyZ => Semicolon,
        KeyX => KeyQ,
        KeyC => KeyJ,
        KeyV => KeyK,
        KeyB => KeyX,
        KeyN => KeyB,
        Comma => KeyW,
        Period => KeyV,
        Slash => KeyZ,
        _ => key,
    }
}

/// JIS Colemak's keys (spec §3.6.4): Colemak's letters, each key typing the JIS English key named.
/// `; +`, displaced by o, goes to P, Colemak's semicolon.
#[cfg(feature = "english")]
fn colemak(key: Key) -> Key {
    use Key::*;
    match key {
        KeyE => KeyF,
        KeyR => KeyP,
        KeyT => KeyG,
        KeyY => KeyJ,
        KeyU => KeyL,
        KeyI => KeyU,
        KeyO => KeyY,
        KeyP => Semicolon,
        KeyS => KeyR,
        KeyD => KeyS,
        KeyF => KeyT,
        KeyG => KeyD,
        KeyJ => KeyN,
        KeyK => KeyE,
        KeyL => KeyI,
        Semicolon => KeyO,
        KeyN => KeyK,
        _ => key,
    }
}

/// The stock UK ISO characters (PC legends) on the non-letter keys: Mode C's F8 escape and English
/// UK ISO for a `CJ_` mode (spec §3.6.1): designed for ISO, also supports JIS. A JIS board has ¥
/// and ろ, which ISO lacks, and lacks the ISO extra key and the key left of 1 (a JIS board's
/// 半角/全角): ろ types the extra key's characters and ¥ those of the key left of 1, so every UK
/// character is reachable on JIS too.
#[cfg(feature = "english")]
fn stock_uk_on_jis(key: Key) -> Option<(char, char)> {
    match key {
        Key::IntlRo => stock_uk(Key::IntlBackslash),
        Key::IntlYen => stock_uk(Key::Backquote),
        _ => stock_uk(key),
    }
}

/// readings. `Backslash` is the ISO key left of Enter (# ~).
fn stock_uk(key: Key) -> Option<(char, char)> {
    use Key::*;
    let pair = match key {
        Backquote => ('`', '¬'),
        Digit1 => ('1', '!'),
        Digit2 => ('2', '"'),
        Digit3 => ('3', '£'),
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
        Semicolon => (';', ':'),
        Quote => ('\'', '@'),
        Backslash => ('#', '~'),
        IntlBackslash => ('\\', '|'),
        Comma => (',', '<'),
        Period => ('.', '>'),
        Slash => ('/', '?'),
        _ => return None,
    };
    Some(pair)
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
