//! The predictive layer (spec §10, draft): ranks readings of the staged, uncommitted text.
//!
//! Optional twice over. Built only with the `predict` cargo feature (on by default; a lite build
//! uses `--no-default-features`), and active only once a host loads a [`Predictor`] into the engine.
//! Without one, the engine is exactly the deterministic lite engine: nothing here runs.
//!
//! The engine supplies the readings ([`Readings`]); this module only scores and orders them.
//! Scores are in bits per key: positive leans Korean. The core is a likelihood comparison of the
//! same keystrokes under two key-trigram models (English spelling vs JIS Korean typing), trained by
//! `minecraft/tools/build_ranker.py` into `engine/predict/ranker.bin`.

use std::collections::HashSet;

/// Which reading a prediction is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Kind {
    /// JIS Korean, as composed (한글).
    Korean = 0,
    /// The same keys on JIS QWERTY (gksrmf).
    English = 1,
    /// English with a capital first letter (Gksrmf).
    EnglishCapitalized = 2,
    /// English in capitals (GKSRMF).
    EnglishUpper = 3,
    /// What plain 2-set would compose: no implied ㅇ, digits as digits (ㅠㅠ, not 유유).
    TwoSet = 4,
    /// Korean with every final kept where a vowel would take it (말이, not 마리).
    KoreanKeepFinals = 5,
    /// The hanja form of the whole Korean word, if the dictionary has one. Lean on purpose.
    Hanja = 6,
}

impl Kind {
    fn language(self) -> Language {
        match self {
            Kind::English | Kind::EnglishCapitalized | Kind::EnglishUpper => Language::English,
            _ => Language::Korean,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Korean,
    English,
}

/// The readings of the staged text, from the engine.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Readings {
    pub korean: String,
    pub english: String,
    pub two_set: String,
    pub keep_finals: String,
    /// The hanja form of the whole Korean reading, when it's a dictionary word of 2+ syllables.
    pub hanja: Option<String>,
}

/// What the predictor knows besides the keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Context {
    /// The language of the last committed word: close calls lean the same way.
    pub previous: Option<Language>,
    /// The staged word is a command name (after "/"): always English.
    pub command: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Prediction {
    pub text: String,
    pub kind: Kind,
    /// Higher is likelier. Only the order matters to hosts.
    pub score: f64,
}

/// At most this many predictions are offered.
pub const MAX_PREDICTIONS: usize = 4;

const SIZE: usize = 27;
const START: usize = 26;
const END: usize = 26;
/// Shift on these can be Korean (tense consonants, ㅒ ㅖ); Shift on others mid-word too (Shift+vowel).
const KOREAN_SHIFTS: &str = "QWERTOP";
const IEUNG_SHIFTS: &str = "YUIHJKLBNM";
/// Canonical 2-set keys for the dedicated number-row jamo (0 is ㅘ, 1–5 ㅃ ㅉ ㄸ ㄲ ㅆ, 6–9 ㅟ ㅝ ㅚ ㅢ).
const DIGIT_KEYS: [&str; 10] = ["hk", "q", "w", "e", "r", "t", "nl", "nj", "hl", "ml"];

const LEXICON: f64 = 2.0;
const STICKY: f64 = 0.5;
const BARE_JAMO: f64 = 1.5;
/// Alternatives rank below the reading they're an alternative to.
const ALTERNATIVE: f64 = 1.0;
/// Hanja sit at the bottom: there if wanted, never in the way.
const HANJA: f64 = 8.0;

/// The trained model and word lists.
pub struct Predictor {
    english: Vec<u8>,
    korean: Vec<u8>,
    english_words: HashSet<String>,
    korean_words: HashSet<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BadModel;

impl Predictor {
    /// `model`: `predict/ranker.bin`. `english_words` / `korean_words`: `predict/common-*.txt`.
    pub fn parse(model: &[u8], english_words: &str, korean_words: &str) -> Result<Predictor, BadModel> {
        let table = SIZE * SIZE * SIZE;
        if model.len() != 8 + 2 * table || &model[..4] != b"JKRK" || model[4..8] != [0, 1, 0, SIZE as u8] {
            return Err(BadModel);
        }
        let words = |text: &str| -> HashSet<String> {
            text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(String::from).collect()
        };
        Ok(Predictor {
            english: model[8..8 + table].to_vec(),
            korean: model[8 + table..].to_vec(),
            english_words: words(english_words),
            korean_words: words(korean_words),
        })
    }

    /// The readings, ranked, without duplicates, at most [`MAX_PREDICTIONS`].
    pub fn predict(&self, readings: &Readings, context: Context) -> Vec<Prediction> {
        if readings.korean.is_empty() && readings.english.is_empty() {
            return Vec::new();
        }
        let margin = self.margin(readings, context);
        let english = &readings.english;
        let mut out = vec![
            Prediction { text: readings.korean.clone(), kind: Kind::Korean, score: margin },
            Prediction { text: english.clone(), kind: Kind::English, score: -margin },
        ];
        // Case variants only when the word reads as English: otherwise they'd just be clutter.
        let lowercase = english.chars().any(|c| c.is_ascii_lowercase());
        if margin < 0.0 && lowercase {
            out.push(Prediction { text: capitalized(english), kind: Kind::EnglishCapitalized, score: -margin - ALTERNATIVE });
            out.push(Prediction { text: english.to_uppercase(), kind: Kind::EnglishUpper, score: -margin - 2.0 * ALTERNATIVE });
        }
        for (text, kind) in [(&readings.two_set, Kind::TwoSet), (&readings.keep_finals, Kind::KoreanKeepFinals)] {
            if text.is_empty() || *text == readings.korean {
                continue;
            }
            // An alternative Korean reading that is a known word (ㅠㅠ, 말이) can win.
            let known = if self.korean_words.contains(text.as_str()) { 2.0 * LEXICON } else { 0.0 };
            out.push(Prediction { text: text.clone(), kind, score: margin - ALTERNATIVE + known });
        }
        if let Some(hanja) = &readings.hanja {
            out.push(Prediction { text: hanja.clone(), kind: Kind::Hanja, score: margin - HANJA });
        }

        out.sort_by(|a, b| b.score.total_cmp(&a.score));
        let mut seen = HashSet::new();
        out.retain(|p| !p.text.is_empty() && seen.insert(p.text.clone()));
        // The hanja penalty keeps it last; it shows only when there's room.
        out.truncate(MAX_PREDICTIONS);
        out
    }

    /// Korean minus English, with the rules on top of the model.
    fn margin(&self, readings: &Readings, context: Context) -> f64 {
        let (korean, english) = (&readings.korean, &readings.english);
        if context.command {
            return -99.0;
        }
        if korean == english {
            return 0.0;
        }
        let chars: Vec<char> = english.chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            let mid_word = i > 0 && chars[i - 1].is_alphabetic();
            if c.is_ascii_uppercase() && !KOREAN_SHIFTS.contains(c) && !(mid_word && IEUNG_SHIFTS.contains(c)) {
                return -99.0;
            }
        }
        let mut margin = self.key_margin(english);
        if self.korean_words.contains(korean.as_str()) {
            margin += LEXICON;
        } else {
            margin -= BARE_JAMO * korean.chars().filter(|&c| is_jamo(c)).count() as f64;
        }
        if self.english_words.contains(&english.to_lowercase()) {
            margin -= LEXICON;
        }
        match context.previous {
            Some(Language::Korean) => margin += STICKY,
            Some(Language::English) => margin -= STICKY,
            None => {}
        }
        margin
    }

    /// Korean minus English in average log2 probability per key, over the letter runs.
    fn key_margin(&self, english: &str) -> f64 {
        let (mut total, mut keys) = (0.0, 0usize);
        let mut run = String::new();
        for c in english.chars().chain([' ']) {
            if c.is_ascii_alphabetic() {
                run.push(c.to_ascii_lowercase());
            } else if let Some(d) = c.to_digit(10) {
                run.push_str(DIGIT_KEYS[d as usize]);
            } else if !run.is_empty() {
                total += log_probability(&self.korean, &run) - log_probability(&self.english, &run);
                keys += run.len() + 1;
                run.clear();
            }
        }
        if keys == 0 { 0.0 } else { total / keys as f64 }
    }
}

fn log_probability(table: &[u8], keys: &str) -> f64 {
    let (mut a, mut b) = (START, START);
    let mut total = 0.0;
    for c in keys.bytes().map(|k| (k - b'a') as usize).chain([END]) {
        total -= table[(a * SIZE + b) * SIZE + c] as f64 / 16.0;
        (a, b) = (b, c);
    }
    total
}

fn capitalized(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars.as_str().to_lowercase().chars()).collect()).unwrap_or_default()
}

fn is_jamo(c: char) -> bool {
    ('\u{3131}'..='\u{318E}').contains(&c)
}

impl Language {
    pub fn of(kind: Kind) -> Language {
        kind.language()
    }
}
