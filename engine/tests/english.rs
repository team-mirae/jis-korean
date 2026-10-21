//! JIS English, mode `A_QWERTY` (spec §3.6.2), and its press and hold (spec §3.6.3). Built with the
//! `english` feature: `cargo test --features english`.
#![cfg(feature = "english")]

use jis_korean::{Engine, Key, Layout, Mode, Mods, Out, Output, output_in};

const REPEAT: Mods = Mods { repeat: true, ..Mods::NONE };
const SHIFT_REPEAT: Mods = Mods { shift: true, repeat: true, ..REPEAT };
const CAPS: Mods = Mods { caps: true, ..Mods::NONE };

fn english() -> Engine {
    let mut engine = Engine::new();
    engine.set_mode(Mode::from_id(3).expect("mode 3 is built with the english feature"));
    engine
}

fn key(code: &str) -> Key {
    Key::from_code(code)
}

/// Types `code` and holds it until the list opens (one auto-repeat).
fn hold(engine: &mut Engine, code: &str, shift: bool) -> Output {
    let typed = engine.key(key(code), if shift { Mods::SHIFTED } else { Mods::NONE });
    assert!(!typed.consumed, "{code} goes to the app first");
    engine.key(key(code), if shift { SHIFT_REPEAT } else { REPEAT })
}

fn list(out: &Output) -> Vec<String> {
    out.candidates.as_ref().map_or(Vec::new(), |c| c.items.iter().map(|i| i.hanja.clone()).collect())
}

#[test]
fn it_is_mode_3_a_qwerty_and_not_korean() {
    let mode = Mode::from_id(3).unwrap();
    assert_eq!((mode.id(), mode.code(), mode.name(), mode.is_korean()), (3, "A_QWERTY", "JIS English", false));
}

#[test]
fn every_key_goes_to_the_app() {
    let mut engine = english();
    // All but F9 and F10 (and Shift+Z), which convert the word just typed (spec §3.6.4).
    for code in ["KeyG", "KeyK", "KeyS", "Digit2", "BracketLeft", "IntlYen", "IntlRo", "Space", "Enter", "Backspace", "F6", "F7", "F8"] {
        for mods in [Mods::NONE, Mods::SHIFTED] {
            let out = engine.key(key(code), mods);
            assert!(!out.consumed && out.commit.is_empty() && out.preedit.is_empty(), "{code} {mods:?}: {out:?}");
        }
    }
}

/// The legends the macOS keyboard layout is generated from: JIS English, Shift+Z is just Z.
#[test]
fn it_types_jis_english() {
    let mode = Mode::from_id(3).unwrap();
    let typed = |code: &str, shift: bool| output_in(mode, Layout::JisKorean, key(code), shift).map(Out::char);
    assert_eq!(typed("KeyG", false), Some('g'));
    assert_eq!(typed("KeyG", true), Some('G'));
    assert_eq!(typed("KeyZ", true), Some('Z'));
    assert_eq!(typed("Digit2", true), Some('"'));
    assert_eq!(typed("Digit7", true), Some('\''));
    assert_eq!(typed("BracketLeft", false), Some('@'));
    assert_eq!(typed("IntlYen", false), Some('¥'));
    assert_eq!(typed("IntlRo", false), Some('\\'));
}

#[test]
fn switching_in_commits_what_korean_left_unconfirmed() {
    let mut engine = Engine::new();
    engine.key(key("KeyG"), Mods::NONE);
    engine.key(key("KeyK"), Mods::NONE);
    engine.set_mode(Mode::from_id(3).unwrap());
    let out = engine.key(key("KeyA"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("하", false));
}

#[test]
fn holding_u_offers_u_umlaut() {
    let mut engine = english();
    let out = hold(&mut engine, "KeyU", false);
    assert!(out.consumed);
    assert_eq!(out.take_back, 1, "the u already typed is taken back");
    assert_eq!(out.preedit, "u");
    assert_eq!(list(&out), ["û", "ü", "ù", "ú", "ū"]);
    assert_eq!(out.candidates.as_ref().unwrap().selected, 5, "none selected");
    // Still held: nothing more, and nothing taken back again.
    let out = engine.key(key("KeyU"), REPEAT);
    assert!(out.consumed);
    assert_eq!((out.take_back, out.preedit.as_str()), (0, "u"));
    let out = engine.key(key("Digit2"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed, out.preedit.as_str()), ("ü", true, ""));
    assert!(out.candidates.is_none());
}

#[test]
fn shift_holds_offer_capitals() {
    let mut engine = english();
    let out = hold(&mut engine, "KeyU", true);
    assert_eq!(out.preedit, "U");
    assert_eq!(list(&out)[1], "Ü");
    assert_eq!(engine.key(key("Digit2"), Mods::NONE).commit, "Ü");
}

#[test]
fn arrows_choose_and_enter_commits() {
    let mut engine = english();
    hold(&mut engine, "KeyE", false);
    let out = engine.key(key("ArrowRight"), Mods::NONE);
    assert_eq!((out.preedit.as_str(), out.candidates.as_ref().unwrap().selected), ("è", 0));
    let out = engine.key(key("ArrowLeft"), Mods::NONE);
    assert_eq!(out.preedit, "ę", "left wraps to the last");
    let out = engine.key(key("Enter"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("ę", true));
    // From nothing selected, left goes to the last.
    hold(&mut engine, "KeyE", false);
    assert_eq!(engine.key(key("ArrowUp"), Mods::NONE).preedit, "ę");
}

#[test]
fn escape_keeps_the_letter() {
    let mut engine = english();
    hold(&mut engine, "KeyE", false);
    engine.key(key("ArrowRight"), Mods::NONE);
    let out = engine.key(key("Escape"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("e", true));
}

#[test]
fn any_other_key_keeps_what_shows_and_goes_to_the_app() {
    let mut engine = english();
    hold(&mut engine, "KeyA", false);
    let out = engine.key(key("KeyB"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("a", false));
    hold(&mut engine, "KeyA", false);
    engine.key(key("ArrowRight"), Mods::NONE);
    let out = engine.key(key("Space"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("à", false));
    // Enter with nothing chosen: the letter, and Enter goes on.
    hold(&mut engine, "KeyA", false);
    let out = engine.key(key("Enter"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("a", false));
    // A digit past the list: the letter, then the digit.
    hold(&mut engine, "KeyL", false);
    let out = engine.key(key("Digit5"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("l", false));
    // A shortcut: the letter, then the shortcut.
    hold(&mut engine, "KeyL", false);
    let cmd = Mods { meta: true, ..Mods::NONE };
    let out = engine.key(key("KeyC"), cmd);
    assert_eq!((out.commit.as_str(), out.consumed), ("l", false));
}

#[test]
fn backspace_removes_the_letter() {
    let mut engine = english();
    hold(&mut engine, "KeyO", false);
    let out = engine.key(key("Backspace"), Mods::NONE);
    assert!(out.consumed && out.commit.is_empty() && out.preedit.is_empty() && out.candidates.is_none());
}

#[test]
fn flush_keeps_what_shows() {
    let mut engine = english();
    hold(&mut engine, "KeyN", false);
    engine.key(key("ArrowDown"), Mods::NONE);
    assert_eq!(engine.flush().commit, "ñ");
}

#[test]
fn other_held_keys_repeat_as_usual() {
    let mut engine = english();
    // No variants: the repeat goes to the app.
    engine.key(key("KeyB"), Mods::NONE);
    assert!(!engine.key(key("KeyB"), REPEAT).consumed);
    assert!(!engine.key(key("Digit1"), REPEAT).consumed);
    // A repeat of a key that wasn't the last one pressed, or with Cmd, opens nothing.
    engine.key(key("KeyX"), Mods::NONE);
    assert!(!engine.key(key("KeyU"), REPEAT).consumed);
    let cmd = Mods { meta: true, ..Mods::NONE };
    engine.key(key("KeyU"), cmd);
    assert!(!engine.key(key("KeyU"), Mods { repeat: true, ..cmd }).consumed);
    // Shift changed between the press and the repeat: the press doesn't count.
    engine.key(key("KeyU"), Mods::NONE);
    assert!(!engine.key(key("KeyU"), SHIFT_REPEAT).consumed);
}

/// The Korean modes don't read the repeat flag: a held ㅕ repeats as before.
#[test]
fn korean_modes_ignore_the_repeat() {
    let mut engine = Engine::new();
    engine.key(key("KeyU"), Mods::NONE);
    let out = engine.key(key("KeyU"), REPEAT);
    assert!(out.consumed && out.take_back == 0 && out.candidates.is_none());
    assert_eq!(out.preedit, "여여");
}

fn typed(engine: &mut Engine, codes: &[&str]) {
    for code in codes {
        assert!(!engine.key(key(code), Mods::NONE).consumed, "{code} goes to the app");
    }
}

fn shift_z(engine: &mut Engine) -> Output {
    engine.key(key("KeyZ"), Mods::SHIFTED)
}

/// Spec §3.6.4: x, then Shift+Z, is X: the x already typed is taken back as X.
#[test]
fn shift_z_capitalizes_the_word_just_typed() {
    let mut engine = english();
    typed(&mut engine, &["KeyX"]);
    let out = shift_z(&mut engine);
    assert_eq!((out.take_back, out.preedit.as_str(), out.consumed), (1, "X", true));
    let out = engine.key(key("Space"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("X", false), "kept, and the space goes on");
}

#[test]
fn f10_cycles_upper_capitalized_as_typed_and_f9_is_full_width() {
    let mut engine = english();
    typed(&mut engine, &["KeyA", "KeyB", "KeyC"]);
    let out = engine.key(key("F10"), Mods::NONE);
    assert_eq!((out.take_back, out.preedit.as_str()), (3, "ABC"));
    let out = engine.key(key("F10"), Mods::NONE);
    assert_eq!((out.take_back, out.preedit.as_str()), (0, "Abc"));
    assert_eq!(shift_z(&mut engine).preedit, "abc");
    assert_eq!(engine.key(key("F9"), Mods::NONE).preedit, "ａｂｃ");
    assert_eq!(engine.key(key("F9"), Mods::NONE).preedit, "ＡＢＣ");
    // Esc: the word as typed.
    let out = engine.key(key("Escape"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("abc", true));
}

#[test]
fn f9_first_converts_as_typed() {
    let mut engine = english();
    typed(&mut engine, &["KeyA", "Digit1"]);
    let out = engine.key(key("F9"), Mods::NONE);
    assert_eq!((out.take_back, out.preedit.as_str()), (2, "ａ１"));
}

#[test]
fn the_word_is_what_the_app_has() {
    let mut engine = english();
    // Backspace shortens it; digits are part of it.
    typed(&mut engine, &["KeyA", "KeyB", "KeyC", "Backspace", "Digit1"]);
    assert_eq!(engine.key(key("F10"), Mods::NONE).preedit, "AB1");
    // A space, a symbol or a shortcut ends it.
    for end in [("Space", Mods::NONE), ("Digit2", Mods::SHIFTED), ("KeyV", Mods { meta: true, ..Mods::NONE })] {
        let mut engine = english();
        typed(&mut engine, &["KeyA"]);
        engine.key(key(end.0), end.1);
        let out = engine.key(key("F10"), Mods::NONE);
        assert_eq!((out.consumed, out.take_back, out.preedit.as_str()), (true, 0, ""), "after {}", end.0);
    }
    // So does anything that moves the caret elsewhere.
    let mut engine = english();
    typed(&mut engine, &["KeyA"]);
    engine.flush();
    assert_eq!(engine.key(key("F10"), Mods::NONE).take_back, 0);
    // With nothing typed, Shift+Z does nothing (and isn't typed).
    let mut engine = english();
    let out = shift_z(&mut engine);
    assert!(out.consumed && out.commit.is_empty() && out.preedit.is_empty());
}

#[test]
fn caps_lock_is_the_case_the_app_typed() {
    let mut engine = english();
    for code in ["KeyA", "KeyB"] {
        engine.key(key(code), CAPS);
    }
    assert_eq!(engine.key(key("F10"), Mods::NONE).preedit, "AB");
    assert_eq!(engine.key(key("F10"), Mods::NONE).preedit, "Ab");
    // Held with Caps Lock: capitals.
    let mut engine = english();
    engine.key(key("KeyU"), CAPS);
    let out = engine.key(key("KeyU"), Mods { repeat: true, ..CAPS });
    assert_eq!((out.preedit.as_str(), list(&out)[1].as_str()), ("U", "Ü"));
}

#[test]
fn f10_after_a_press_and_hold_takes_the_letter_too() {
    let mut engine = english();
    typed(&mut engine, &["KeyA", "KeyB"]);
    hold(&mut engine, "KeyU", false);
    let out = engine.key(key("F10"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.take_back, out.preedit.as_str()), ("u", 3, "ABU"));
}
