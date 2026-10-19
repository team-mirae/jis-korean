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

use crate::Dictionary;

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
    /// Hanja: the dictionary form of a whole Korean word (lean, penalized to last), or one of the
    /// everyday exceptions (news country names, dates, weekdays), which aren't penalized.
    Hanja = 6,
    /// JIS Korean with the implied ㅇ left out: bare vowels (ㅠㅠ, not 유유).
    BareVowels = 7,
    /// An English word with a Korean particle or ending attached (report에, Anthropic에서).
    Mixed = 8,
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
    /// JIS Korean without the implied ㅇ.
    pub bare: String,
    /// How many syllables of the Korean reading have an implied ㅇ.
    pub implied: usize,
    /// The keys split into an English start and a Korean ending, at each place near the end.
    pub splits: Vec<Split>,
}

/// One way to read the staged keys as English followed by Korean.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Split {
    pub english: String,
    pub korean: String,
}

/// What the predictor knows besides the keys.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Context {
    /// The language of the last committed word: close calls lean the same way.
    pub previous: Option<Language>,
    /// The staged word is a command name (after "/"): always English.
    pub command: bool,
    /// The user's tuning.
    pub weights: Weights,
}

/// A scenario the user can tune (spec §10.5). The discriminant is the C ABI id (`JK_WEIGHT_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Weight {
    Korean = 0,
    English = 1,
    /// The implied ㅇ, against the bare vowel.
    Implied = 2,
    /// English + particle words (report에).
    Mixed = 3,
    TwoSet = 4,
    KeepFinals = 5,
    Hanja = 6,
}

impl Weight {
    pub const ALL: [Weight; 7] =
        [Weight::Korean, Weight::English, Weight::Implied, Weight::Mixed, Weight::TwoSet, Weight::KeepFinals, Weight::Hanja];

    pub fn from_id(id: u32) -> Option<Weight> {
        Weight::ALL.get(id as usize).copied()
    }
}

/// User tuning: one multiplier per scenario on its learned probability. 1.0 is as learned; English
/// 0.8 makes English readings 0.8 times as likely, Korean 1.2 makes Korean 1.2 times as likely.
/// In score terms each weight adds log2(weight) bits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weights([f64; 7]);

impl Default for Weights {
    fn default() -> Self {
        Weights([1.0; 7])
    }
}

impl Weights {
    /// Lowest and highest weight accepted; values outside are clamped.
    pub const MIN: f64 = 0.1;
    pub const MAX: f64 = 10.0;

    pub fn get(&self, which: Weight) -> f64 {
        self.0[which as usize]
    }

    pub fn set(&mut self, which: Weight, value: f64) {
        self.0[which as usize] = if value.is_finite() { value.clamp(Self::MIN, Self::MAX) } else { 1.0 };
    }

    /// The weight in bits (log2), the unit scores are in.
    fn bits(&self, which: Weight) -> f64 {
        self.get(which).log2()
    }
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
/// An English reading that is a noun in its normalized form (WordNet): nouns weigh more than verbs
/// and other forms, which get no bonus. A little less than the hand-written lists.
const NOUN: f64 = 1.5;
const STICKY: f64 = 0.5;
const BARE_JAMO: f64 = 1.5;
/// Alternatives rank below the reading they're an alternative to.
const ALTERNATIVE: f64 = 1.0;
/// Hanja sit at the bottom: there if wanted, never in the way.
const HANJA: f64 = 8.0;
/// A known particle after an English start (report에): strong evidence of a mixed word.
const PARTICLE: f64 = 3.0;
/// How much more English the start must read than the whole keys for a split to count (bits per
/// key). Smaller differences are noise: `small` must not become smal이.
const SPLIT_EDGE: f64 = 0.5;
/// How English the start must read on its own (bits per key), so random keys (kanx) don't count.
const SPLIT_START: f64 = 1.0;

/// Particles and endings that attach to English words in mixed Korean text.
const PARTICLES: &[&str] = &[
    "은", "는", "이", "가", "을", "를", "의", "에", "에서", "에게", "에는", "에서는", "에서의", "에도",
    "과", "와", "도", "만", "로", "으로", "로는", "으로는", "로도", "으로도", "까지", "부터", "처럼", "보다",
    "이나", "나", "랑", "이랑", "한테", "께서", "이다", "입니다", "이고", "이며", "이라고", "라고", "이라는",
    "라는", "인", "적", "적인", "적으로", "들", "들이", "들은", "들을", "들의", "하는", "한", "하고", "했다",
    "하다", "해서", "하면", "된", "되는", "화", "화된", "용", "형", "급", "씨", "님",
];

/// OECD members whose hanja abbreviation is in everyday use in Korean news.
const COUNTRIES: &[(char, &str)] = &[
    ('미', "美"), ('일', "日"), ('한', "韓"), ('영', "英"), ('불', "佛"), ('독', "獨"),
    ('이', "伊以"), ('가', "加"), ('호', "濠"), ('토', "土"),
];
/// Dates and weekdays: 年 月 日 曰 and 月火水木金土日.
const DATES: &[(char, &str)] = &[
    ('년', "年"), ('연', "年"), ('월', "月"), ('일', "日"), ('왈', "曰"),
    ('화', "火"), ('수', "水"), ('목', "木"), ('금', "金"), ('토', "土"),
];

/// The trained model and word lists.
pub struct Predictor {
    english: Vec<u8>,
    korean: Vec<u8>,
    english_words: HashSet<String>,
    korean_words: HashSet<String>,
    english_nouns: HashSet<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BadModel;

impl Predictor {
    /// `model`: `predict/ranker.bin`. `english_words` / `korean_words`: `predict/common-*.txt`.
    /// `english_nouns`: `predict/english-nouns.txt` (may be empty: no noun bonus).
    pub fn parse(model: &[u8], english_words: &str, korean_words: &str, english_nouns: &str) -> Result<Predictor, BadModel> {
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
            english_nouns: words(english_nouns),
        })
    }

    /// The readings, ranked, without duplicates, at most [`MAX_PREDICTIONS`]. The dictionary (if
    /// any) is only used for stems in front of 曰.
    pub fn predict(&self, readings: &Readings, context: Context, dictionary: Option<&Dictionary>) -> Vec<Prediction> {
        if readings.korean.is_empty() && readings.english.is_empty() {
            return Vec::new();
        }
        let w = context.weights;
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
            let weight = if kind == Kind::TwoSet { Weight::TwoSet } else { Weight::KeepFinals };
            out.push(Prediction { text: text.clone(), kind, score: margin - ALTERNATIVE + known + w.bits(weight) });
        }
        if let Some(hanja) = &readings.hanja {
            out.push(Prediction { text: hanja.clone(), kind: Kind::Hanja, score: margin - HANJA + w.bits(Weight::Hanja) });
        }
        // Everyday hanja aren't penalized: they tie with the Korean reading and follow it.
        for text in hanja_exceptions(&readings.korean, dictionary) {
            out.push(Prediction { text, kind: Kind::Hanja, score: margin + w.bits(Weight::Hanja) });
        }
        if let Some(mut bare) = self.bare(readings, margin) {
            // Each implied ㅇ is weight times as likely as its bare vowel.
            bare.score -= readings.implied as f64 * w.bits(Weight::Implied);
            out.push(bare);
        }
        if let Some(mut mixed) = self.mixed(readings, -margin) {
            // When the whole keys make good Korean (kanxms is 아무튼), a split is only an
            // alternative to it, never ahead of it.
            if margin > 0.0 {
                mixed.score = mixed.score.min(margin - ALTERNATIVE);
            }
            mixed.score += w.bits(Weight::Mixed);
            out.push(mixed);
        }

        out.sort_by(|a, b| b.score.total_cmp(&a.score));
        let mut seen = HashSet::new();
        out.retain(|p| !p.text.is_empty() && seen.insert(p.text.clone()));
        // The hanja penalty keeps it last; it shows only when there's room.
        out.truncate(MAX_PREDICTIONS);
        out
    }

    /// The bare-vowel reading, weighed by how likely each implied ㅇ is: likely in real words
    /// (아이, 이거), unlikely in runs of one vowel (유유 → ㅠㅠ) and in chat words.
    fn bare(&self, readings: &Readings, margin: f64) -> Option<Prediction> {
        if readings.implied == 0 || readings.bare.is_empty() || readings.bare == readings.korean {
            return None;
        }
        let mut odds = 0.0;
        if self.korean_words.contains(readings.bare.as_str()) {
            odds += 2.0 * LEXICON;
        }
        if repeated_implied(&readings.korean) {
            odds += LEXICON;
        }
        if self.korean_words.contains(readings.korean.as_str()) {
            odds -= LEXICON;
        }
        Some(Prediction { text: readings.bare.clone(), kind: Kind::BareVowels, score: margin - ALTERNATIVE + odds })
    }

    /// The best English-start + particle split (report에, Anthropic에서, RLHF에), if any. It's
    /// offered when the English start reads clearly more English than the whole keys
    /// ([`SPLIT_EDGE`]; a capitalized name always qualifies) and the whole isn't a known English
    /// word. Then the particle is the better explanation for the last keys, and the split ranks
    /// above the whole-word English reading (`english_score`).
    fn mixed(&self, readings: &Readings, english_score: f64) -> Option<Prediction> {
        let whole = -self.english_margin(&readings.english);
        if self.english_words.contains(&readings.english.to_lowercase()) {
            return None;
        }
        readings
            .splits
            .iter()
            .filter(|s| PARTICLES.contains(&s.korean.as_str()))
            .filter(|s| s.english.chars().all(|c| c.is_ascii_alphabetic()) && s.english.len() >= 2)
            .map(|s| (-self.english_margin(&s.english), format!("{}{}", s.english, s.korean)))
            .filter(|(start, _)| *start >= 99.0 || (*start >= SPLIT_START && *start - whole >= SPLIT_EDGE))
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(start, text)| Prediction { text, kind: Kind::Mixed, score: start.max(english_score) + PARTICLE })
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
        let mut margin = self.english_margin(english);
        if self.korean_words.contains(korean.as_str()) {
            margin += LEXICON;
        } else {
            margin -= BARE_JAMO * korean.chars().filter(|&c| is_jamo(c)).count() as f64;
        }
        match context.previous {
            Some(Language::Korean) => margin += STICKY,
            Some(Language::English) => margin -= STICKY,
            None => {}
        }
        // The user's tuning: per-key bits for each language.
        margin + context.weights.bits(Weight::Korean) - context.weights.bits(Weight::English)
    }

    /// Korean minus English for keys read as English: the Shift rule, the model and the English
    /// word list (the Korean-side rules need the Korean reading).
    fn english_margin(&self, english: &str) -> f64 {
        let chars: Vec<char> = english.chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            let mid_word = i > 0 && chars[i - 1].is_alphabetic();
            if c.is_ascii_uppercase() && !KOREAN_SHIFTS.contains(c) && !(mid_word && IEUNG_SHIFTS.contains(c)) {
                return -99.0;
            }
        }
        let mut margin = self.key_margin(english);
        let lower = english.to_lowercase();
        if self.english_words.contains(&lower) {
            margin -= LEXICON;
        } else if self.english_nouns.contains(&lower) {
            margin -= NOUN;
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

/// The everyday hanja for a Korean reading: digits + 年/月/日 (2026年), a run of country, date or
/// weekday syllables (韓美, 月火水木金), or a dictionary stem + 曰 (孔子曰).
pub fn hanja_exceptions(korean: &str, dictionary: Option<&Dictionary>) -> Vec<String> {
    let hanja = |c: char| -> Vec<char> {
        let mut options: Vec<char> = COUNTRIES.iter().chain(DATES).filter(|(k, _)| *k == c).flat_map(|(_, h)| h.chars()).collect();
        options.dedup();
        options
    };
    let mut out = Vec::new();
    let digits: String = korean.chars().take_while(char::is_ascii_digit).collect();
    let rest: Vec<char> = korean[digits.len()..].chars().collect();
    if !digits.is_empty() {
        if let [unit @ ('년' | '월' | '일')] = rest[..] {
            out.push(format!("{digits}{}", hanja(unit)[0]));
        }
        return out;
    }
    let options: Vec<Vec<char>> = rest.iter().map(|&c| hanja(c)).collect();
    if !rest.is_empty() && rest.len() <= 7 && options.iter().all(|o| !o.is_empty()) {
        out.push(options.iter().map(|o| o[0]).collect());
        if let [single] = &options[..] {
            out.extend(single.iter().skip(1).map(|h| h.to_string()));
        }
    }
    if rest.len() >= 2 && rest.last() == Some(&'왈') {
        let stem: String = rest[..rest.len() - 1].iter().collect();
        if let Some(c) = dictionary.and_then(|d| d.lookup(&stem).first()) {
            out.push(format!("{}曰", c.hanja));
        }
    }
    out
}

/// Two identical syllables in a row that start on ㅇ and have no final (유유, 우우): more likely
/// bare vowels typed for chat (ㅠㅠ) than words.
fn repeated_implied(korean: &str) -> bool {
    let chars: Vec<char> = korean.chars().collect();
    chars.windows(2).any(|w| {
        let n = w[0] as u32;
        w[0] == w[1] && (0xC544..=0xC7A3).contains(&n) && (n - 0xAC00).is_multiple_of(28)
    })
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
