//! Golden tests: key sequences → document text and preedit. Every example in docs/spec.md is here.
//!
//! Keys are canonical codes separated by spaces. `S-` holds Shift and `C-` holds Ctrl.

use jis_korean::{Engine, Key, Mods};

/// A host document: committed text, plus what passed-through keys do in a plain text field.
struct Host {
    engine: Engine,
    doc: String,
    preedit: String,
}

impl Host {
    fn new() -> Self {
        Host { engine: Engine::new(), doc: String::new(), preedit: String::new() }
    }

    fn keys(&mut self, keys: &str) -> &mut Self {
        for token in keys.split_whitespace() {
            let mut mods = Mods::NONE;
            let mut code = token;
            loop {
                if let Some(rest) = code.strip_prefix("S-") {
                    mods.shift = true;
                    code = rest;
                } else if let Some(rest) = code.strip_prefix("C-") {
                    mods.ctrl = true;
                    code = rest;
                } else {
                    break;
                }
            }
            let key = Key::from_code(code);
            assert!(key != Key::Other || code == "Other" || code == "ArrowLeft", "unknown key {code}");
            let out = self.engine.key(key, mods);
            self.doc.push_str(&out.commit);
            self.preedit = out.preedit;
            if !out.consumed && !mods.is_shortcut() {
                match key {
                    Key::Enter => self.doc.push('\n'),
                    Key::Backspace => {
                        self.doc.pop();
                    }
                    _ => {}
                }
            }
        }
        self
    }

    fn flush(&mut self) -> &mut Self {
        let out = self.engine.flush();
        self.doc.push_str(&out.commit);
        self.preedit = out.preedit;
        self
    }
}

/// Types `keys` and checks the preedit; nothing may be committed.
#[track_caller]
fn pre(keys: &str, expected: &str) {
    let mut h = Host::new();
    h.keys(keys);
    assert_eq!((h.doc.as_str(), h.preedit.as_str()), ("", expected), "keys: {keys}");
}

/// Types `keys` and checks the document and the preedit.
#[track_caller]
fn doc(keys: &str, expected_doc: &str, expected_preedit: &str) {
    let mut h = Host::new();
    h.keys(keys);
    assert_eq!((h.doc.as_str(), h.preedit.as_str()), (expected_doc, expected_preedit), "keys: {keys}");
}

// §1 Core principle

#[test]
fn shift_q_and_key_1_both_type_ssangbieup() {
    pre("S-KeyQ KeyK", "빠");
    pre("Digit1 KeyK", "빠");
}

#[test]
fn o_then_a_and_key_0_both_type_wa() {
    pre("KeyR KeyH KeyK", "과");
    pre("KeyR Digit0", "과");
}

#[test]
fn double_finals_compose() {
    pre("KeyE KeyK KeyF KeyR", "닭");
    pre("KeyD KeyJ KeyQ KeyT", "없");
}

#[test]
fn double_consonant_keys_match_shifted_letters() {
    for (digit, letter) in [("Digit1", "S-KeyQ"), ("Digit2", "S-KeyW"), ("Digit3", "S-KeyE"), ("Digit4", "S-KeyR"), ("Digit5", "S-KeyT")] {
        let mut a = Host::new();
        let mut b = Host::new();
        a.keys(&format!("{digit} KeyK {digit}"));
        b.keys(&format!("{letter} KeyK {letter}"));
        assert_eq!(a.preedit, b.preedit, "{digit} vs {letter}");
    }
}

#[test]
fn compound_vowel_keys_match_sequences() {
    pre("KeyR Digit6", "귀");
    pre("KeyR KeyN KeyL", "귀");
    pre("KeyR Digit7", "궈");
    pre("KeyR KeyN KeyJ", "궈");
    pre("KeyR Digit8", "괴");
    pre("KeyR KeyH KeyL", "괴");
    pre("KeyR Digit9", "긔");
    pre("KeyR KeyM KeyL", "긔");
}

// §3 Key map

#[test]
fn shift_number_row_types_digits() {
    pre("S-Digit1 S-Digit2 S-Digit3 S-Digit4 S-Digit5 S-Digit6 S-Digit7 S-Digit8 S-Digit9 S-Digit0", "1234567890");
}

#[test]
fn phone_numbers_type_with_shift_held() {
    pre("S-Digit0 S-Digit1 S-Digit0 S-Semicolon S-Digit1 S-Digit2", "010-12");
}

#[test]
fn symbol_keys() {
    pre("Minus S-Minus", "!?");
    pre("Equal S-Equal", "()");
    pre("IntlYen S-IntlYen", "—^");
    pre("BracketLeft S-BracketLeft", "'\"");
    pre("BracketRight S-BracketRight Backslash S-Backslash", "[「]」");
    pre("Semicolon S-Semicolon", "~-");
    pre("Quote S-Quote", ":;");
    pre("Comma S-Comma Period S-Period Slash S-Slash", ",,..··");
    pre("IntlRo S-IntlRo", "\\_");
}

#[test]
fn parentheses_on_the_caret_key_and_caret_on_shift_yen() {
    // The ^ key types ( and ) (spec §3.1); ^ is on Shift+¥, so ^^ is two Shift+¥.
    pre("Equal KeyG KeyK KeyS S-Equal", "(한)");
    pre("S-IntlYen S-IntlYen", "^^");
    // F9 / F10 never turn them into ^ ~: a word after ( converts and ( stays; text ending in )
    // converts that symbol run, as typed (§5.3: ) half-width is ) ).
    pre("Equal KeyR KeyK F10", "(rk");
    pre("Equal KeyR KeyK S-Equal F10", "(가)");
    pre("Equal KeyR KeyK S-Equal F9", "(가）");
}

#[test]
fn f9_and_f10_convert_every_symbol_with_a_form_in_that_width() {
    // 「 」 are full-width already: F10 makes them half-width, F9 leaves them.
    pre("S-BracketRight KeyR KeyK S-Backslash F10", "「가｣");
    pre("S-BracketRight KeyR KeyK S-Backslash F9", "「가」");
    pre("S-BracketRight KeyR KeyK S-Backslash F10 F9", "「가」");
    // A symbol with no other form stays, and the run goes on past it: · and — have none.
    pre("Minus Slash Minus F9", "！·！");
    pre("IntlYen S-Digit1 F9", "—１");
    pre("Minus Slash Minus F9 F10", "!·!");
}

#[test]
fn shift_ignored_on_vowels_without_shifted_forms() {
    pre("KeyR S-KeyK", "가");
}

#[test]
fn shift_still_held_after_a_double_final_moves_it_like_any_final() {
    // ㄲ and ㅆ can be finals: the vowel after them takes them, as after ㄴ or ㄱ (2-set too).
    pre("KeyR KeyK S-KeyR S-KeyK KeyD KeyL", "가까이");
    pre("KeyQ KeyL S-KeyT S-KeyK", "비싸");
    pre("KeyQ KeyK S-KeyR S-KeyK KeyX", "바깥");
    pre("KeyD KeyL S-KeyT S-KeyJ", "이써");
    // 있어 the other ways: the same vowel again, or the dedicated ㅆ key, after which Shift is new.
    pre("KeyD KeyL S-KeyT KeyJ KeyJ", "있어");
    pre("KeyD KeyL Digit5 S-KeyJ", "있어");
    // Only right after the double: a Shift+vowel later still starts on ㅇ.
    pre("KeyD KeyL S-KeyT S-KeyJ S-KeyK", "이써아");
}

// §4.1 Automaton

#[test]
fn final_moves_to_next_syllable() {
    pre("KeyG KeyK KeyS KeyK", "하나");
    pre("KeyE KeyK KeyF KeyR KeyK", "달가");
}

#[test]
fn double_final_splits_before_consonant_that_cannot_join() {
    pre("KeyD KeyK KeyS KeyW KeyD KeyK", "앉아");
    pre("KeyD KeyK KeyS KeyW KeyK", "안자");
    pre("KeyD KeyJ KeyQ KeyT KeyE KeyK", "없다");
}

#[test]
fn tense_consonants_that_cannot_be_finals_start_a_syllable() {
    pre("KeyR KeyK S-KeyE", "가ㄸ");
    pre("KeyR KeyK S-KeyE KeyK", "가따");
    pre("KeyR KeyK Digit1 KeyK", "가빠");
}

#[test]
fn lone_consonants() {
    pre("KeyR", "ㄱ");
    pre("KeyR KeyS", "ㄱㄴ");
}

#[test]
fn dedicated_compound_vowel_does_not_combine_further() {
    pre("KeyR Digit0 KeyL", "과이");
}

// §4.2 Implied ㅇ

#[test]
fn vowel_with_no_initial_gets_implied_ieung() {
    pre("KeyK", "아");
    pre("KeyH KeyK", "와");
    pre("KeyK KeyS", "안");
    pre("Minus KeyK", "!아");
}

#[test]
fn vowel_that_cannot_join_gets_implied_ieung() {
    pre("KeyR KeyK KeyJ", "가어");
    pre("KeyR KeyH Digit0", "고와");
    pre("KeyR KeyK Digit8", "가외");
}

#[test]
fn f6_drops_implied_ieung_and_commits() {
    doc("KeyK F6", "ㅏ", "");
    doc("KeyR KeyK KeyJ F6", "가ㅓ", "");
    doc("KeyH KeyK F6", "ㅘ", "");
    doc("KeyR KeyH Digit0 F6", "고ㅘ", "");
}

#[test]
fn f6_keeps_implied_ieung_once_it_has_a_final() {
    doc("KeyK KeyS F6", "안", "");
}

#[test]
fn vowel_after_f6_gets_implied_ieung_again() {
    doc("KeyK F6 KeyK", "ㅏ", "아");
    doc("KeyK F6 Backspace KeyJ", "", "어");
}

#[test]
fn implied_ieung_moves_like_a_real_initial() {
    pre("KeyK KeyS KeyK", "아나");
}

// §4.2 Same vowel again keeps the final

#[test]
fn same_vowel_again_puts_the_final_back() {
    pre("KeyA KeyK KeyF KeyL", "마리");
    pre("KeyA KeyK KeyF KeyL KeyL", "말이");
    pre("KeyE KeyK KeyF KeyR KeyK KeyK", "닭아");
    pre("KeyR KeyK KeyD KeyK KeyK", "강아");
    pre("KeyA KeyK KeyF Digit0 Digit0", "말와");
}

#[test]
fn final_toggle_needs_the_same_vowel_right_away() {
    pre("KeyA KeyK KeyF KeyL KeyK", "마리아");
    pre("KeyA KeyK KeyF KeyH KeyK", "마롸");
    doc("KeyA KeyK KeyF KeyL F6 KeyL", "마리", "이");
    pre("KeyA KeyK KeyF KeyL KeyL KeyL", "말이이");
}

#[test]
fn final_toggle_then_f6_and_backspace() {
    // F6 commits the double-press ㅇ as it is.
    doc("KeyA KeyK KeyF KeyL KeyL F6", "말이", "");
    pre("KeyA KeyK KeyF KeyL KeyL Backspace", "마리");
}

// §4.3 Backspace

#[test]
fn backspace_removes_one_jamo_keystroke() {
    pre("KeyG KeyK KeyS Backspace", "하");
    pre("KeyG KeyK KeyS Backspace Backspace", "ㅎ");
    pre("KeyR KeyH KeyK Backspace", "고");
    pre("KeyR Digit0 Backspace", "ㄱ");
    pre("KeyR KeyK KeyJ Backspace", "가");
}

#[test]
fn backspace_with_nothing_unconfirmed_passes_through() {
    doc("KeyR KeyK Space Backspace Backspace", "", "");
}

// §5 Commit

#[test]
fn space_commits_and_types_a_space() {
    doc("KeyG KeyK KeyS Space", "한 ", "");
}

#[test]
fn enter_commits_then_passes_through() {
    doc("KeyG KeyK KeyS Enter", "한\n", "");
    doc("Enter", "\n", "");
}

#[test]
fn shift_enter_commits_only() {
    doc("KeyG KeyK KeyS S-Enter", "한", "");
}

#[test]
fn enter_commits_setting() {
    let mut h = Host::new();
    h.engine.set_enter_commits(true);
    h.keys("KeyG KeyK KeyS Enter");
    assert_eq!(h.doc, "한");
}

#[test]
fn shortcut_commits_and_passes_through() {
    doc("KeyG KeyK KeyS C-KeyC", "한", "");
}

#[test]
fn other_keys_commit_and_pass_through() {
    doc("KeyG KeyK KeyS ArrowLeft", "한", "");
}

#[test]
fn modifier_alone_changes_nothing() {
    pre("KeyG KeyK KeyS ShiftLeft", "한");
}

#[test]
fn flush_commits_everything() {
    let mut h = Host::new();
    h.keys("KeyG KeyK KeyS Minus").flush();
    assert_eq!((h.doc.as_str(), h.preedit.as_str()), ("한!", ""));
}

// §6 F6

#[test]
fn f6_commits() {
    doc("KeyG KeyK KeyS F6", "한", "");
    doc("KeyG KeyK KeyS F6 KeyK", "한", "아");
    doc("KeyG KeyK KeyS F6 KeyR", "한", "ㄱ");
    doc("KeyR KeyK F6 KeyS", "가", "ㄴ");
    doc("KeyG KeyK KeyS F6 KeyK F6", "한ㅏ", "");
    doc("Minus F6", "!", "");
}

#[test]
fn f6_with_nothing_unconfirmed_is_a_no_op() {
    doc("F6", "", "");
}

#[test]
fn function_keys_with_nothing_unconfirmed_go_to_the_app() {
    for key in [Key::F6, Key::F7, Key::F8, Key::F9, Key::F10] {
        assert!(!Engine::new().key(key, Mods::NONE).consumed, "{key:?}");
    }
    // Shift+Z is a JIS Korean key, not a real F10: taken either way.
    assert!(Engine::new().key(Key::KeyZ, Mods::SHIFTED).consumed);
    // With something unconfirmed they act and are taken.
    let mut e = Engine::new();
    e.key(Key::KeyR, Mods::NONE);
    let out = e.key(Key::F6, Mods::NONE);
    assert!(out.consumed);
    assert_eq!(out.commit, "ㄱ");
}

// §6 F8 (reserved)

#[test]
fn f8_is_reserved_and_does_nothing() {
    // With something unconfirmed it's taken and changes nothing; the keys after it type as ever.
    pre("KeyG KeyK F8 KeyS", "한");
    pre("F8 Digit1 S-Digit2", "ㅃ2");
    pre("KeyG KeyK S-F8 Digit1", "하ㅃ");
    // With nothing unconfirmed it goes to the app.
    let mut e = Engine::new();
    assert!(!e.key(Key::F8, Mods::NONE).consumed);
    e.key(Key::KeyG, Mods::NONE);
    assert!(e.key(Key::F8, Mods::NONE).consumed);
    assert!(!e.is_escaped());
}

// §6.2 F9 / F10

#[test]
fn f9_f10_cycle() {
    let mut h = Host::new();
    let mut step = |keys: &str, expected: &str| {
        h.keys(keys);
        assert_eq!(h.preedit, expected, "after {keys}");
    };
    step("KeyG KeyK KeyS", "한");
    step("F10", "gks");
    step("F10", "GKS");
    step("F10", "Gks");
    step("F9", "Ｇｋｓ");
    step("F6", "");
    assert_eq!(h.doc, "한", "F6 restores the Hangul and commits it");
}

#[test]
fn switching_width_keeps_case() {
    pre("KeyG KeyK KeyS F10 F10 F9", "ＧＫＳ");
}

#[test]
fn conversion_uses_raw_keys() {
    pre("S-KeyQ KeyK F10", "Qk");
    pre("Digit1 KeyK F10", "1k");
    pre("KeyK F10", "k");
}

#[test]
fn conversion_word_ends_at_symbols_and_f6() {
    pre("KeyG KeyK KeyS Minus KeyG KeyK KeyS F10", "한!gks");
    doc("KeyG KeyK F6 KeyG KeyK F10", "하", "gk");
}

#[test]
fn conversion_with_no_word_is_a_no_op() {
    pre("F10", "");
    doc("KeyG KeyK KeyS F6 F9", "한", "");
}

#[test]
fn input_after_conversion_ends_it() {
    pre("KeyG KeyK KeyS F10 KeyK", "gks아");
    doc("KeyG KeyK KeyS F10 KeyR F6", "gksㄱ", "");
}

#[test]
fn space_commits_converted_text() {
    doc("KeyG KeyK KeyS F9 Space", "ｇｋｓ ", "");
}

// English reading and clear (for hosts that rank Korean against English)

#[test]
fn english_reads_the_same_keys_on_stock_jis() {
    let mut e = Engine::new();
    let mut last = None;
    for (code, shift) in [("KeyH", false), ("KeyE", false), ("KeyL", false), ("KeyL", false), ("KeyO", false), ("Digit1", true)] {
        last = Some(e.key(Key::from_code(code), if shift { Mods::SHIFTED } else { Mods::NONE }));
    }
    let out = last.unwrap();
    // h e l → 오디, then the second l is the same vowel right after a move: 옫이 (double press).
    assert_eq!(out.preedit, "옫이애1");
    assert_eq!(out.english, "hello!");
    assert_eq!(e.english(), "hello!");
}

#[test]
fn english_keeps_case_dedicated_digits_and_symbols() {
    let mut e = Engine::new();
    for (code, shift) in [("KeyQ", true), ("Digit1", false), ("Minus", false), ("Slash", false), ("IntlYen", false)] {
        e.key(Key::from_code(code), if shift { Mods::SHIFTED } else { Mods::NONE });
    }
    assert_eq!(e.preedit(), "ㅃㅃ!·—");
    assert_eq!(e.english(), "Q1-/¥");
}

#[test]
fn clear_discards_without_committing() {
    let mut e = Engine::new();
    e.key(Key::from_code("KeyG"), Mods::NONE);
    e.key(Key::from_code("KeyK"), Mods::NONE);
    e.clear();
    assert_eq!((e.preedit().as_str(), e.english().as_str()), ("", ""));
    assert_eq!(e.flush().commit, "");
}

// §4.2 Shift+vowel starts a syllable on ㅇ

#[test]
fn shift_vowel_starts_a_syllable_and_leaves_the_final() {
    pre("KeyA KeyK KeyF S-KeyL", "말이");
    pre("KeyR KeyK KeyS S-KeyK", "간아");
    pre("KeyE KeyK KeyF KeyR S-KeyK", "닭아");
    pre("KeyR KeyK KeyS S-KeyJ KeyS", "간언");
}

#[test]
fn shift_vowel_does_not_combine_with_the_vowel_before() {
    pre("KeyH S-KeyL", "오이");
    pre("KeyH KeyL", "외");
    pre("KeyR KeyN S-KeyJ", "구어");
}

#[test]
fn shift_vowel_can_still_combine_with_the_vowel_after() {
    pre("KeyR KeyK KeyS S-KeyH KeyK", "간와");
}

#[test]
fn shift_vowel_on_its_own_is_like_the_implied_ieung() {
    pre("S-KeyK", "아");
    pre("Minus S-KeyK", "!아");
}

#[test]
fn f6_keeps_a_shift_vowel_ieung() {
    // Asked for with Shift, so unlike the implied ㅇ, F6 commits it as it is.
    doc("KeyA KeyK KeyF S-KeyL F6", "말이", "");
    doc("KeyA KeyK KeyF KeyL F6", "마리", "");
}

#[test]
fn backspace_removes_the_shift_vowel_in_one_press() {
    pre("KeyA KeyK KeyF S-KeyL Backspace", "말");
}

#[test]
fn shift_on_o_and_p_still_types_yae_and_ye() {
    pre("KeyR S-KeyO", "걔");
    pre("KeyR KeyK KeyS S-KeyO", "가냬");
    pre("KeyR KeyK KeyS S-KeyP", "가녜");
}

#[test]
fn shift_on_consonants_is_unchanged() {
    pre("KeyR KeyK S-KeyS KeyK", "가나");
    pre("KeyR KeyK S-KeyR KeyK", "가까");
}

#[test]
fn english_reading_keeps_the_shifted_vowel_letter() {
    let mut e = Engine::new();
    for (code, shift) in [("KeyA", false), ("KeyK", false), ("KeyF", false), ("KeyL", true)] {
        e.key(Key::from_code(code), if shift { Mods::SHIFTED } else { Mods::NONE });
    }
    assert_eq!((e.preedit().as_str(), e.english().as_str()), ("말이", "akfL"));
}

#[test]
fn shift_still_held_after_a_double_types_the_syllable() {
    pre("S-KeyR S-KeyK", "까");
    pre("S-KeyT S-KeyK S-KeyD", "쌍");
    pre("KeyR S-KeyK", "가");
}

// §6 Shift+Z is F10

#[test]
fn shift_z_is_f10() {
    pre("KeyG KeyK KeyS S-KeyZ", "gks");
    pre("KeyG KeyK KeyS S-KeyZ S-KeyZ", "GKS");
    pre("KeyG KeyK KeyS S-KeyZ S-KeyZ S-KeyZ", "Gks");
    pre("KeyG KeyK KeyS S-KeyZ F9", "ｇｋｓ");
    doc("KeyG KeyK KeyS S-KeyZ F6", "한", "");
}

#[test]
fn plain_z_still_types_kieuk() {
    pre("KeyZ KeyZ", "ㅋㅋ");
    pre("KeyZ KeyK", "카");
}

#[test]
fn the_engine_says_which_keys_commit_what_stands() {
    let e = Engine::new();
    for key in [Key::Space, Key::Enter, Key::Escape, Key::ArrowDown, Key::ArrowLeft, Key::Other, Key::Backquote] {
        assert!(e.commits(key, Mods::NONE), "{key:?}");
    }
    assert!(e.commits(Key::KeyG, Mods { ctrl: true, ..Mods::NONE }));
    for key in [Key::KeyG, Key::Digit1, Key::Minus, Key::Backspace, Key::F6, Key::F10, Key::Modifier] {
        assert!(!e.commits(key, Mods::NONE), "{key:?}");
    }
    // Shift+Z is F10 here, not a commit.
    assert!(!e.commits(Key::KeyZ, Mods::SHIFTED));
}
