//! The mode contract (spec §3.6, docs/modes.md), checked for every mode, and Mode B's own examples.

use jis_korean::{Engine, Key, Layout, Mode, Mods, Out, output_in};

const LETTERS: &str = "QWERTYUIOPASDFGHJKLZXCVBNM";

fn letter_keys() -> impl Iterator<Item = Key> {
    LETTERS.chars().map(|c| Key::from_code(&format!("Key{c}")))
}

#[test]
fn ids_are_unique_and_round_trip() {
    for (i, mode) in Mode::ALL.iter().enumerate() {
        assert_eq!(Mode::from_id(mode.id()), *mode);
        assert!(Mode::ALL[..i].iter().all(|m| m.id() != mode.id() && m.name() != mode.name()), "{mode:?} reuses an id or name");
        assert!(!mode.name().is_empty());
    }
    assert_eq!(Mode::from_id(u32::MAX), Mode::A, "unknown ids fall back to Mode A");
    assert_eq!(Mode::default(), Mode::A);
}

#[test]
fn every_mode_keeps_standard_two_set_on_the_letter_keys() {
    for &mode in Mode::ALL {
        for layout in [Layout::JisKorean, Layout::Escaped] {
            for key in letter_keys() {
                for shift in [false, true] {
                    assert_eq!(output_in(mode, layout, key, shift), output_in(Mode::A, Layout::JisKorean, key, shift), "{mode:?} {key:?}");
                }
            }
        }
    }
}

#[test]
fn every_mode_has_a_stock_reading_for_each_key_it_maps() {
    for &mode in Mode::ALL {
        for &key in Key::ALL {
            for shift in [false, true] {
                if let Some(Out::Char(_)) = output_in(mode, Layout::JisKorean, key, shift) {
                    assert!(matches!(output_in(mode, Layout::Escaped, key, shift), Some(Out::Char(_))), "{mode:?} {key:?} has no stock character");
                }
            }
        }
    }
}

#[test]
fn every_mode_leaves_the_control_keys_alone() {
    for &mode in Mode::ALL {
        for key in [Key::Space, Key::Enter, Key::Backspace, Key::F6, Key::F10, Key::Escape, Key::Modifier, Key::Other] {
            for layout in [Layout::JisKorean, Layout::Escaped] {
                assert_eq!(output_in(mode, layout, key, false), None, "{mode:?} {key:?}");
            }
        }
    }
}

// Mode B (spec §3.6)

fn mode_b(keys: &str) -> Engine {
    let mut e = Engine::new();
    e.set_mode(Mode::B);
    for token in keys.split_whitespace() {
        let (mods, code) = match token.strip_prefix("S-") {
            Some(code) => (Mods::SHIFTED, code),
            None => (Mods::NONE, token),
        };
        e.key(Key::from_code(code), mods);
    }
    e
}

#[test]
fn mode_b_number_row_is_mode_a_and_reads_as_us() {
    let e = mode_b("Digit2 S-Digit2 Digit0 S-Digit0");
    assert_eq!(e.preedit(), "ㅉ2와0"); // ㅘ gets the implied ㅇ (spec §4.2), as in Mode A
    // Shift+2 reads @ on US ANSI, not " as on JIS.
    assert_eq!(e.english(), "2@0)");
}

#[test]
fn mode_b_brackets_keep_corner_brackets_on_shift() {
    let e = mode_b("BracketLeft S-BracketLeft BracketRight S-BracketRight");
    assert_eq!(e.preedit(), "[「]」");
    assert_eq!(e.english(), "[{]}");
}

#[test]
fn mode_b_other_symbols_are_us_ansi() {
    let e = mode_b("Backquote S-Backquote Minus S-Minus Equal S-Equal Backslash S-Backslash Semicolon S-Semicolon Quote S-Quote Comma S-Comma Period S-Period Slash S-Slash");
    assert_eq!(e.preedit(), "`~-_=+\\|;:'\",<.>/?");
    assert_eq!(e.english(), e.preedit());
}

#[test]
fn mode_b_shift_z_is_f10() {
    // 한 (G K S), then Shift+Z converts it as F10 does.
    assert_eq!(mode_b("KeyG KeyK KeyS S-KeyZ").preedit(), "gks");
}

#[test]
fn mode_b_escape_is_standard_two_set_on_us_keys() {
    let e = mode_b("F8 KeyG KeyK S-Digit2 BracketLeft");
    assert_eq!(e.preedit(), "하@[");
}

#[test]
fn jis_only_keys_pass_through_in_mode_b_and_backquote_in_mode_a() {
    let mut e = Engine::new();
    e.set_mode(Mode::B);
    for key in [Key::IntlYen, Key::IntlRo] {
        assert!(!e.key(key, Mods::NONE).consumed, "{key:?}");
    }
    let mut a = Engine::new();
    assert!(!a.key(Key::Backquote, Mods::NONE).consumed);
}

#[test]
fn mode_a_is_unchanged() {
    let mut e = Engine::new();
    for (code, shift) in [("Digit2", true), ("BracketLeft", false), ("Equal", false)] {
        e.key(Key::from_code(code), if shift { Mods::SHIFTED } else { Mods::NONE });
    }
    assert_eq!((e.preedit().as_str(), e.english().as_str()), ("2'(", "\"@^"));
}
