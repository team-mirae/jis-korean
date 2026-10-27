//! The training contract's engine side (spec §10.12, docs/training-contract.md): the user's own
//! words added to the predictor, and examples of what they meant learned from.
#![cfg(feature = "predict")]

use std::sync::Arc;

use jis_korean::learn::Learner;
use jis_korean::plan::{keys_for_word, parse_keys};
use jis_korean::predict::Predictor;
use jis_korean::{Engine, Key, Mods};

fn predictor() -> Arc<Predictor> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/predict/");
    let read = |f: &str| std::fs::read_to_string(format!("{dir}{f}")).unwrap();
    let model = std::fs::read(format!("{dir}ranker.bin")).unwrap();
    let learned = std::fs::read(format!("{dir}learned.bin")).unwrap();
    // The learned ranker (spec §10.13); the legacy build ignores it and ranks by the rules.
    let p = Predictor::parse(&model, &read("common-en.txt"), &read("common-ko.txt"), &read("english-nouns.txt")).unwrap();
    Arc::new(p.with_learned(&learned).unwrap())
}

fn top(p: &Arc<Predictor>, keys: &str) -> String {
    let mut e = Engine::new();
    e.set_predictor(Some(p.clone()));
    for press in parse_keys(keys).unwrap() {
        e.key(press.key, if press.shift { Mods::SHIFTED } else { Mods::NONE });
    }
    e.predictions()[0].text.clone()
}

#[test]
fn the_users_own_words_make_their_spellings_known() {
    // 바둑이 typed lazily (바두기's keys): 바둑 isn't in the corpora's word list…
    let keys = "KeyQ KeyK KeyE KeyN KeyR KeyL";
    let base = predictor();
    assert_eq!(top(&base, keys), "바두기");
    // …the user's vocabulary has it: a noun and its particle now come first.
    let mine = Arc::new(base.with_user_words("바둑\t50\n# a comment\n데이터센터\n", "kubernetes\t3\n"));
    assert_eq!(top(&mine, keys), "바둑이");
    // The base predictor is unchanged.
    assert_eq!(top(&base, keys), "바두기");
}

#[test]
fn an_example_teaches_where_the_meant_reading_was() {
    let mut e = Engine::new();
    e.set_predictor(Some(predictor()));
    // 산이 typed lazily is listed below 사니: learning from it records that choice.
    let keys = parse_keys("KeyT KeyK KeyS KeyL").unwrap();
    assert_eq!(e.learn_example(Some(&keys), "산이"), Some(1));
    assert_ne!(*e.learner(), Learner::default());
    assert!(e.take_learned_changed());
    // Without keys, the cheapest keys for the word are used.
    assert_eq!(e.learn_example(None, "안녕"), Some(0));
    // Not listed at all: nothing learned from it.
    let before = e.learner().clone();
    assert_eq!(e.learn_example(Some(&keys), "고양이"), None);
    assert_eq!(*e.learner(), before);
    // No predictor: nothing to learn from.
    assert_eq!(Engine::new().learn_example(None, "안녕"), None);
}

#[test]
fn keys_parse_and_plan() {
    let keys = parse_keys("KeyT S-KeyL Digit5").unwrap();
    assert_eq!((keys[1].key, keys[1].shift), (Key::KeyL, true));
    assert_eq!(keys[2].key, Key::Digit5);
    assert!(parse_keys("KeyT NotAKey").is_none());
    // Mixed words plan by run: report + 를.
    let planned = keys_for_word("report를").unwrap();
    assert_eq!(planned.len(), "reportfmf".len());
}

fn all(p: &Arc<Predictor>, keys: &str) -> Vec<String> {
    let mut e = Engine::new();
    e.set_predictor(Some(p.clone()));
    for press in parse_keys(keys).unwrap() {
        e.key(press.key, if press.shift { Mods::SHIFTED } else { Mods::NONE });
    }
    e.predictions().iter().map(|p| p.text.clone()).collect()
}

#[test]
fn an_avoided_word_steps_aside_with_every_particle() {
    // g o d reads as god (a common English word) above 행, and god을 above 행을.
    let (god, god_eul) = ("KeyG KeyO KeyD", "KeyG KeyO KeyD KeyD KeyM KeyF");
    let base = predictor();
    assert_eq!(top(&base, god), "god");
    // The rules put god을 first; the learned ranker (spec §10.13) prefers 행을 (행 + 을, a Korean
    // noun and its particle) there, with god을 next.
    if base.is_learned() {
        assert_eq!(all(&base, god_eul)[1], "god을");
    } else {
        assert_eq!(top(&base, god_eul), "god을");
    }
    // One entry covers the word and every particle after it.
    let mine = Arc::new(base.with_avoided("", "god\n"));
    assert_eq!(top(&mine, god), "행");
    assert_eq!(top(&mine, god_eul), "행을");
    assert_eq!(top(&mine, "KeyG KeyO KeyD KeyD KeyP KeyT KeyJ"), "행에서");
    // Still listed, to be picked on purpose.
    assert!(all(&mine, god).contains(&"god".to_string()));
    // English ignores case, but a capital typed at the start asked for it.
    let upper = Arc::new(base.with_avoided("", "GOD\n"));
    assert_eq!(top(&upper, god), "행");
    assert_eq!(top(&mine, "S-KeyG KeyO KeyD"), top(&base, "S-KeyG KeyO KeyD"));
    // The base predictor is unchanged.
    assert_eq!(top(&base, god), "god");
}

#[test]
fn a_strongly_avoided_word_is_never_on_top() {
    // 바두기 (바둑이 typed lazily): avoided strongly, the next reading moves up, and it's still listed.
    let keys = "KeyQ KeyK KeyE KeyN KeyR KeyL";
    let base = predictor();
    assert_eq!(top(&base, keys), "바두기");
    let mine = Arc::new(base.with_avoided("바두기\tstrong\n", ""));
    let readings = all(&mine, keys);
    assert_ne!(readings[0], "바두기");
    assert!(readings.contains(&"바두기".to_string()));
    // Avoid wins over the user's own vocabulary.
    let both = Arc::new(base.with_user_words("바두기\t1000\n", "").with_avoided("바두기\tstrong\n", ""));
    assert_eq!(top(&base.with_user_words("바두기\t1000\n", "").into(), keys), "바두기");
    assert_ne!(top(&both, keys), "바두기");
}

#[test]
fn avoiding_a_word_leaves_unrelated_readings_alone() {
    let base = predictor();
    let mine = Arc::new(base.with_avoided("마리\tstrong\n행\n", "god\nsmall\n"));
    for keys in ["KeyG KeyK KeyS KeyR KeyM KeyF", "KeyQ KeyK KeyE KeyN KeyR KeyL", "KeyR KeyO KeyD", "KeyD KeyK KeyS KeyS KeyU KeyD", "KeyS KeyE KeyR KeyV KeyE KeyR KeyF KeyM KeyF"] {
        assert_eq!(all(&mine, keys), all(&base, keys), "{keys}");
    }
}
