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
    Arc::new(Predictor::parse(&model, &read("common-en.txt"), &read("common-ko.txt"), &read("english-nouns.txt")).unwrap())
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
