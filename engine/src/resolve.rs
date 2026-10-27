//! Resolving keys and text (spec §8.3): what a key types, and which key presses type a text. For
//! programs that drive or check JIS Korean from outside, such as the macOS public API.
//!
//! Everything runs on a copy of a live engine, so the answers match it exactly: its mode, and with
//! predictions on, its predictor, weights and what it has learned. The copy starts with nothing
//! staged and records nothing it learns. Keys apply as a shell applies them (spec §10): with
//! predictions on, the marked text shows the top prediction unless that is the Korean reading,
//! and Space and Enter commit what is shown; Space then adds a space, and Enter, passed on, a new
//! line.
//!
//! A key press is written as its canonical `KeyboardEvent.code`, with `S-` for Shift held:
//! `KeyG`, `S-Digit2`. One press is one key, with or without Shift.

use std::collections::HashMap;

use crate::plan::{Press, is_compat_jamo, is_syllable};
use crate::{Engine, Key, Layout, Mods, output_with};

/// The most presses in a combination: a longer range is cut to it.
pub const MAX_PRESSES: i64 = 16;
/// The most combinations returned, fewest presses first.
pub const MAX_COMBINATIONS: usize = 1000;
/// Engine states tried per search, so a search always ends quickly.
const BUDGET: usize = 50_000;

/// Reads a key press: `KeyG`, `S-KeyG` or `Shift+KeyG`. None for anything else, including
/// modifiers alone and keys outside the engine (`Other`).
pub fn parse_press(text: &str) -> Option<Press> {
    let text = text.trim();
    let (shift, code) = match text.strip_prefix("S-").or_else(|| text.strip_prefix("Shift+")) {
        Some(code) => (true, code),
        None => (false, text),
    };
    let key = Key::from_code(code);
    (key != Key::Other && key != Key::Modifier).then_some(Press { key, shift })
}

/// What the key types on its own, in the engine's mode: a jamo or a character. None when it types
/// nothing by itself (F-keys, Space, Shift+Z, which is F10), or when the press isn't a key.
pub fn key_input(template: &Engine, press: &str) -> Option<String> {
    let p = parse_press(press)?;
    if p.key == Key::KeyZ && p.shift {
        return None;
    }
    output_with(template.mode(), template.dedicated_symbols(), Layout::JisKorean, p.key, p.shift).map(|out| out.char().to_string())
}

/// What the field shows after `context` (text typed so far, still unconfirmed) and then the key:
/// the context with the key's effect, including anything the key commits. None when the press
/// isn't a key or no keys type the context.
pub fn key_input_after(template: &Engine, context: &str, press: &str) -> Option<String> {
    let p = parse_press(press)?;
    let mut field = Field::new(template);
    if !context.is_empty() {
        for press in route_to(template, context)? {
            field.press(press);
        }
    }
    field.press(p);
    Some(field.screen())
}

/// What the field shows after typing `presses` from an empty field. None if one isn't a key.
pub fn type_keys(template: &Engine, presses: &[impl AsRef<str>]) -> Option<String> {
    let mut field = Field::new(template);
    for press in presses {
        field.press(parse_press(press.as_ref())?);
    }
    Some(field.screen())
}

/// Every combination of key presses that types `text` from an empty field, with between `min` and
/// `max` presses (inclusive): fewest presses first, then fewest Shifts, at most
/// [`MAX_COMBINATIONS`]. Empty when `min` is below 1, `max` is below `min`, or nothing fits. A `max`
/// above [`MAX_PRESSES`] is cut to it.
///
/// The search follows key presses whose text heads toward `text` (its Hangul jamo, its letters as
/// typed or converted with F9/F10, or a hanja's reading before F7), and keeps a combination only
/// when typing it really shows `text`.
pub fn keys_for_output(template: &Engine, text: &str, min: i64, max: i64) -> Vec<Vec<String>> {
    if min < 1 || max < min || text.is_empty() {
        return Vec::new();
    }
    let (min, max) = (min as usize, max.min(MAX_PRESSES) as usize);
    let targets = targets(template, text);
    let alphabet = alphabet(template, text, &targets);
    let mut found: Vec<Vec<Press>> = Vec::new();
    let mut frontier = vec![(Field::new(template), Vec::<Press>::new())];
    let mut budget = BUDGET;
    for depth in 1..=max {
        let mut next = Vec::new();
        'expand: for (field, presses) in &frontier {
            for &p in &alphabet {
                if !field.can_press(p) {
                    continue;
                }
                if budget == 0 {
                    break 'expand;
                }
                budget -= 1;
                let mut child = field.clone();
                child.press(p);
                if child.same_as(field) {
                    continue;
                }
                let mut route = presses.clone();
                route.push(p);
                if depth >= min && child.screen() == text {
                    found.push(route.clone());
                }
                if depth < max && child.heads_toward(&targets) {
                    next.push((child, route));
                }
            }
        }
        frontier = next;
        if frontier.is_empty() || found.len() >= MAX_COMBINATIONS * 4 {
            break;
        }
    }
    found.sort_by_key(|route| (route.len(), route.iter().filter(|p| p.shift).count()));
    found.truncate(MAX_COMBINATIONS);
    found.into_iter().map(|route| route.iter().map(Press::to_string).collect()).collect()
}

/// The first key route found that types `text`, preferring plain keys: how the context of
/// [`key_input_after`] is staged. Keys whose own Korean reading is `text` come first, as a person
/// types it; only if there are none does a match through the shown prediction count.
fn route_to(template: &Engine, text: &str) -> Option<Vec<Press>> {
    let targets = targets(template, text);
    let alphabet = alphabet(template, text, &targets);
    let limit = 4 * text.chars().count() + 8;
    let as_typed = |f: &Field| f.committed.clone() + &f.engine.preedit() == text;
    let as_shown = |f: &Field| f.screen() == text;
    for done in [&as_typed as &dyn Fn(&Field) -> bool, &as_shown] {
        let mut budget = BUDGET;
        let mut route = Vec::new();
        if search(&Field::new(template), done, &targets, &alphabet, &mut route, limit, &mut budget) {
            return Some(route);
        }
    }
    None
}

/// Depth-first, in the alphabet's order, along presses heading toward a target.
fn search(
    field: &Field,
    done: &dyn Fn(&Field) -> bool,
    targets: &[Vec<char>],
    alphabet: &[Press],
    route: &mut Vec<Press>,
    limit: usize,
    budget: &mut usize,
) -> bool {
    if done(field) {
        return true;
    }
    if route.len() >= limit {
        return false;
    }
    for &p in alphabet {
        if *budget == 0 {
            return false;
        }
        if !field.can_press(p) {
            continue;
        }
        *budget -= 1;
        let mut child = field.clone();
        child.press(p);
        if child.same_as(field) {
            continue;
        }
        if done(&child) || child.heads_toward(targets) {
            route.push(p);
            if search(&child, done, targets, alphabet, route, limit, budget) {
                return true;
            }
            route.pop();
        }
    }
    false
}

/// The presses a search for `text` tries, plain before Shift: the keys whose character (in the
/// mode's layout) or letter is part of `text`, F6, and only where `text` needs them F7 (hanja), F9 (full-width), F10 (letters), Space
/// and Enter. Backspace, Esc, the arrows, Tab and Shift+Enter are left out: they never shorten a way
/// to type something.
fn alphabet(template: &Engine, text: &str, targets: &[Vec<char>]) -> Vec<Press> {
    let (mode, symbols) = (template.mode(), template.dedicated_symbols());
    let wanted: std::collections::HashSet<char> = targets.iter().flatten().copied().collect();
    let part_of_text = |c: char| stream(&c.to_string()).iter().all(|c| wanted.contains(c));
    let useful = |p: Press| {
        output_with(mode, symbols, Layout::JisKorean, p.key, p.shift).is_some_and(|out| part_of_text(out.char()))
            || p.key.letter().or(p.key.digit()).is_some_and(|c| wanted.contains(&c))
    };
    let mut presses = Vec::new();
    for shift in [false, true] {
        presses.extend(Key::ALL.iter().map(|&key| Press { key, shift }).filter(|&p| {
            output_with(mode, symbols, Layout::JisKorean, p.key, false).is_some() && useful(p)
        }));
    }
    let has = |f: &dyn Fn(char) -> bool| text.chars().any(f);
    let mut keys = vec![Key::F6];
    if has(&is_hanja) {
        keys.push(Key::F7);
    }
    if has(&|c| crate::half_width(c) != c) {
        keys.push(Key::F9);
    }
    if has(&|c| c.is_ascii_alphanumeric()) {
        keys.push(Key::F10);
    }
    if has(&|c| c == ' ') {
        keys.push(Key::Space);
    }
    if has(&|c| c == '\n') {
        keys.push(Key::Enter);
    }
    presses.extend(keys.into_iter().map(|key| Press { key, shift: false }));
    presses
}

/// A text field with JIS Korean typing into it: committed text, and the engine's staged text.
#[derive(Clone)]
struct Field {
    engine: Engine,
    committed: String,
}

impl Field {
    fn new(template: &Engine) -> Field {
        let mut engine = template.clone();
        engine.clear();
        engine.activate();
        #[cfg(feature = "predict")]
        engine.set_recording(false);
        Field { engine, committed: String::new() }
    }

    /// A key that types nothing in the current layout would pass through to the app, whose text
    /// we can't know: those aren't pressed.
    fn can_press(&self, p: Press) -> bool {
        matches!(p.key, Key::F6 | Key::F7 | Key::F9 | Key::F10 | Key::Space | Key::Enter)
            || output_with(self.engine.mode(), self.engine.dedicated_symbols(), Layout::JisKorean, p.key, p.shift).is_some()
    }

    /// The top prediction, when the marked text shows it instead of the Korean reading.
    fn shown_prediction(&self) -> Option<String> {
        #[cfg(feature = "predict")]
        {
            if self.engine.preedit().is_empty() || self.engine.hanja_open() {
                return None;
            }
            let top = self.engine.predictions().into_iter().next()?;
            (top.kind != crate::predict::Kind::Korean).then_some(top.text)
        }
        #[cfg(not(feature = "predict"))]
        None
    }

    fn press(&mut self, p: Press) {
        if matches!(p.key, Key::Space | Key::Enter) && self.shown_prediction().is_some() {
            #[cfg(feature = "predict")]
            {
                self.committed += &self.engine.commit_prediction(0).commit;
                if p.key == Key::Space {
                    self.committed.push(' ');
                    return;
                }
            }
        }
        let out = self.engine.key(p.key, if p.shift { Mods::SHIFTED } else { Mods::NONE });
        self.committed += &out.commit;
        if !out.consumed && p.key == Key::Enter {
            self.committed.push('\n');
        }
    }

    /// Whether a press changed nothing (F6 with nothing staged, F9 with no word, …): such presses
    /// aren't a way to type anything, so searches skip them.
    fn same_as(&self, before: &Field) -> bool {
        self.committed == before.committed
            && self.engine.preedit() == before.engine.preedit()
            && self.engine.english() == before.engine.english()
            && self.engine.hanja_open() == before.engine.hanja_open()
            && !self.engine.hanja_open()
    }

    /// What the field shows: the committed text, then the marked text.
    fn screen(&self) -> String {
        let marked = self.shown_prediction().unwrap_or_else(|| self.engine.preedit());
        self.committed.clone() + &marked
    }

    /// Whether the field is on the way to one of the targets: its Korean, or its keys read as
    /// letters (before F9/F10), lead into a target, or a hanja list is open.
    fn heads_toward(&self, targets: &[Vec<char>]) -> bool {
        if self.engine.hanja_open() {
            return true;
        }
        let korean = stream(&(self.committed.clone() + &self.engine.preedit()));
        let english = stream(&(self.committed.clone() + &self.engine.english()));
        // The implied ㅇ (spec §4.2) shows before F6 can drop it (ㅠ shows as 유 on the way to ㅠ).
        let bare = stream(&(self.committed.clone() + &self.engine.preedit_without_implied()));
        targets.iter().any(|t| t.starts_with(&korean) || t.starts_with(&english) || t.starts_with(&bare))
    }
}

/// The text a search aims for, as jamo streams: the text itself, and with its hanja replaced by
/// their readings (typed before F7). At most 8.
fn targets(template: &Engine, text: &str) -> Vec<Vec<char>> {
    let mut variants = vec![String::new()];
    // Building the reverse index reads the whole dictionary: only for text with hanja.
    let readings = if text.chars().any(is_hanja) { hanja_readings(template) } else { HashMap::new() };
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if is_hanja(chars[i]) {
            let end = (i..chars.len()).find(|&j| !is_hanja(chars[j])).unwrap_or(chars.len());
            let run: String = chars[i..end].iter().collect();
            let ways: Vec<String> = match readings.get(&run) {
                Some(word) => word.clone(),
                None => chars[i..end]
                    .iter()
                    .fold(vec![String::new()], |acc, c| {
                        let single = readings.get(&c.to_string()).cloned().unwrap_or_default();
                        acc.iter().flat_map(|a| single.iter().map(move |r| a.clone() + r)).take(8).collect()
                    }),
            };
            let ways = if ways.is_empty() { vec![run] } else { ways };
            variants = variants.iter().flat_map(|v| ways.iter().map(move |w| v.clone() + w)).take(8).collect();
            i = end;
        } else {
            variants.iter_mut().for_each(|v| v.push(chars[i]));
            i += 1;
        }
    }
    variants.iter().map(|v| stream(v)).collect()
}

/// hanja word or character → its readings, best-ranked first.
fn hanja_readings(template: &Engine) -> HashMap<String, Vec<String>> {
    let mut found: HashMap<String, Vec<(usize, String)>> = HashMap::new();
    if let Some(dictionary) = template.dictionary() {
        for (reading, candidates) in dictionary.entries() {
            for (position, candidate) in candidates.iter().enumerate() {
                found.entry(candidate.hanja.clone()).or_default().push((position, reading.to_string()));
            }
        }
    }
    found
        .into_iter()
        .map(|(hanja, mut readings)| {
            readings.sort();
            (hanja, readings.into_iter().map(|(_, r)| r).collect())
        })
        .collect()
}

fn is_hanja(c: char) -> bool {
    matches!(c, '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}')
}

/// Text as a stream for prefix checks: Hangul split into jamo, with double and compound jamo split
/// into their parts; letters lowercased and full-width forms made half-width.
fn stream(text: &str) -> Vec<char> {
    const L: &str = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ";
    const V: &str = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ";
    const T: &str = " ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ";
    let mut out = Vec::new();
    let push_jamo = |out: &mut Vec<char>, j: char| match split(j) {
        Some((a, b)) => out.extend([a, b]),
        None => out.push(j),
    };
    for c in text.chars() {
        if is_syllable(c) {
            let s = c as u32 - 0xAC00;
            push_jamo(&mut out, L.chars().nth((s / 588) as usize).unwrap_or(c));
            push_jamo(&mut out, V.chars().nth((s % 588 / 28) as usize).unwrap_or(c));
            if s % 28 != 0 {
                push_jamo(&mut out, T.chars().nth((s % 28) as usize).unwrap_or(c));
            }
        } else if is_compat_jamo(c) {
            push_jamo(&mut out, c);
        } else {
            let c = match c {
                '\u{FF01}'..='\u{FF5E}' => char::from_u32(c as u32 - 0xFEE0).unwrap_or(c),
                '\u{3000}' => ' ',
                _ => c,
            };
            out.extend(c.to_lowercase());
        }
    }
    out
}

/// A double or compound jamo's parts.
fn split(j: char) -> Option<(char, char)> {
    Some(match j {
        'ㄲ' => ('ㄱ', 'ㄱ'),
        'ㄸ' => ('ㄷ', 'ㄷ'),
        'ㅃ' => ('ㅂ', 'ㅂ'),
        'ㅆ' => ('ㅅ', 'ㅅ'),
        'ㅉ' => ('ㅈ', 'ㅈ'),
        'ㄳ' => ('ㄱ', 'ㅅ'),
        'ㄵ' => ('ㄴ', 'ㅈ'),
        'ㄶ' => ('ㄴ', 'ㅎ'),
        'ㄺ' => ('ㄹ', 'ㄱ'),
        'ㄻ' => ('ㄹ', 'ㅁ'),
        'ㄼ' => ('ㄹ', 'ㅂ'),
        'ㄽ' => ('ㄹ', 'ㅅ'),
        'ㄾ' => ('ㄹ', 'ㅌ'),
        'ㄿ' => ('ㄹ', 'ㅍ'),
        'ㅀ' => ('ㄹ', 'ㅎ'),
        'ㅄ' => ('ㅂ', 'ㅅ'),
        'ㅘ' => ('ㅗ', 'ㅏ'),
        'ㅙ' => ('ㅗ', 'ㅐ'),
        'ㅚ' => ('ㅗ', 'ㅣ'),
        'ㅝ' => ('ㅜ', 'ㅓ'),
        'ㅞ' => ('ㅜ', 'ㅔ'),
        'ㅟ' => ('ㅜ', 'ㅣ'),
        'ㅢ' => ('ㅡ', 'ㅣ'),
        _ => return None,
    })
}
