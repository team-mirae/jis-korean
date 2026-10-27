//! The learned ranker (spec §10.13): a linear model over features of each reading, trained on
//! text typed through the engine (`examples/learn_data.rs`, `tools/train_learned.py`), with weights
//! in `predict/learned.bin`.
//!
//! The readings are the rules' ([`Predictor::candidates`]), and so are the hard guarantees, applied
//! after scoring: a capital that settles English, command names and acronyms (what the rules rule
//! out stays below the rest), hanja and strongly avoided words never on top, the avoid list's
//! penalty, a scenario weighed 0 off, duplicates removed, at most [`MAX_PREDICTIONS`]. The model only
//! orders what's left. Scores are in bits, like the rules': the user's weights (§10.5) and what the
//! learner learned (§10.11) add log2(weight) to their scenario.
//!
//! Deterministic: features are hashed with a fixed hash into a fixed table, and nothing iterates a
//! hash map. Never panics on any input: a model is checked when it's parsed.

use std::collections::BTreeSet;

use crate::Dictionary;
use crate::predict::{
    self, AVOIDED, BadModel, Candidate, Context, Kind, Language, MAX_PREDICTIONS, Origin, Prediction, Predictor, Readings, Weight, Weights,
};

/// `learned.bin`'s format version.
pub const FORMAT: u32 = 2;
/// The feature set's version: a model trained on other features is refused.
pub const FEATURES: u32 = 1;
/// The hash table's size, in bits, that the training data and the shipped model use.
pub const DEFAULT_BITS: u32 = 20;
const HEADER: usize = 24;
/// A legacy score at or below this was settled by a rule (a capital at the start, a command, an
/// acronym): those readings stay below the others.
const RULED_OUT: f64 = -50.0;
/// The rules' scores count up to this many bits either way.
pub const LEGACY_CLIP: f64 = 20.0;

/// The learned weights: one per hashed feature.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    bits: u32,
    /// How far, in bits, the model may move a reading from the rules' score: its correction is
    /// `reach · tanh(w·x / reach)`. The rules' strong verdicts, the user's weights (§10.5), what the
    /// learner learned (§10.11) and the avoid list stay in force against a nudge.
    reach: f64,
    /// Each weight in bits is `weights[i] · scale` (kept as in the file: 2 bytes each).
    scale: f32,
    weights: Vec<i16>,
}

impl Model {
    /// `learned.bin`: `JKLR`, then little-endian u32 format version, u32 feature-set version, u32
    /// table bits, f32 scale, f32 reach (0.5–16 bits), then 2^bits i16 weights (each times the
    /// scale is the weight in bits).
    pub fn parse(bytes: &[u8]) -> Result<Model, BadModel> {
        let u32_at = |i: usize| bytes.get(i..i + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        if bytes.get(..4) != Some(b"JKLR".as_slice()) || u32_at(4) != Some(FORMAT) || u32_at(8) != Some(FEATURES) {
            return Err(BadModel);
        }
        let bits = u32_at(12).ok_or(BadModel)?;
        let scale = f32::from_bits(u32_at(16).ok_or(BadModel)?);
        let reach = f32::from_bits(u32_at(20).ok_or(BadModel)?) as f64;
        if !(8..=24).contains(&bits) || !scale.is_finite() || !(0.5..=16.0).contains(&reach) || bytes.len() != HEADER + 2 * (1usize << bits) {
            return Err(BadModel);
        }
        let weights = bytes[HEADER..].as_chunks::<2>().0.iter().map(|&b| i16::from_le_bytes(b)).collect();
        Ok(Model { bits, reach, scale, weights })
    }

    /// The bytes [`Model::parse`] reads, for weights in bits (tests and tools).
    pub fn encode(bits: u32, reach: f32, weights: &[f32]) -> Vec<u8> {
        let max = weights.iter().fold(0f32, |m, w| m.max(w.abs()));
        let scale = if max > 0.0 { max / i16::MAX as f32 } else { 1.0 };
        let mut out = Vec::with_capacity(HEADER + 2 * weights.len());
        out.extend_from_slice(b"JKLR");
        for v in [FORMAT, FEATURES, bits, scale.to_bits(), reach.to_bits()] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for w in weights {
            out.extend_from_slice(&((w / scale).round() as i16).to_le_bytes());
        }
        out
    }

    pub fn bits(&self) -> u32 {
        self.bits
    }

    /// What a bit of the user's weights (§10.5) and the learner's (§10.11) counts for here: twice
    /// the reach, at least 1. The model can move two readings apart by twice its reach; a weight
    /// at the learner's clamp (2×, one bit) can then always undo that nudge, as it could the
    /// rules' order.
    fn weight_gain(&self) -> f64 {
        (2.0 * self.reach).max(1.0)
    }

    /// The model's correction to the rules' score, in bits, within ±reach.
    fn correction(&self, features: &[(u32, f32)]) -> f64 {
        let sum: f64 = features.iter().map(|&(i, v)| self.weights.get(i as usize).map_or(0.0, |&w| (w as f32 * v) as f64)).sum::<f64>()
            * self.scale as f64;
        self.reach * (sum / self.reach).tanh()
    }
}

/// The table index of a feature named by `parts` (for the trainer: which features it leaves
/// unregularized).
pub fn index_of(parts: &[&str], bits: u32) -> u32 {
    let mut f = Features { bits, out: Vec::new() };
    f.add(parts, 1.0);
    f.out[0].0
}

/// One distinct reading with its features, for ranking or training.
#[derive(Clone, Debug, PartialEq)]
pub struct Featured {
    pub text: String,
    pub kind: Kind,
    pub origin: Origin,
    /// The rules' score (with neutral weights).
    pub legacy: f64,
    /// Where the rules would rank it, among the distinct readings (0 = top).
    pub legacy_rank: usize,
    /// (hashed feature, value).
    pub features: Vec<(u32, f32)>,
    /// Every copy of this text among the readings: kind, origin and the rules' score. The user's
    /// weights go to the copy they lift most, as the rules keep the highest copy.
    pub copies: Vec<(Kind, Origin, f64)>,
}

/// The readings ranked by the learned model, with the rules' guarantees on top.
pub fn rank(p: &Predictor, model: &Model, readings: &Readings, context: Context, dictionary: Option<&Dictionary>) -> Vec<Prediction> {
    let list = featurize(p, readings, context, dictionary, model.bits);
    let w = context.weights;
    // One vowel key alone is its syllable, never a letter (spec §10.3 rule 4): English readings stay
    // below, as for what a rule ruled out.
    let lone_vowel = readings.implied > 0 && readings.english.len() == 1 && readings.english.chars().all(|c| c.is_ascii_lowercase());
    let mut scored: Vec<(Prediction, bool, usize, Origin)> = Vec::with_capacity(list.len());
    let mut strong = BTreeSet::new();
    for f in list {
        // The copy the user's weights lift most, as the rules keep the highest copy: its kind is
        // the one shown, and the score moves by its weight bits and its rules' score below the
        // representative's (0 when there are no weights).
        let mut kind = f.kind;
        let mut lift = f64::NEG_INFINITY;
        for &(copy, origin, legacy) in &f.copies {
            let l = legacy - f.legacy + model.weight_gain() * scenario_bits(&w, copy, origin, readings, &f.text);
            if l > lift || lift == f64::NEG_INFINITY {
                (kind, lift) = (copy, l);
            }
        }
        if f.copies.is_empty() {
            lift = model.weight_gain() * scenario_bits(&w, f.kind, f.origin, readings, &f.text);
        }
        let bits = lift;
        let mut prediction = Prediction { text: f.text, kind, score: 0.0 };
        let mut score = base(f.legacy) + model.correction(&f.features) + bits;
        // The avoid list (spec §10.12): a rule, after scoring.
        if let Some(is_strong) = p.avoided(&prediction, &readings.english) {
            score -= AVOIDED;
            if is_strong {
                strong.insert(prediction.text.clone());
            }
        }
        if !score.is_finite() {
            continue; // a scenario weighed 0
        }
        prediction.score = score;
        let english = matches!(f.origin, Origin::English | Origin::Capitalized | Origin::Upper);
        scored.push((prediction, f.legacy <= RULED_OUT || (lone_vowel && english), f.legacy_rank, f.origin));
    }
    // What a rule ruled out stays below the rest; then the model's order, ties in the rules' order.
    scored.sort_by(|a, b| a.1.cmp(&b.1).then(b.0.score.total_cmp(&a.0.score)).then(a.2.cmp(&b.2)));
    // Hanja by rule (spec §10.3 rule 6, §10.4): the dictionary's form last, shown when there's
    // room; an everyday one right after the Korean reading.
    let (hanja, rest): (Vec<_>, Vec<_>) = scored.into_iter().partition(|s| s.3 == Origin::Hanja);
    let (everyday, rest): (Vec<_>, Vec<_>) = rest.into_iter().partition(|s| s.3 == Origin::EverydayHanja);
    let mut out: Vec<Prediction> = rest.into_iter().map(|s| s.0).collect();
    let at = out.iter().position(|p| p.text == readings.korean).map_or(out.len(), |i| i + 1);
    for (i, s) in everyday.into_iter().enumerate() {
        out.insert((at + i).min(out.len()), s.0);
    }
    out.extend(hanja.into_iter().map(|s| s.0));
    // Hanja and strongly avoided words are never on top: the first other reading moves up.
    let held = |p: &Prediction| p.kind == Kind::Hanja || strong.contains(&p.text);
    if out.first().is_some_and(held)
        && let Some(i) = out.iter().position(|p| !held(p))
    {
        let other = out.remove(i);
        out.insert(0, other);
    }
    out.truncate(MAX_PREDICTIONS);
    out
}

/// What the model's score starts from: the rules' score, within ±[`LEGACY_CLIP`] bits (the
/// model learns corrections to it).
pub fn base(legacy: f64) -> f64 {
    legacy.clamp(-LEGACY_CLIP, LEGACY_CLIP)
}

/// The user's weights (and the learned ones) in bits for a reading, as the rules add them: Korean
/// and English per language, and each alternative's own scenario.
fn scenario_bits(w: &Weights, kind: Kind, origin: Origin, readings: &Readings, text: &str) -> f64 {
    let bits = |which: Weight| w.get(which).log2();
    let korean = bits(Weight::Korean);
    match kind {
        Kind::English | Kind::EnglishCapitalized | Kind::EnglishUpper => bits(Weight::English),
        Kind::Korean => korean,
        Kind::TwoSet => korean + bits(Weight::TwoSet),
        Kind::KoreanKeepFinals => korean + bits(Weight::KeepFinals),
        Kind::ImpliedL => korean + bits(Weight::ImpliedL),
        Kind::Hanja => korean + bits(Weight::Hanja),
        Kind::BareVowels => {
            let two_set = if text == readings.two_set { bits(Weight::TwoSet) } else { 0.0 };
            korean - readings.implied as f64 * bits(Weight::Implied) + two_set
        }
        Kind::Mixed if origin == Origin::Pieces => 0.0,
        Kind::Mixed => bits(Weight::Mixed),
    }
}

/// The context with the user's weights left out (they're added after scoring), except that a
/// scenario weighed 0 stays off.
fn neutral(context: Context) -> Context {
    let mut weights = Weights::default();
    for which in Weight::ALL {
        if context.weights.get(which) == 0.0 {
            weights.set(which, 0.0);
        }
    }
    Context { weights, ..context }
}

/// The distinct readings of the staged text with their features: each text once, as the copy the
/// rules score highest, with where its other copies came from.
pub fn featurize(p: &Predictor, readings: &Readings, context: Context, dictionary: Option<&Dictionary>, bits: u32) -> Vec<Featured> {
    let context = neutral(context);
    let mut candidates: Vec<Candidate> =
        p.candidates(readings, context, dictionary).into_iter().filter(|c| !c.prediction.text.is_empty() && c.prediction.score.is_finite()).collect();
    // Stable: equal scores keep the rules' order of generation.
    candidates.sort_by(|a, b| b.prediction.score.total_cmp(&a.prediction.score));
    // Each text once: its highest copy, where its other copies came from, and every copy.
    type Group = (Candidate, BTreeSet<Origin>, Vec<(Kind, Origin, f64)>);
    let mut groups: Vec<Group> = Vec::new();
    for c in candidates {
        let copy = (c.prediction.kind, c.origin, c.prediction.score);
        match groups.iter_mut().find(|(g, _, _)| g.prediction.text == c.prediction.text) {
            Some((_, also, copies)) => {
                also.insert(c.origin);
                copies.push(copy);
            }
            None => groups.push((c, BTreeSet::new(), vec![copy])),
        }
    }
    let Some(top) = groups.first().map(|(c, _, _)| c.prediction.score) else { return Vec::new() };
    let keys = Keys::of(p, readings, context);
    groups
        .into_iter()
        .enumerate()
        .map(|(rank, (c, also, copies))| {
            let mut f = Features { bits, out: Vec::with_capacity(160) };
            features(&mut f, p, readings, &keys, &c, &also, rank, top);
            Featured {
                text: c.prediction.text,
                kind: c.prediction.kind,
                origin: c.origin,
                legacy: c.prediction.score,
                legacy_rank: rank,
                features: f.out,
                copies,
            }
        })
        .collect()
}

/// What's the same for every reading of the keys.
struct Keys {
    /// The keys, as typed.
    chars: Vec<char>,
    /// Their shape flags.
    shape: Vec<&'static str>,
    key_margin: f64,
    /// The Korean reading's lexicon count bucket, and the keys' English one.
    korean_lexicon: u32,
    english_lexicon: u32,
    english_noun: bool,
    previous: &'static str,
    /// Digits, then a Korean counter.
    counter: bool,
    kept: usize,
}

impl Keys {
    fn of(p: &Predictor, readings: &Readings, context: Context) -> Keys {
        let english = &readings.english;
        let chars: Vec<char> = english.chars().collect();
        let lower = english.to_lowercase();
        let upper = |c: &char| c.is_ascii_uppercase();
        let caps = chars.iter().filter(|c| upper(c)).count();
        let letters = chars.iter().filter(|c| c.is_ascii_alphabetic()).count();
        let mut shape = Vec::new();
        let first = chars.first().copied().unwrap_or(' ');
        if first.is_ascii_uppercase() {
            shape.push("initial capital");
            if predict::IEUNG_SHIFTS.contains(first) {
                shape.push("initial ieung capital");
            } else if predict::KOREAN_SHIFTS.contains(first) {
                shape.push("initial korean shift");
            } else {
                shape.push("initial english capital");
            }
        }
        if chars.len() >= 2 && chars[..2].iter().all(upper) {
            shape.push("acronym start");
        }
        if letters >= 2 && caps == letters {
            shape.push("all capitals");
        }
        if caps > 0 && chars.iter().filter(|c| upper(c)).all(|&c| predict::KOREAN_SHIFTS.contains(c) || predict::IEUNG_SHIFTS.contains(c)) {
            shape.push("capitals korean can shift");
        }
        if chars.iter().any(char::is_ascii_lowercase) {
            shape.push("lowercase");
        }
        if caps > 0 && chars.last().is_some_and(char::is_ascii_lowercase) {
            shape.push("capitals then lowercase end");
        }
        if chars.iter().any(char::is_ascii_digit) {
            shape.push("digit");
        }
        if first.is_ascii_digit() {
            shape.push("starts with digit");
        }
        if predict::has_shift_vowel(english) {
            shape.push("shift vowel inside");
        }
        if chars.iter().any(|c| !c.is_ascii_alphanumeric()) {
            shape.push("punctuation");
        }
        shape.push(match caps {
            0 => "capitals 0",
            1 => "capitals 1",
            2 => "capitals 2",
            3..=4 => "capitals 3-4",
            _ => "capitals 5+",
        });
        let rest = readings.korean.trim_start_matches(|c: char| c.is_ascii_digit() || ".,'~".contains(c));
        let counter = first.is_ascii_digit()
            && !rest.is_empty()
            && (predict::COUNTERS.contains(&rest) || predict::COUNTERS.iter().any(|c| rest.strip_prefix(c).is_some_and(|t| predict::PARTICLES.contains(&t))));
        Keys {
            chars,
            shape,
            key_margin: p.key_margin(english),
            korean_lexicon: count_bucket(p.korean_count(&readings.korean)),
            english_lexicon: count_bucket(p.english_count(&lower)),
            english_noun: p.is_noun(&lower),
            previous: match context.previous {
                Some(Language::Korean) => "korean",
                Some(Language::English) => "english",
                None => "none",
            },
            counter,
            kept: readings.kept.len(),
        }
    }
}

/// log2(count + 1), rounded: 0 for an unknown word, 1–21 for known ones.
fn count_bucket(count: Option<u32>) -> u32 {
    count.map_or(0, |n| 1 + ((n as f64 + 1.0).log2().round() as u32).min(20))
}

/// A coarse class of reading: what the keys' features cross with.
fn class(origin: Origin, kind: Kind) -> &'static str {
    match origin {
        Origin::English | Origin::Capitalized | Origin::Upper => "en",
        Origin::Mixed | Origin::Head => "mx",
        Origin::Hanja | Origin::EverydayHanja => "hj",
        Origin::Pieces => match kind {
            Kind::English => "en",
            Kind::Korean => "ko",
            _ => "pc",
        },
        _ => "ko",
    }
}

fn origin_name(origin: Origin) -> &'static str {
    match origin {
        Origin::Korean => "K",
        Origin::English => "E",
        Origin::Capitalized => "C",
        Origin::Upper => "U",
        Origin::TwoSet => "2",
        Origin::KeepFinals => "F",
        Origin::Kept => "k",
        Origin::ImpliedL => "L",
        Origin::Hanja => "H",
        Origin::EverydayHanja => "h",
        Origin::Bare => "B",
        Origin::Mixed => "M",
        Origin::Head => "A",
        Origin::Pieces => "P",
    }
}

/// A character's class in a reading's text.
fn char_class(c: char) -> char {
    match c {
        '\u{AC00}'..='\u{D7A3}' if (c as u32 - 0xAC00).is_multiple_of(28) => 'H',
        '\u{AC00}'..='\u{D7A3}' => 'F',
        '\u{3131}'..='\u{318E}' => 'J',
        'a'..='z' => 'a',
        'A'..='Z' => 'A',
        '0'..='9' => 'd',
        '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{F900}'..='\u{FAFF}' => 'X',
        _ => 'p',
    }
}

/// The hashed features of one reading.
struct Features {
    bits: u32,
    out: Vec<(u32, f32)>,
}

impl Features {
    /// A feature named by its parts (joined unambiguously), with a value.
    fn add(&mut self, parts: &[&str], value: f32) {
        let mut h = Fnv::new();
        for part in parts {
            h.write(part.as_bytes());
            h.write(&[0xFF]);
        }
        self.push(h.finish(), value);
    }

    /// A feature named by parts and characters (n-grams).
    fn add_chars(&mut self, parts: &[&str], chars: &[char], value: f32) {
        let mut h = Fnv::new();
        for part in parts {
            h.write(part.as_bytes());
            h.write(&[0xFF]);
        }
        let mut buf = [0u8; 4];
        for c in chars {
            h.write(c.encode_utf8(&mut buf).as_bytes());
        }
        self.push(h.finish(), value);
    }

    fn push(&mut self, hash: u64, value: f32) {
        if value != 0.0 && value.is_finite() {
            self.out.push(((mix(hash) >> (64 - self.bits)) as u32, value));
        }
    }
}

/// FNV-1a, 64 bits: fixed, so the same features hash the same everywhere.
struct Fnv(u64);

impl Fnv {
    fn new() -> Fnv {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

/// splitmix64's finalizer: spreads FNV's bits before the table index takes the top ones.
fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn clip(x: f64, limit: f64) -> f32 {
    x.clamp(-limit, limit) as f32
}

fn small(n: usize, max: usize) -> String {
    n.min(max).to_string()
}

#[allow(clippy::too_many_arguments)]
fn features(f: &mut Features, p: &Predictor, readings: &Readings, keys: &Keys, c: &Candidate, also: &BTreeSet<Origin>, rank: usize, top: f64) {
    let text = &c.prediction.text;
    let legacy = c.prediction.score;
    let o = origin_name(c.origin);
    let cls = class(c.origin, c.prediction.kind);
    let kind = format!("{:?}", c.prediction.kind);

    // Where it came from.
    f.add(&["origin", o], 1.0);
    f.add(&["kind", o, &kind], 1.0);
    for other in also {
        f.add(&["also", o, origin_name(*other)], 1.0);
    }

    // The rules' verdict: the score, the rank, the gap to the top, a settled rule.
    f.add(&["legacy"], clip(legacy, 20.0) / 4.0);
    f.add(&["legacy by origin", o], clip(legacy, 20.0) / 4.0);
    f.add(&["legacy rank", &small(rank, 5)], 1.0);
    f.add(&["legacy rank by origin", o, &small(rank, 3)], 1.0);
    f.add(&["legacy gap"], clip(top - legacy, 20.0) / 4.0);
    if legacy <= RULED_OUT {
        f.add(&["ruled out", o], 1.0);
    }
    if legacy >= -RULED_OUT {
        f.add(&["settled", o], 1.0);
    }

    // The key model: Korean minus English, per key.
    f.add(&["key margin", cls], clip(keys.key_margin, 8.0) / 2.0);
    f.add(&["key margin bucket", cls, &((keys.key_margin * 2.0).round().clamp(-12.0, 12.0) as i32).to_string()], 1.0);

    // The lexicons, for the keys' readings and for this text.
    f.add(&["korean reading lexicon", o, &keys.korean_lexicon.to_string()], 1.0);
    f.add(&["keys english lexicon", o, &keys.english_lexicon.to_string()], 1.0);
    if keys.english_noun {
        f.add(&["keys noun", o], 1.0);
    }
    match cls {
        "en" => {
            let lower = text.to_lowercase();
            f.add(&["english lexicon", o, &count_bucket(p.english_count(&lower)).to_string()], 1.0);
            if p.is_noun(&lower) {
                f.add(&["noun", o], 1.0);
            }
        }
        _ => f.add(&["korean lexicon", o, &count_bucket(p.korean_count(text)).to_string()], 1.0),
    }

    // The keys' capitals, digits and punctuation, and the context.
    for flag in &keys.shape {
        f.add(&["shape", flag, o], 1.0);
    }
    f.add(&["previous", keys.previous, o], 1.0);
    f.add(&["previous by class", keys.previous, cls], 1.0);
    if keys.counter {
        f.add(&["digits then counter", o], 1.0);
    }

    // Lengths, the implied ㅇ and kept finals, bare jamo.
    let chars: Vec<char> = text.chars().collect();
    f.add(&["keys length", o, &small(keys.chars.len(), 16)], 1.0);
    f.add(&["text length", o, &small(chars.len(), 12)], 1.0);
    f.add(&["implied", o, &small(readings.implied, 4)], 1.0);
    f.add(&["kept variants", o, &small(keys.kept, 3)], 1.0);
    f.add(&["implied l variants", o, &small(readings.implied_l.len(), 2)], 1.0);
    let jamo = chars.iter().filter(|&&c| predict::is_jamo(c)).count();
    f.add(&["jamo", o, &small(jamo, 4)], 1.0);
    if jamo > 0 && jamo == chars.len() {
        f.add(&["all jamo", o, if chars.iter().all(|&c| c == chars[0]) { "one" } else { "mixed" }], 1.0);
    }
    if c.origin != Origin::Korean && cls == "ko" {
        let korean: Vec<char> = readings.korean.chars().collect();
        let differ = chars.iter().zip(&korean).filter(|(a, b)| a != b).count() + chars.len().abs_diff(korean.len());
        f.add(&["syllables changed", o, &small(differ, 4)], 1.0);
    }

    // English + particle: the split's shape.
    if let Some(split) = &c.split {
        let start = &split.english;
        let ending = &split.korean;
        let lower = start.to_lowercase();
        f.add(&["split ending length", &small(ending.chars().count(), 6)], 1.0);
        f.add(&["split start length", &small(start.chars().count(), 12)], 1.0);
        f.add(&["split ending", ending], 1.0);
        f.add(&["split particle", &predict::PARTICLES.contains(&ending.as_str()).to_string()], 1.0);
        f.add(&["split verb ending", &p.is_ending(ending).to_string()], 1.0);
        f.add(&["split acronym tail", &p.acronym_tail(readings, split).to_string()], 1.0);
        f.add(&["split start lexicon", &count_bucket(p.english_count(&lower)).to_string()], 1.0);
        if p.is_noun(&lower) {
            f.add(&["split start noun"], 1.0);
        }
        f.add(&["split start margin"], clip(-p.start_margin(start), 10.0) / 2.0);
        f.add(&["split start margin over whole"], clip(p.start_margin(&readings.english) - p.start_margin(start), 10.0) / 2.0);
        let acronym = predict::is_acronym(start.trim_start_matches(|c: char| c.is_ascii_digit()));
        f.add(&["split start acronym", &acronym.to_string(), &start.chars().any(|c| c.is_ascii_lowercase()).to_string()], 1.0);
        f.add(&["split start capital", &start.starts_with(|c: char| c.is_ascii_uppercase()).to_string()], 1.0);
        f.add(&["split ending lexicon", &count_bucket(p.korean_count(ending)).to_string()], 1.0);
        if let Some(head) = readings.korean.strip_suffix(ending.as_str()) {
            f.add(&["split korean head lexicon", &count_bucket(p.korean_count(head)).to_string()], 1.0);
        }
        f.add(&["split ending key margin"], clip(p.key_margin(readings.english.get(start.len()..).unwrap_or("")), 8.0) / 2.0);
    }
    if c.origin == Origin::Head
        && let Some(head) = &readings.head
    {
        f.add(&["head lexicon", &count_bucket(p.korean_count(&head.korean)).to_string()], 1.0);
        f.add(&["head acronym length", &small(head.acronym.chars().count(), 5)], 1.0);
        f.add(&["head rest", if head.rest.is_empty() { "none" } else if p.is_ending(&head.rest) { "ending" } else { "word" }], 1.0);
        f.add(&["head key margin"], clip(p.key_margin(&head.keys), 8.0) / 2.0);
    }

    // The keys' n-grams (1–5), lowercase, crossed with the class; and their case shape.
    let mut marked: Vec<char> = Vec::with_capacity(keys.chars.len() + 2);
    marked.push('^');
    marked.extend(keys.chars.iter().map(|c| c.to_ascii_lowercase()));
    marked.push('$');
    for n in 1..=5 {
        for window in marked.windows(n) {
            f.add_chars(&["key ngram", cls], window, 1.0);
        }
    }
    let shape: Vec<char> = keys.chars.iter().map(|&c| char_class(c)).collect();
    for n in 2..=3 {
        for window in shape.windows(n) {
            f.add_chars(&["key shape", cls], window, 1.0);
        }
    }

    // The text's character classes (1–4), and its syllables (1–2) where it's Korean.
    let mut classes: Vec<char> = Vec::with_capacity(chars.len() + 2);
    classes.push('^');
    classes.extend(chars.iter().map(|&c| char_class(c)));
    classes.push('$');
    for n in 1..=4 {
        for window in classes.windows(n) {
            f.add_chars(&["text class", o], window, 1.0);
        }
    }
    if cls != "en" {
        let mut syllables: Vec<char> = Vec::with_capacity(chars.len() + 2);
        syllables.push('^');
        syllables.extend(chars.iter().filter(|&&c| !c.is_ascii()));
        syllables.push('$');
        for n in 1..=2 {
            for window in syllables.windows(n) {
                f.add_chars(&["syllables", cls], window, 1.0);
            }
        }
        if let Some(&last) = chars.last() {
            f.add_chars(&["last", o], &[last], 1.0);
        }
        if chars.len() >= 2 {
            f.add_chars(&["last two", o], &chars[chars.len() - 2..], 1.0);
        }
    } else {
        let lower: Vec<char> = text.to_lowercase().chars().collect();
        for n in 2..=3 {
            if lower.len() >= n {
                f.add_chars(&["english suffix", o], &lower[lower.len() - n..], 1.0);
            }
        }
    }
}
