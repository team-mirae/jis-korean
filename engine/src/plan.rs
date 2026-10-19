//! Keystroke planning: how many key presses a text takes on JIS Korean, and on conventional 2-set.
//!
//! [`Robot`] plans keystrokes for a text on the JIS Korean layout, types them through the engine,
//! and checks after every character that the screen (committed + unconfirmed text) shows exactly
//! the text so far. Keys come from the engine's own layout tables, not a copy. It plays the layout
//! the way a fluent typist would: dedicated keys, the implied ㅇ, Shift+vowel and the double press
//! where they save keystrokes, F8 for symbols only stock JIS has, F10 or an input-source switch for
//! Latin words, and F7 for hanja. Where several plans work it takes the cheapest (Shift counts as
//! a press).
//!
//! [`count`] wraps it for hosts (the web playground) together with the conventional estimate.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Arc;

use crate::{Dictionary, Engine, Key, Layout, Mods, Out, Output, PAGE_SIZE, output};

/// What a symbol costs when it isn't on a layout's legends: an OS shortcut, the character palette or
/// copying it from a search all take at least this many presses. Applies to both layouts.
pub const OFF_LEGEND: usize = 3;

/// Key presses for one text, both ways (Shift counted as a press).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Characters counted (after normalization; untypable ones are left out).
    pub chars: usize,
    /// JIS Korean, as the robot types it.
    pub jis: usize,
    /// Conventional 2-set on a JIS keyboard (estimate, see [`conventional`]).
    pub conventional: usize,
    /// Characters no key types (①, ×, …), counted as [`OFF_LEGEND`] presses on both layouts.
    pub off_keyboard: usize,
    /// Characters left out of both counts (invisible control characters, robot failures).
    pub dropped: usize,
}

/// Counts the key presses for `text` on JIS Korean and on conventional 2-set. With a dictionary,
/// hanja are typed with F7; without one they're left out.
pub fn count(text: &str, dictionary: Option<Arc<Dictionary>>) -> Counts {
    let mut robot = Robot::new(dictionary);
    let typed = robot.type_text(text);
    let report = &robot.report;
    Counts {
        chars: report.chars,
        jis: report.presses,
        conventional: report.baseline,
        off_keyboard: report.pasted,
        dropped: report.dropped.values().sum::<usize>() + report.failures.len(),
    }
    .with_typed(&typed)
}

impl Counts {
    fn with_typed(self, typed: &str) -> Counts {
        Counts { chars: typed.chars().count(), ..self }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Press {
    pub key: Key,
    pub shift: bool,
}

impl Press {
    fn new(key: Key) -> Press {
        Press { key, shift: false }
    }

    fn cost(self) -> usize {
        1 + self.shift as usize
    }
}

impl fmt::Display for Press {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.shift {
            write!(f, "S-")?;
        }
        write!(f, "{}", self.key.code())
    }
}

/// What a robot run did.
#[derive(Debug, Default)]
pub struct Report {
    /// Characters of the (normalized) text.
    pub chars: usize,
    /// Key presses, with each Shift counted as one more.
    pub presses: usize,
    pub shifts: usize,
    /// F6–F10 presses.
    pub function_keys: usize,
    /// Jamo typed with a number-row key (ㅃ ㅉ ㄸ ㄲ ㅆ ㅟ ㅝ ㅚ ㅢ ㅘ).
    pub dedicated_keys: usize,
    pub implied_ieung: usize,
    pub double_presses: usize,
    pub hanja: usize,
    pub latin_words: usize,
    /// Latin words typed by switching the input source to English and back (when cheaper than F10).
    pub latin_switches: usize,
    /// Syllables started with Shift+vowel.
    pub shift_vowels: usize,
    pub escapes: usize,
    /// Estimated presses for the same text on standard 2-set with stock JIS symbols.
    pub baseline: usize,
    /// Characters the robot couldn't type, with what was on screen.
    pub failures: Vec<String>,
    /// Characters no key types, entered from outside the keyboard at [`OFF_LEGEND`] presses each.
    pub pasted: usize,
    /// Invisible control characters, removed during normalization.
    pub dropped: BTreeMap<char, usize>,
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let per = |n: usize| n as f64 / self.chars.max(1) as f64;
        writeln!(f, "characters        {}", self.chars)?;
        writeln!(f, "key presses       {} ({:.3} per character, Shift included)", self.presses, per(self.presses))?;
        writeln!(f, "  Shift           {}", self.shifts)?;
        writeln!(f, "  F6–F10          {}", self.function_keys)?;
        writeln!(f, "standard 2-set    ~{} ({:.3} per character, estimate)", self.baseline, per(self.baseline))?;
        if self.baseline > 0 {
            let saved = self.baseline as f64 - self.presses as f64;
            writeln!(f, "saved             ~{:.0} presses ({:.1}%)", saved, 100.0 * saved / self.baseline as f64)?;
        }
        writeln!(f, "dedicated keys    {}", self.dedicated_keys)?;
        writeln!(f, "implied ㅇ        {}", self.implied_ieung)?;
        writeln!(f, "double presses    {}", self.double_presses)?;
        writeln!(f, "hanja via F7      {}", self.hanja)?;
        writeln!(f, "Shift+vowel       {}", self.shift_vowels)?;
        writeln!(f, "Latin via F10     {}", self.latin_words)?;
        writeln!(f, "Latin via switch  {}", self.latin_switches)?;
        writeln!(f, "F8 escapes        {}", self.escapes)?;
        writeln!(f, "off the keyboard  {} (×{OFF_LEGEND} presses)", self.pasted)?;
        writeln!(f, "failures          {}", self.failures.len())?;
        if !self.dropped.is_empty() {
            let dropped: Vec<String> = self.dropped.iter().map(|(c, n)| format!("{c}×{n}")).collect();
            writeln!(f, "dropped (no key)  {}", dropped.join(" "))?;
        }
        for failure in self.failures.iter().take(10) {
            writeln!(f, "  ✗ {failure}")?;
        }
        Ok(())
    }
}

// --- The layout, read back from the engine

/// Ways to type each character and jamo directly, per layout, cheapest first.
struct Keymap {
    chars: HashMap<(Layout, char), Press>,
    jamo: HashMap<(Layout, char), Vec<Press>>,
}

impl Keymap {
    fn new() -> Keymap {
        let mut chars = HashMap::new();
        let mut jamo: HashMap<(Layout, char), Vec<Press>> = HashMap::new();
        for layout in [Layout::JisKorean, Layout::Escaped] {
            for shift in [false, true] {
                for &key in Key::ALL {
                    match output(layout, key, shift) {
                        Some(Out::Char(c)) => {
                            chars.entry((layout, c)).or_insert(Press { key, shift });
                        }
                        Some(Out::Jamo(j)) => {
                            let ways = jamo.entry((layout, j)).or_default();
                            if ways.iter().all(|p| p.key != key) {
                                ways.push(Press { key, shift });
                            }
                        }
                        None => {}
                    }
                }
            }
        }
        for ways in jamo.values_mut() {
            ways.sort_by_key(|p| p.cost());
        }
        Keymap { chars, jamo }
    }

    /// Ways to type a jamo: direct keys, then the 2-set sequence for compounds.
    fn jamo_ways(&self, layout: Layout, j: char) -> Vec<Vec<Press>> {
        let direct = |j: char| self.jamo.get(&(layout, j)).cloned().unwrap_or_default();
        let mut ways: Vec<Vec<Press>> = direct(j).into_iter().map(|p| vec![p]).collect();
        if let Some((a, b)) = compound(j)
            && let (Some(&pa), Some(&pb)) = (direct(a).first(), direct(b).first())
        {
            ways.push(vec![pa, pb]);
        }
        ways
    }

    fn char_press(&self, layout: Layout, c: char) -> Option<Press> {
        self.chars.get(&(layout, c)).copied()
    }
}

/// Compound jamo and their 2-set sequences (spec §4.1).
fn compound(j: char) -> Option<(char, char)> {
    Some(match j {
        'ㅘ' => ('ㅗ', 'ㅏ'),
        'ㅙ' => ('ㅗ', 'ㅐ'),
        'ㅚ' => ('ㅗ', 'ㅣ'),
        'ㅝ' => ('ㅜ', 'ㅓ'),
        'ㅞ' => ('ㅜ', 'ㅔ'),
        'ㅟ' => ('ㅜ', 'ㅣ'),
        'ㅢ' => ('ㅡ', 'ㅣ'),
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
        _ => return None,
    })
}

const INITIALS: &[char] = &['ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ'];
const MEDIALS: &[char] = &[
    'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ', 'ㅞ', 'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ',
];
const FINALS: &[char] = &[
    'ㄱ', 'ㄲ', 'ㄳ', 'ㄴ', 'ㄵ', 'ㄶ', 'ㄷ', 'ㄹ', 'ㄺ', 'ㄻ', 'ㄼ', 'ㄽ', 'ㄾ', 'ㄿ', 'ㅀ', 'ㅁ', 'ㅂ', 'ㅄ', 'ㅅ', 'ㅆ', 'ㅇ',
    'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];

fn is_syllable(c: char) -> bool {
    ('가'..='힣').contains(&c)
}

fn is_compat_jamo(c: char) -> bool {
    ('ㄱ'..='ㅣ').contains(&c)
}

fn is_hanja(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c) || ('\u{F900}'..='\u{FAFF}').contains(&c)
}

fn decompose(s: char) -> (char, char, Option<char>) {
    let n = s as u32 - 0xAC00;
    let t = (n % 28) as usize;
    (INITIALS[(n / 588) as usize], MEDIALS[((n % 588) / 28) as usize], (t > 0).then(|| FINALS[t - 1]))
}

// --- Normalization

/// Replaces characters that have a typable equivalent and drops the rest.
/// Replaces characters that have a typable equivalent and drops invisible control characters.
/// Anything else stays: what neither layout has a key for costs [`OFF_LEGEND`] presses.
pub fn normalize(text: &str) -> (String, BTreeMap<char, usize>) {
    let mut out = String::new();
    let mut dropped = BTreeMap::new();
    for c in text.chars() {
        let replacement = match c {
            'ㆍ' | '･' => "·",
            '㈜' => "(주)",
            '“' | '”' => "\"",
            '‘' | '’' => "'",
            '∼' | '～' => "~",
            '…' => "...",
            '–' => "-",
            '\u{a0}' | '\t' => " ",
            _ => {
                if c.is_control() && c != '\n' {
                    *dropped.entry(c).or_insert(0) += 1;
                } else {
                    out.push(c);
                }
                continue;
            }
        };
        out.push_str(replacement);
    }
    (out, dropped)
}

// --- The robot

#[derive(Clone)]
struct Screen {
    engine: Engine,
    doc: String,
}

impl Screen {
    fn press(&mut self, p: Press) -> Output {
        let out = self.engine.key(p.key, if p.shift { Mods::SHIFTED } else { Mods::NONE });
        self.doc.push_str(&out.commit);
        if !out.consumed {
            match p.key {
                Key::Enter => self.doc.push('\n'),
                Key::Backspace => {
                    self.doc.pop();
                }
                _ => {}
            }
        }
        out
    }

    fn text(&self) -> String {
        self.doc.clone() + &self.engine.preedit()
    }

    fn ends_in_hangul(&self) -> bool {
        self.engine.preedit().chars().last().is_some_and(|c| is_syllable(c) || is_compat_jamo(c))
    }
}

#[derive(Clone, Default)]
struct Plan {
    presses: Vec<Press>,
    dedicated: usize,
    implied: bool,
    double: bool,
    shift_vowel: bool,
}

impl Plan {
    fn cost(&self) -> usize {
        self.presses.iter().map(|p| p.cost()).sum()
    }
}

pub struct Robot {
    keymap: Keymap,
    /// hanja → reading, for F7.
    readings: HashMap<char, String>,
    screen: Screen,
    expected: String,
    pub log: Vec<Press>,
    pub report: Report,
}

impl Robot {
    /// Without a dictionary, hanja can't be typed and are left out.
    pub fn new(dictionary: Option<Arc<Dictionary>>) -> Robot {
        let mut engine = Engine::new();
        // hanja → its single-syllable reading where it ranks highest.
        let mut readings: HashMap<char, (String, usize)> = HashMap::new();
        if let Some(dictionary) = dictionary {
            for (reading, candidates) in dictionary.entries() {
                if reading.chars().count() != 1 {
                    continue;
                }
                for (position, candidate) in candidates.iter().enumerate() {
                    let mut chars = candidate.hanja.chars();
                    if let (Some(h), None) = (chars.next(), chars.next()) {
                        let best = readings.entry(h).or_insert((reading.to_string(), position));
                        if position < best.1 {
                            *best = (reading.to_string(), position);
                        }
                    }
                }
            }
            engine.set_dictionary(dictionary);
        }
        Robot {
            keymap: Keymap::new(),
            readings: readings.into_iter().map(|(h, (r, _))| (h, r)).collect(),
            screen: Screen { engine, doc: String::new() },
            expected: String::new(),
            log: Vec::new(),
            report: Report::default(),
        }
    }

    pub fn can_type(&self, c: char) -> bool {
        is_syllable(c)
            || c.is_ascii_alphanumeric()
            || c == ' '
            || c == '\n'
            || (is_compat_jamo(c) && !self.keymap.jamo_ways(Layout::JisKorean, c).is_empty())
            || self.keymap.char_press(Layout::JisKorean, c).is_some()
            || self.keymap.char_press(Layout::Escaped, c).is_some()
            || (is_hanja(c) && self.readings.contains_key(&c))
    }

    /// Types `text` after normalizing it. Returns the normalized text the screen should show.
    pub fn type_text(&mut self, text: &str) -> String {
        let (target, dropped) = normalize(text);
        self.report.dropped = dropped;
        let chars: Vec<char> = target.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let next = chars.get(i + 1).copied();
            let c = chars[i];
            let before = self.screen.clone();
            let typed = if c.is_ascii_alphabetic() {
                let end = chars[i..].iter().position(|c| !c.is_ascii_alphabetic()).map_or(chars.len(), |n| i + n);
                let word: String = chars[i..end].iter().collect();
                let ok = self.type_latin(&word);
                i = end;
                if ok { Ok(word.chars().count()) } else { Err(word) }
            } else {
                i += 1;
                self.type_char(c, next).map(|_| 1).map_err(|_| c.to_string())
            };
            match typed {
                Ok(n) => self.report.chars += n,
                Err(what) => {
                    self.screen = before;
                    let tail: String = self.screen.text().chars().rev().take(12).collect::<Vec<_>>().into_iter().rev().collect();
                    self.report.failures.push(format!("{what:?} after …{tail}"));
                }
            }
        }
        let out = self.screen.engine.flush();
        self.screen.doc.push_str(&out.commit);
        self.report.baseline = baseline(&self.keymap, &self.expected);
        self.expected.clone()
    }

    /// The document after the run.
    pub fn document(&self) -> &str {
        &self.screen.doc
    }

    fn layout(&self) -> Layout {
        if self.screen.engine.is_escaped() { Layout::Escaped } else { Layout::JisKorean }
    }

    /// Presses keys for real and counts them.
    fn press_all(&mut self, presses: &[Press]) -> Output {
        let mut last = Output::default();
        for &p in presses {
            last = self.screen.press(p);
            self.log.push(p);
            self.report.presses += p.cost();
            self.report.shifts += p.shift as usize;
            if matches!(p.key, Key::F6 | Key::F7 | Key::F8 | Key::F9 | Key::F10) {
                self.report.function_keys += 1;
            }
        }
        last
    }

    /// Tries `plans` in cost order on a copy of the screen, then presses the first that shows
    /// `want`.
    fn try_plans(&mut self, mut plans: Vec<Plan>, want: &str) -> Option<Plan> {
        plans.sort_by_key(Plan::cost);
        let plan = plans.into_iter().find(|plan| {
            let mut screen = self.screen.clone();
            plan.presses.iter().for_each(|&p| {
                screen.press(p);
            });
            screen.text() == want
        })?;
        self.press_all(&plan.presses);
        Some(plan)
    }

    /// A character no key types (①, ×, emoji, a hanja without a dictionary): entered from outside
    /// the keyboard, which commits what's composing, for a flat [`OFF_LEGEND`] presses.
    fn paste(&mut self, c: char) {
        let out = self.screen.engine.flush();
        self.screen.doc.push_str(&out.commit);
        self.screen.doc.push(c);
        self.report.presses += OFF_LEGEND;
        self.report.pasted += 1;
    }

    /// Commits first if the unconfirmed text ends in Hangul that F7 or F10 would pick up.
    fn break_word(&mut self) {
        if self.screen.ends_in_hangul() {
            self.press_all(&[Press::new(Key::F6)]);
        }
    }

    fn type_char(&mut self, c: char, next: Option<char>) -> Result<(), ()> {
        let want = format!("{}{c}", self.expected);
        if is_syllable(c) {
            // F6 before Latin, hanja or a bare jamo would drop an implied ㅇ, so don't use one then.
            let f6_next = next.is_some_and(|n| n.is_ascii_alphabetic() || is_hanja(n) || is_compat_jamo(n));
            let plans = self.syllable_plans(c, !f6_next);
            let plan = self.try_plans(plans, &want).ok_or(())?;
            self.report.dedicated_keys += plan.dedicated;
            self.report.implied_ieung += plan.implied as usize;
            self.report.shift_vowels += plan.shift_vowel as usize;
            self.report.double_presses += plan.double as usize;
        } else if is_hanja(c) && self.readings.contains_key(&c) {
            self.type_hanja(c, &want)?;
        } else if is_hanja(c) {
            self.paste(c);
        } else if is_compat_jamo(c) {
            self.break_word();
            let ways = self.keymap.jamo_ways(self.layout(), c);
            // A bare jamo: type it, then F6 commits it as it stands.
            let plans = ways
                .into_iter()
                .map(|mut presses| {
                    presses.push(Press::new(Key::F6));
                    Plan { presses, ..Plan::default() }
                })
                .collect();
            self.try_plans(plans, &want).ok_or(())?;
        } else if c == ' ' {
            self.press_all(&[Press::new(Key::Space)]);
        } else if c == '\n' {
            self.press_all(&[Press::new(Key::Enter)]);
        } else {
            match (self.keymap.char_press(Layout::JisKorean, c), self.keymap.char_press(Layout::Escaped, c)) {
                (Some(p), _) => {
                    self.press_all(&[p]);
                }
                (None, Some(p)) => {
                    // Not on the JIS Korean legends (( ) % @ …): a flat OFF_LEGEND presses, as for
                    // any symbol a layout lacks. Typed through the F8 escape so the screen check
                    // still holds, but those keys aren't counted, and the escape ends right away.
                    let escape = Press::new(Key::F8);
                    let back = Press { key: Key::F8, shift: true };
                    for press in [escape, p, back] {
                        self.screen.press(press);
                        self.log.push(press);
                    }
                    self.report.presses += OFF_LEGEND;
                    self.report.escapes += 1;
                }
                (None, None) => self.paste(c),
            }
        }
        if self.screen.text() != want {
            return Err(());
        }
        self.expected = want;
        Ok(())
    }

    fn syllable_plans(&self, s: char, allow_implied: bool) -> Vec<Plan> {
        let layout = self.layout();
        let (initial, medial, last) = decompose(s);
        let dedicated = |presses: &[Press]| presses.iter().filter(|p| p.key.digit().is_some()).count();
        let medials = self.keymap.jamo_ways(layout, medial);
        let finals: Vec<Vec<Press>> = match last {
            None => vec![vec![]],
            Some(f) => self.keymap.jamo_ways(layout, f),
        };
        let mut plans = Vec::new();
        /// How a syllable's keys begin: what comes before the vowel, the vowel's own keys, and
        /// which shortcut that is.
        struct Start {
            before: Vec<Press>,
            vowel: Vec<Press>,
            implied: bool,
            double: bool,
            shift_vowel: bool,
        }
        let start = |before: Vec<Press>, vowel: Vec<Press>, implied, double, shift_vowel| Start {
            before,
            vowel,
            implied,
            double,
            shift_vowel,
        };
        for m in &medials {
            let mut starts: Vec<Start> = Vec::new();
            if initial == 'ㅇ' && allow_implied {
                // Implied ㅇ, and the double press for when a final would move: 말+ㅣ+ㅣ → 말이.
                starts.push(start(vec![], m.clone(), true, false, false));
                starts.push(start(vec![m[0]], m.clone(), false, true, false));
                // Shift+vowel starts a syllable on ㅇ (spec §4.2), where Shift adds nothing else.
                if layout == Layout::JisKorean
                    && !m[0].shift
                    && output(layout, m[0].key, true) == output(layout, m[0].key, false)
                {
                    let mut shifted = m.clone();
                    shifted[0].shift = true;
                    starts.push(start(vec![], shifted, false, false, true));
                }
            }
            for i in self.keymap.jamo_ways(layout, initial) {
                starts.push(start(i, m.clone(), false, false, false));
            }
            for s in &starts {
                for f in &finals {
                    let presses: Vec<Press> = s.before.iter().chain(&s.vowel).chain(f).copied().collect();
                    let used = dedicated(&presses);
                    plans.push(Plan {
                        presses,
                        dedicated: used,
                        implied: s.implied,
                        double: s.double,
                        shift_vowel: s.shift_vowel,
                    });
                }
            }
        }
        // Prefer the IME's own shortcuts on ties (the sort in try_plans is stable).
        plans.sort_by_key(|p| {
            std::cmp::Reverse(p.dedicated + p.implied as usize + p.double as usize + p.shift_vowel as usize)
        });
        plans
    }

    fn type_hanja(&mut self, c: char, want: &str) -> Result<(), ()> {
        let reading = self.readings.get(&c).cloned().ok_or(())?;
        self.break_word();
        let shown = format!("{}{reading}", self.expected);
        let syllable = reading.chars().next().ok_or(())?;
        let plans = self.syllable_plans(syllable, false);
        self.try_plans(plans, &shown).ok_or(())?;
        let out = self.press_all(&[Press::new(Key::F7)]);
        let candidates = out.candidates.ok_or(())?;
        let index = candidates.items.iter().position(|cand| cand.hanja.chars().eq([c])).ok_or(())?;
        let pages = vec![Press::new(Key::ArrowRight); index / PAGE_SIZE];
        let digit = Key::from_code(&format!("Digit{}", index % PAGE_SIZE + 1));
        self.press_all(&pages);
        self.press_all(&[Press::new(digit)]);
        self.report.hanja += 1;
        if self.screen.text() == want { Ok(()) } else { Err(()) }
    }

    /// A Latin word: its keys, then F10 once (as typed), twice (UPPER) or three times
    /// (Capitalized), or a switch to English and back, whichever is cheapest.
    fn type_latin(&mut self, word: &str) -> bool {
        let want = format!("{}{word}", self.expected);
        // Switching the input source to English and back: two presses, plus the word as typed.
        let switch = 2 + word.chars().map(|c| 1 + c.is_ascii_uppercase() as usize).sum::<usize>();
        // F10 plans: as typed (Shift on capitals), or lowercase keys then UPPER / Capitalized.
        let upper = word.chars().filter(|c| c.is_ascii_uppercase()).count();
        let mut f10 = word.len() + upper + 1;
        if upper == word.len() {
            f10 = f10.min(word.len() + 2);
        } else if upper == 1 && word.starts_with(|c: char| c.is_ascii_uppercase()) {
            f10 = f10.min(word.len() + 3);
        }
        // Plus F6 first if the unconfirmed text ends in Hangul.
        let f10 = f10 + self.screen.ends_in_hangul() as usize;
        if switch < f10 {
            // Switching input sources commits whatever is composing, then types the word as is.
            let out = self.screen.engine.flush();
            self.screen.doc.push_str(&out.commit);
            self.screen.doc.push_str(word);
            self.report.presses += switch;
            self.report.shifts += word.chars().filter(|c| c.is_ascii_uppercase()).count();
            self.report.latin_switches += 1;
            self.expected = want;
            return true;
        }
        self.break_word();
        let key = |c: char, shift: bool| Press { key: Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), shift };
        let f10 = |n: usize| vec![Press::new(Key::F10); n];
        let as_typed: Vec<Press> = word.chars().map(|c| key(c, c.is_ascii_uppercase())).chain(f10(1)).collect();
        let lower: Vec<Press> = word.chars().map(|c| key(c, false)).collect();
        let mut plans = vec![Plan { presses: as_typed, ..Plan::default() }];
        for presses in 2..=3 {
            plans.push(Plan { presses: lower.iter().copied().chain(f10(presses)).collect(), ..Plan::default() });
        }
        if self.try_plans(plans, &want).is_none() {
            return false;
        }
        self.report.latin_words += 1;
        self.expected = want;
        true
    }
}

/// Estimated presses on conventional 2-set (Apple's 2-Set Korean on a JIS keyboard): no dedicated
/// keys, Shift for doubles and ㅒ ㅖ, two keys for compound vowels, digits unshifted, stock JIS
/// symbols, a 한/영 switch into and out of each Latin run, [`OFF_LEGEND`] presses for symbols stock
/// JIS lacks (—, ·, 「」), and three for each hanja (reading + 한자 + pick, counted low).
pub fn conventional(text: &str) -> usize {
    baseline(&Keymap::new(), text)
}

fn baseline(keymap: &Keymap, text: &str) -> usize {
    let jamo_cost = |j: char| {
        keymap
            .jamo_ways(Layout::Escaped, j)
            .iter()
            .map(|w| w.iter().map(|p| p.cost()).sum::<usize>())
            .min()
            .unwrap_or(2)
    };
    let mut total = 0;
    let mut in_latin = false;
    for c in text.chars() {
        if c.is_ascii_alphabetic() != in_latin {
            in_latin = c.is_ascii_alphabetic();
            total += 1;
        }
        total += if is_syllable(c) {
            let (i, m, f) = decompose(c);
            jamo_cost(i) + jamo_cost(m) + f.map_or(0, jamo_cost)
        } else if is_hanja(c) {
            3 // reading (≈1 syllable, counted low) + 한자 + pick
        } else if c.is_ascii_alphabetic() {
            1 + c.is_ascii_uppercase() as usize
        } else if let Some(p) = keymap.char_press(Layout::Escaped, c) {
            p.cost()
        } else if c == ' ' || c == '\n' || is_compat_jamo(c) {
            1
        } else {
            OFF_LEGEND
        };
    }
    total
}
