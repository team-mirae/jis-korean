//! The 2-set composition automaton (spec §4.1).

const INITIALS: &str = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ";
const MEDIALS: &str = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ";
const FINALS: &str = "ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ";

fn index_of(table: &str, c: char) -> Option<usize> {
    table.chars().position(|t| t == c)
}

pub fn is_medial(c: char) -> bool {
    index_of(MEDIALS, c).is_some()
}

fn compound_medial(a: char, b: char) -> Option<char> {
    Some(match (a, b) {
        ('ㅗ', 'ㅏ') => 'ㅘ',
        ('ㅗ', 'ㅐ') => 'ㅙ',
        ('ㅗ', 'ㅣ') => 'ㅚ',
        ('ㅜ', 'ㅓ') => 'ㅝ',
        ('ㅜ', 'ㅔ') => 'ㅞ',
        ('ㅜ', 'ㅣ') => 'ㅟ',
        ('ㅡ', 'ㅣ') => 'ㅢ',
        _ => return None,
    })
}

fn double_final(a: char, b: char) -> Option<char> {
    Some(match (a, b) {
        ('ㄱ', 'ㅅ') => 'ㄳ',
        ('ㄴ', 'ㅈ') => 'ㄵ',
        ('ㄴ', 'ㅎ') => 'ㄶ',
        ('ㄹ', 'ㄱ') => 'ㄺ',
        ('ㄹ', 'ㅁ') => 'ㄻ',
        ('ㄹ', 'ㅂ') => 'ㄼ',
        ('ㄹ', 'ㅅ') => 'ㄽ',
        ('ㄹ', 'ㅌ') => 'ㄾ',
        ('ㄹ', 'ㅍ') => 'ㄿ',
        ('ㄹ', 'ㅎ') => 'ㅀ',
        ('ㅂ', 'ㅅ') => 'ㅄ',
        _ => return None,
    })
}

/// A syllable as it was before a vowel took its final, kept so pressing that vowel again can put
/// the final back: 말+ㅣ → 마리, +ㅣ → 말이.
#[derive(Clone, Copy)]
struct MovedFinal {
    out_len: usize,
    initial: Option<char>,
    medial: Option<char>,
    finals: (Option<char>, Option<char>),
    vowel: char,
}

/// Composes jamo into text. Feed it jamo, plain characters and markers in order, then call
/// [`Composer::finish`].
#[derive(Default)]
pub struct Composer {
    out: String,
    initial: Option<char>,
    medial: Option<char>,
    // A double final is kept as its two parts, so a following vowel can take the second.
    finals: (Option<char>, Option<char>),
    /// Items fed so far; positions below are indices into the caller's sequence.
    fed: usize,
    /// Position of the vowel that started the current syllable on an implied ㅇ, while that
    /// syllable has no final.
    implied_at: Option<usize>,
    /// Set by [`Composer::bare`]: the next vowel gets no implied ㅇ.
    bare_next: bool,
    /// Set right after a vowel took the previous syllable's final.
    moved: Option<MovedFinal>,
}

impl Composer {
    pub fn jamo(&mut self, j: char) {
        let moved = self.moved.take();
        if is_medial(j) {
            match moved {
                Some(m) if m.vowel == j => self.restore_final(m),
                _ => self.vowel(j),
            }
        } else {
            self.consonant(j);
        }
        self.advance();
    }

    fn vowel(&mut self, j: char) {
        if self.initial.is_some() && self.medial.is_none() {
            self.medial = Some(j);
        } else if let (Some(m), (None, _)) = (self.medial, self.finals)
            && let Some(compound) = compound_medial(m, j)
        {
            self.medial = Some(compound);
        } else if let (Some(first), second) = self.finals {
            // The (last) final moves to the next syllable: 닭+ㅏ → 달가.
            let before = MovedFinal {
                out_len: self.out.len(),
                initial: self.initial,
                medial: self.medial,
                finals: self.finals,
                vowel: j,
            };
            let moved = match second {
                Some(second) => {
                    self.finals.1 = None;
                    second
                }
                None => {
                    self.finals.0 = None;
                    first
                }
            };
            self.flush();
            self.initial = Some(moved);
            self.medial = Some(j);
            self.moved = Some(before);
        } else {
            self.flush();
            self.vowel_syllable(j);
        }
    }

    /// The same vowel again right after it took a final: the final goes back, and the vowel
    /// starts its own syllable. 마리+ㅣ → 말이, 달가+ㅏ → 닭아.
    fn restore_final(&mut self, m: MovedFinal) {
        self.out.truncate(m.out_len);
        self.initial = m.initial;
        self.medial = m.medial;
        self.finals = m.finals;
        self.flush();
        // This ㅇ was asked for by the double press, so F6 keeps it: no `implied_at`.
        self.medial = Some(m.vowel);
        self.initial = Some('ㅇ');
    }

    /// Starts a syllable with vowel `j` and no typed initial.
    fn vowel_syllable(&mut self, j: char) {
        self.medial = Some(j);
        if !self.bare_next {
            // A vowel with no initial gets an implied ㅇ: ㅏ → 아, 가+ㅓ → 가어.
            self.initial = Some('ㅇ');
            self.implied_at = Some(self.fed);
        }
    }

    fn consonant(&mut self, j: char) {
        let open = self.initial.is_some() && self.medial.is_some();
        match self.finals {
            (None, _) if open && index_of(FINALS, j).is_some() => {
                self.finals.0 = Some(j);
                self.implied_at = None;
            }
            (Some(first), None) if open && double_final(first, j).is_some() => {
                self.finals.1 = Some(j)
            }
            _ => {
                self.flush();
                self.initial = Some(j);
            }
        }
    }

    /// Shift+vowel after a syllable: that syllable is finished as it is (its final stays) and the
    /// vowel starts a new one on ㅇ: 말+⇧ㅣ → 말이, 오+⇧ㅣ → 오이. The ㅇ was asked for, so F6
    /// keeps it (no `implied_at`).
    pub fn ieung_vowel(&mut self, j: char) {
        self.moved = None;
        if self.initial.is_some() && self.medial.is_none() {
            // An initial waiting for its vowel takes it as usual, so Shift still held from a
            // double (⇧ㄲ ⇧ㅏ) types 까.
            self.vowel(j);
            self.advance();
            return;
        }
        self.flush();
        self.initial = Some('ㅇ');
        self.medial = Some(j);
        self.advance();
    }

    pub fn text(&mut self, c: char) {
        self.moved = None;
        self.flush();
        self.out.push(c);
        self.advance();
    }

    /// Ends the syllable being composed and lets the next vowel stand without an implied ㅇ.
    pub fn bare(&mut self) {
        self.moved = None;
        self.flush();
        self.advance();
        self.bare_next = true;
    }

    /// Where the current syllable's vowel starts, if that syllable has an implied ㅇ and no
    /// final yet. F6 then drops the ㅇ by putting a bare marker at that position.
    pub fn implied_at(&self) -> Option<usize> {
        self.implied_at
    }

    fn advance(&mut self) {
        self.fed += 1;
        self.bare_next = false;
    }

    pub fn finish(mut self) -> String {
        self.flush();
        self.out
    }

    fn flush(&mut self) {
        self.implied_at = None;
        let finals = std::mem::take(&mut self.finals);
        let initial = self.initial.take();
        let medial = self.medial.take();
        let last = match finals {
            (Some(a), Some(b)) => double_final(a, b),
            (a, _) => a,
        };
        if let (Some(i), Some(m)) = (initial, medial)
            && let Some(syllable) = syllable(i, m, last)
        {
            self.out.push(syllable);
            return;
        }
        self.out.extend([initial, medial, finals.0, finals.1].into_iter().flatten());
    }
}

fn syllable(initial: char, medial: char, last: Option<char>) -> Option<char> {
    let i = index_of(INITIALS, initial)? as u32;
    let m = index_of(MEDIALS, medial)? as u32;
    let f = match last {
        Some(f) => index_of(FINALS, f)? as u32 + 1,
        None => 0,
    };
    char::from_u32(0xAC00 + (i * 21 + m) * 28 + f)
}
