//! The English modes (spec §3.6.2) and their press and hold (spec §3.6.3). Built with the
//! `english` feature: `cargo test --features english`.
#![cfg(feature = "english")]

use jis_korean::{Engine, Key, Layout, Mode, Mods, Out, Output, output_in};

const REPEAT: Mods = Mods { repeat: true, ..Mods::NONE };
const OPTION: Mods = Mods { alt: true, ..Mods::NONE };
const CAPS: Mods = Mods { caps: true, ..Mods::NONE };

fn english() -> Engine {
    let mut engine = Engine::new();
    engine.set_mode(Mode::from_id(3).expect("mode 3 is built with the english feature"));
    engine
}

fn key(code: &str) -> Key {
    Key::from_code(code)
}

/// ⌥ + `code` (with Shift if `shift`), which opens the letter's variants list.
fn option(engine: &mut Engine, code: &str, shift: bool) -> Output {
    let out = engine.key(key(code), Mods { shift, ..OPTION });
    assert!(out.consumed, "⌥{code} opens its list");
    out
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
    // Shift+Z is Z and F9/F10 go to the app too: the English modes have no conversions.
    for code in ["KeyG", "KeyK", "KeyS", "KeyZ", "KeyX", "Digit2", "BracketLeft", "IntlYen", "IntlRo", "Space", "Enter", "Backspace", "F6", "F7", "F8", "F9", "F10"] {
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
fn option_u_offers_u_umlaut_without_typing_u() {
    let mut engine = english();
    let out = option(&mut engine, "KeyU", false);
    assert!(out.commit.is_empty(), "nothing is typed before a pick");
    assert_eq!(out.preedit, "u");
    assert_eq!(list(&out), ["û", "ü", "ù", "ú", "ū"]);
    assert_eq!(out.candidates.as_ref().unwrap().selected, 5, "none selected");
    // ⌥U's auto-repeat: nothing more.
    let out = engine.key(key("KeyU"), Mods { repeat: true, ..OPTION });
    assert_eq!((out.consumed, out.preedit.as_str()), (true, "u"));
    let out = engine.key(key("Digit2"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed, out.preedit.as_str()), ("ü", true, ""));
    assert!(out.candidates.is_none());
}

#[test]
fn option_shift_offers_capitals() {
    let mut engine = english();
    let out = option(&mut engine, "KeyU", true);
    assert_eq!(out.preedit, "U");
    assert_eq!(list(&out)[1], "Ü");
    assert_eq!(engine.key(key("Digit2"), Mods::NONE).commit, "Ü");
}

#[test]
fn digits_1_to_0_pick_the_first_ten() {
    let mut engine = english();
    option(&mut engine, "KeyA", false);
    assert_eq!(engine.key(key("Digit8"), Mods::NONE).commit, "ā");
    // 0 is the tenth: past a's eight, it keeps the letter and is typed after it.
    option(&mut engine, "KeyA", false);
    let out = engine.key(key("Digit0"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("a", false));
    option(&mut engine, "KeyL", false);
    let out = engine.key(key("Digit2"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("l", false));
}

#[test]
fn the_same_option_letter_again_steps_through() {
    let mut engine = english();
    option(&mut engine, "KeyE", false);
    assert_eq!(option(&mut engine, "KeyE", false).preedit, "è");
    assert_eq!(option(&mut engine, "KeyE", false).preedit, "é");
    assert_eq!(engine.key(key("Enter"), Mods::NONE).commit, "é");
}

#[test]
fn arrows_choose_and_enter_commits() {
    let mut engine = english();
    option(&mut engine, "KeyE", false);
    let out = engine.key(key("ArrowRight"), Mods::NONE);
    assert_eq!((out.preedit.as_str(), out.candidates.as_ref().unwrap().selected), ("è", 0));
    let out = engine.key(key("ArrowLeft"), Mods::NONE);
    assert_eq!(out.preedit, "ę", "left wraps to the last");
    let out = engine.key(key("Enter"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("ę", true));
    // From nothing selected, left goes to the last.
    option(&mut engine, "KeyE", false);
    assert_eq!(engine.key(key("ArrowUp"), Mods::NONE).preedit, "ę");
    // Enter with nothing chosen: the letter, and Enter stops there.
    engine.key(key("Escape"), Mods::NONE);
    option(&mut engine, "KeyE", false);
    let out = engine.key(key("Enter"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("e", true));
}

#[test]
fn escape_and_backspace_leave_nothing() {
    let mut engine = english();
    for code in ["Escape", "Backspace"] {
        option(&mut engine, "KeyE", false);
        engine.key(key("ArrowRight"), Mods::NONE);
        let out = engine.key(key(code), Mods::NONE);
        assert!(out.consumed && out.commit.is_empty() && out.preedit.is_empty() && out.candidates.is_none(), "{code}");
    }
}

#[test]
fn any_other_key_keeps_what_shows_and_goes_to_the_app() {
    let mut engine = english();
    option(&mut engine, "KeyA", false);
    let out = engine.key(key("KeyB"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("a", false));
    option(&mut engine, "KeyA", false);
    engine.key(key("ArrowRight"), Mods::NONE);
    let out = engine.key(key("Space"), Mods::NONE);
    assert_eq!((out.commit.as_str(), out.consumed), ("à", false));
    // A shortcut: the letter, then the shortcut.
    option(&mut engine, "KeyL", false);
    let cmd = Mods { meta: true, ..Mods::NONE };
    let out = engine.key(key("KeyC"), cmd);
    assert_eq!((out.commit.as_str(), out.consumed), ("l", false));
    // Another ⌥ + letter: the letter, and its own list.
    option(&mut engine, "KeyO", false);
    let out = option(&mut engine, "KeyN", false);
    assert_eq!((out.commit.as_str(), out.preedit.as_str(), list(&out)[0].as_str()), ("o", "n", "ñ"));
}

#[test]
fn flush_keeps_what_shows() {
    let mut engine = english();
    option(&mut engine, "KeyN", false);
    engine.key(key("ArrowDown"), Mods::NONE);
    assert_eq!(engine.flush().commit, "ñ");
}

#[test]
fn a_list_left_open_by_a_mode_switch_keeps_its_letter_before_the_korean() {
    // The host switched to Mode A without a flush while the list was open: the letter stays
    // where it was, and the Korean typed after it is neither hidden nor lost on commit.
    let mut engine = english();
    option(&mut engine, "KeyE", false);
    engine.set_mode(Mode::A);
    for code in ["KeyG", "KeyK"] {
        engine.key(key(code), Mods::NONE);
    }
    assert_eq!(engine.key(key("KeyS"), Mods::NONE).preedit, "e한");
    let out = engine.key(key("Space"), Mods::NONE);
    assert_eq!(out.commit, "e한 ");
    assert!(engine.preedit().is_empty());
    assert_eq!(engine.key(key("KeyK"), Mods::NONE).preedit, "아");
}

#[test]
fn held_keys_repeat_as_usual() {
    let mut engine = english();
    // No press and hold: a held letter's repeats go to the app.
    engine.key(key("KeyU"), Mods::NONE);
    assert!(!engine.key(key("KeyU"), REPEAT).consumed);
    // ⌥ on a letter without variants, or with ⌘ or Ctrl, goes to the app (ABC's ⌥ layer, shortcuts).
    for mods in [OPTION, Mods { meta: true, ..OPTION }, Mods { ctrl: true, ..OPTION }] {
        assert!(!engine.key(key("KeyB"), mods).consumed);
    }
    assert!(!engine.key(key("KeyU"), Mods { meta: true, ..OPTION }).consumed);
}

/// The Korean modes don't read the repeat flag or ⌥: a held ㅕ repeats as before.
#[test]
fn korean_modes_ignore_the_repeat_and_option() {
    let mut engine = Engine::new();
    engine.key(key("KeyU"), Mods::NONE);
    let out = engine.key(key("KeyU"), REPEAT);
    assert!(out.consumed && out.candidates.is_none());
    assert_eq!(out.preedit, "여여");
    assert!(engine.key(key("KeyE"), OPTION).candidates.is_none());
}

#[test]
fn caps_lock_offers_capitals() {
    let mut engine = english();
    let out = engine.key(key("KeyU"), Mods { alt: true, ..CAPS });
    assert_eq!((out.preedit.as_str(), list(&out)[1].as_str()), ("U", "Ü"));
}

/// `B_QWERTY` (9) and `CJ_QWERTY` (10): JIS English on US ANSI and UK ISO symbols (spec §3.6.2).
#[test]
fn us_and_uk_english_are_modes_9_and_10() {
    for (id, code, name) in [(9, "B_QWERTY", "US English"), (10, "CJ_QWERTY", "UK English")] {
        let mode = Mode::from_id(id).unwrap();
        assert_eq!((mode.code(), mode.name(), mode.is_korean()), (code, name, false));
    }
}

#[test]
fn us_and_uk_english_type_their_own_symbols() {
    let typed = |id: u32, code: &str, shift: bool| {
        output_in(Mode::from_id(id).unwrap(), Layout::JisKorean, key(code), shift).map(Out::char)
    };
    // US ANSI.
    assert_eq!(typed(9, "KeyG", true), Some('G'));
    assert_eq!(typed(9, "Digit2", true), Some('@'));
    assert_eq!(typed(9, "BracketLeft", false), Some('['));
    assert_eq!(typed(9, "Quote", true), Some('"'));
    assert_eq!(typed(9, "Backquote", false), Some('`'));
    // UK ISO.
    assert_eq!(typed(10, "Digit2", true), Some('"'));
    assert_eq!(typed(10, "Digit3", true), Some('£'));
    assert_eq!(typed(10, "Quote", true), Some('@'));
    assert_eq!(typed(10, "Backslash", false), Some('#'));
    assert_eq!(typed(10, "IntlBackslash", false), Some('\\'));
    // Designed for ISO, also supports JIS: ろ is the ISO extra key, ¥ the key left of 1.
    assert_eq!((typed(10, "IntlRo", false), typed(10, "IntlRo", true)), (Some('\\'), Some('|')));
    assert_eq!((typed(10, "IntlYen", false), typed(10, "IntlYen", true)), (Some('`'), Some('¬')));
    // US English has no JIS keys.
    assert_eq!((typed(9, "IntlRo", false), typed(9, "IntlYen", false)), (None, None));
}

#[test]
fn us_and_uk_english_pass_keys_through_and_hold() {
    for id in [9, 10] {
        let mut engine = Engine::new();
        engine.set_mode(Mode::from_id(id).unwrap());
        for code in ["KeyG", "Digit2", "BracketLeft", "Quote", "Space", "Enter"] {
            assert!(!engine.key(key(code), Mods::SHIFTED).consumed, "mode {id}: {code}");
        }
        assert_eq!(option(&mut engine, "KeyU", false).preedit, "u");
        assert_eq!(engine.key(key("Digit2"), Mods::NONE).commit, "ü");
    }
}

/// `A_DVORAK` (7) and `A_COLEMAK` (8): JIS English with Dvorak's and Colemak's letters (spec
/// §3.6.4).
#[test]
fn jis_dvorak_and_colemak_are_modes_7_and_8() {
    for (id, code, name) in [(7, "A_DVORAK", "JIS Dvorak"), (8, "A_COLEMAK", "JIS Colemak")] {
        let mode = Mode::from_id(id).unwrap();
        assert_eq!((mode.code(), mode.name(), mode.is_korean()), (code, name, false));
    }
}

/// They only move keys: every key JIS English has types the same pair somewhere, once, and every
/// key they don't move types what JIS English does.
#[test]
fn jis_dvorak_and_colemak_move_jis_english_keys() {
    let qwerty = Mode::from_id(3).unwrap();
    let pairs = |mode: Mode| {
        let mut pairs: Vec<(char, char)> = Key::ALL
            .iter()
            .filter_map(|&k| Some((output_in(mode, Layout::JisKorean, k, false)?.char(), output_in(mode, Layout::JisKorean, k, true)?.char())))
            .collect();
        pairs.sort();
        pairs
    };
    for id in [7, 8] {
        let mode = Mode::from_id(id).unwrap();
        assert_eq!(pairs(mode), pairs(qwerty), "mode {id}");
        for code in ["Digit1", "Digit2", "Digit7", "Minus", "Equal", "IntlYen", "BracketLeft", "BracketRight", "Quote", "Backslash", "IntlRo", "KeyA", "KeyM"] {
            for shift in [false, true] {
                assert_eq!(
                    output_in(mode, Layout::JisKorean, key(code), shift),
                    output_in(qwerty, Layout::JisKorean, key(code), shift),
                    "mode {id}: {code}"
                );
            }
        }
    }
}

#[test]
fn jis_dvorak_types_dvorak_letters_and_jis_symbols() {
    let typed = |code: &str, shift: bool| output_in(Mode::from_id(7).unwrap(), Layout::JisKorean, key(code), shift).map(Out::char);
    let row = |codes: &[&str]| codes.iter().map(|c| typed(c, false).unwrap()).collect::<String>();
    assert_eq!(row(&["KeyQ", "KeyW", "KeyE", "KeyR", "KeyT", "KeyY", "KeyU", "KeyI", "KeyO", "KeyP"]), "/,.pyfgcrl");
    assert_eq!(row(&["KeyA", "KeyS", "KeyD", "KeyF", "KeyG", "KeyH", "KeyJ", "KeyK", "KeyL", "Semicolon"]), "aoeuidhtns");
    assert_eq!(row(&["KeyZ", "KeyX", "KeyC", "KeyV", "KeyB", "KeyN", "KeyM", "Comma", "Period", "Slash"]), ";qjkxbmwvz");
    // The displaced JIS keys keep their Shift pairs.
    assert_eq!((typed("KeyQ", true), typed("KeyW", true), typed("KeyE", true), typed("KeyZ", true)), (Some('?'), Some('<'), Some('>'), Some('+')));
    assert_eq!((typed("Semicolon", true), typed("Slash", true)), (Some('S'), Some('Z')));
    // Dvorak's ' " are JIS's Shift+7 and Shift+2.
    assert_eq!((typed("Digit7", true), typed("Digit2", true)), (Some('\''), Some('"')));
}

#[test]
fn jis_colemak_types_colemak_letters_and_jis_symbols() {
    let typed = |code: &str, shift: bool| output_in(Mode::from_id(8).unwrap(), Layout::JisKorean, key(code), shift).map(Out::char);
    let row = |codes: &[&str]| codes.iter().map(|c| typed(c, false).unwrap()).collect::<String>();
    assert_eq!(row(&["KeyQ", "KeyW", "KeyE", "KeyR", "KeyT", "KeyY", "KeyU", "KeyI", "KeyO", "KeyP"]), "qwfpgjluy;");
    assert_eq!(row(&["KeyA", "KeyS", "KeyD", "KeyF", "KeyG", "KeyH", "KeyJ", "KeyK", "KeyL", "Semicolon"]), "arstdhneio");
    assert_eq!(row(&["KeyZ", "KeyX", "KeyC", "KeyV", "KeyB", "KeyN", "KeyM", "Comma", "Period", "Slash"]), "zxcvbkm,./");
    assert_eq!((typed("KeyP", true), typed("Semicolon", true)), (Some('+'), Some('O')));
}

#[test]
fn jis_dvorak_and_colemak_pass_keys_through() {
    for id in [7, 8] {
        let mut engine = Engine::new();
        engine.set_mode(Mode::from_id(id).unwrap());
        for code in ["KeyQ", "KeyZ", "Semicolon", "Digit2", "BracketLeft", "Space", "Enter", "F9", "F10"] {
            for mods in [Mods::NONE, Mods::SHIFTED, Mods { meta: true, ..Mods::NONE }] {
                let out = engine.key(key(code), mods);
                assert!(!out.consumed && out.commit.is_empty() && out.preedit.is_empty(), "mode {id}: {code} {mods:?}: {out:?}");
            }
        }
    }
}

/// ⌥ offers the variants of the letter the key types, not of its JIS English letter.
#[test]
fn jis_dvorak_and_colemak_offer_the_letter_typed() {
    let mut dvorak = Engine::new();
    dvorak.set_mode(Mode::from_id(7).unwrap());
    // Dvorak's u is on the F key.
    let out = option(&mut dvorak, "KeyF", false);
    assert_eq!((out.preedit.as_str(), list(&out)[1].as_str()), ("u", "ü"));
    assert_eq!(dvorak.key(key("Digit2"), Mods::NONE).commit, "ü");
    // ⌥Q (Dvorak's /) and ⌥U (g) have no variants: ABC's ⌥ layer.
    assert!(!dvorak.key(key("KeyQ"), OPTION).consumed);
    assert!(!dvorak.key(key("KeyU"), OPTION).consumed);
    // Semicolon types s.
    assert_eq!(list(&option(&mut dvorak, "Semicolon", true)), ["Ś", "Š"]);

    let mut colemak = Engine::new();
    colemak.set_mode(Mode::from_id(8).unwrap());
    // Colemak's o is on the semicolon key; its e on K.
    assert_eq!(option(&mut colemak, "Semicolon", false).preedit, "o");
    assert_eq!(colemak.key(key("Digit1"), Mods::NONE).commit, "ô");
    assert_eq!(option(&mut colemak, "KeyK", false).preedit, "e");
    colemak.key(key("Escape"), Mods::NONE);
    // P types ;, which has no variants.
    assert!(!colemak.key(key("KeyP"), OPTION).consumed);
}
