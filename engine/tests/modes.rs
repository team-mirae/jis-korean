//! The mode contract (spec §3.6, docs/modes.md), checked for every mode, and Modes B and C's own
//! examples.

use jis_korean::{Engine, Key, Layout, Mode, Mods, Out, output_in, output_with};

const LETTERS: &str = "QWERTYUIOPASDFGHJKLZXCVBNM";

fn letter_keys() -> impl Iterator<Item = Key> {
    LETTERS.chars().map(|c| Key::from_code(&format!("Key{c}")))
}

#[test]
fn ids_are_unique_and_round_trip() {
    for (i, mode) in Mode::ALL.iter().enumerate() {
        assert_eq!(Mode::from_id(mode.id()), Some(*mode));
        assert!(
            Mode::ALL[..i].iter().all(|m| m.id() != mode.id() && m.name() != mode.name() && m.code() != mode.code()),
            "{mode:?} reuses an id, a name or a mode name"
        );
        assert!(!mode.name().is_empty());
    }
    #[cfg(not(feature = "english"))]
    assert_eq!(Mode::from_id(3), None, "JIS English is built only with the english feature");
    for id in [4, 9, 31, 99, 10_000, u32::MAX] {
        assert_eq!(Mode::from_id(id), None, "id {id} has no mode in this build: it's refused, not replaced");
    }
    assert_eq!(Mode::default(), Mode::A);
}

/// Spec §3.6.1: mode names are upper-case snake case, and each namespace has its id range.
#[test]
fn mode_names_and_ids_are_in_their_ranges() {
    for &mode in Mode::ALL {
        let (id, code) = (mode.id(), mode.code());
        let words: Vec<&str> = code.split('_').collect();
        assert!(
            words.iter().all(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()))
                && code.starts_with(|c: char| c.is_ascii_uppercase()),
            "{code} isn't upper-case snake case"
        );
        let in_range = match words.as_slice() {
            _ if (10_000..=11_000).contains(&id) => true, // a preview's temporary id
            [_] => id <= 2,                              // A, B and C keep their letters
            ["A", ..] => id <= 14,
            ["B" | "C", ..] => (15..=30).contains(&id),
            _ => (31..=99).contains(&id),
        };
        assert!(in_range, "{code} can't have id {id}");
    }
}

/// Mode C (id 2) is the ISO mode: no other mode may take its id or letter.
#[test]
fn mode_c_is_the_iso_mode() {
    for &mode in Mode::ALL {
        let is_c = mode.id() == 2 || format!("{mode:?}") == "C";
        assert!(!is_c || mode.name().contains("ISO"), "{mode:?} ({}) takes Mode C, reserved for ISO", mode.name());
    }
}

#[test]
fn every_mode_keeps_standard_two_set_on_the_letter_keys() {
    for &mode in Mode::ALL.iter().filter(|m| m.is_korean()) {
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
fn every_mode_keeps_standard_two_set_with_its_symbol_keys_off() {
    for &mode in Mode::ALL.iter().filter(|m| m.is_korean()) {
        for key in letter_keys() {
            for shift in [false, true] {
                assert_eq!(output_with(mode, false, Layout::JisKorean, key, shift), output_in(Mode::A, Layout::JisKorean, key, shift), "{mode:?} {key:?}");
            }
        }
    }
}

#[test]
fn optional_keys_type_stock_when_off_and_nothing_else_changes() {
    for &mode in Mode::ALL.iter().filter(|m| m.is_korean()) {
        for &key in Key::ALL {
            for shift in [false, true] {
                let off = output_with(mode, false, Layout::JisKorean, key, shift);
                if mode.optional_keys().contains(&key) {
                    // Off, an optional key types the stock keyboard's character, as F8 would.
                    assert_eq!(off, output_in(mode, Layout::Escaped, key, shift), "{mode:?} {key:?}");
                    assert!(matches!(off, Some(Out::Char(_))), "{mode:?} {key:?} has no stock character");
                } else {
                    assert_eq!(off, output_in(mode, Layout::JisKorean, key, shift), "{mode:?} {key:?}");
                }
                // The F8 escape is the stock keyboard either way.
                assert_eq!(output_with(mode, false, Layout::Escaped, key, shift), output_in(mode, Layout::Escaped, key, shift));
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

// Mode C (spec §3.6)

fn mode_c(keys: &str, symbols: bool) -> Engine {
    let mut e = Engine::new();
    e.set_mode(Mode::C);
    e.set_dedicated_symbols(symbols);
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
fn mode_c_number_row_is_mode_a_and_reads_as_uk() {
    let e = mode_c("Digit2 S-Digit2 Digit3 S-Digit3", true);
    assert_eq!(e.preedit(), "ㅉ2ㄸ3");
    // Shift+2 and Shift+3 read " and £ on UK ISO.
    assert_eq!(e.english(), "2\"3£");
}

#[test]
fn mode_c_extra_key_types_middle_dot_and_em_dash() {
    let e = mode_c("IntlBackslash S-IntlBackslash", true);
    assert_eq!(e.preedit(), "·—");
    assert_eq!(e.english(), "\\|");
}

#[test]
fn mode_c_brackets_keep_corner_brackets_on_shift() {
    let e = mode_c("BracketLeft S-BracketLeft BracketRight S-BracketRight", true);
    assert_eq!(e.preedit(), "[「]」");
}

#[test]
fn mode_c_other_symbols_are_uk_iso() {
    let e = mode_c("Backquote S-Backquote Minus S-Minus Equal S-Equal Semicolon S-Semicolon Quote S-Quote Backslash S-Backslash Comma S-Comma Period S-Period Slash S-Slash", true);
    assert_eq!(e.preedit(), "`¬-_=+;:'@#~,<.>/?");
}

#[test]
fn mode_c_symbol_keys_off_type_uk() {
    // Off, [ ] and the extra key type UK ISO's own characters, not the dedicated ones.
    let e = mode_c("KeyG KeyK BracketLeft S-BracketLeft BracketRight S-BracketRight IntlBackslash S-IntlBackslash", false);
    assert_eq!(e.preedit(), "하[{]}\\|");
    assert_eq!(e.english(), "gk[{]}\\|");
    // Everything else is as with them on.
    let e = mode_c("Digit1 KeyK S-Digit1 Minus", false);
    assert_eq!(e.preedit(), "빠1-");
}

#[test]
fn mode_c_shift_z_is_f10_and_escape_is_uk() {
    assert_eq!(mode_c("KeyG KeyK KeyS S-KeyZ", true).preedit(), "gks");
    assert_eq!(mode_c("F8 KeyG KeyK S-Digit3 IntlBackslash BracketLeft", true).preedit(), "하£\\[");
}

#[test]
fn mode_c_keys_pass_through_in_other_modes() {
    for mode in [Mode::A, Mode::B] {
        let mut e = Engine::new();
        e.set_mode(mode);
        assert!(!e.key(Key::IntlBackslash, Mods::NONE).consumed, "{mode:?}");
    }
    let mut c = Engine::new();
    c.set_mode(Mode::C);
    for key in [Key::IntlYen, Key::IntlRo] {
        assert!(!c.key(key, Mods::NONE).consumed, "{key:?}");
    }
}

/// Spec §3.6.1 over the C ABI: modes are listed by position, and an unknown id is refused.
#[test]
fn c_abi_lists_modes_by_position_and_refuses_unknown_ids() {
    use jis_korean::ffi::*;
    let ids: Vec<u32> = (0..jk_mode_count()).map(|i| jk_mode_id_at(i)).collect();
    assert_eq!(ids, Mode::ALL.iter().map(|m| m.id()).collect::<Vec<_>>());
    assert_eq!(jk_mode_id_at(jk_mode_count()), u32::MAX);
    assert_eq!(jk_mode_code_len(2), 1);
    assert_eq!((jk_mode_name_len(9_999), jk_mode_code_len(9_999)), (0, 0));
    let id = |key: Key| Key::ALL.iter().position(|&k| k == key).unwrap() as u32;
    assert_ne!(jk_legend_in(id(Key::Digit1), 0, 0, 1), 0);
    assert_eq!(jk_legend_in(id(Key::Digit1), 0, 0, 4), 0);

    let engine = jk_new();
    assert_eq!(jk_set_mode(engine, 1), 1);
    assert_eq!(jk_set_mode(engine, 4), 0, "no mode 4 in this build");
    // Still Mode B: Shift+[ is 「 there.
    jk_key(engine, id(Key::BracketLeft), Mods::SHIFT);
    assert_eq!(unsafe { std::slice::from_raw_parts(jk_preedit_ptr(engine), jk_preedit_len(engine)) }, "「".as_bytes());
    unsafe { jk_free(engine) };
}
