//! The unconfirmed segment, commit rules and F6–F10 (spec §5 and §6).

use crate::hangul::Composer;
use crate::key::{Key, Mods};
use crate::layout::{self, Layout, Out};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    /// A jamo, with the Latin letter or digit of the key that typed it (for F9/F10).
    Jamo { jamo: char, raw: Option<char> },
    Text(char),
    /// F6: the syllable before it is finished.
    Break,
    /// F6 on an implied ㅇ: like `Break`, and the vowel after it stands bare.
    Bare,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Width {
    Full,
    Half,
}

/// An F9/F10 conversion in progress (spec §6.2).
struct Conversion {
    start: usize,
    original: Vec<Token>,
    width: Width,
    /// 0 = as typed, 1 = UPPER, 2 = Capitalized.
    case: u8,
}

/// The result of one engine call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    /// Text to insert into the document, before the preedit.
    pub commit: String,
    /// The whole unconfirmed segment, to show as marked text. Empty when nothing is unconfirmed.
    pub preedit: String,
    /// `false`: after applying `commit` and `preedit`, pass the key on to the app.
    pub consumed: bool,
}

pub struct Engine {
    segment: Vec<Token>,
    conversion: Option<Conversion>,
    layout: Layout,
    enter_commits: bool,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Engine { segment: Vec::new(), conversion: None, layout: Layout::JisKorean, enter_commits: false }
    }

    /// Setting: plain Enter commits without passing Enter on, as Shift+Enter does (spec §5.1).
    pub fn set_enter_commits(&mut self, on: bool) {
        self.enter_commits = on;
    }

    /// Whether F8 has escaped to standard 2-set.
    pub fn is_escaped(&self) -> bool {
        self.layout == Layout::Escaped
    }

    /// The IME was switched back to: leave any F8 escape (spec §6.1).
    pub fn activate(&mut self) {
        self.layout = Layout::JisKorean;
    }

    /// Commits everything unconfirmed. Call on focus loss, app switch and IME switch.
    pub fn flush(&mut self) -> Output {
        Output { commit: self.take_segment(), preedit: String::new(), consumed: true }
    }

    pub fn preedit(&self) -> String {
        compose(&self.segment)
    }

    pub fn key(&mut self, key: Key, mods: Mods) -> Output {
        if key == Key::Modifier {
            return self.unchanged(false);
        }
        if mods.is_shortcut() {
            return self.commit(String::new(), false);
        }
        if !matches!(key, Key::F6 | Key::F9 | Key::F10) {
            self.conversion = None;
        }
        match key {
            Key::F6 => {
                self.hangul();
                self.unchanged(true)
            }
            // 한자: planned (spec §7). Reserved so apps never see it.
            Key::F7 => self.unchanged(true),
            Key::F8 => {
                self.layout = if mods.shift { Layout::JisKorean } else { Layout::Escaped };
                self.unchanged(true)
            }
            Key::F9 => {
                self.convert(Width::Full);
                self.unchanged(true)
            }
            Key::F10 => {
                self.convert(Width::Half);
                self.unchanged(true)
            }
            Key::Space => self.commit(" ".into(), true),
            Key::Enter if self.segment.is_empty() => self.unchanged(false),
            Key::Enter => {
                let commit_only = mods.shift || self.enter_commits;
                self.commit(String::new(), commit_only)
            }
            Key::Backspace if self.segment.is_empty() => self.unchanged(false),
            Key::Backspace => {
                while matches!(self.segment.last(), Some(Token::Break | Token::Bare)) {
                    self.segment.pop();
                }
                self.segment.pop();
                // A bare marker left at the end would make the next vowel bare too.
                while self.segment.last() == Some(&Token::Bare) {
                    self.segment.pop();
                }
                self.unchanged(true)
            }
            _ => match layout::output(self.layout, key, mods.shift) {
                Some(Out::Jamo(jamo)) => {
                    let raw = key.letter().map(|c| if mods.shift { c.to_ascii_uppercase() } else { c });
                    self.segment.push(Token::Jamo { jamo, raw: raw.or(key.digit()) });
                    self.unchanged(true)
                }
                Some(Out::Char(c)) => {
                    self.segment.push(Token::Text(c));
                    self.unchanged(true)
                }
                None => self.commit(String::new(), false),
            },
        }
    }

    /// F6: undo an F9/F10 conversion, or drop an implied ㅇ, or else end the syllable being
    /// composed.
    fn hangul(&mut self) {
        if let Some(conversion) = self.conversion.take() {
            self.segment.truncate(conversion.start);
            self.segment.extend(conversion.original);
        } else if let Some(at) = composer_for(&self.segment).implied_at() {
            // 아 → ㅏ, 가어 → 가ㅓ: the vowel stands bare, and its syllable is finished.
            self.segment.insert(at, Token::Bare);
            self.segment.push(Token::Break);
        } else if matches!(self.segment.last(), Some(Token::Jamo { .. })) {
            self.segment.push(Token::Break);
        }
    }

    /// F9/F10: the current word as its keys' alphanumerics, cycling case on repeat presses.
    fn convert(&mut self, width: Width) {
        match &mut self.conversion {
            Some(conversion) => {
                if conversion.width == width {
                    conversion.case = (conversion.case + 1) % 3;
                }
                conversion.width = width;
            }
            None => {
                let start = self.segment.len()
                    - self
                        .segment
                        .iter()
                        .rev()
                        .take_while(|t| matches!(t, Token::Jamo { raw: Some(_), .. }))
                        .count();
                if start == self.segment.len() {
                    return;
                }
                let original = self.segment[start..].to_vec();
                self.conversion = Some(Conversion { start, original, width, case: 0 });
            }
        }
        let Some(conversion) = &self.conversion else { return };
        let raw: String = conversion
            .original
            .iter()
            .filter_map(|t| match t {
                Token::Jamo { raw, .. } => *raw,
                _ => None,
            })
            .collect();
        let cased = match conversion.case {
            0 => raw,
            1 => raw.to_uppercase(),
            _ => {
                let mut chars = raw.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().chain(chars.as_str().to_lowercase().chars()).collect())
                    .unwrap_or_default()
            }
        };
        let start = conversion.start;
        let width = conversion.width;
        self.segment.truncate(start);
        self.segment.extend(cased.chars().map(|c| {
            Token::Text(if width == Width::Full { full_width(c) } else { c })
        }));
    }

    fn take_segment(&mut self) -> String {
        self.conversion = None;
        compose(&std::mem::take(&mut self.segment))
    }

    /// Commits the segment followed by `suffix`.
    fn commit(&mut self, suffix: String, consumed: bool) -> Output {
        let mut commit = self.take_segment();
        commit.push_str(&suffix);
        Output { commit, preedit: String::new(), consumed }
    }

    fn unchanged(&self, consumed: bool) -> Output {
        Output { commit: String::new(), preedit: self.preedit(), consumed }
    }
}

fn composer_for(segment: &[Token]) -> Composer {
    let mut composer = Composer::default();
    for token in segment {
        match *token {
            Token::Jamo { jamo, .. } => composer.jamo(jamo),
            Token::Text(c) => composer.text(c),
            Token::Break => composer.syllable_break(),
            Token::Bare => composer.bare(),
        }
    }
    composer
}

fn compose(segment: &[Token]) -> String {
    composer_for(segment).finish()
}

/// The full-width form of a printable ASCII character (spec §5.3).
pub fn full_width(c: char) -> char {
    match c {
        '!'..='~' => char::from_u32(c as u32 + 0xFEE0).unwrap_or(c),
        ' ' => '\u{3000}',
        _ => c,
    }
}
