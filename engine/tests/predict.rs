//! The predictive layer (spec §10, draft), with the real model. Only built with `predict`.
#![cfg(feature = "predict")]

use std::sync::Arc;

use jis_korean::predict::{Kind, Predictor};
use jis_korean::{Dictionary, Engine, Key, Mods};

fn predictor() -> Arc<Predictor> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/predict/");
    let read = |f: &str| std::fs::read_to_string(format!("{dir}{f}")).unwrap();
    let model = std::fs::read(format!("{dir}ranker.bin")).unwrap();
    Arc::new(Predictor::parse(&model, &read("common-en.txt"), &read("common-ko.txt")).unwrap())
}

/// Types keys (`S-` = Shift) into an engine with the predictor, returning it.
fn typed(keys: &str) -> Engine {
    let mut e = Engine::new();
    e.set_predictor(Some(predictor()));
    for token in keys.split_whitespace() {
        let (mods, code) = match token.strip_prefix("S-") {
            Some(code) => (Mods::SHIFTED, code),
            None => (Mods::NONE, token),
        };
        e.key(Key::from_code(code), mods);
    }
    e
}

fn top(keys: &str) -> (String, Kind) {
    let p = typed(keys).predictions();
    (p[0].text.clone(), p[0].kind)
}

fn texts(keys: &str) -> Vec<String> {
    typed(keys).predictions().into_iter().map(|p| p.text).collect()
}

#[test]
fn korean_and_english_words_rank_first() {
    assert_eq!(top("KeyG KeyK KeyS KeyR KeyM KeyF"), ("한글".into(), Kind::Korean));
    assert_eq!(top("KeyH KeyE KeyL KeyL KeyO"), ("hello".into(), Kind::English));
    assert_eq!(top("KeyD KeyK KeyS KeyS KeyU KeyD"), ("안녕".into(), Kind::Korean));
}

#[test]
fn both_main_readings_are_always_offered() {
    let t = texts("KeyG KeyK KeyS KeyR KeyM KeyF");
    assert!(t.contains(&"한글".to_string()) && t.contains(&"gksrmf".to_string()), "{t:?}");
}

#[test]
fn two_set_reading_brings_back_bare_jamo() {
    // JIS Korean reads b b as 유유 (implied ㅇ); plain 2-set gives the chat word ㅠㅠ, which wins.
    let p = typed("KeyB KeyB").predictions();
    assert_eq!(p[0].text, "ㅠㅠ", "{p:?}");
    assert_eq!(p[0].kind, Kind::TwoSet);
    assert!(p.iter().any(|x| x.text == "유유"));
}

#[test]
fn two_set_reading_types_digits() {
    let t = texts("KeyR KeyK Digit1");
    assert!(t.contains(&"가1".to_string()), "{t:?}");
}

#[test]
fn keep_finals_offers_the_other_split() {
    let t = texts("KeyA KeyK KeyF KeyL");
    assert!(t.contains(&"마리".to_string()) && t.contains(&"말이".to_string()), "{t:?}");
}

#[test]
fn hanja_is_lean_and_last() {
    let mut e = typed("KeyG KeyK KeyR KeyR KeyY");
    e.set_dictionary(Arc::new(Dictionary::parse("학교\t學校\t\n학\t學\t배울 학\n")));
    let p = e.predictions();
    assert_eq!(p.last().map(|x| (x.text.as_str(), x.kind)), Some(("學校", Kind::Hanja)), "{p:?}");
    assert_eq!(p.iter().filter(|x| x.kind == Kind::Hanja).count(), 1);
    // No single-syllable hanja.
    let mut e = typed("KeyG KeyK KeyR");
    e.set_dictionary(Arc::new(Dictionary::parse("학\t學\t배울 학\n")));
    assert!(e.predictions().iter().all(|x| x.kind != Kind::Hanja));
}

#[test]
fn at_most_four_and_no_duplicates() {
    for keys in ["KeyH KeyE KeyL KeyL KeyO", "KeyB KeyB", "KeyA KeyK KeyF KeyL", "Digit1"] {
        let t = texts(keys);
        assert!(t.len() <= jis_korean::predict::MAX_PREDICTIONS, "{keys}: {t:?}");
        let mut unique = t.clone();
        unique.dedup();
        assert_eq!(unique.len(), t.len(), "{keys}: {t:?}");
    }
}

#[test]
fn committing_a_prediction_sets_the_context() {
    let mut e = typed("KeyD KeyK KeyS KeyS KeyU KeyD");
    let english = e.predictions().iter().position(|p| p.kind == Kind::English).unwrap();
    assert_eq!(e.commit_prediction(english).commit, "dkssud");
    assert_eq!(e.preedit(), "");
}

#[test]
fn without_a_predictor_the_engine_is_lite() {
    let mut e = Engine::new();
    e.key(Key::from_code("KeyG"), Mods::NONE);
    assert!(e.predictions().is_empty());
}

#[test]
fn bad_models_are_rejected() {
    assert!(Predictor::parse(b"nope", "", "").is_err());
}

#[test]
fn english_case_variants_only_for_english_words() {
    let korean = typed("KeyG KeyK KeyS KeyR KeyM KeyF").predictions();
    assert!(korean.iter().all(|p| !matches!(p.kind, Kind::EnglishCapitalized | Kind::EnglishUpper)), "{korean:?}");
    let english = texts("KeyH KeyE KeyL KeyL KeyO");
    assert!(english.contains(&"Hello".to_string()) && english.contains(&"HELLO".to_string()), "{english:?}");
}
