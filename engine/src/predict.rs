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

use std::collections::{HashMap, HashSet};

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
    /// Korean with an implied ㄹ after a ㄴ final (전력 for 저녁 typed lazily: ㄴ + ㄹ sounds ㄹㄹ).
    ImpliedL = 9,
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
    /// Korean with one final kept before the vowel after it, one variant per place a final moved
    /// (산이 for 사니; 어린이 for 어리니).
    pub kept: Vec<String>,
    /// Korean with an implied ㄹ after a ㄴ final, one variant per place (전력 for 저녁).
    pub implied_l: Vec<String>,
    /// The hanja form of the whole Korean reading, when it's a dictionary word of 2+ syllables.
    pub hanja: Option<String>,
    /// JIS Korean without the implied ㅇ.
    pub bare: String,
    /// How many syllables of the Korean reading have an implied ㅇ.
    pub implied: usize,
    /// The keys split into an English start and a Korean ending, at each place near the end.
    pub splits: Vec<Split>,
    /// The pieces between punctuation typed inside the word, each with its own readings (empty
    /// when there's no such punctuation).
    pub pieces: Vec<Piece>,
}

/// A piece of a staged word between punctuation, and the punctuation after it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Piece {
    pub readings: Readings,
    pub separator: String,
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
    /// How the implied-ㅇ alternatives are ranked.
    pub ranker: Ranker,
}

/// How the predictor ranks the implied-ㅇ alternatives: the bare vowels (ㅠㅠ against 유유) and the
/// kept final (말이 against 마리). Everything else ranks the same either way.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Ranker {
    /// Hand-written rules: one step below the Korean reading, runs of one vowel lean bare (§10.3).
    #[default]
    Rules,
    /// Effort (experimental, spec §10.7): the keys a typist who meant the alternative would have
    /// had to press and didn't, by count and finger travel.
    Travel(Effort),
    /// Route preference (experimental, spec §10.8): how strongly a typist who meant a reading
    /// would prefer the route the keys took over the other one, each jamo or the optimized route.
    Route(Preference),
}

impl Ranker {
    /// The C ABI id (`JK_RANKER_*`): 0 rules, 1 travel with the default costs.
    pub fn from_id(id: u32) -> Option<Ranker> {
        match id {
            0 => Some(Ranker::Rules),
            1 => Some(Ranker::Travel(Effort::default())),
            2 => Some(Ranker::Route(Preference::default())),
            _ => None,
        }
    }
}

/// How strongly a typist prefers JIS Korean's optimized route (implied ㅇ, dedicated keys,
/// Shift+vowel, double press) over typing each jamo: the odds of taking it are
/// σ(press × presses saved + travel × travel saved − bias). `omission`: bits per key press a reading
/// needs that the keys left out (말이 typed as 마리).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preference {
    pub press: f64,
    pub travel: f64,
    pub bias: f64,
    pub omission: f64,
}

impl Default for Preference {
    /// Near the best of the grid in `examples/sxs.rs` (2026-10-19, retrained on the corpora):
    /// presses saved don't move the preference, travel saved slightly lowers it (dedicated keys
    /// cost travel and are taken anyway), and a typist takes JIS Korean's route about 3 times in 4.
    fn default() -> Self {
        Preference { press: 0.0, travel: -0.5, bias: -1.0, omission: 3.0 }
    }
}

impl Preference {
    /// The chance a typist takes a route that saves `presses` and `travel` over the other one
    /// (negative: costs more). Equal routes are a coin toss when `bias` is 0.
    fn odds(&self, presses: f64, travel: f64) -> f64 {
        let x = self.press * presses + self.travel * travel - self.bias;
        (1.0 / (1.0 + (-x).exp())).clamp(0.02, 0.98)
    }
}

/// What skipped effort costs, in bits: per key press left out (Shift counted), and per key unit of
/// finger travel left out. A negative cost is a credit: the more travel a reading saves (F6 is far),
/// the likelier the typist relied on it. The defaults are the best of the grid in `examples/sxs.rs`
/// (2026-10-19, retrained on the corpora): both zero, so only the lexicon decides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effort {
    pub press: f64,
    pub travel: f64,
}

impl Default for Effort {
    fn default() -> Self {
        Effort { press: 0.0, travel: 0.0 }
    }
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
    /// An implied ㄹ after a ㄴ final (전력). 0 turns it off.
    ImpliedL = 7,
}

impl Weight {
    pub const ALL: [Weight; 8] = [
        Weight::Korean,
        Weight::English,
        Weight::Implied,
        Weight::Mixed,
        Weight::TwoSet,
        Weight::KeepFinals,
        Weight::Hanja,
        Weight::ImpliedL,
    ];

    pub fn from_id(id: u32) -> Option<Weight> {
        Weight::ALL.get(id as usize).copied()
    }
}

/// User tuning: one multiplier per scenario on its learned probability. 1.0 is as learned; English
/// 0.8 makes English readings 0.8 times as likely, Korean 1.2 makes Korean 1.2 times as likely.
/// In score terms each weight adds log2(weight) bits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weights([f64; Weight::ALL.len()]);

impl Default for Weights {
    fn default() -> Self {
        Weights([1.0; Weight::ALL.len()])
    }
}

impl Weights {
    /// Lowest and highest weight accepted; values outside are clamped.
    pub const MIN: f64 = 0.1;
    pub const MAX: f64 = 10.0;

    pub fn get(&self, which: Weight) -> f64 {
        self.0[which as usize]
    }

    /// Sets a weight, clamped to [`Weights::MIN`]–[`Weights::MAX`]. 0 turns an alternative reading
    /// off entirely (not Korean or English, which always show).
    pub fn set(&mut self, which: Weight, value: f64) {
        let off = value == 0.0 && !matches!(which, Weight::Korean | Weight::English);
        self.0[which as usize] = if off {
            0.0
        } else if value.is_finite() {
            value.clamp(Self::MIN, Self::MAX)
        } else {
            1.0
        };
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
/// The count a hand-written lexicon word stands for: as common as a frequent chat word.
const HAND_COUNT: u32 = 1000;
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
/// A word the user never means (training file `avoid`, spec §10.12): its readings drop this far.
/// A strong one is also never on top.
const AVOIDED: f64 = 4.0;

/// Particles and endings that attach to English words in mixed Korean text.
const PARTICLES: &[&str] = &[
    "은", "는", "이", "가", "을", "를", "의", "에", "에서", "에게", "에는", "에서는", "에서의", "에도",
    "과", "와", "도", "만", "로", "으로", "로는", "으로는", "로도", "으로도", "까지", "부터", "까지는", "까지의", "부터는", "부터의", "부로", "처럼", "보다",
    "이나", "나", "랑", "이랑", "한테", "께서", "이다", "입니다", "이고", "이며", "이라고", "라고", "이라는",
    "라는", "인", "적", "적인", "적으로", "들", "들이", "들은", "들을", "들의", "하는", "한", "하고", "했다",
    "하다", "해서", "하면", "된", "되는", "화", "화된", "용", "형", "급", "씨", "님",
];

/// `word` or `word<TAB>count` lines (`#` lines are comments) into word counts; a word without a
/// count stands for a common one ([`HAND_COUNT`]).
fn word_counts(text: &str) -> HashMap<String, u32> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| match l.split_once('\t') {
            Some((word, count)) => (word.trim().to_string(), count.trim().parse().unwrap_or(HAND_COUNT)),
            None => (l.to_string(), HAND_COUNT),
        })
        .collect()
}

/// First syllables of 하다 / 되다 forms that turn an English word into a verb (push했어).
const VERB_STARTS: &str = "하한할함합해했되된될됨됩돼됐";

/// Particles, copula forms and endings that begin with a vowel (on ㅇ), longest first where one
/// starts another doesn't matter (prefix match). From 한글 맞춤법 제14·15·19항's examples and common
/// usage: where these follow a known word, its final is kept in spelling.
const VOWEL_PARTICLES_AND_ENDINGS: &[&str] = &[
    // 조사 and the copula
    "이", "은", "을", "의", "에", "에서", "에게", "에는", "에도", "으로", "이나", "이랑", "이며", "이야", "아", "이다",
    "입니다", "이에요", "이었", "이고", "이지만", "이면", "이니까", "인", "일", "임", "으로서", "으로써", "으로부터",
    "이라", "이라도", "이란", "이라는", "이랑",
    // 어미 and -이/-음/-히
    "아", "어", "았", "었", "아서", "어서", "아도", "어도", "아야", "어야", "아요", "어요", "으니", "으면", "으며",
    "으러", "으려", "은", "을", "음", "으세", "으시", "읍시", "으므로", "을까", "을게", "을수록", "히",
];

/// Particles that follow only a consonant (after a vowel they're 는 를 와 로 랑 나): the whole
/// rest of a word after a ㅇ final, they show the final was kept (소비량은, not 소비랴은).
/// Not 이: too many words end in a vowel and 이 (아이, 오이), typed with ㅇ (d k d l).
const AFTER_CONSONANT_ONLY: &[&str] = &["은", "을", "과", "으로", "으로는", "으로서", "으로써", "으로부터", "으로도", "이랑", "이나"];

/// The least weight an implied ㅇ after a ㄴ or ㄹ final gets (bits) when what comes before is a
/// known word (기준입니다 typed r l w n s l q s l e k, 당신이, 산이, 말이): more than an
/// alternative's one-step penalty, so a noun and its particle win when the word lists don't say
/// otherwise. Elsewhere the corpus odds stand (드리겠습니다 stays, not 들이겠습니다).
const IMPLIED_AFTER_FLOOR: f64 = 1.5;

/// How far a word-initial Shift+vowel capital leans English when it doesn't settle it (bits).
const CAPITAL_LEAN: f64 = 2.0;

/// A word read piece by piece around its punctuation ranks above the whole-word readings: the
/// pieces were each read the likeliest way.
const PIECEWISE: f64 = 1.0;

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
#[derive(Clone)]
pub struct Predictor {
    english: Vec<u8>,
    korean: Vec<u8>,
    /// Lexicon words with their counts in the training sentences (hand-written ones: [`HAND_COUNT`]).
    english_words: HashMap<String, u32>,
    korean_words: HashMap<String, u32>,
    english_nouns: HashSet<String>,
    /// Bits for an implied ㅇ after a kept final, per (final, following syllable) (spec §10.3),
    /// learned from the corpora (`#!implied-after` lines in common-ko.txt).
    implied_after: HashMap<(char, char), f64>,
    /// Words the user never means, each `true` if strong (never on top). English ones lowercase.
    avoided_korean: HashMap<String, bool>,
    avoided_english: HashMap<String, bool>,
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
        let words = word_counts;
        Ok(Predictor {
            english: model[8..8 + table].to_vec(),
            korean: model[8 + table..].to_vec(),
            english_words: words(english_words),
            korean_words: words(korean_words),
            english_nouns: words(english_nouns).into_keys().collect(),
            implied_after: korean_words
                .lines()
                .filter_map(|l| l.strip_prefix("#!implied-after "))
                .filter_map(|rest| {
                    let mut fields = rest.split_whitespace();
                    let final_ = fields.next()?.chars().next()?;
                    let syllable = fields.next()?.chars().next()?;
                    Some(((final_, syllable), fields.next()?.parse::<f64>().ok()?))
                })
                .map(|(key, odds)| (key, odds.clamp(-2.0 * LEXICON, 2.0 * LEXICON)))
                .collect(),
            avoided_korean: HashMap::new(),
            avoided_english: HashMap::new(),
        })
    }

    /// This predictor with the user's own words added (spec §10.12, the training contract): lines of
    /// `word` or `word<TAB>count` as in common-*.txt. A word already known keeps the larger count;
    /// a word without a count counts as a common one ([`HAND_COUNT`]).
    pub fn with_user_words(&self, korean: &str, english: &str) -> Predictor {
        let mut next = self.clone();
        let merge = |into: &mut HashMap<String, u32>, text: &str, lowercase: bool| {
            for (word, count) in word_counts(text) {
                let entry = into.entry(if lowercase { word.to_lowercase() } else { word }).or_insert(0);
                *entry = (*entry).max(count);
            }
        };
        merge(&mut next.korean_words, korean, false);
        merge(&mut next.english_words, english, true);
        next
    }

    /// This predictor with words the user never means (spec §10.12, the training contract's
    /// `avoid`): lines of `word` or `word<TAB>strong`. Wherever a reading is an avoided word, alone
    /// or with a particle after it (god, god을, 마리가), it drops [`AVOIDED`] bits; a strong one is
    /// also never on top. English matches ignore case, but keys starting with a capital are taken as
    /// asked for (God typed with Shift+G stays). An avoided word wins over the same word in the vocabulary.
    pub fn with_avoided(&self, korean: &str, english: &str) -> Predictor {
        let mut next = self.clone();
        let merge = |into: &mut HashMap<String, bool>, text: &str, lowercase: bool| {
            for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
                let (word, strength) = line.split_once('\t').unwrap_or((line, ""));
                let word = if lowercase { word.trim().to_lowercase() } else { word.trim().to_string() };
                let strong = into.get(&word).copied().unwrap_or(false) || strength.trim() == "strong";
                into.insert(word, strong);
            }
        };
        merge(&mut next.avoided_korean, korean, false);
        merge(&mut next.avoided_english, english, true);
        next
    }

    /// Whether a prediction is a word the user never means, and if so whether strongly. `keys`: the
    /// English reading, the keys as typed.
    fn avoided(&self, p: &Prediction, keys: &str) -> Option<bool> {
        // A word, alone or followed by a particle.
        let find = |list: &HashMap<String, bool>, text: &str| {
            list.get(text).copied().or_else(|| {
                PARTICLES.iter().find_map(|particle| text.strip_suffix(particle).and_then(|word| list.get(word).copied()))
            })
        };
        if p.kind == Kind::Hanja {
            return None;
        }
        if p.kind.language() == Language::Korean && p.kind != Kind::Mixed {
            return if self.avoided_korean.is_empty() { None } else { find(&self.avoided_korean, &p.text) };
        }
        // English, or the English start of a mixed word. A capital at the start settled English
        // (spec §10.3 rule 2): that spelling was asked for.
        if self.avoided_english.is_empty() || keys.starts_with(|c: char| c.is_ascii_uppercase()) {
            return None;
        }
        let start = p.text.trim_end_matches(|c: char| !c.is_ascii());
        if start.is_empty() || !start.chars().all(|c| c.is_ascii_alphabetic()) {
            return None;
        }
        self.avoided_english.get(&start.to_lowercase()).copied()
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
        // The bare-vowel reading is usually the 2-set one too (ㅏㅣ for k l): then it's one row,
        // weighed by both, so the implied ㅇ weight moves it either way.
        let bare = match context.ranker {
            Ranker::Rules => self.bare(readings, margin),
            Ranker::Travel(costs) => self.bare_by_effort(readings, margin, costs),
            Ranker::Route(preference) => self.bare_by_route(readings, margin, preference),
        };
        let bare = bare.map(|mut bare| {
            // Each implied ㅇ is weight times as likely as its bare vowel.
            bare.score -= readings.implied as f64 * w.bits(Weight::Implied);
            if bare.text == readings.two_set {
                bare.score += w.bits(Weight::TwoSet);
            }
            bare
        });
        for (text, kind) in [(&readings.two_set, Kind::TwoSet), (&readings.keep_finals, Kind::KoreanKeepFinals)] {
            if text.is_empty() || *text == readings.korean || bare.as_ref().is_some_and(|b| b.text == *text) {
                continue;
            }
            // An alternative Korean reading that is a known word (ㅠㅠ, 말이) can win. Not the plain
            // 2-set one when the keys have a Shift+vowel: that Shift asked for the ㅇ (열어 typed
            // d u f ⇧J isn't 여러).
            let shift_vowel = kind == Kind::TwoSet && has_shift_vowel(english);
            let known = if self.korean_words.contains_key(text.as_str()) && !shift_vowel {
                self.lexicon_odds(readings, text)
            } else {
                0.0
            };
            let weight = if kind == Kind::TwoSet { Weight::TwoSet } else { Weight::KeepFinals };
            let score = match context.ranker {
                Ranker::Travel(costs) if kind == Kind::KoreanKeepFinals => {
                    let Some(skipped) = self.skipped(readings, text, costs) else { continue };
                    margin + self.lexicon_odds(readings, text) - skipped
                }
                Ranker::Route(preference) => {
                    let Some(route) = self.route_odds(readings, text, preference) else { continue };
                    margin + self.lexicon_odds(readings, text) + jamo_prior(text, &readings.korean) + route
                }
                _ => margin - ALTERNATIVE + known,
            };
            out.push(Prediction { text: text.clone(), kind, score: score + w.bits(weight) });
        }
        // A ㄴ or ㄹ final kept before 이 / 의 / 와 / 에 / 아 on an implied ㅇ (산이 against 사니), by the
        // odds learned from the corpora. Same row as the kept finals; the higher copy stays.
        for kept in &readings.kept {
            // Only what it keeps beyond the typed reading counts (술의 typed with d is in both).
            let bits = self.implied_after_bits(kept, dictionary) - self.implied_after_bits(&readings.korean, dictionary);
            // Both ways: a common typed word (아니) keeps beating a rare kept reading (안이).
            let known = self.lexicon_odds(readings, kept);
            let score = margin - ALTERNATIVE + bits + known + w.bits(Weight::KeepFinals);
            out.push(Prediction { text: kept.clone(), kind: Kind::KoreanKeepFinals, score });
        }
        // An implied ㄹ after a ㄴ final (전력 for 저녁): an alternative, one step below unless a
        // known word; the counts decide between two known words.
        if w.get(Weight::ImpliedL) > 0.0 {
            for text in &readings.implied_l {
                let score = margin - ALTERNATIVE + self.lexicon_odds(readings, text) + w.bits(Weight::ImpliedL);
                out.push(Prediction { text: text.clone(), kind: Kind::ImpliedL, score });
            }
        }
        if let Some(hanja) = &readings.hanja {
            out.push(Prediction { text: hanja.clone(), kind: Kind::Hanja, score: margin - HANJA + w.bits(Weight::Hanja) });
        }
        // Everyday hanja aren't penalized: they tie with the Korean reading and follow it.
        for text in hanja_exceptions(&readings.korean, dictionary) {
            out.push(Prediction { text, kind: Kind::Hanja, score: margin + w.bits(Weight::Hanja) });
        }
        out.extend(bare);
        if let Some(mut mixed) = self.mixed(readings, -margin) {
            // When the whole keys make a Korean word (kanxms is 아무튼; a word in the lexicon or in
            // the hanja dictionary), a split is only an alternative to it, never ahead of it. Junk
            // Korean (slack에 as 님차에) doesn't hold it back.
            // A word started with Shift+vowel (오픈 as H v m s) counts as Korean the same way.
            let word = self.korean_words.contains_key(readings.korean.as_str())
                || readings.hanja.is_some()
                || self.shift_vowel_start(&readings.english);
            if margin > 0.0 && word {
                mixed.score = mixed.score.min(margin - ALTERNATIVE);
            }
            mixed.score += w.bits(Weight::Mixed);
            out.push(mixed);
        }

        // Punctuation inside the word: each piece read on its own (Research(이하, AI)를).
        if readings.pieces.len() > 1 {
            let text: String = readings
                .pieces
                .iter()
                .map(|piece| {
                    let top = self.predict(&piece.readings, context, dictionary).into_iter().next();
                    top.map_or(String::new(), |p| p.text) + &piece.separator
                })
                .collect();
            // Also when it's a whole-word reading (work), 있습니다.): there the punctuation drags
            // that reading's score down, and this copy, ranked higher, is the one kept.
            let kind = if text == readings.korean {
                Kind::Korean
            } else if text == *english {
                Kind::English
            } else {
                Kind::Mixed
            };
            out.push(Prediction { text, kind, score: margin.abs() + PIECEWISE });
        }

        // Words the user never means drop, and a strong one is never on top (below).
        let mut strong = HashSet::new();
        for p in &mut out {
            if let Some(is_strong) = self.avoided(p, english) {
                p.score -= AVOIDED;
                if is_strong {
                    strong.insert(p.text.clone());
                }
            }
        }
        // A scenario weighed 0 is off: its readings go.
        out.retain(|p| p.score.is_finite());
        out.sort_by(|a, b| b.score.total_cmp(&a.score));
        let mut seen = HashSet::new();
        out.retain(|p| !p.text.is_empty() && seen.insert(p.text.clone()));
        // Hard guard: a hanja reading is never on top, whatever the weights (a user's 10×, or
        // learning): hanja are picked with ↓. Nor is a strongly avoided word. The first other
        // reading moves up.
        let held = |p: &Prediction| p.kind == Kind::Hanja || strong.contains(&p.text);
        if out.first().is_some_and(held)
            && let Some(i) = out.iter().position(|p| !held(p))
        {
            let other = out.remove(i);
            out.insert(0, other);
        }
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
        if self.korean_words.contains_key(readings.bare.as_str()) {
            odds += self.lexicon_odds(readings, &readings.bare);
        }
        if repeated_implied(&readings.korean).is_some_and(|syllable| self.chat_run(syllable)) {
            odds += LEXICON;
        }
        if self.korean_words.contains_key(readings.korean.as_str()) && !self.korean_words.contains_key(readings.bare.as_str()) {
            odds -= LEXICON;
        }
        Some(Prediction { text: readings.bare.clone(), kind: Kind::BareVowels, score: margin - ALTERNATIVE + odds })
    }

    /// The bare-vowel reading by effort (spec §10.7): the lexicon's evidence, less the effort a
    /// typist who meant the bare vowels skipped (the F6 after each).
    fn bare_by_effort(&self, readings: &Readings, margin: f64, costs: Effort) -> Option<Prediction> {
        if readings.implied == 0 || readings.bare.is_empty() || readings.bare == readings.korean {
            return None;
        }
        let skipped = self.skipped(readings, &readings.bare, costs)?;
        let score = margin + self.lexicon_odds(readings, &readings.bare) - skipped;
        Some(Prediction { text: readings.bare.clone(), kind: Kind::BareVowels, score })
    }

    /// The bare-vowel reading by route preference (spec §10.8).
    fn bare_by_route(&self, readings: &Readings, margin: f64, preference: Preference) -> Option<Prediction> {
        if readings.implied == 0 || readings.bare.is_empty() || readings.bare == readings.korean {
            return None;
        }
        let route = self.route_odds(readings, &readings.bare, preference)?;
        let score = margin + self.lexicon_odds(readings, &readings.bare) + jamo_prior(&readings.bare, &readings.korean) + route;
        Some(Prediction { text: readings.bare.clone(), kind: Kind::BareVowels, score })
    }

    /// log2 P(keys | alternative) − log2 P(keys | Korean reading), in bits, by route preference.
    ///
    /// The Korean reading: the keys are its each-jamo route (taken with the chance the optimized
    /// one wasn't preferred) or a route with shortcuts (taken with the chance they were, for what
    /// they saved over each jamo). An alternative: the keys are its each-jamo route (ㅠㅠ as b b,
    /// taken unless its optimized route saves effort, which for bare vowels it doesn't: F6 is
    /// extra), or they leave out keys it needs, at `omission` bits per press.
    fn route_odds(&self, readings: &Readings, alternative: &str, preference: Preference) -> Option<f64> {
        let keys = crate::plan::keys_of(&readings.english);
        let typed = crate::plan::effort(&keys);
        let log2 = |p: f64| p.clamp(0.02, 1.0).log2();
        // Taking the each-jamo route `jamo` when `best` was there to be had. No cheaper route at
        // all: the each-jamo route is the only one (마리, ㅠㅠ), taken for sure.
        let chose_jamo = |jamo: crate::plan::Effort, best: crate::plan::Effort| {
            if best.presses >= jamo.presses && best.travel >= jamo.travel - 1e-9 {
                return 0.0;
            }
            log2(1.0 - preference.odds(jamo.presses as f64 - best.presses as f64, jamo.travel - best.travel))
        };
        let korean = {
            let jamo_keys = crate::plan::each_jamo(&readings.korean)?;
            let jamo = crate::plan::effort(&jamo_keys);
            if jamo_keys == keys {
                chose_jamo(jamo, crate::plan::cheapest(&readings.korean)?)
            } else {
                log2(preference.odds(jamo.presses as f64 - typed.presses as f64, jamo.travel - typed.travel))
            }
        };
        let jamo_keys = crate::plan::each_jamo(alternative)?;
        let (best, best_keys) = crate::plan::cheapest_route(alternative)?;
        let alt = if jamo_keys == keys {
            chose_jamo(crate::plan::effort(&jamo_keys), best)
        } else {
            // Keys left out (or extra): the fewest edits from the typed keys to either route.
            let edits = crate::plan::edits(&keys, &jamo_keys).min(crate::plan::edits(&keys, &best_keys));
            -preference.omission * edits as f64
        };
        Some(alt - korean)
    }

    /// Whether chat doubles this vowel as bare jamo more than as syllables: ㅠㅠ is more common than
    /// 유유, while 워워 beats ㅝㅝ (the lexicon's counts decide).
    fn chat_run(&self, syllable: char) -> bool {
        let medial = ((syllable as u32 - 0xAC00) % 588 / 28) as usize;
        let Some(vowel) = char::from_u32(0x314F + medial as u32) else { return false };
        let count = |s: String| self.korean_words.get(&s).copied().unwrap_or(0);
        let bare = count(format!("{vowel}{vowel}"));
        bare > 0 && bare > count(format!("{syllable}{syllable}"))
    }

    /// The kept-final bits of a reading: for each final followed by a syllable on ㅇ (by its vowel:
    /// 입 as 이), at least [`IMPLIED_AFTER_FLOOR`] where the reading up to it is a known word of two
    /// syllables or more and the rest begins with a particle or ending (기준 + 입니다); else the
    /// corpus odds for ㄴ and ㄹ (산이 against 사니), and a step against for other finals. A double
    /// final always counts as kept (값이).
    fn implied_after_bits(&self, text: &str, dictionary: Option<&Dictionary>) -> f64 {
        const FINALS: &str = " ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ";
        let chars: Vec<char> = text.chars().collect();
        // The lexicon only: the hanja dictionary's rare readings (세민) aren't a typist's nouns.
        let _ = dictionary;
        // A word ending in a 하다 / 되다 modifier (반영한, 사용될) is a verb form, not a noun a
        // particle follows: its final is an ending's (반영하는, not 반영한은).
        let known = |word: &str| {
            word.chars().count() >= 2
                && self.korean_words.contains_key(word)
                && !(word.chars().count() >= 3 && word.ends_with(['한', '할', '된', '될']))
        };
        (1..chars.len())
            .filter_map(|i| {
                let n = (chars[i - 1] as u32).checked_sub(0xAC00).filter(|&n| n < 11172)?;
                let final_ = FINALS.chars().nth((n % 28) as usize)?;
                // The next syllable counts by its vowel, whatever its final (입 as 이).
                let next = (chars[i] as u32).checked_sub(0xAC00).filter(|&n| n < 11172)?;
                let open = char::from_u32(0xAC00 + next - next % 28)?;
                if (open as u32 - 0xAC00) / 588 != 11 {
                    return None; // not a syllable on ㅇ: nothing kept here
                }
                let stem: String = chars[..i].iter().collect();
                // A double final is spelled whole before a vowel (값이, 많이): only its second half
                // moves, in pronunciation (표준 발음법 제14항). When the stem is a word: two keys
                // can also just fall together (지난주 isn't 지낝우).
                if "ㄳㄵㄶㄺㄻㄼㄽㄾㄿㅀㅄ".contains(final_) {
                    return Some(if self.korean_words.contains_key(stem.as_str()) { LEXICON } else { -LEXICON });
                }
                let rest: String = chars[i..].iter().collect();
                // 한글 맞춤법 제14·15항: a final is spelled where it is when a particle or ending
                // follows the word it ends (기준 + 입니다, 당신 + 이, 바둑 + 이). Within one morpheme
                // it moves (사람, 지난주).
                let boundary = known(&stem) && VOWEL_PARTICLES_AND_ENDINGS.iter().any(|p| rest.starts_with(p));
                // A ㅇ final "moved" leaves the next syllable as it is (량은 → 랴은), so a particle
                // that only follows consonants (은 을 이 과 으로 …) proves it was kept: 소비량은.
                if final_ == 'ㅇ' && AFTER_CONSONANT_ONLY.contains(&rest.as_str()) {
                    return Some(LEXICON);
                }
                // The corpus odds where learned (ㄴ, ㄹ); else a step against keeping it.
                let odds = self.implied_after.get(&(final_, open)).copied().unwrap_or(-LEXICON);
                Some(if boundary { odds.max(IMPLIED_AFTER_FLOOR) } else { odds })
            })
            .sum()
    }

    /// Lexicon evidence for an alternative Korean reading against the Korean one: a known word
    /// against an unknown one leans its way; when both are words, their counts decide, within two
    /// lexicon steps (ㅠㅠ is far more common than 유유; 저녁 and 전역 are both common, so the
    /// reading the keys spell keeps its place).
    fn lexicon_odds(&self, readings: &Readings, alternative: &str) -> f64 {
        match (self.korean_words.get(alternative), self.korean_words.get(readings.korean.as_str())) {
            (Some(_), None) => 2.0 * LEXICON,
            (None, Some(_)) => -LEXICON,
            (Some(&a), Some(&k)) => (a as f64 / k as f64).log2().clamp(-2.0 * LEXICON, 2.0 * LEXICON),
            (None, None) => 0.0,
        }
    }

    /// The effort, in bits, that the staged keys leave out of the cheapest way to type
    /// `alternative`. None when the alternative can't be typed.
    fn skipped(&self, readings: &Readings, alternative: &str, costs: Effort) -> Option<f64> {
        let full = crate::plan::cheapest(alternative)?;
        let typed = crate::plan::effort_of_keys(&readings.english);
        let presses = full.presses.saturating_sub(typed.presses) as f64;
        let travel = (full.travel - typed.travel).max(0.0);
        Some(costs.press * presses + costs.travel * travel)
    }

    /// The best English-start + particle split (report에, Anthropic에서, RLHF에), if any. It's
    /// offered when the English start reads clearly more English than the whole keys
    /// ([`SPLIT_EDGE`]; a capitalized name always qualifies) and the whole isn't a known English
    /// word. Then the particle is the better explanation for the last keys, and the split ranks
    /// above the whole-word English reading (`english_score`).
    fn mixed(&self, readings: &Readings, english_score: f64) -> Option<Prediction> {
        // Measured the way the starts are, so a Shift+vowel capital weighs the same on both.
        let whole = -self.start_margin(&readings.english);
        let lower = readings.english.to_lowercase();
        if self.english_words.contains_key(&lower) || self.english_nouns.contains(&lower) {
            return None;
        }
        readings
            .splits
            .iter()
            .filter(|s| self.is_ending(&s.korean))
            .filter(|s| s.english.chars().all(|c| c.is_ascii_alphabetic()) && s.english.len() >= 2)
            // Among starts that are surely English (capitals), the longer start wins (Android에, not
            // Androi에 with d p as 에; SSD에, Docker와), except an acronym that trails off in a
            // lowercase letter: that letter belongs to the ending (API를, not APIf을).
            .map(|s| {
                let acronym = s.english.chars().take(2).all(|c| c.is_ascii_uppercase());
                let tail = acronym && s.english.ends_with(|c: char| c.is_ascii_lowercase());
                (-self.start_margin(&s.english), (!tail, s.english.len()), format!("{}{}", s.english, s.korean))
            })
            .filter(|(start, _, _)| *start >= 99.0 || (*start >= SPLIT_START && *start - whole >= SPLIT_EDGE))
            .max_by(|a, b| a.0.min(99.0).total_cmp(&b.0.min(99.0)).then(a.1.cmp(&b.1)))
            .map(|(start, _, text)| Prediction { text, kind: Kind::Mixed, score: start.max(english_score) + PARTICLE })
    }

    /// [`Predictor::english_margin`] for the English start of a split: a Shift+vowel capital at the
    /// start (오픈 as H v m s) only leans English, unless the start is a known English word, so it
    /// doesn't make the split certain the way Anthropic's A does.
    fn start_margin(&self, start: &str) -> f64 {
        let margin = self.english_margin(start);
        let lower = start.to_lowercase();
        // Two capitals in a row is an acronym (KPI와, UI가): that settles it as before.
        let acronym = start.chars().take(2).filter(char::is_ascii_uppercase).count() == 2;
        if margin <= -99.0
            && !acronym
            && start.starts_with(|c: char| IEUNG_SHIFTS.contains(c))
            && !self.english_words.contains_key(&lower)
            && !self.english_nouns.contains(&lower)
        {
            return self.english_margin_with(start, true) - CAPITAL_LEAN;
        }
        margin
    }

    /// The keys start with a Shift+vowel capital that doesn't settle English: not an acronym, not a
    /// known English word.
    fn shift_vowel_start(&self, english: &str) -> bool {
        let lower = english.to_lowercase();
        english.starts_with(|c: char| IEUNG_SHIFTS.contains(c))
            && english.chars().take(2).filter(char::is_ascii_uppercase).count() < 2
            && !self.english_words.contains_key(&lower)
            && !self.english_nouns.contains(&lower)
    }

    /// A Korean ending that can follow an English word: a particle, or a known word made of 하다 /
    /// 되다 forms (push했어, deploy했는데).
    fn is_ending(&self, korean: &str) -> bool {
        PARTICLES.contains(&korean)
            || (korean.starts_with(|c: char| VERB_STARTS.contains(c)) && self.korean_words.contains_key(korean))
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
        // A word can start with Shift+vowel too (있습니다 as L T t m q s l e k). Unless the keys
        // spell a known English word, that capital leans English instead of settling it, and a
        // known Korean word cancels the lean.
        let lower = english.to_lowercase();
        if margin <= -99.0
            && english.starts_with(|c: char| IEUNG_SHIFTS.contains(c))
            && !self.english_words.contains_key(&lower)
            && !self.english_nouns.contains(&lower)
        {
            let known = self.korean_words.contains_key(korean.as_str()) || readings.hanja.is_some();
            margin = self.english_margin_with(english, true) - if known { 0.0 } else { CAPITAL_LEAN };
        }
        if self.korean_words.contains_key(korean.as_str()) {
            margin += LEXICON;
        } else {
            margin -= BARE_JAMO * korean.chars().filter(|&c| is_jamo(c)).count() as f64;
        }
        match context.previous {
            Some(Language::Korean) => margin += STICKY,
            Some(Language::English) => margin -= STICKY,
            None => {}
        }
        // One vowel key is a syllable on an implied ㅇ (ㅗ → 오), and a lone lowercase letter
        // is hardly ever an English word: Korean keeps it, or the first key of every word
        // starting with ㅗ ㅕ ㅐ ㅜ … would show as h u o n.
        if readings.implied > 0 && english.len() == 1 && english.chars().all(|c| c.is_ascii_lowercase()) {
            margin = margin.max(ALTERNATIVE);
        }
        // A known Korean word against keys that spell no English word (몇 against auc, 어디야
        // against jeli, ㅇㅈ against dw): Korean, unless the Shift rule settled it.
        let lower = english.to_lowercase();
        if margin > -99.0
            && self.korean_words.contains_key(korean.as_str())
            && !self.english_words.contains_key(&lower)
            && !self.english_nouns.contains(&lower)
        {
            margin = margin.max(ALTERNATIVE);
        }
        // The user's tuning: per-key bits for each language.
        margin + context.weights.bits(Weight::Korean) - context.weights.bits(Weight::English)
    }

    /// Korean minus English for keys read as English: the Shift rule, the model and the English
    /// word list (the Korean-side rules need the Korean reading).
    fn english_margin(&self, english: &str) -> f64 {
        self.english_margin_with(english, false)
    }

    /// [`Predictor::english_margin`]; `initial_ieung`: a Shift+vowel capital at the start doesn't
    /// settle it either (see `margin`).
    fn english_margin_with(&self, english: &str, initial_ieung: bool) -> f64 {
        let chars: Vec<char> = english.chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            // After any key of the word (a number-row key too: 있 as l 5, then Shift+ㅡ), a Shift
            // on a vowel key is Shift+vowel, starting a syllable on ㅇ (spec §4.2).
            let ieung_ok = (i > 0 && chars[i - 1].is_alphanumeric()) || (i == 0 && initial_ieung);
            if c.is_ascii_uppercase() && !KOREAN_SHIFTS.contains(c) && !(ieung_ok && IEUNG_SHIFTS.contains(c)) {
                return -99.0;
            }
        }
        let mut margin = self.key_margin(english);
        let lower = english.to_lowercase();
        if self.english_words.contains_key(&lower) {
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
        // A date, alone or with its particle (1日, 1日까지, 2026年부터).
        if let [unit @ ('년' | '월' | '일'), ref tail @ ..] = rest[..] {
            let tail: String = tail.iter().collect();
            if tail.is_empty() || PARTICLES.contains(&tail.as_str()) {
                out.push(format!("{digits}{}{tail}", hanja(unit)[0]));
            }
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

/// The prior against bare jamo, relative to the Korean reading's own: one step per jamo inside a
/// word (ㅗㅗ백년), one step for a word of jamo only (ㅓㅏㅂ), none for a run of one jamo, the
/// chat genre (ㅠㅠㅠ, ㅋㅋ). A property of the text, not of the keys.
fn jamo_prior(alternative: &str, korean: &str) -> f64 {
    let jamo = |s: &str| s.chars().filter(|&c| is_jamo(c)).count() as f64;
    let mut chars = alternative.chars();
    let first = chars.next();
    let own = if alternative.chars().all(is_jamo) {
        if chars.all(|c| Some(c) == first) { 0.0 } else { 1.0 }
    } else {
        jamo(alternative)
    };
    -BARE_JAMO * (own - jamo(korean))
}

/// Two identical syllables in a row that start on ㅇ and have no final (유유, 우우): the
/// syllable, if any. They may be bare vowels typed for chat (ㅠㅠ); see `chat_run`.
fn repeated_implied(korean: &str) -> Option<char> {
    let chars: Vec<char> = korean.chars().collect();
    chars
        .windows(2)
        .find(|w| {
            let n = w[0] as u32;
            w[0] == w[1] && (0xC544..=0xC7A3).contains(&n) && (n - 0xAC00).is_multiple_of(28)
        })
        .map(|w| w[0])
}

/// Whether the keys have a Shift+vowel after another key (a capital on a vowel key mid-word).
fn has_shift_vowel(english: &str) -> bool {
    let chars: Vec<char> = english.chars().collect();
    (1..chars.len()).any(|i| IEUNG_SHIFTS.contains(chars[i]) && chars[i - 1].is_alphanumeric())
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
