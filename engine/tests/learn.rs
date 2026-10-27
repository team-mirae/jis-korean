//! Learning from the user's choices (spec §10.11), with simulated users: each has a habit the
//! defaults don't match, types a round of words again and again, and picks the reading they want
//! with ↓ when it's listed. Learning should bring their readings to the top within a few rounds,
//! within its clamp, and never past the hard guards.
#![cfg(feature = "predict")]

use std::sync::Arc;

use jis_korean::learn::{self, Learner};
use jis_korean::plan::Style;
use jis_korean::predict::{Kind, Predictor, Ranker, Weight};
use jis_korean::trust::type_text;
use jis_korean::{Dictionary, Engine, Key, Mods};

fn predictor() -> Arc<Predictor> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/predict/");
    let read = |f: &str| std::fs::read_to_string(format!("{dir}{f}")).unwrap();
    let model = std::fs::read(format!("{dir}ranker.bin")).unwrap();
    Arc::new(Predictor::parse(&model, &read("common-en.txt"), &read("common-ko.txt"), &read("english-nouns.txt")).unwrap())
}

fn dictionary() -> Arc<Dictionary> {
    Arc::new(Dictionary::parse(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/dict/hanja.tsv")).unwrap()))
}

/// A round of (keys, wanted text). The first `habit` items are the user's habit; the rest are
/// ordinary words they accept as ranked.
struct User {
    name: &'static str,
    items: &'static [(&'static str, &'static str)],
    habit: usize,
}

const NEUTRAL: [(&str, &str); 4] = [("dkssud", "안녕"), ("tkfkd", "사랑"), ("wjsur", "저녁"), ("eogkrry", "대학교")];

const BARE_VOWELS: User = User {
    name: "bare-vowel chatter",
    items: &[("hh", "ㅗㅗ"), ("jj", "ㅓㅓ"), ("kk", "ㅏㅏ"), ("yy", "ㅛㅛ"), ("dkssud", "안녕"), ("tkfkd", "사랑"), ("wjsur", "저녁"), ("eogkrry", "대학교")],
    habit: 4,
};

const KOREAN_FIRST: User = User {
    name: "Korean-first typist",
    items: &[("so", "내"), ("to", "새"), ("do", "애"), ("dkssud", "안녕"), ("tkfkd", "사랑"), ("wjsur", "저녁"), ("eogkrry", "대학교")],
    habit: 3,
};

const HANJA_LOVER: User = User {
    name: "hanja lover",
    items: &[("dkssud", "安寧"), ("eogkrry", "大學校"), ("tkfkd", "舍廊"), ("wjsur", "저녁"), ("rh", "고")],
    habit: 3,
};

/// Types `rounds` rounds of the user's items with learning on. Returns, per round, how many habit
/// items were the top prediction, and the learner at the end. Asserts no hanja is ever on top.
fn simulate(user: &User, rounds: usize) -> (Vec<usize>, Learner) {
    let mut e = Engine::new();
    e.set_dictionary(dictionary());
    e.set_predictor(Some(predictor()));
    e.set_learning(true);
    let mut hits = Vec::new();
    for _ in 0..rounds {
        let mut round = 0;
        for (i, (keys, want)) in user.items.iter().enumerate() {
            for c in keys.chars() {
                e.key(Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), Mods::NONE);
            }
            let list = e.predictions();
            assert_ne!(list.first().map(|p| p.kind), Some(Kind::Hanja), "{}: hanja on top for {keys}: {list:?}", user.name);
            let at = list.iter().position(|p| p.text == *want);
            round += (i < user.habit && at == Some(0)) as usize;
            e.commit_prediction(at.unwrap_or(0));
        }
        hits.push(round);
    }
    (hits, e.learner().clone())
}

#[test]
fn a_bare_vowel_chatter_gets_bare_vowels_first() {
    let (hits, learner) = simulate(&BARE_VOWELS, 12);
    eprintln!("{}: habit hits per round {hits:?}, Implied ×{:.2}, Korean ×{:.2}", BARE_VOWELS.name, learner.weight(Weight::Implied), learner.weight(Weight::Korean));
    assert_eq!(hits[0], 0);
    // Half the habit within three rounds. ㅗㅗ and ㅏㅏ stay a ↓ away: 오오 and 아아 are words
    // the lexicon knows, and the clamp (0.5–2×) keeps learning from overturning that.
    assert!(hits[3] >= BARE_VOWELS.habit / 2, "{hits:?}");
    assert!(hits.windows(2).all(|w| w[1] >= w[0]), "{hits:?}");
    // Bare-vowel picks don't demote Korean against English (the neutral words, accepted over
    // their English readings, nudge Korean up a little).
    assert!(learner.weight(Weight::Korean) >= 1.0 && learner.weight(Weight::English) <= 1.0);
    // Bare vowels wanted: the implied ㅇ is less likely.
    assert!(learner.weight(Weight::Implied) < 1.0 && learner.weight(Weight::Implied) >= learn::MIN);
}

#[test]
fn a_korean_first_typist_gets_korean_first() {
    let (hits, learner) = simulate(&KOREAN_FIRST, 12);
    eprintln!("{}: habit hits per round {hits:?}, Korean ×{:.2}, English ×{:.2}", KOREAN_FIRST.name, learner.weight(Weight::Korean), learner.weight(Weight::English));
    // The first round already gets some right: a Korean word committed tips the next close call.
    assert!(hits[0] < KOREAN_FIRST.habit);
    assert_eq!(*hits.last().unwrap(), KOREAN_FIRST.habit, "{hits:?}");
    assert!(learner.weight(Weight::English) < 1.0 && learner.weight(Weight::English) >= learn::MIN);
}

#[test]
fn a_hanja_lover_still_never_gets_hanja_first() {
    let (hits, learner) = simulate(&HANJA_LOVER, 20);
    eprintln!("{}: habit hits per round {hits:?}, Hanja ×{:.2}", HANJA_LOVER.name, learner.weight(Weight::Hanja));
    // Learning raises hanja, but the guard keeps them below the top: they stay a ↓ away.
    assert!(learner.weight(Weight::Hanja) > 1.5);
    assert!(hits.iter().all(|&h| h == 0), "{hits:?}");
}

#[test]
fn learning_off_learns_nothing_and_ordinary_words_stay_put() {
    let mut e = Engine::new();
    e.set_predictor(Some(predictor()));
    for (keys, _) in BARE_VOWELS.items.iter().cycle().take(40) {
        for c in keys.chars() {
            e.key(Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), Mods::NONE);
        }
        e.commit_prediction(1);
    }
    assert_eq!(*e.learner(), Learner::default());
    // A trained chatter still types ordinary words as before.
    let (_, learner) = simulate(&BARE_VOWELS, 12);
    let mut e = Engine::new();
    e.set_predictor(Some(predictor()));
    e.set_learner(learner);
    e.set_learning(true);
    for (keys, want) in NEUTRAL {
        for c in keys.chars() {
            e.key(Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), Mods::NONE);
        }
        assert_eq!(e.predictions()[0].text, want, "{keys}");
        e.commit_prediction(0);
    }
}

#[test]
fn a_manual_weight_wins_over_the_learned_one_and_regret_counts() {
    let (_, learner) = simulate(&BARE_VOWELS, 12);
    let mut e = Engine::new();
    e.set_predictor(Some(predictor()));
    e.set_learner(learner.clone());
    e.set_learning(true);
    e.set_prediction_weight(Weight::Implied, 0.5);
    assert_eq!(e.effective_weights().get(Weight::Implied), 0.5);
    e.clear_prediction_weight(Weight::Implied);
    assert_eq!(e.effective_weights().get(Weight::Implied), learner.weight(Weight::Implied));
    // Accept a top prediction, then Backspace right away: a regret, which changes the counts.
    for c in "dkssud".chars() {
        e.key(Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), Mods::NONE);
    }
    e.commit_prediction(0);
    let before = e.learner().clone();
    e.key(Key::Backspace, Mods::NONE);
    assert_ne!(*e.learner(), before);
}

#[test]
fn deleting_only_the_space_after_an_accepted_word_is_no_regret() {
    let mut e = Engine::new();
    e.set_predictor(Some(predictor()));
    e.set_learning(true);
    for c in "dkssud".chars() {
        e.key(Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), Mods::NONE);
    }
    // Space takes the top prediction and adds a space.
    assert!(e.key_after_prediction(0, Key::Space, Mods::NONE).commit.ends_with(' '));
    let before = e.learner().clone();
    // The first Backspace only deletes the space…
    e.key(Key::Backspace, Mods::NONE);
    assert_eq!(*e.learner(), before);
    // …the next one goes into the word: that's regret.
    e.key(Key::Backspace, Mods::NONE);
    assert_ne!(*e.learner(), before);
}

#[test]
fn the_owners_texts_stay_clean_while_learning() {
    // An attentive typist with learning on types the owner's texts five times over, in several
    // styles: no failures, and no more ↓ in the last round (the cheapest way) than in the first.
    let (p, d) = (predictor(), dictionary());
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpora-en-ko/");
    let text = ["mixed-script-1-business.txt", "korean-1-business.txt"]
        .map(|f| std::fs::read_to_string(format!("{dir}{f}")).unwrap())
        .join("\n");
    let mut learner = Learner::default();
    let mut arrows = Vec::new();
    // Typed the cheapest way, then in mixes of the valid ways (as real typists do).
    for style in [Style::Optimal, Style::Mixed(1), Style::Mixed(2), Style::Mixed(3), Style::Optimal] {
        let (report, learned) = type_text(&text, p.clone(), Some(d.clone()), Ranker::Rules, style, true, Some(learner));
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        assert!(report.hanja_on_top.is_empty());
        arrows.push(report.arrows);
        learner = learned.unwrap();
    }
    eprintln!("↓ per round on the owner's texts: {arrows:?}");
    assert!(arrows.last() <= arrows.first(), "{arrows:?}");
}

#[test]
fn an_excluded_app_uses_what_was_learned_but_records_nothing() {
    let (_, learner) = simulate(&BARE_VOWELS, 12);
    let mut e = Engine::new();
    e.set_predictor(Some(predictor()));
    e.set_learner(learner.clone());
    e.set_learning(true);
    e.set_recording(false);
    // The learned weights apply…
    assert_eq!(e.effective_weights().get(Weight::Implied), learner.weight(Weight::Implied));
    // …but choices aren't recorded.
    for c in "hh".chars() {
        e.key(Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), Mods::NONE);
    }
    e.commit_prediction(1);
    e.key(Key::Backspace, Mods::NONE);
    assert_eq!(*e.learner(), learner);
}

#[test]
fn a_commit_key_on_the_korean_top_is_an_accept_and_a_flush_is_no_choice() {
    // §10.11 Accept: the top one committed while other kinds were shown, also when the top is
    // the Korean reading and a plain Space commits it. A focus loss (flush) is nobody's choice.
    let typed = |e: &mut Engine| {
        for c in "dkssud".chars() {
            e.key(Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), Mods::NONE);
        }
        let list = e.predictions();
        assert_eq!(list.first().map(|p| p.kind), Some(Kind::Korean), "{list:?}");
        assert!(list.len() > 1, "{list:?}");
    };
    let mut e = Engine::new();
    e.set_predictor(Some(predictor()));
    e.set_learning(true);
    typed(&mut e);
    assert_eq!(e.key(Key::Space, Mods::NONE).commit, "안녕 ");
    assert_ne!(*e.learner(), Learner::default());

    let mut e = Engine::new();
    e.set_predictor(Some(predictor()));
    e.set_learning(true);
    typed(&mut e);
    assert_eq!(e.flush().commit, "안녕");
    assert_eq!(*e.learner(), Learner::default());
    typed(&mut e);
    let english = e.predictions().iter().position(|p| p.kind == Kind::English).unwrap();
    assert_eq!(e.flush_prediction(english).commit, "dkssud");
    assert_eq!(*e.learner(), Learner::default());
}
