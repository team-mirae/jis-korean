//! Non-negotiable baselines: the core principle (spec §1). Every way of typing Korean on
//! conventional 2-set (KS X 5002) still types what it always did, in every mode.
//!
//! "Conventional" means each syllable starts with its consonant key (ㅇ included), compound vowels
//! and double finals are typed as two keys, and double consonants and ㅒ ㅖ take Shift. Typing this
//! way never leaves a bare vowel, so the spec's three exceptions (the implied ㅇ, Shift+vowel,
//! Shift+Z) never come into play.
//!
//! The 2-set tables below are written out here, independent of the engine's own tables, so a
//! change to the engine can't quietly change what these tests expect. A failure here is a spec
//! change, not an implementation detail.

use jis_korean::{Engine, Key, Layout, Mode, Mods, Out, output_in};

/// A Korean pangram: every initial (19), medial (21) and final (27) at least once.
const PANGRAM: &str = "다람쥐 헌 쳇바퀴에 타고파. 외곬으로 앉아 닭값도 없이 넋 놓고 흙을 핥던 젊은이 여덟이, 많이 \
    싫었지만 곧 시를 읊었다. 얘야, 왜 웬 귀뚜라미가 의자 위 쇠붙이 밑에서 우니? 뭐요, 계곡 밖 부엌 앞 밭에 핀 꽃잎이 \
    좋고 숲 속 낮엔 빛이 있다는 것을 유리 입구에서 봐. 쌀 뜨는 아빠는 짜장을 찌고 옷을 꿰매며 뺨을 비볐다.";

const INITIALS: &str = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ";
const MEDIALS: &str = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ";
const FINALS: &str = " ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ";

/// A key press: a `KeyboardEvent.code` and Shift.
type Press = (&'static str, bool);

/// KS X 5002: the key (and Shift) for each jamo that has its own key.
fn key_of(jamo: char) -> Press {
    match jamo {
        'ㄱ' => ("KeyR", false),
        'ㄲ' => ("KeyR", true),
        'ㄴ' => ("KeyS", false),
        'ㄷ' => ("KeyE", false),
        'ㄸ' => ("KeyE", true),
        'ㄹ' => ("KeyF", false),
        'ㅁ' => ("KeyA", false),
        'ㅂ' => ("KeyQ", false),
        'ㅃ' => ("KeyQ", true),
        'ㅅ' => ("KeyT", false),
        'ㅆ' => ("KeyT", true),
        'ㅇ' => ("KeyD", false),
        'ㅈ' => ("KeyW", false),
        'ㅉ' => ("KeyW", true),
        'ㅊ' => ("KeyC", false),
        'ㅋ' => ("KeyZ", false),
        'ㅌ' => ("KeyX", false),
        'ㅍ' => ("KeyV", false),
        'ㅎ' => ("KeyG", false),
        'ㅏ' => ("KeyK", false),
        'ㅐ' => ("KeyO", false),
        'ㅑ' => ("KeyI", false),
        'ㅒ' => ("KeyO", true),
        'ㅓ' => ("KeyJ", false),
        'ㅔ' => ("KeyP", false),
        'ㅕ' => ("KeyU", false),
        'ㅖ' => ("KeyP", true),
        'ㅗ' => ("KeyH", false),
        'ㅛ' => ("KeyY", false),
        'ㅜ' => ("KeyN", false),
        'ㅠ' => ("KeyB", false),
        'ㅡ' => ("KeyM", false),
        'ㅣ' => ("KeyL", false),
        _ => panic!("{jamo} has no key of its own in 2-set"),
    }
}

/// Compound vowels and double finals, typed as two jamo in 2-set.
fn parts(jamo: char) -> Option<(char, char)> {
    Some(match jamo {
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

/// A Hangul syllable's initial, medial and final (if any).
fn split(syllable: char) -> (char, char, Option<char>) {
    let s = syllable as u32 - 0xAC00;
    let pick = |table: &str, i: u32| table.chars().nth(i as usize).unwrap();
    let t = s % 28;
    (pick(INITIALS, s / 588), pick(MEDIALS, s % 588 / 28), (t != 0).then(|| pick(FINALS, t)))
}

fn every_syllable() -> impl Iterator<Item = char> {
    (0xAC00..=0xD7A3).map(|c| char::from_u32(c).unwrap())
}

/// How a typist presses a jamo: the conventional key, or one of the variants below.
#[derive(Clone, Copy)]
enum Style {
    /// KS X 5002 as taught: Shift only for ㄲ ㄸ ㅃ ㅆ ㅉ ㅒ ㅖ.
    Conventional,
    /// Shift held anyway on the consonant keys where 2-set ignores it (Shift+A is still ㅁ): all but
    /// Q W E R T, where Shift types the double, and Z, where Shift+Z is F10 (a spec exception).
    ShiftHeldOnConsonants,
    /// JIS Korean's shortcuts: 1–5 for ㅃ ㅉ ㄸ ㄲ ㅆ, 6–0 for ㅟ ㅝ ㅚ ㅢ ㅘ (spec §3.1). Mode A
    /// and Mode B share the number row.
    DedicatedKeys,
}

fn jamo_presses(jamo: char, style: Style) -> Vec<Press> {
    if let Style::DedicatedKeys = style {
        let dedicated = match jamo {
            'ㅃ' => Some("Digit1"),
            'ㅉ' => Some("Digit2"),
            'ㄸ' => Some("Digit3"),
            'ㄲ' => Some("Digit4"),
            'ㅆ' => Some("Digit5"),
            'ㅟ' => Some("Digit6"),
            'ㅝ' => Some("Digit7"),
            'ㅚ' => Some("Digit8"),
            'ㅢ' => Some("Digit9"),
            'ㅘ' => Some("Digit0"),
            _ => None,
        };
        if let Some(code) = dedicated {
            return vec![(code, false)];
        }
    }
    if let Some((a, b)) = parts(jamo) {
        return [jamo_presses(a, style), jamo_presses(b, style)].concat();
    }
    let (code, shift) = key_of(jamo);
    let shift_ignored = INITIALS.contains(jamo) && !["KeyQ", "KeyW", "KeyE", "KeyR", "KeyT", "KeyZ"].contains(&code);
    let shift = shift || (matches!(style, Style::ShiftHeldOnConsonants) && shift_ignored);
    vec![(code, shift)]
}

fn syllable_presses(syllable: char, style: Style) -> Vec<Press> {
    let (initial, medial, last) = split(syllable);
    let mut presses = jamo_presses(initial, style);
    presses.extend(jamo_presses(medial, style));
    if let Some(last) = last {
        presses.extend(jamo_presses(last, style));
    }
    presses
}

/// The keys for a text: Hangul as `style`, a space as Space, anything else on the mode's key for it.
fn text_presses(text: &str, mode: Mode, style: Style) -> Vec<Press> {
    let mut presses = Vec::new();
    for c in text.chars() {
        match c {
            '가'..='힣' => presses.extend(syllable_presses(c, style)),
            ' ' => presses.push(("Space", false)),
            _ => {
                let press = Key::ALL
                    .iter()
                    .flat_map(|&key| [(key, false), (key, true)])
                    .find(|&(key, shift)| output_in(mode, Layout::JisKorean, key, shift) == Some(Out::Char(c)))
                    .unwrap_or_else(|| panic!("no key types {c:?} in {mode:?}"));
                presses.push((press.0.code(), press.1));
            }
        }
    }
    presses
}

/// Types the presses into a fresh engine, then commits: the text that ends up in the document.
fn typed(mode: Mode, presses: &[Press]) -> String {
    let mut engine = Engine::new();
    engine.set_mode(mode);
    let mut doc = String::new();
    for &(code, shift) in presses {
        let out = engine.key(Key::from_code(code), if shift { Mods::SHIFTED } else { Mods::NONE });
        assert!(out.consumed, "{code} wasn't used");
        doc += &out.commit;
    }
    doc + &engine.flush().commit
}

#[test]
fn the_pangram_covers_every_jamo() {
    let (mut initials, mut medials, mut finals) = (String::new(), String::new(), String::new());
    for c in PANGRAM.chars().filter(|c| ('가'..='힣').contains(c)) {
        let (i, m, f) = split(c);
        initials.push(i);
        medials.push(m);
        finals.extend(f);
    }
    for (name, table, seen) in [("initial", INITIALS, &initials), ("medial", MEDIALS, &medials), ("final", FINALS.trim(), &finals)] {
        for jamo in table.chars() {
            assert!(seen.contains(jamo), "the pangram has no {name} {jamo}");
        }
    }
}

#[test]
fn the_pangram_types_conventionally_in_every_mode() {
    for &mode in Mode::ALL {
        for style in [Style::Conventional, Style::ShiftHeldOnConsonants, Style::DedicatedKeys] {
            let presses = text_presses(PANGRAM, mode, style);
            assert_eq!(typed(mode, &presses), PANGRAM, "{mode:?}");
        }
    }
}

#[test]
fn every_syllable_types_conventionally_in_every_mode() {
    for &mode in Mode::ALL {
        for syllable in every_syllable() {
            for style in [Style::Conventional, Style::ShiftHeldOnConsonants, Style::DedicatedKeys] {
                let presses = syllable_presses(syllable, style);
                assert_eq!(typed(mode, &presses), syllable.to_string(), "{mode:?} {presses:?}");
            }
        }
    }
}

/// The next syllable's consonant must leave the one before intact, even when it can join that
/// syllable's final as a double final (각 + ㅅ → 갃) and is then taken back by the vowel (각사).
#[test]
fn every_syllable_then_every_initial_types_both() {
    for syllable in every_syllable() {
        for initial in INITIALS.chars() {
            let next = char::from_u32(0xAC00 + INITIALS.chars().position(|c| c == initial).unwrap() as u32 * 588).unwrap();
            let presses = [syllable_presses(syllable, Style::Conventional), syllable_presses(next, Style::Conventional)].concat();
            assert_eq!(typed(Mode::A, &presses), format!("{syllable}{next}"), "{presses:?}");
        }
    }
}

/// The spec's own core-principle examples (§1).
#[test]
fn the_core_principle_examples() {
    let presses = |keys: &[Press]| keys.to_vec();
    // Shift+Q is still ㅃ, even though 1 also types ㅃ.
    assert_eq!(typed(Mode::A, &presses(&[("KeyQ", true), ("KeyK", false)])), "빠");
    assert_eq!(typed(Mode::A, &presses(&[("Digit1", false), ("KeyK", false)])), "빠");
    // ㅗ then ㅏ is still ㅘ, even though 0 also types ㅘ.
    assert_eq!(typed(Mode::A, &presses(&[("KeyR", false), ("KeyH", false), ("KeyK", false)])), "과");
    assert_eq!(typed(Mode::A, &presses(&[("KeyR", false), ("Digit0", false)])), "과");
    // Double finals compose as usual: ㄹ+ㄱ → ㄺ.
    assert_eq!(typed(Mode::A, &presses(&[("KeyE", false), ("KeyK", false), ("KeyF", false), ("KeyR", false)])), "닭");
}
