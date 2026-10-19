//! The unconfirmed segment, commit rules and F6–F10 (spec §5, §6 and §7).

use std::sync::Arc;

use crate::hangul::Composer;
use crate::hanja::{Candidate, Dictionary};
use crate::key::{Key, Mods};
use crate::layout::{self, Layout, Out};

/// Candidates per page of the hanja list; Digit1–Digit9 pick on the current page.
pub const PAGE_SIZE: usize = 9;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    /// A jamo, with the Latin letter or digit of the key that typed it (for F9/F10).
    Jamo { jamo: char, raw: Option<char> },
    /// Text from a conversion or a hanja pick.
    Text(char),
    /// A character typed by a non-letter key, with what that key types on stock JIS (for the
    /// English reading).
    Typed(char, char),
    /// F6 on an implied ㅇ: the vowel after it stands bare. Only exists while F6 commits.
    Bare,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Width {
    Full,
    Half,
}

/// An F9/F10 conversion in progress (spec §6.2).
#[derive(Clone)]
struct Conversion {
    start: usize,
    original: Vec<Token>,
    width: Width,
    /// 0 = as typed, 1 = UPPER, 2 = Capitalized.
    case: u8,
}

/// An open hanja candidate list (spec §7). The segment is left as typed until a pick.
#[derive(Clone)]
struct HanjaList {
    /// Tokens from here on are replaced by the pick.
    start: usize,
    candidates: Vec<Candidate>,
    selected: usize,
}

/// The candidate list to show, while one is open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Candidates {
    pub items: Vec<Candidate>,
    pub selected: usize,
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
    /// The hanja candidate list, while one is open.
    pub candidates: Option<Candidates>,
    /// The same keystrokes read as English on stock JIS (see [`Engine::english`]).
    pub english: String,
}

#[derive(Clone)]
pub struct Engine {
    segment: Vec<Token>,
    conversion: Option<Conversion>,
    hanja: Option<HanjaList>,
    layout: Layout,
    enter_commits: bool,
    dictionary: Option<Arc<Dictionary>>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Engine {
            segment: Vec::new(),
            conversion: None,
            hanja: None,
            layout: Layout::JisKorean,
            enter_commits: false,
            dictionary: None,
        }
    }

    /// Setting: plain Enter commits without passing Enter on, as Shift+Enter does (spec §5.1).
    pub fn set_enter_commits(&mut self, on: bool) {
        self.enter_commits = on;
    }

    /// The hanja dictionary for F7. Without one, F7 does nothing. Engines can share one.
    pub fn set_dictionary(&mut self, dictionary: Arc<Dictionary>) {
        self.dictionary = Some(dictionary);
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
    /// An open hanja list commits its selected candidate.
    pub fn flush(&mut self) -> Output {
        self.accept_hanja();
        self.commit(String::new(), true)
    }

    /// Discards everything unconfirmed, for hosts that commit a reading of their own (the
    /// Minecraft mod picks between [`Engine::preedit`] and [`Engine::english`]).
    pub fn clear(&mut self) {
        self.segment.clear();
        self.conversion = None;
        self.hanja = None;
    }

    /// The unconfirmed keystrokes read as English: each key as it types on stock JIS (letters
    /// with their case, digits, JIS symbols). Converted and hanja text stays as shown.
    pub fn english(&self) -> String {
        self.segment
            .iter()
            .filter_map(|t| match *t {
                Token::Jamo { jamo, raw } => Some(raw.unwrap_or(jamo)),
                Token::Text(c) => Some(c),
                Token::Typed(_, english) => Some(english),
                Token::Bare => None,
            })
            .collect()
    }

    /// The unconfirmed text, with the selected hanja candidate in place while a list is open.
    pub fn preedit(&self) -> String {
        match &self.hanja {
            Some(list) => compose(&self.segment[..list.start]) + &list.candidates[list.selected].hanja,
            None => compose(&self.segment),
        }
    }

    pub fn key(&mut self, key: Key, mods: Mods) -> Output {
        if key == Key::Modifier {
            return self.unchanged(false);
        }
        if self.hanja.is_some()
            && let Some(out) = self.hanja_key(key, mods)
        {
            return out;
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
                self.commit(String::new(), true)
            }
            Key::F7 => {
                self.open_hanja();
                self.unchanged(true)
            }
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
                self.segment.pop();
                self.unchanged(true)
            }
            _ => match layout::output(self.layout, key, mods.shift) {
                Some(Out::Jamo(jamo)) => {
                    let raw = key.letter().map(|c| if mods.shift { c.to_ascii_uppercase() } else { c });
                    self.segment.push(Token::Jamo { jamo, raw: raw.or(key.digit()) });
                    self.unchanged(true)
                }
                Some(Out::Char(c)) => {
                    let english = layout::output(Layout::Escaped, key, mods.shift).map_or(c, Out::char);
                    self.segment.push(Token::Typed(c, english));
                    self.unchanged(true)
                }
                None => self.commit(String::new(), false),
            },
        }
    }

    /// A key while the hanja list is open. `None`: the list took the pick and the key goes on
    /// to the normal handling.
    fn hanja_key(&mut self, key: Key, mods: Mods) -> Option<Output> {
        let list = self.hanja.as_mut()?;
        let count = list.candidates.len();
        let page_start = list.selected / PAGE_SIZE * PAGE_SIZE;
        if !mods.is_shortcut() {
            match key {
                Key::F7 | Key::ArrowDown if !mods.shift => list.selected = (list.selected + 1) % count,
                Key::F7 | Key::ArrowUp => list.selected = (list.selected + count - 1) % count,
                Key::ArrowRight => list.selected = (page_start + PAGE_SIZE).min(count - 1),
                Key::ArrowLeft => list.selected = page_start.saturating_sub(PAGE_SIZE),
                Key::Enter => {
                    self.accept_hanja();
                }
                Key::Escape | Key::Backspace => self.hanja = None,
                Key::F6 => {
                    // Back to Hangul (spec §6): F6 commits the reading as typed.
                    self.hanja = None;
                    return Some(self.commit(String::new(), true));
                }
                _ => match key.digit().and_then(|d| d.to_digit(10)) {
                    Some(n @ 1..=9) if !mods.shift && page_start + (n as usize) <= count => {
                        list.selected = page_start + n as usize - 1;
                        self.accept_hanja();
                    }
                    _ => {
                        self.accept_hanja();
                        return None;
                    }
                },
            }
            return Some(self.unchanged(true));
        }
        self.accept_hanja();
        None
    }

    /// F7: open the candidate list for the longest dictionary word at the end of the segment.
    fn open_hanja(&mut self) {
        let Some(dictionary) = &self.dictionary else { return };
        let text = compose(&self.segment);
        let Some((reading, candidates)) = dictionary.longest_suffix(&text) else { return };
        let prefix = &text[..text.len() - reading.len()];
        // The tokens that typed the reading, so earlier tokens stay editable. When the reading
        // doesn't fall on a token boundary (a final it took, say), the prefix becomes plain text.
        let start = (0..self.segment.len())
            .rev()
            .find(|&k| compose(&self.segment[k..]) == reading && compose(&self.segment[..k]) == prefix)
            .unwrap_or_else(|| {
                self.segment = prefix.chars().map(Token::Text).chain(reading.chars().map(Token::Text)).collect();
                prefix.chars().count()
            });
        self.hanja = Some(HanjaList { start, candidates: candidates.to_vec(), selected: 0 });
    }

    fn accept_hanja(&mut self) {
        if let Some(list) = self.hanja.take() {
            self.segment.truncate(list.start);
            self.segment.extend(list.candidates[list.selected].hanja.chars().map(Token::Text));
        }
    }

    /// F6, before it commits: undo an F9/F10 conversion, or drop an implied ㅇ (아 → ㅏ,
    /// 가어 → 가ㅓ).
    fn hangul(&mut self) {
        if let Some(conversion) = self.conversion.take() {
            self.segment.truncate(conversion.start);
            self.segment.extend(conversion.original);
        } else if let Some(at) = composer_for(&self.segment).implied_at() {
            self.segment.insert(at, Token::Bare);
        }
    }

    /// F9/F10: the current word as its keys' alphanumerics, or a run of ASCII symbols and
    /// digits in the other width, cycling case on repeat presses.
    fn convert(&mut self, width: Width) {
        match &mut self.conversion {
            Some(conversion) => {
                if conversion.width == width {
                    conversion.case = (conversion.case + 1) % 3;
                }
                conversion.width = width;
            }
            None => {
                let symbols = matches!(self.segment.last(), Some(Token::Text(c) | Token::Typed(c, _)) if c.is_ascii_graphic());
                let run = self
                    .segment
                    .iter()
                    .rev()
                    .take_while(|t| match t {
                        Token::Text(c) | Token::Typed(c, _) => symbols && c.is_ascii_graphic(),
                        Token::Jamo { raw, .. } => !symbols && raw.is_some(),
                        Token::Bare => false,
                    })
                    .count();
                if run == 0 {
                    return;
                }
                let start = self.segment.len() - run;
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
                Token::Text(c) | Token::Typed(c, _) => Some(*c),
                Token::Bare => None,
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
        self.hanja = None;
        compose(&std::mem::take(&mut self.segment))
    }

    /// Commits the segment followed by `suffix`.
    fn commit(&mut self, suffix: String, consumed: bool) -> Output {
        let mut commit = self.take_segment();
        commit.push_str(&suffix);
        Output { commit, preedit: String::new(), consumed, candidates: None, english: String::new() }
    }

    fn unchanged(&self, consumed: bool) -> Output {
        let candidates = self
            .hanja
            .as_ref()
            .map(|list| Candidates { items: list.candidates.clone(), selected: list.selected });
        Output { commit: String::new(), preedit: self.preedit(), consumed, candidates, english: self.english() }
    }
}

fn composer_for(segment: &[Token]) -> Composer {
    let mut composer = Composer::default();
    for token in segment {
        match *token {
            Token::Jamo { jamo, .. } => composer.jamo(jamo),
            Token::Text(c) | Token::Typed(c, _) => composer.text(c),
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
