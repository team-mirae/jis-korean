//! Typing a text the trusting way (spec §10.10): the fewest presses, letting predictions do the
//! rest. Each word is typed by its cheapest keys: Hangul by JIS Korean's optimized route, Latin
//! letter by letter (no F10), hanja by their reading (no F7), symbols by their key (pasted when
//! no JIS Korean legend has them). Space or Enter then commits the top prediction, as
//! the shells do. A word fails when what's committed isn't the word. Hanja are the exception: they
//! are never the top prediction (spec §10.3), so a word with hanja is picked with ↓ from the list,
//! each ↓ a press; it fails only when it isn't listed.
//!
//! The measure that matters is [`Trusting::effort`]: presses plus what fixing the failures costs,
//! against [`Trusting::deterministic`], the presses without predictions ([`plan::count`]).

use std::sync::Arc;

use crate::plan::{self, Metric, Press, Robot, Style, each_jamo, is_compat_jamo, is_hanja, is_syllable, keys_of, normalize};
use crate::learn::Learner;
use crate::predict::{Kind, Predictor, Ranker};
use crate::{Dictionary, Engine, Key, Mods};

/// A word whose committed text wasn't the word.
#[derive(Clone, Debug)]
pub struct Failure {
    pub want: String,
    pub got: String,
    /// The predictions shown when it was committed.
    pub list: Vec<String>,
    /// The keys pressed for it, spelled as Latin (a capital is Shift).
    pub keys: String,
}

/// What typing a text the trusting way took.
#[derive(Clone, Debug, Default)]
pub struct Trusting {
    pub words: usize,
    pub chars: usize,
    pub failures: Vec<Failure>,
    /// Characters wrong in what was committed (edit distance to the text, per word).
    pub char_errors: usize,
    /// Key presses (Shift counted, Space and Enter included).
    pub presses: usize,
    /// Presses to fix the failures: ↓ to the wanted prediction when it was listed, else Backspace
    /// over what was committed and the word again without predictions.
    pub corrections: usize,
    /// The same text without predictions ([`plan::count`]).
    pub deterministic: usize,
    /// Words with hanja picked from the list with ↓, and the ↓ presses (counted in `presses`; for
    /// an attentive typist, ↓ to any wanted word).
    pub hanja_picked: usize,
    pub arrows: usize,
    /// Words where a hanja reading was the top prediction (never wanted: hanja are picked).
    pub hanja_on_top: Vec<String>,
}

impl Trusting {
    /// Presses to the right text: what was typed plus the corrections.
    pub fn effort(&self) -> usize {
        self.presses + self.corrections
    }
}

/// Types `text` the trusting way with `predictor` ranked by `ranker`.
pub fn type_trusting(text: &str, predictor: Arc<Predictor>, dictionary: Option<Arc<Dictionary>>, ranker: Ranker) -> Trusting {
    type_text(text, predictor, dictionary, ranker, Style::Optimal, false, None).0
}

/// Types `text` like [`type_trusting`], with Hangul typed in `style` (the cheapest way, every jamo,
/// or a mix of the valid ways), and, when `attentive`, picking the wanted prediction with ↓
/// whenever it's listed (each ↓ a press, as for hanja). With `learner`, learning is on from that
/// state (spec §10.11), and what it learned comes back.
pub fn type_text(
    text: &str,
    predictor: Arc<Predictor>,
    dictionary: Option<Arc<Dictionary>>,
    ranker: Ranker,
    style: Style,
    attentive: bool,
    learner: Option<Learner>,
) -> (Trusting, Option<Learner>) {
    let (text, _) = normalize(text);
    let robot = Robot::new(dictionary.clone());
    let mut engine = Engine::new();
    if let Some(d) = &dictionary {
        engine.set_dictionary(d.clone());
    }
    engine.set_predictor(Some(predictor));
    engine.set_ranker(ranker);
    if let Some(learner) = learner.clone() {
        engine.set_learner(learner);
        engine.set_learning(true);
    }
    let mut report = Trusting { deterministic: plan::count(&text, dictionary.clone()).jis, ..Trusting::default() };
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            report.presses += 1; // Enter
        }
        for (j, word) in line.split(' ').enumerate() {
            if j > 0 {
                report.presses += 1; // Space
            }
            if word.is_empty() {
                continue;
            }
            // A mix differs word to word, the same on every run.
            let style = match style {
                Style::Mixed(seed) => Style::Mixed(seed ^ (report.words as u64 + 1).wrapping_mul(0x9E37_79B9)),
                other => other,
            };
            type_word(word, &robot, &mut engine, dictionary.clone(), style, attentive, &mut report);
        }
    }
    let learned = learner.map(|_| engine.learner().clone());
    (report, learned)
}

#[allow(clippy::too_many_arguments)]
fn type_word(
    word: &str,
    robot: &Robot,
    engine: &mut Engine,
    dictionary: Option<Arc<Dictionary>>,
    style: Style,
    attentive: bool,
    report: &mut Trusting,
) {
    let mut got = String::new();
    let mut keys = String::new();
    let press = |engine: &mut Engine, presses: &[Press], got: &mut String, keys: &mut String, report: &mut Trusting| {
        report.presses += presses.iter().map(|p| p.cost()).sum::<usize>();
        for p in presses {
            got.push_str(&engine.key(p.key, if p.shift { Mods::SHIFTED } else { Mods::NONE }).commit);
            keys.push_str(&spell(*p));
        }
    };
    let chars: Vec<char> = word.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // A run of one kind: Latin letters, syllables, or jamo.
        let kind = |c: char| if c.is_ascii_alphabetic() { 0 } else if is_syllable(c) { 1 } else if is_compat_jamo(c) { 2 } else { 3 };
        let end = if kind(c) == 3 { i + 1 } else { chars[i..].iter().position(|&d| kind(d) != kind(c)).map_or(chars.len(), |n| i + n) };
        let run: String = chars[i..end].iter().collect();
        let presses = match kind(c) {
            // Shift+Z is F10 (spec §6): a capital Z goes in as z, left to the predictions.
            0 => keys_of(&run).into_iter().map(|p| if p.key == Key::KeyZ { Press::new(Key::KeyZ) } else { p }).collect(),
            1 => hangul_presses(&run, style),
            2 => each_jamo(&run).unwrap_or_default(),
            // A hanja: by the reading that brings it into the list (年 after digits is 년),
            // else its best-ranked one.
            _ if is_hanja(c) => {
                let readings = robot.hanja_readings(c);
                let listed = |reading: &String| {
                    let mut trial = engine.clone();
                    for p in hangul_presses(reading, Style::Optimal) {
                        trial.key(p.key, if p.shift { Mods::SHIFTED } else { Mods::NONE });
                    }
                    trial.predictions().iter().any(|p| p.text.contains(c))
                };
                readings.iter().find(|r| listed(r)).or(readings.first()).map(|r| hangul_presses(r, Style::Optimal)).unwrap_or_default()
            }
            _ => match robot.symbol_presses(c) {
                Some(p) => p,
                None => {
                    // No key: pasted, which commits what's staged first (as the shells do).
                    got.push_str(&commit_top(engine).0);
                    got.push(c);
                    report.presses += plan::OFF_LEGEND;
                    keys.push('▢');
                    i = end;
                    continue;
                }
            },
        };
        press(engine, &presses, &mut got, &mut keys, report);
        i = end;
    }
    let list: Vec<(String, Kind)> = engine.predictions().into_iter().map(|p| (p.text, p.kind)).collect();
    if list.first().is_some_and(|(_, kind)| *kind == Kind::Hanja) {
        report.hanja_on_top.push(word.to_string());
    }
    let before = got.clone();
    // A word with hanja (or any word, for an attentive typist): picked from the list with ↓ when
    // it's there.
    let pick = match list.iter().position(|(t, _)| format!("{before}{t}") == word) {
        Some(index) if attentive || word.chars().any(is_hanja) => {
            report.hanja_picked += word.chars().any(is_hanja) as usize;
            report.arrows += index;
            report.presses += index;
            index
        }
        _ => 0,
    };
    let commit = if list.is_empty() { engine.flush().commit } else { engine.commit_prediction(pick).commit };
    let list: Vec<String> = list.into_iter().map(|(t, _)| t).collect();
    got.push_str(&commit);
    report.words += 1;
    report.chars += word.chars().count();
    if got != word {
        report.char_errors += edit_distance(&got, word);
        report.corrections += match list.iter().position(|t| format!("{before}{t}") == word) {
            Some(index) => index,
            None => got.chars().count() + plan::count(word, dictionary).jis,
        };
        report.failures.push(Failure { want: word.to_string(), got, list, keys });
    }
}

/// The keys for a run of syllables on its own, in `style` (the cheapest: implied ㅇ, dedicated
/// keys, double press).
fn hangul_presses(run: &str, style: Style) -> Vec<Press> {
    let mut robot = Robot::new(None);
    robot.metric = Metric::Presses;
    robot.style = style;
    robot.type_text(run);
    robot.log.clone()
}

/// Commits the top prediction (or the staged text as is, without predictions). Returns the commit
/// and the predictions that were shown.
fn commit_top(engine: &mut Engine) -> (String, Vec<String>) {
    let list: Vec<String> = engine.predictions().into_iter().map(|p| p.text).collect();
    let out = if list.is_empty() { engine.flush() } else { engine.commit_prediction(0) };
    (out.commit, list)
}

fn spell(p: Press) -> String {
    let code = p.key.code();
    let s = code.strip_prefix("Key").or_else(|| code.strip_prefix("Digit")).unwrap_or(code).to_string();
    if p.shift { s.to_uppercase() } else if p.key.letter().is_some() { s.to_lowercase() } else { s }
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let up = row[j + 1];
            row[j + 1] = (up + 1).min(row[j] + 1).min(diagonal + (ca != cb) as usize);
            diagonal = up;
        }
    }
    row[b.len()]
}
