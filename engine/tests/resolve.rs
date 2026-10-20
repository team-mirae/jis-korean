//! Resolving keys and text (spec §8.3), on the deterministic engine. With predictions on, see
//! tests/predict.rs.

use jis_korean::resolve::{MAX_PRESSES, key_input, key_input_after, keys_for_output, type_keys};
use jis_korean::{Dictionary, Engine, Mode};

fn in_mode(mode: Mode) -> Engine {
    let mut e = Engine::new();
    e.set_mode(mode);
    e
}

fn has(found: &[Vec<String>], keys: &str) -> bool {
    found.iter().any(|route| route.join(" ") == keys)
}

#[test]
fn a_key_alone_types_its_character_in_the_mode() {
    let (a, b) = (in_mode(Mode::A), in_mode(Mode::B));
    assert_eq!(key_input(&a, "KeyG").as_deref(), Some("ㅎ"));
    assert_eq!(key_input(&a, "S-KeyQ").as_deref(), Some("ㅃ"));
    assert_eq!(key_input(&a, "Equal").as_deref(), Some("("));
    assert_eq!(key_input(&b, "Equal").as_deref(), Some("="));
    assert_eq!(key_input(&b, "S-BracketLeft").as_deref(), Some("「"));
    // Not characters: F-keys, Space, Shift+Z (F10), unknown keys.
    for press in ["F6", "Space", "S-KeyZ", "Bogus", "", "S-"] {
        assert_eq!(key_input(&a, press), None, "{press}");
    }
    assert_eq!(key_input(&b, "IntlYen"), None);
}

#[test]
fn a_key_after_context_shows_the_whole_field() {
    let e = Engine::new();
    assert_eq!(key_input_after(&e, "하", "KeyS").as_deref(), Some("한"));
    assert_eq!(key_input_after(&e, "한", "KeyK").as_deref(), Some("하나"));
    assert_eq!(key_input_after(&e, "", "KeyK").as_deref(), Some("아")); // the implied ㅇ (spec §4.2)
    assert_eq!(key_input_after(&e, "한", "Space").as_deref(), Some("한 "));
    assert_eq!(key_input_after(&e, "한", "F10").as_deref(), Some("gks"));
    assert_eq!(key_input_after(&e, "한", "Nope"), None);
    assert_eq!(key_input_after(&in_mode(Mode::B), "한", "F8").as_deref(), Some("한"));
}

#[test]
fn keys_for_a_character() {
    let e = Engine::new();
    let found = keys_for_output(&e, "ㅃ", 1, 1);
    assert!(has(&found, "Digit1") && has(&found, "S-KeyQ"), "{found:?}");
    assert!(found.iter().all(|route| route.len() == 1));
    // Fewest presses, then fewest Shifts, first.
    assert_eq!(found[0], ["Digit1"]);
}

#[test]
fn keys_for_a_word_within_the_range() {
    let e = Engine::new();
    let found = keys_for_output(&e, "한", 3, 3);
    assert!(has(&found, "KeyG KeyK KeyS"), "{found:?}");
    assert!(keys_for_output(&e, "한", 1, 2).is_empty());
    // F10 after the Hangul keys converts to letters.
    assert!(has(&keys_for_output(&e, "gks", 4, 4), "KeyG KeyK KeyS F10"));
}

#[test]
fn keys_depend_on_the_mode() {
    // " is the F8 escape's Shift+2 on JIS; @ on US ANSI.
    assert!(has(&keys_for_output(&in_mode(Mode::A), "\"", 2, 2), "F8 S-Digit2"));
    assert!(has(&keys_for_output(&in_mode(Mode::B), "@", 2, 2), "F8 S-Digit2"));
    assert!(!has(&keys_for_output(&in_mode(Mode::B), "\"", 2, 2), "F8 S-Digit2"));
}

#[test]
fn every_combination_types_the_text() {
    for e in [in_mode(Mode::A), in_mode(Mode::B)] {
        for text in ["까", "와", "읽", "한 ", "ㅠㅠ", "1", "가!"] {
            let found = keys_for_output(&e, text, 1, 4);
            assert!(!found.is_empty(), "{text}");
            for route in found {
                assert!((1..=4).contains(&route.len()));
                assert_eq!(type_keys(&e, &route).as_deref(), Some(text), "{route:?}");
            }
        }
    }
}

#[test]
fn hanja_through_f7() {
    let mut e = Engine::new();
    let tsv = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/dict/hanja.tsv")).unwrap();
    e.set_dictionary(std::sync::Arc::new(Dictionary::parse(&tsv)));
    // 학교 (G K R R Y), then F7: its top hanja.
    let found = keys_for_output(&e, "學校", 6, 6);
    assert!(has(&found, "KeyG KeyK KeyR KeyR KeyY F7"), "{found:?}");
}

#[test]
fn nonsensical_ranges_give_nothing() {
    let e = Engine::new();
    for (min, max) in [(0, 3), (-1, 3), (3, 2), (-5, -1), (0, 0)] {
        assert!(keys_for_output(&e, "한", min, max).is_empty(), "{min}..{max}");
    }
    assert!(keys_for_output(&e, "", 1, 3).is_empty());
    // A huge max is cut to MAX_PRESSES, not refused.
    assert!(has(&keys_for_output(&e, "한", 3, i64::MAX), "KeyG KeyK KeyS"));
    assert!(keys_for_output(&e, "한", MAX_PRESSES + 1, i64::MAX).is_empty());
}
