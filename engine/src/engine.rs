//! The unconfirmed segment, commit rules and F6–F10 (spec §5, §6 and §7).

use std::sync::Arc;

use crate::hangul::{self, Composer, Options};
#[cfg(feature = "predict")]
use crate::learn::{self, Event, Learner};
#[cfg(feature = "predict")]
use crate::predict::{Context, Language, Piece, Prediction, Predictor, Ranker, Readings, Split, Weight, Weights};
use crate::hanja::{Candidate, Dictionary};
use crate::key::{Key, Mods};
use crate::layout::{self, Layout, Mode, Out};

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
    /// Shift+vowel (spec §4.2): the vowel starts its own syllable on ㅇ, leaving the previous
    /// final where it is.
    IeungVowel { jamo: char, raw: Option<char> },
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
    /// The segment as typed, when opening the list turned it into plain characters (a reading
    /// not on a keystroke boundary): closing the list without a pick puts it back.
    typed: Option<Vec<Token>>,
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
    /// How many characters just before the insertion point the preedit takes back: they're removed
    /// from the document and shown as the preedit instead. 1 when JIS English opens a press-and-hold
    /// list on the letter already typed (spec §3.6.3); otherwise 0.
    pub take_back: usize,
}

#[derive(Clone)]
pub struct Engine {
    segment: Vec<Token>,
    conversion: Option<Conversion>,
    hanja: Option<HanjaList>,
    layout: Layout,
    /// The mode: which keyboard's key map (spec §3.6).
    mode: Mode,
    /// Setting: Dedicated symbol keys (spec §3.6), for modes with optional symbol keys.
    symbols: bool,
    enter_commits: bool,
    dictionary: Option<Arc<Dictionary>>,
    /// JIS English's press-and-hold list, while one is open (spec §3.6.3).
    #[cfg(feature = "english")]
    held: Option<crate::hold::Held>,
    /// The letter key JIS English typed last (key, Shift), whose auto-repeat opens the list.
    #[cfg(feature = "english")]
    last_letter: Option<(Key, bool)>,
    /// The predictive layer; `None` is the deterministic lite engine.
    #[cfg(feature = "predict")]
    predictor: Option<Arc<Predictor>>,
    /// The language of the last committed word, for the predictor.
    #[cfg(feature = "predict")]
    previous: Option<Language>,
    /// The user's tuning of the predictions.
    #[cfg(feature = "predict")]
    weights: Weights,
    /// How the implied-ㅇ alternatives are ranked (spec §10.7).
    #[cfg(feature = "predict")]
    ranker: Ranker,
    /// Learning from the user's choices (spec §10.11): off unless the host turns it on.
    #[cfg(feature = "predict")]
    learner: Learner,
    #[cfg(feature = "predict")]
    learning: bool,
    /// Whether choices are recorded while learning is on: off for apps the user excluded (what was
    /// learned still applies there).
    #[cfg(feature = "predict")]
    recording: bool,
    /// Scenarios the user set by hand: their weight wins over the learned one.
    #[cfg(feature = "predict")]
    manual: [bool; Weight::ALL.len()],
    /// The scenario of a top prediction just accepted, for a Backspace right after (regret).
    #[cfg(feature = "predict")]
    accepted: Option<Weight>,
    /// The learner changed since the host last exported it.
    #[cfg(feature = "predict")]
    learned_changed: bool,
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
            mode: Mode::A,
            symbols: true,
            enter_commits: false,
            dictionary: None,
            #[cfg(feature = "english")]
            held: None,
            #[cfg(feature = "english")]
            last_letter: None,
            #[cfg(feature = "predict")]
            predictor: None,
            #[cfg(feature = "predict")]
            previous: None,
            #[cfg(feature = "predict")]
            weights: Weights::default(),
            #[cfg(feature = "predict")]
            ranker: Ranker::default(),
            #[cfg(feature = "predict")]
            learner: Learner::default(),
            #[cfg(feature = "predict")]
            learning: false,
            #[cfg(feature = "predict")]
            recording: true,
            #[cfg(feature = "predict")]
            manual: [false; Weight::ALL.len()],
            #[cfg(feature = "predict")]
            accepted: None,
            #[cfg(feature = "predict")]
            learned_changed: false,
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

    /// The hanja dictionary, if one was set.
    pub fn dictionary(&self) -> Option<Arc<Dictionary>> {
        self.dictionary.clone()
    }

    /// Setting: the mode, Mode A (JIS), Mode B (US ANSI) or Mode C (UK ISO) (spec §3.6). Takes
    /// effect from the next key.
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// Setting: Dedicated symbol keys (spec §3.6), on by default. Off, the mode's optional symbol
    /// keys ([`Mode::optional_keys`], Mode C's [ ] and the ISO extra key) type the mode's stock
    /// characters (UK ISO in Mode C). Modes without optional keys ignore it.
    pub fn set_dedicated_symbols(&mut self, on: bool) {
        self.symbols = on;
    }

    /// Whether the mode's optional symbol keys are on (spec §3.6).
    pub fn dedicated_symbols(&self) -> bool {
        self.symbols
    }

    /// The mode (spec §3.6).
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Whether a hanja candidate list is open (F7).
    pub fn hanja_open(&self) -> bool {
        self.hanja.is_some()
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
        // Not the user's choice: nothing is learned from it (spec §10.11).
        self.commit_quietly(String::new(), true)
    }

    /// Discards everything unconfirmed, for hosts that commit a reading of their own (the
    /// Minecraft mod picks between [`Engine::preedit`] and [`Engine::english`]).
    pub fn clear(&mut self) {
        self.segment.clear();
        self.conversion = None;
        self.hanja = None;
    }

    /// The unconfirmed keystrokes read as English: each key as it types on the stock keyboard,
    /// JIS or US ANSI (letters with their case, digits, the keyboard's symbols). Converted and hanja text stays as shown.
    pub fn english(&self) -> String {
        english_of(&self.segment)
    }

    /// The unconfirmed text without the implied ㅇ (spec §4.2): what F6 would leave of plain
    /// implied syllables (ㅠ for 유). For resolving keys (crate::resolve).
    pub fn preedit_without_implied(&self) -> String {
        compose_with(&self.segment, Options { no_implied: true, ..Options::default() })
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
        // JIS English (spec §3.6.2): nothing Korean applies; keys go to the app.
        #[cfg(feature = "english")]
        if !self.mode.is_korean() {
            return self.english_key(key, mods);
        }
        // A press-and-hold letter still open from JIS English (the mode changed under it): it
        // stays where it was, as plain text before what's typed now.
        #[cfg(feature = "english")]
        if let Some(held) = self.held.take() {
            self.last_letter = None;
            self.segment.insert(0, Token::Text(held.shown()));
        }
        // Backspace right after accepting a top prediction: a weak sign it was wrong.
        #[cfg(feature = "predict")]
        if let Some(kind) = self.accepted.take()
            && key == Key::Backspace
            && self.segment.is_empty()
        {
            self.learn(&Event::Regret(kind));
        }
        // Shift+Z is F10 (spec §6): F10 isn't reachable everywhere (Windows browsers take it for
        // their menu), and Shift adds nothing to ㅋ. Standard 2-set (the F8 escape) keeps it as ㅋ.
        if key == Key::KeyZ && mods.shift && !mods.is_shortcut() && self.layout == Layout::JisKorean {
            // With predictions on, when the top prediction is already exactly what F10 would give,
            // Shift+Z skips ahead: F10 twice (gks → GKS). F10 itself is unchanged.
            #[cfg(feature = "predict")]
            if self.conversion.is_none() && self.top_prediction_is_f10() {
                self.key(Key::F10, Mods::NONE);
            }
            return self.key(Key::F10, Mods::NONE);
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
                // Korean, forced: the context, but no choice among predictions.
                #[cfg(feature = "predict")]
                if !self.segment.is_empty() {
                    self.previous = Some(Language::Korean);
                }
                self.commit_quietly(String::new(), true)
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
            _ => match layout::output_with(self.mode, self.symbols, self.layout, key, mods.shift) {
                Some(Out::Jamo(jamo)) => {
                    let raw = key.letter().map(|c| if mods.shift { c.to_ascii_uppercase() } else { c });
                    let raw = raw.or(key.digit());
                    // Shift on a vowel key whose Shift types nothing else (all but O P): the vowel
                    // starts its own syllable on ㅇ (spec §4.2).
                    let ieung = mods.shift
                        && self.layout == Layout::JisKorean
                        && hangul::is_medial(jamo)
                        && layout::output_with(self.mode, self.symbols, self.layout, key, false) == Some(Out::Jamo(jamo));
                    self.segment.push(if ieung { Token::IeungVowel { jamo, raw } } else { Token::Jamo { jamo, raw } });
                    self.unchanged(true)
                }
                Some(Out::Char(c)) => {
                    // The English reading is the stock keyboard's character, so predictions never
                    // imply JIS on US ANSI (Shift+2 reads @ there, " on JIS).
                    let english = layout::output_with(self.mode, self.symbols, Layout::Escaped, key, mods.shift).map_or(c, Out::char);
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
                Key::Escape | Key::Backspace => self.close_hanja(),
                Key::F6 => {
                    // Back to Hangul (spec §6): F6 commits the reading as typed.
                    self.close_hanja();
                    return Some(self.commit_quietly(String::new(), true));
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
        let mut typed = None;
        let start = (0..self.segment.len())
            .rev()
            .find(|&k| compose(&self.segment[k..]) == reading && compose(&self.segment[..k]) == prefix)
            .unwrap_or_else(|| {
                let text = prefix.chars().map(Token::Text).chain(reading.chars().map(Token::Text)).collect();
                typed = Some(std::mem::replace(&mut self.segment, text));
                prefix.chars().count()
            });
        self.hanja = Some(HanjaList { start, typed, candidates: candidates.to_vec(), selected: 0 });
    }

    /// Closes the hanja list without a pick: the segment is as it was typed, still editable
    /// (spec §7.2).
    fn close_hanja(&mut self) {
        if let Some(typed) = self.hanja.take().and_then(|list| list.typed) {
            self.segment = typed;
        }
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
                // A symbol run (spec §5.3) is typed symbols and digits, and symbols an earlier
                // conversion left; letters and digits it left belong to a word, which never mixes in.
                let in_run = |t: &Token| match *t {
                    Token::Typed(c, _) => c.is_ascii_graphic(),
                    Token::Text(c) => c.is_ascii_punctuation(),
                    _ => false,
                };
                let symbols = self.segment.last().is_some_and(in_run);
                let run = self
                    .segment
                    .iter()
                    .rev()
                    .take_while(|t| match t {
                        Token::Text(_) | Token::Typed(..) => symbols && in_run(t),
                        Token::Jamo { raw, .. } | Token::IeungVowel { raw, .. } => !symbols && raw.is_some(),
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
                Token::Jamo { raw, .. } | Token::IeungVowel { raw, .. } => *raw,
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

    /// Turns the predictive layer on (`Some`) or off (`None`, the lite engine). Engines can share one.
    #[cfg(feature = "predict")]
    pub fn set_predictor(&mut self, predictor: Option<Arc<Predictor>>) {
        self.predictor = predictor;
    }

    /// Tunes one scenario of the predictions (spec §10.5): a multiplier on its learned probability.
    /// Sets a scenario's weight by hand; it then wins over the learned one.
    #[cfg(feature = "predict")]
    pub fn set_prediction_weight(&mut self, which: Weight, value: f64) {
        self.weights.set(which, value);
        self.manual[which as usize] = true;
    }

    /// Hands a scenario's weight back to learning (or 1.0 when learning is off).
    #[cfg(feature = "predict")]
    pub fn clear_prediction_weight(&mut self, which: Weight) {
        self.weights.set(which, 1.0);
        self.manual[which as usize] = false;
    }

    /// Turns learning from the user's choices on or off (spec §10.11). Off keeps what was learned
    /// but stops using and adding to it.
    #[cfg(feature = "predict")]
    pub fn set_learning(&mut self, on: bool) {
        self.learning = on;
        self.accepted = None;
    }

    /// Whether choices are recorded (spec §10.11): off in an app the user excluded from learning,
    /// where what was learned still applies. On by default.
    #[cfg(feature = "predict")]
    pub fn set_recording(&mut self, on: bool) {
        self.recording = on;
        self.accepted = None;
    }

    #[cfg(feature = "predict")]
    pub fn learner(&self) -> &Learner {
        &self.learner
    }

    /// Whether a predictor is loaded (the full engine with predictions on).
    #[cfg(feature = "predict")]
    pub fn predictions_available(&self) -> bool {
        self.predictor.is_some()
    }

    /// Learns from one example of what the user meant (spec §10.12, the training contract): types
    /// `keys` (or, without them, the cheapest keys for `intended`) into a scratch engine with this
    /// one's predictor and weights, and where `intended` is among the predictions, records that it
    /// was chosen, as learning does. Returns where it was listed (0 = on top), or None when it
    /// wasn't (or there's no predictor). Learns even with learning off: the user asked for it.
    #[cfg(feature = "predict")]
    pub fn learn_example(&mut self, keys: Option<&[crate::plan::Press]>, intended: &str) -> Option<usize> {
        let planned;
        let keys = match keys {
            Some(keys) => keys,
            None => {
                planned = crate::plan::keys_for_word(intended)?;
                &planned
            }
        };
        let mut scratch = Engine::new();
        scratch.dictionary = self.dictionary.clone();
        scratch.predictor = Some(self.predictor.clone()?);
        scratch.weights = self.weights;
        scratch.manual = self.manual;
        scratch.ranker = self.ranker;
        scratch.learner = self.learner.clone();
        scratch.learning = true;
        for press in keys {
            scratch.key(press.key, if press.shift { Mods::SHIFTED } else { Mods::NONE });
        }
        let at = scratch.predictions().iter().position(|p| p.text == intended)?;
        scratch.commit_prediction(at);
        self.learner = scratch.learner;
        self.learned_changed = true;
        Some(at)
    }

    /// Replaces what was learned (from the host's storage, or a reset with the default).
    #[cfg(feature = "predict")]
    pub fn set_learner(&mut self, learner: Learner) {
        self.learner = learner;
        self.learned_changed = false;
    }

    /// Whether the learner changed since the last call (the host then stores it).
    #[cfg(feature = "predict")]
    pub fn take_learned_changed(&mut self) -> bool {
        std::mem::take(&mut self.learned_changed)
    }

    #[cfg(feature = "predict")]
    fn learn(&mut self, event: &Event) {
        if self.learning && self.recording {
            self.learner = self.learner.after(event);
            self.learned_changed = true;
        }
    }

    /// The weights the predictor uses: the user's where set by hand, else the learned ones while
    /// learning is on, else 1.0.
    #[cfg(feature = "predict")]
    pub fn effective_weights(&self) -> Weights {
        let mut weights = self.weights;
        for which in Weight::ALL {
            if !self.manual[which as usize] && self.learning {
                weights.set(which, self.learner.weight(which));
            }
        }
        weights
    }

    /// Chooses how the implied-ㅇ alternatives are ranked (spec §10.7).
    #[cfg(feature = "predict")]
    pub fn set_ranker(&mut self, ranker: Ranker) {
        self.ranker = ranker;
    }

    #[cfg(feature = "predict")]
    pub fn prediction_weights(&self) -> Weights {
        self.weights
    }

    /// Whether the top prediction is byte-identical to what F10 would make of the staged text now.
    #[cfg(feature = "predict")]
    fn top_prediction_is_f10(&self) -> bool {
        let Some(top) = self.predictions().into_iter().next() else { return false };
        let mut preview = self.clone();
        preview.key(Key::F10, Mods::NONE);
        top.text == preview.preedit()
    }

    /// The readings of the staged text (spec §10), with its pieces between punctuation when there's
    /// any (Research(이하 is Research, then 이하).
    #[cfg(feature = "predict")]
    pub fn readings(&self) -> Readings {
        let mut readings = self.readings_of(&self.segment);
        let mut start = 0;
        let mut pieces = Vec::new();
        for (i, token) in self.segment.iter().enumerate() {
            if let Token::Typed(c, _) = *token
                && is_separator(c)
            {
                pieces.push(Piece { readings: self.readings_of(&self.segment[start..i]), separator: c.to_string() });
                start = i + 1;
            }
        }
        if !pieces.is_empty() {
            pieces.push(Piece { readings: self.readings_of(&self.segment[start..]), separator: String::new() });
            readings.pieces = pieces;
        }
        readings
    }

    #[cfg(feature = "predict")]
    fn readings_of(&self, segment: &[Token]) -> Readings {
        let korean = compose(segment);
        let hanja = self.dictionary.as_ref().and_then(|d| {
            let (reading, candidates) = d.longest_suffix(&korean)?;
            // Lean: only a whole word of two syllables or more.
            (reading == korean && reading.chars().count() >= 2).then(|| candidates[0].hanja.clone())
        });
        // Every way to read the keys as an English start plus a Korean ending (report에), ending
        // short: the predictor keeps the ones whose ending is a particle.
        let splits = (segment.len().saturating_sub(MAX_ENDING_KEYS).max(1)..segment.len())
            .map(|k| Split { english: typed_of(&segment[..k]), korean: compose(&segment[k..]) })
            .collect();
        Readings {
            english: typed_of(segment),
            two_set: compose_with(&two_set_tokens(segment), Options { standard: true, ..Options::default() }),
            keep_finals: compose_with(segment, Options { keep_finals: true, ..Options::default() }),
            kept: kept_variants(segment),
            implied_l: implied_l_variants(segment),
            bare: compose_with(segment, Options { no_implied: true, ..Options::default() }),
            implied: composer_for(segment).implied_count(),
            splits,
            hanja,
            korean,
            pieces: Vec::new(),
        }
    }

    /// The ranked predictions for the staged text; empty without a predictor (the lite engine).
    #[cfg(feature = "predict")]
    pub fn predictions(&self) -> Vec<Prediction> {
        match &self.predictor {
            Some(p) if !self.segment.is_empty() => p.predict(
                &self.readings(),
                Context { previous: self.previous, command: false, weights: self.effective_weights(), ranker: self.ranker },
                self.dictionary.as_deref(),
            ),
            _ => Vec::new(),
        }
    }

    /// Commits prediction `index` (from [`Engine::predictions`]) instead of the Korean reading.
    #[cfg(feature = "predict")]
    pub fn commit_prediction(&mut self, index: usize) -> Output {
        let shown = self.predictions();
        let Some(chosen) = shown.get(index).cloned() else { return self.unchanged(true) };
        let kinds = shown.iter().map(|p| learn::scenario(p.kind)).collect();
        self.learn(&Event::Chose { shown: kinds, chosen: index });
        self.accepted = (index == 0 && self.learning).then(|| learn::scenario(chosen.kind));
        self.previous = Some(Language::of(chosen.kind));
        self.take_segment();
        Output { commit: chosen.text, consumed: true, ..Output::default() }
    }

    /// A key that commits (Space, Enter, a shortcut, …) while prediction `index` is what shows:
    /// that prediction stands in for the unconfirmed text, then the key does what it does to
    /// it (spec §10): Space adds a space, Enter passes on unless it only commits (Shift+Enter or
    /// the setting), and so on. Learns the choice as [`Engine::commit_prediction`] does.
    #[cfg(feature = "predict")]
    pub fn key_after_prediction(&mut self, index: usize, key: Key, mods: Mods) -> Output {
        let shown = self.predictions();
        let Some(chosen) = shown.get(index).cloned() else { return self.key(key, mods) };
        let kinds = shown.iter().map(|p| learn::scenario(p.kind)).collect();
        self.learn(&Event::Chose { shown: kinds, chosen: index });
        self.previous = Some(Language::of(chosen.kind));
        self.conversion = None;
        self.hanja = None;
        self.segment = chosen.text.chars().map(Token::Text).collect();
        // The choice is learned above: the key's own commit learns nothing more.
        let learning = std::mem::replace(&mut self.learning, false);
        let out = self.key(key, mods);
        self.learning = learning;
        // After the key, which clears what was accepted before it and sets the context.
        self.accepted = (index == 0 && self.learning).then(|| learn::scenario(chosen.kind));
        self.previous = Some(Language::of(chosen.kind));
        out
    }

    /// Commits prediction `index` because the host must (focus loss, a click elsewhere): like
    /// [`Engine::flush`], it isn't a choice, so nothing is learned.
    #[cfg(feature = "predict")]
    pub fn flush_prediction(&mut self, index: usize) -> Output {
        let Some(chosen) = self.predictions().into_iter().nth(index) else { return self.flush() };
        self.previous = Some(Language::of(chosen.kind));
        self.take_segment();
        Output { commit: chosen.text, consumed: true, ..Output::default() }
    }

    fn take_segment(&mut self) -> String {
        // JIS English stages nothing but a press-and-hold letter: what it shows is what commits,
        // before anything typed after switching to a Korean mode with the list still open.
        #[cfg(feature = "english")]
        let held = self.held.take().map(|held| held.shown().to_string()).unwrap_or_default();
        #[cfg(not(feature = "english"))]
        let held = String::new();
        self.conversion = None;
        self.hanja = None;
        held + &compose(&std::mem::take(&mut self.segment))
    }

    /// A key in JIS English (spec §3.6.2, §3.6.3): every key goes to the app, except while a held
    /// letter's list is open, and the auto-repeat that opens it.
    #[cfg(feature = "english")]
    fn english_key(&mut self, key: Key, mods: Mods) -> Output {
        // Whatever a Korean mode left unconfirmed commits before the key, as on any switch.
        let mut commit = if self.held.is_none() { self.take_segment() } else { String::new() };
        self.layout = Layout::JisKorean;
        if let Some(held) = self.held.as_mut() {
            let count = held.variants.len();
            let pick = key.digit().and_then(|d| d.to_digit(10)).filter(|&d| (1..=count as u32).contains(&d));
            let mut handled = !mods.is_shortcut();
            if mods.repeat && key == held.key {
                // Still held: nothing more.
            } else if !handled {
            } else if let Some(d) = pick {
                held.selected = Some(d as usize - 1);
                return self.commit(String::new(), true);
            } else {
                match key {
                    Key::ArrowRight | Key::ArrowDown => held.selected = Some(held.selected.map_or(0, |i| (i + 1) % count)),
                    Key::ArrowLeft | Key::ArrowUp => {
                        held.selected = Some(held.selected.map_or(count - 1, |i| (i + count - 1) % count))
                    }
                    Key::Enter if held.selected.is_some() => return self.commit(String::new(), true),
                    Key::Escape => {
                        held.selected = None;
                        return self.commit(String::new(), true);
                    }
                    Key::Backspace => {
                        // The letter goes too, as Backspace would have taken it.
                        self.held = None;
                        self.last_letter = None;
                        return Output { consumed: true, ..Output::default() };
                    }
                    _ => handled = false,
                }
            }
            if handled {
                return self.held_output();
            }
            // Any other key keeps what's shown, then goes to the app.
            commit = self.take_segment();
        }
        let upper = mods.shift || mods.caps;
        if !mods.is_shortcut()
            && mods.repeat
            && self.last_letter == Some((key, mods.shift))
            && let (Some(variants), Some(letter)) = (crate::hold::variants(key, upper), key.letter())
        {
            let letter = if upper { letter.to_ascii_uppercase() } else { letter };
            self.held = Some(crate::hold::Held { key, letter, variants, selected: None });
            self.last_letter = None;
            return Output { take_back: 1, ..self.held_output() };
        }
        self.last_letter = (!mods.is_shortcut() && !mods.repeat && key.letter().is_some()).then_some((key, mods.shift));
        Output { commit, consumed: false, ..Output::default() }
    }

    /// The open press-and-hold list: the shown character as the preedit, the variants as the list
    /// (none selected is `selected` = the count).
    #[cfg(feature = "english")]
    fn held_output(&self) -> Output {
        let Some(held) = &self.held else { return Output { consumed: true, ..Output::default() } };
        let items = held.variants.iter().map(|v| Candidate { hanja: v.to_string(), comment: String::new() }).collect();
        Output {
            preedit: held.shown().to_string(),
            consumed: true,
            candidates: Some(Candidates { items, selected: held.selected.unwrap_or(held.variants.len()) }),
            ..Output::default()
        }
    }

    /// Commits the segment followed by `suffix`, for a key that commits: the Korean reading was
    /// the user's choice (spec §10.11 Accept, when it was the top prediction with others shown),
    /// and the context for the next word (§10.3).
    fn commit(&mut self, suffix: String, consumed: bool) -> Output {
        #[cfg(feature = "predict")]
        if !self.segment.is_empty() && self.hanja.is_none() {
            let shown = self.predictions();
            if let Some(top) = shown.first()
                && top.kind == crate::predict::Kind::Korean
                && shown.len() > 1
            {
                let kinds = shown.iter().map(|p| learn::scenario(p.kind)).collect();
                self.learn(&Event::Chose { shown: kinds, chosen: 0 });
                self.accepted = self.learning.then_some(Weight::Korean);
            }
            self.previous = Some(Language::Korean);
        }
        self.commit_quietly(suffix, consumed)
    }

    /// Commits the segment followed by `suffix`, learning nothing.
    fn commit_quietly(&mut self, suffix: String, consumed: bool) -> Output {
        let mut commit = self.take_segment();
        commit.push_str(&suffix);
        Output { commit, consumed, ..Output::default() }
    }

    fn unchanged(&self, consumed: bool) -> Output {
        let candidates = self
            .hanja
            .as_ref()
            .map(|list| Candidates { items: list.candidates.clone(), selected: list.selected });
        Output { preedit: self.preedit(), consumed, candidates, english: self.english(), ..Output::default() }
    }
}

/// Keystrokes read as English on the mode's stock keyboard (see [`Engine::english`]).
fn english_of(segment: &[Token]) -> String {
    segment
        .iter()
        .filter_map(|t| match *t {
            Token::Jamo { jamo, raw } | Token::IeungVowel { jamo, raw } => Some(raw.unwrap_or(jamo)),
            Token::Text(c) => Some(c),
            Token::Typed(_, english) => Some(english),
            Token::Bare => None,
        })
        .collect()
}

/// The staged keys composed with one final kept before the vowel after it (산이 for 사니, 어린이
/// for 어리니), one variant per place the composition moves a final: the vowel there starts its
/// own syllable on ㅇ, as Shift+vowel types it (spec §4.2, §10.3).
#[cfg(feature = "predict")]
fn kept_variants(segment: &[Token]) -> Vec<String> {
    let moved = compose(segment);
    let mut out: Vec<String> = Vec::new();
    for (k, token) in segment.iter().enumerate() {
        let Token::Jamo { jamo, raw } = *token else { continue };
        if k == 0 || !hangul::is_medial(jamo) {
            continue;
        }
        let mut variant = segment.to_vec();
        variant[k] = Token::IeungVowel { jamo, raw };
        let kept = compose(&variant);
        if kept != moved && kept.chars().count() == moved.chars().count() && !out.contains(&kept) {
            out.push(kept);
        }
    }
    out
}

/// The staged keys composed with an implied ㄹ after a ㄴ final (전력 for 저녁 typed lazily: ㄴ
/// then ㄹ sounds ㄹㄹ, so the ㄹ is easy to leave out), one variant per place a ㄴ final moved
/// onto a vowel (spec §10.5).
#[cfg(feature = "predict")]
fn implied_l_variants(segment: &[Token]) -> Vec<String> {
    let moved = compose(segment);
    let mut out: Vec<String> = Vec::new();
    for (k, token) in segment.iter().enumerate() {
        let Token::Jamo { jamo, .. } = *token else { continue };
        let after_n = k > 0 && matches!(segment[k - 1], Token::Jamo { jamo: 'ㄴ', .. });
        if !after_n || !hangul::is_medial(jamo) {
            continue;
        }
        let mut variant = segment.to_vec();
        variant.insert(k, Token::Jamo { jamo: 'ㄹ', raw: None });
        let text = compose(&variant);
        if text != moved && text.chars().count() == moved.chars().count() && !out.contains(&text) {
            out.push(text);
        }
    }
    out
}

/// The keys read as English for predictions: letters with their case, and non-letter keys as they
/// typed (a ; typed is ;, where F10 would give the stock JIS key's character).
#[cfg(feature = "predict")]
fn typed_of(segment: &[Token]) -> String {
    segment
        .iter()
        .filter_map(|t| match *t {
            Token::Jamo { jamo, raw } | Token::IeungVowel { jamo, raw } => Some(raw.unwrap_or(jamo)),
            Token::Text(c) | Token::Typed(c, _) => Some(c),
            Token::Bare => None,
        })
        .collect()
}

/// Punctuation that separates the pieces of a staged word (spec §10.10): anything typed that isn't
/// a letter or digit.
#[cfg(feature = "predict")]
fn is_separator(c: char) -> bool {
    !c.is_alphanumeric()
}

/// The longest Korean ending considered when splitting into English + Korean (에서는 is 6 keys).
#[cfg(feature = "predict")]
const MAX_ENDING_KEYS: usize = 8;

fn composer_for(segment: &[Token]) -> Composer {
    composer_with(segment, Options::default())
}

fn composer_with(segment: &[Token], options: Options) -> Composer {
    let mut composer = Composer::with(options);
    for token in segment {
        match *token {
            Token::Jamo { jamo, .. } => composer.jamo(jamo),
            Token::Text(c) | Token::Typed(c, _) => composer.text(c),
            Token::Bare => composer.bare(),
            Token::IeungVowel { jamo, .. } => composer.ieung_vowel(jamo),
        }
    }
    composer
}

fn compose(segment: &[Token]) -> String {
    composer_for(segment).finish()
}

fn compose_with(segment: &[Token], options: Options) -> String {
    composer_with(segment, options).finish()
}

/// The segment as plain 2-set on stock JIS would have it: number-row keys type digits, symbol keys
/// their stock JIS characters (composing then happens with `Options::standard`).
#[cfg(feature = "predict")]
fn two_set_tokens(segment: &[Token]) -> Vec<Token> {
    segment
        .iter()
        .map(|t| match *t {
            Token::Jamo { raw: Some(d), .. } if d.is_ascii_digit() => Token::Text(d),
            Token::Typed(_, english) => Token::Text(english),
            Token::Bare => Token::Bare,
            ref other => other.clone(),
        })
        .collect()
}

/// The full-width form of a printable ASCII character (spec §5.3).
pub fn full_width(c: char) -> char {
    match c {
        '!'..='~' => char::from_u32(c as u32 + 0xFEE0).unwrap_or(c),
        ' ' => '\u{3000}',
        _ => c,
    }
}
