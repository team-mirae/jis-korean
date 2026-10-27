//! F7 hanja conversion (spec §7) and F9/F10 on symbol runs (spec §5.3).

use std::sync::Arc;

use jis_korean::{Dictionary, Engine, Key, Mods, Output, PAGE_SIZE};

const DICT: &str = "\
학교\t學校\t
학교\t學敎\t
학\t學\t배울 학
학\t鶴\t학 학
교\t校\t학교 교
한\t韓\t나라 한
한\t漢\t한수 한
가\t可\t옳을 가
가\t家\t집 가
가\t加\t더할 가
가\t歌\t노래 가
가\t價\t값 가
가\t街\t거리 가
가\t假\t거짓 가
가\t佳\t아름다울 가
가\t暇\t겨를 가
가\t架\t시렁 가
가\t賈\t값 가
";

/// Runs keys like the golden tests (`S-` = Shift), returning (document, last output).
fn run(engine: &mut Engine, keys: &str) -> (String, Output) {
    let mut doc = String::new();
    let mut last = Output::default();
    for token in keys.split_whitespace() {
        let (shift, code) = match token.strip_prefix("S-") {
            Some(code) => (true, code),
            None => (false, token),
        };
        let mods = if shift { Mods::SHIFTED } else { Mods::NONE };
        let key = Key::from_code(code);
        last = engine.key(key, mods);
        doc.push_str(&last.commit);
        if !last.consumed && key == Key::Enter {
            doc.push('\n');
        }
    }
    (doc, last)
}

fn engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_dictionary(Arc::new(Dictionary::parse(DICT)));
    engine
}

#[track_caller]
fn check(keys: &str, doc: &str, preedit: &str) {
    let (d, out) = run(&mut engine(), keys);
    assert_eq!((d.as_str(), out.preedit.as_str()), (doc, preedit), "keys: {keys}");
}

fn selected(out: &Output) -> Option<(&str, usize)> {
    let c = out.candidates.as_ref()?;
    Some((c.items[c.selected].hanja.as_str(), c.selected))
}

// §7 Opening the list

#[test]
fn f7_opens_the_longest_word_and_previews_the_first_candidate() {
    let (_, out) = run(&mut engine(), "KeyG KeyK KeyR KeyR KeyY");
    assert_eq!(out.preedit, "학교");
    let (_, out) = run(&mut engine(), "KeyG KeyK KeyR KeyR KeyY F7");
    assert_eq!(out.preedit, "學校");
    assert_eq!(selected(&out), Some(("學校", 0)));
    assert_eq!(out.candidates.unwrap().items.len(), 2);
}

#[test]
fn f7_converts_only_the_matching_suffix() {
    // 나학 → only 학 is in the dictionary.
    check("KeyS KeyK KeyG KeyK KeyR F7", "", "나學");
}

#[test]
fn f7_without_a_match_or_dictionary_does_nothing() {
    let (_, out) = run(&mut engine(), "KeyS KeyK F7");
    assert_eq!((out.preedit.as_str(), out.candidates.is_none()), ("나", true));
    let (_, out) = run(&mut Engine::new(), "KeyG KeyK KeyS F7");
    assert_eq!((out.preedit.as_str(), out.candidates.is_none()), ("한", true));
    // With nothing unconfirmed, F7 goes to the app (spec §6).
    let (_, out) = run(&mut engine(), "F7");
    assert!(!out.consumed && out.candidates.is_none());
}

#[test]
fn f7_needs_complete_syllables() {
    check("KeyG KeyK KeyS KeyR F7", "", "한ㄱ");
}

// §7 Moving and picking

#[test]
fn f7_and_arrows_move_the_selection() {
    let (_, out) = run(&mut engine(), "KeyG KeyK KeyS F7 F7");
    assert_eq!(selected(&out), Some(("漢", 1)));
    let (_, out) = run(&mut engine(), "KeyG KeyK KeyS F7 F7 F7");
    assert_eq!(selected(&out), Some(("韓", 0)), "wraps around");
    let (_, out) = run(&mut engine(), "KeyG KeyK KeyS F7 ArrowUp");
    assert_eq!(selected(&out), Some(("漢", 1)));
    let (_, out) = run(&mut engine(), "KeyG KeyK KeyS F7 S-F7");
    assert_eq!(selected(&out), Some(("漢", 1)));
    let (_, out) = run(&mut engine(), "KeyG KeyK KeyS F7 ArrowDown");
    assert_eq!(selected(&out), Some(("漢", 1)));
}

#[test]
fn arrows_left_and_right_page() {
    assert_eq!(PAGE_SIZE, 9);
    // 11 candidates: page 2 is 架 賈.
    let (_, out) = run(&mut engine(), "KeyR KeyK F7 ArrowRight");
    assert_eq!(selected(&out), Some(("架", 9)));
    let (_, out) = run(&mut engine(), "KeyR KeyK F7 ArrowRight ArrowRight");
    assert_eq!(selected(&out), Some(("賈", 10)), "stops at the last candidate");
    let (_, out) = run(&mut engine(), "KeyR KeyK F7 ArrowRight ArrowLeft");
    assert_eq!(selected(&out), Some(("可", 0)));
}

#[test]
fn number_keys_pick_on_the_current_page() {
    check("KeyR KeyK F7 Digit2", "", "家");
    check("KeyR KeyK F7 Digit9", "", "暇");
    check("KeyR KeyK F7 ArrowRight Digit2", "", "賈");
}

#[test]
fn number_past_the_end_picks_current_then_types() {
    // Page 2 has two candidates, so 3 picks the selected 架 and then types ㄸ.
    check("KeyR KeyK F7 ArrowRight Digit3", "", "架ㄸ");
}

#[test]
fn enter_picks_without_a_new_line() {
    let (doc, out) = run(&mut engine(), "KeyG KeyK KeyS F7 F7 Enter");
    assert_eq!((doc.as_str(), out.preedit.as_str()), ("", "漢"));
    assert!(out.candidates.is_none());
}

#[test]
fn other_keys_pick_then_act() {
    check("KeyG KeyK KeyS F7 Space", "韓 ", "");
    check("KeyG KeyK KeyS F7 KeyR KeyK", "", "韓가");
    check("KeyG KeyK KeyS F7 Minus", "", "韓!");
}

#[test]
fn escape_and_backspace_cancel_to_the_hangul() {
    let (_, out) = run(&mut engine(), "KeyG KeyK KeyS F7 F7 Escape");
    assert_eq!((out.preedit.as_str(), out.candidates.is_none()), ("한", true));
    check("KeyG KeyK KeyS F7 Backspace", "", "한");
    // The jamo are still editable after a cancel.
    check("KeyG KeyK KeyS F7 Escape Backspace", "", "하");
}

#[test]
fn f6_cancels_and_commits_the_hangul() {
    check("KeyG KeyK KeyS F7 F7 F6", "한", "");
}

#[test]
fn flush_commits_the_selected_candidate() {
    let mut e = engine();
    run(&mut e, "KeyG KeyK KeyS F7 F7");
    assert_eq!(e.flush().commit, "漢");
}

#[test]
fn earlier_syllables_stay_editable_after_a_pick() {
    check("KeyS KeyK KeyG KeyK KeyR F7 Enter", "", "나學");
    // Backspace removes 學, then the jamo of 나 one at a time.
    check("KeyS KeyK KeyG KeyK KeyR F7 Enter Backspace Backspace", "", "ㄴ");
}

#[test]
fn reading_inside_a_moved_final_still_converts() {
    // 말이 via the double press: 이 isn't a token boundary, so the prefix becomes plain text.
    let mut e = Engine::new();
    e.set_dictionary(Arc::new(Dictionary::parse("이\t李\t오얏 리\n")));
    let (_, out) = run(&mut e, "KeyA KeyK KeyF KeyL KeyL F7");
    assert_eq!(out.preedit, "말李");
}

#[test]
fn cancelling_a_reading_off_a_keystroke_boundary_keeps_the_keys_as_typed() {
    // §7.2: after Esc or Backspace the Hangul stays editable. 말이 typed with the double press
    // went to plain text while the list was open; cancelling gives the keystrokes back, so
    // Backspace takes the second ㅣ (§4.2: 말이 → 마리) and F10 still has the letters.
    for cancel in ["Escape", "Backspace"] {
        let mut e = Engine::new();
        e.set_dictionary(Arc::new(Dictionary::parse("이\t李\t오얏 리\n")));
        let (_, out) = run(&mut e, &format!("KeyA KeyK KeyF KeyL KeyL F7 {cancel}"));
        assert_eq!(out.preedit, "말이", "{cancel}");
        let (_, out) = run(&mut e, "Backspace");
        assert_eq!(out.preedit, "마리", "{cancel}");
        let (_, out) = run(&mut e, "KeyL F10");
        assert_eq!(out.preedit, "akfll", "{cancel}");
    }
}

// §5.3 F9/F10 on symbols and digits

#[test]
fn f9_converts_trailing_symbols_and_digits() {
    check("S-Digit2 S-Digit0 S-Digit2 S-Digit6 F9", "", "２０２６");
    check("KeyG KeyK KeyS Minus F9", "", "한！");
    check("Minus S-Minus F9 F10", "", "!?");
}

#[test]
fn a_symbol_run_never_takes_letters_an_earlier_conversion_left() {
    // §5.3: the run and a word never mix, also when the word was converted first.
    check("KeyG KeyK KeyS F10 Minus F9", "", "gks！");
    check("KeyG KeyK KeyS F10 Minus F9 F9", "", "gks！");
    // Symbols an earlier conversion left are still part of the run.
    check("Minus F9 F10 S-Minus F9", "", "！？");
}

#[test]
fn symbol_run_and_word_are_separate() {
    // The run is the symbols only, so the Hangul before them stays.
    check("KeyG KeyK KeyS S-Digit1 F9", "", "한１");
    // A word after symbols converts on its own.
    check("S-Digit1 KeyG KeyK KeyS F10", "", "1gks");
}

#[test]
fn f6_after_symbol_conversion_restores_and_commits() {
    check("S-Digit1 S-Digit2 F9 F6", "12", "");
}

// The real dictionary

#[test]
fn real_dictionary_loads_and_finds_common_words() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/dict/hanja.tsv");
    let text = std::fs::read_to_string(path).expect("dict/hanja.tsv");
    let dict = Dictionary::parse(&text);
    assert!(dict.len() > 100_000, "{} readings", dict.len());
    let first = |r: &str| dict.lookup(r).first().map(|c| c.hanja.clone());
    assert_eq!(first("사업").as_deref(), Some("事業"));
    assert_eq!(first("전부").as_deref(), Some("全部"));
    assert_eq!(first("학교").as_deref(), Some("學校"));
    assert!(dict.lookup("학").iter().any(|c| c.hanja == "學" && c.comment.contains("배울")));
    // Every hanja in the Samsung corpus is reachable by syllable.
    for (reading, hanja) in [("전", "全"), ("내", "內"), ("외", "外"), ("구", "舊")] {
        assert!(dict.lookup(reading).iter().any(|c| c.hanja == hanja), "{reading} → {hanja}");
    }
}
