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
    pre("Equal S-Equal", "^^");
    pre("IntlYen S-IntlYen", "——");
    pre("BracketLeft S-BracketLeft", "'\"");
    pre("BracketRight S-BracketRight Backslash S-Backslash", "[「]」");
    pre("Semicolon S-Semicolon", "~-");
    pre("Quote S-Quote", ":;");
    pre("Comma S-Comma Period S-Period Slash S-Slash", ",,..··");
    pre("IntlRo S-IntlRo", "\\_");
}

#[test]
fn shift_ignored_on_vowels_without_shifted_forms() {
    pre("KeyR S-KeyK", "가");
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
fn f6_drops_implied_ieung() {
    pre("KeyK F6", "ㅏ");
    pre("KeyR KeyK KeyJ F6", "가ㅓ");
    pre("KeyH KeyK F6", "ㅘ");
    pre("KeyR KeyH Digit0 F6", "고ㅘ");
}

#[test]
fn bare_vowel_is_finished() {
    pre("KeyK F6 KeyK", "ㅏ아");
    pre("KeyK F6 KeyS", "ㅏㄴ");
    pre("KeyK F6 F6", "ㅏ");
}

#[test]
fn f6_after_a_final_just_ends_the_syllable() {
    pre("KeyK KeyS F6 KeyK", "안아");
}

#[test]
fn backspace_after_bare_vowel_restores_implied_ieung() {
    pre("KeyK F6 Backspace KeyJ", "어");
}

#[test]
fn implied_ieung_moves_like_a_real_initial() {
    pre("KeyK KeyS KeyK", "아나");
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
fn f6_ends_the_syllable() {
    pre("KeyG KeyK KeyS F6 KeyK", "한아");
    pre("KeyG KeyK KeyS F6 KeyR", "한ㄱ");
    pre("KeyR KeyK F6 KeyS", "가ㄴ");
    pre("KeyG KeyK KeyS F6 KeyK F6", "한ㅏ");
}

#[test]
fn f6_with_nothing_composing_is_a_no_op() {
    pre("F6", "");
    pre("Minus F6", "!");
}

// §6.1 F8

#[test]
fn f8_escapes_to_stock_jis() {
    pre("F8 Digit1 S-Digit2 BracketLeft Slash", "1\"@/");
    pre("F8 KeyG KeyK KeyS", "한");
}

#[test]
fn f8_is_one_way_and_shift_f8_returns() {
    pre("F8 F8 Digit1", "1");
    pre("F8 S-F8 Digit1", "ㅃ");
}

#[test]
fn activate_leaves_escape() {
    let mut h = Host::new();
    h.keys("F8");
    assert!(h.engine.is_escaped());
    h.engine.activate();
    h.keys("Digit1");
    assert_eq!(h.preedit, "ㅃ");
}

#[test]
fn f8_carries_the_syllable_being_composed() {
    pre("KeyG KeyK F8 KeyS", "한");
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
    step("F6", "한");
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
    pre("KeyG KeyK F6 KeyG KeyK F10", "하gk");
}

#[test]
fn conversion_with_no_word_is_a_no_op() {
    pre("Minus F9", "!");
    pre("F10", "");
}

#[test]
fn input_after_conversion_ends_it() {
    pre("KeyG KeyK KeyS F10 KeyK", "gks아");
    pre("KeyG KeyK KeyS F10 KeyR F6", "gksㄱ");
}

#[test]
fn space_commits_converted_text() {
    doc("KeyG KeyK KeyS F9 Space", "ｇｋｓ ", "");
}
