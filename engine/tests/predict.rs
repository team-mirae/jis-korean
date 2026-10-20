//! The predictive layer (spec §10, draft), with the real model. Only built with `predict`.
#![cfg(feature = "predict")]

use std::sync::Arc;

use jis_korean::predict::{Kind, Predictor};
use jis_korean::{Dictionary, Engine, Key, Mods};

fn predictor() -> Arc<Predictor> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/predict/");
    let read = |f: &str| std::fs::read_to_string(format!("{dir}{f}")).unwrap();
    let model = std::fs::read(format!("{dir}ranker.bin")).unwrap();
    Arc::new(Predictor::parse(&model, &read("common-en.txt"), &read("common-ko.txt"), &read("english-nouns.txt")).unwrap())
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
    // JIS Korean reads b b as 유유 (implied ㅇ); without it, the chat word ㅠㅠ, which wins.
    let p = typed("KeyB KeyB").predictions();
    assert_eq!(p[0].text, "ㅠㅠ", "{p:?}");
    assert_eq!(p[0].kind, Kind::BareVowels);
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
    assert!(Predictor::parse(b"nope", "", "", "").is_err());
}

#[test]
fn english_case_variants_only_for_english_words() {
    let korean = typed("KeyG KeyK KeyS KeyR KeyM KeyF").predictions();
    assert!(korean.iter().all(|p| !matches!(p.kind, Kind::EnglishCapitalized | Kind::EnglishUpper)), "{korean:?}");
    let english = texts("KeyH KeyE KeyL KeyL KeyO");
    assert!(english.contains(&"Hello".to_string()) && english.contains(&"HELLO".to_string()), "{english:?}");
}

// Implied ㅇ

#[test]
fn implied_ieung_stays_in_real_words() {
    // 아이 typed k l: the implied ㅇ is wanted; the bare ㅏㅣ is offered lower, if at all.
    assert_eq!(top("KeyK KeyL"), ("아이".into(), Kind::Korean));
    assert_eq!(top("KeyL KeyR KeyJ"), ("이거".into(), Kind::Korean));
}

#[test]
fn runs_of_one_vowel_lean_bare() {
    for (keys, bare) in [("KeyB KeyB KeyB", "ㅠㅠㅠ"), ("KeyN KeyN", "ㅜㅜ"), ("KeyM KeyM", "ㅡㅡ")] {
        assert_eq!(top(keys).0, bare, "{keys}");
    }
}

// Mixed English + Korean particle

#[test]
fn english_word_with_a_particle_is_one_reading() {
    // Anthropic에서, report에, RLHF에: typed without a break between the English and the particle.
    let keys = |word: &str, ending: &str| -> String {
        word.chars()
            .map(|c| if c.is_ascii_uppercase() { format!("S-Key{c}") } else { format!("Key{}", c.to_ascii_uppercase()) })
            .chain(ending.split(' ').map(String::from))
            .collect::<Vec<_>>()
            .join(" ")
    };
    assert_eq!(top(&keys("Anthropic", "KeyP KeyT KeyJ")), ("Anthropic에서".into(), Kind::Mixed));
    assert_eq!(top(&keys("report", "KeyP")), ("report에".into(), Kind::Mixed));
    assert_eq!(top(&keys("RLHF", "KeyP")), ("RLHF에".into(), Kind::Mixed));
    assert_eq!(top(&keys("server", "KeyF KeyM KeyF")), ("server를".into(), Kind::Mixed));
    assert_eq!(top(&keys("server", "KeyM KeyF")), ("server을".into(), Kind::Mixed));
}

#[test]
fn korean_words_with_particles_stay_korean() {
    assert_eq!(top("KeyG KeyK KeyR KeyR KeyY KeyP KeyT KeyJ").0, "학교에서");
    assert_eq!(top("KeyT KeyK KeyF KeyK KeyA S-KeyL").0, "사람이");
    assert_eq!(top("KeyT KeyJ KeyQ KeyJ KeyP").0, "서버에");
}

// Hanja exceptions: news country names, dates, weekdays (not penalized)

#[test]
fn dates_after_digits_get_their_hanja() {
    let t = texts("S-Digit2 S-Digit0 S-Digit2 S-Digit6 KeyS KeyU KeyS");
    assert!(t.contains(&"2026年".to_string()), "{t:?}");
    let t = texts("S-Digit1 S-Digit9 KeyD KeyL KeyF");
    assert!(t.contains(&"19日".to_string()), "{t:?}");
}

#[test]
fn country_and_weekday_hanja_follow_the_korean_reading() {
    // 미 → 美, right after the Korean reading itself (not penalized, but not ahead of it).
    let p = typed("KeyA KeyL").predictions();
    let korean = p.iter().position(|x| x.text == "미").unwrap();
    assert_eq!((p[korean + 1].text.as_str(), p[korean + 1].kind), ("美", Kind::Hanja), "{p:?}");
    let t = texts("KeyG KeyK KeyS KeyA KeyL");
    assert!(t.contains(&"韓美".to_string()), "{t:?}");
    let t = texts("KeyD KeyN KeyJ KeyF KeyG KeyH KeyK KeyT KeyN KeyA KeyH KeyR KeyR KeyM KeyA");
    assert!(t.contains(&"月火水木金".to_string()), "{t:?}");
}

#[test]
fn stem_and_wal_use_the_dictionary() {
    let mut e = typed("KeyR KeyH KeyD KeyW KeyK KeyD KeyH KeyK KeyF");
    e.set_dictionary(Arc::new(Dictionary::parse("공자\t孔子\t\n")));
    assert!(e.predictions().iter().any(|p| p.text == "孔子曰"), "{:?}", e.predictions());
}

#[test]
fn ordinary_single_syllables_get_no_hanja() {
    // 학 is in the dictionary, but single syllables outside the exceptions never get hanja.
    let mut e = typed("KeyG KeyK KeyR");
    e.set_dictionary(Arc::new(Dictionary::parse("학\t學\t배울 학\n")));
    assert!(e.predictions().iter().all(|p| p.kind != Kind::Hanja));
}

#[test]
fn english_words_ending_in_particle_keys_stay_english() {
    // Last keys that would read as a particle (l = 이, p = 에, mf = 을 …) in real English words.
    for word in ["real", "will", "all", "help", "stop", "map", "small", "full", "deep", "shop", "tell", "skill"] {
        let keys: Vec<String> = word.chars().map(|c| format!("Key{}", c.to_ascii_uppercase())).collect();
        assert_eq!(top(&keys.join(" ")), (word.to_string(), Kind::English), "{word}");
    }
}

// User weights

#[test]
fn weights_shift_close_calls_and_leave_hard_rules_alone() {
    use jis_korean::predict::Weight;
    // gg: Korean (ㅎㅎ) by default; English weighted up wins.
    let mut e = typed("KeyG KeyG");
    let default_top = e.predictions()[0].kind;
    e.set_prediction_weight(Weight::English, 4.0);
    e.set_prediction_weight(Weight::Korean, 0.5);
    assert_eq!(e.predictions()[0].kind, Kind::English, "default top was {default_top:?}");
    // A capital at the start is English whatever the weights say.
    let mut e = typed("S-KeyS KeyT KeyE KeyV KeyE");
    e.set_prediction_weight(Weight::Korean, 10.0);
    assert_eq!(e.predictions()[0].kind, Kind::English);
}

#[test]
fn implied_weight_moves_the_bare_reading() {
    use jis_korean::predict::Weight;
    // 아이: the implied ㅇ wins by default; weighted far down, the bare ㅏㅣ comes first.
    let mut e = typed("KeyK KeyL");
    assert_eq!(e.predictions()[0].text, "아이");
    e.set_prediction_weight(Weight::Implied, 0.1);
    assert_eq!(e.predictions()[0].text, "ㅏㅣ", "{:?}", e.predictions());
}

#[test]
fn weights_are_clamped() {
    use jis_korean::predict::{Weight, Weights};
    let mut w = Weights::default();
    w.set(Weight::Korean, 100.0);
    w.set(Weight::English, -1.0);
    w.set(Weight::Hanja, f64::NAN);
    assert_eq!((w.get(Weight::Korean), w.get(Weight::English), w.get(Weight::Hanja)), (10.0, 0.1, 1.0));
}

// Shift+Z with predictions (spec §6)

#[test]
fn shift_z_skips_ahead_when_the_top_prediction_is_already_f10() {
    // hello: the top prediction is "hello", exactly what F10 gives, so Shift+Z goes on to HELLO.
    assert_eq!(typed("KeyH KeyE KeyL KeyL KeyO S-KeyZ").preedit(), "HELLO");
    // The next Shift+Z steps the cycle once, as F10 does: Hello.
    assert_eq!(typed("KeyH KeyE KeyL KeyL KeyO S-KeyZ S-KeyZ").preedit(), "Hello");
    // F10 itself is unchanged.
    assert_eq!(typed("KeyH KeyE KeyL KeyL KeyO F10").preedit(), "hello");
}

#[test]
fn shift_z_is_plain_f10_when_the_top_prediction_differs() {
    // 한글's top prediction is Korean, not gksrmf: Shift+Z is just F10.
    assert_eq!(typed("KeyG KeyK KeyS KeyR KeyM KeyF S-KeyZ").preedit(), "gksrmf");
}

#[test]
fn shift_z_without_a_predictor_is_plain_f10() {
    let mut e = Engine::new();
    for code in ["KeyH", "KeyE", "KeyL", "KeyL", "KeyO"] {
        e.key(Key::from_code(code), Mods::NONE);
    }
    e.key(Key::from_code("KeyZ"), Mods::SHIFTED);
    assert_eq!(e.preedit(), "hello");
}

// Mixed words must not take over real Korean

/// The keys a JIS Korean typist presses for a Hangul word: 2-set, with the implied ㅇ where it
/// works and the double press after a final (as the robot and the ranker's training do).
fn jis_keys(word: &str) -> Option<Vec<(String, bool)>> {
    const I: [&str; 19] = ["r", "R", "s", "e", "E", "f", "a", "q", "Q", "t", "T", "d", "w", "W", "c", "z", "x", "v", "g"];
    const M: [&str; 21] = ["k", "o", "i", "O", "j", "p", "u", "P", "h", "hk", "ho", "hl", "y", "n", "nj", "np", "nl", "b", "m", "ml", "l"];
    const F: [&str; 28] = ["", "r", "R", "rt", "s", "sw", "sg", "e", "f", "fr", "fa", "fq", "ft", "fx", "fv", "fg", "a", "q", "qt", "t", "T", "d", "w", "c", "z", "x", "v", "g"];
    let combines = |prev_m: usize, next: char| matches!((prev_m, next), (8, 'k' | 'o' | 'l') | (13, 'j' | 'p' | 'l') | (18, 'l'));
    let mut keys = String::new();
    let mut prev: Option<(usize, usize)> = None; // (medial, final)
    for ch in word.chars() {
        let n = (ch as u32).checked_sub(0xAC00).filter(|&n| n < 11172)? as usize;
        let (i, m, f) = (n / 588, n % 588 / 28, n % 28);
        let vowel = M[m];
        if i == 11 {
            match prev {
                Some((_, pf)) if pf > 0 => keys.push_str(&vowel[..1]),
                Some((pm, _)) if combines(pm, vowel.chars().next().unwrap()) => keys.push('d'),
                _ => {}
            }
        } else {
            keys.push_str(I[i]);
        }
        keys.push_str(vowel);
        keys.push_str(F[f]);
        prev = Some((m, f));
    }
    Some(keys.chars().map(|c| (format!("Key{}", c.to_ascii_uppercase()), c.is_ascii_uppercase())).collect())
}

#[test]
fn kanxms_is_amuteun_not_a_mixed_word() {
    assert_eq!(top("KeyK KeyA KeyN KeyX KeyM KeyS"), ("아무튼".into(), Kind::Korean));
}

#[test]
fn real_korean_words_almost_never_rank_as_mixed() {
    // Multi-syllable readings from the hanja dictionary, plus the Samsung corpus if fetched.
    let dict = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/dict/hanja.tsv")).unwrap();
    let mut words: Vec<String> = dict.lines().filter_map(|l| l.split('\t').next()).filter(|r| r.chars().count() >= 2).step_by(40).map(String::from).collect();
    words.dedup();
    if let Ok(corpus) = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/samsung-business.txt")) {
        words.extend(corpus.split(|c: char| !('가'..='힣').contains(&c)).filter(|w| w.chars().count() >= 2).map(String::from));
    }
    let p = predictor();
    // With the hanja dictionary, as the apps run.
    let dictionary = Arc::new(Dictionary::parse(&dict));
    let (mut total, mut mixed, mut english) = (0, Vec::new(), 0);
    for word in &words {
        let Some(keys) = jis_keys(word) else { continue };
        let mut e = Engine::new();
        e.set_dictionary(dictionary.clone());
        e.set_predictor(Some(p.clone()));
        for (code, shift) in &keys {
            e.key(Key::from_code(code), if *shift { Mods::SHIFTED } else { Mods::NONE });
        }
        total += 1;
        match e.predictions().first().map(|x| x.kind) {
            Some(Kind::Mixed) => mixed.push(word.clone()),
            Some(Kind::English | Kind::EnglishCapitalized | Kind::EnglishUpper) => english += 1,
            _ => {}
        }
    }
    eprintln!("{english} of {total} Korean words ranked as English ({:.2}%)", english as f64 * 100.0 / total as f64);
    assert!((english as f64 / total as f64) < 0.03, "{english} of {total} Korean words ranked as English");
    let rate = mixed.len() as f64 / total as f64;
    eprintln!("{} of {total} Korean words ranked as mixed ({:.2}%): {:?}", mixed.len(), rate * 100.0, &mixed[..mixed.len().min(40)]);
    assert!(rate < 0.002, "{:.2}% of Korean words ranked as mixed", rate * 100.0);
}

// English nouns (WordNet lemmas) weigh more than verbs and other forms

#[test]
fn noun_lemmas_lean_english_more_than_other_forms() {
    // The same bonus gap for a noun vs a non-noun: compare against a predictor without nouns.
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/predict/");
    let read = |f: &str| std::fs::read_to_string(format!("{dir}{f}")).unwrap();
    let model = std::fs::read(format!("{dir}ranker.bin")).unwrap();
    let without = Arc::new(Predictor::parse(&model, &read("common-en.txt"), &read("common-ko.txt"), "").unwrap());
    let english_score = |p: &Arc<Predictor>, keys: &str| {
        let mut e = Engine::new();
        e.set_predictor(Some(p.clone()));
        for k in keys.split(' ') {
            e.key(Key::from_code(k), Mods::NONE);
        }
        e.predictions().into_iter().find(|x| x.kind == Kind::English).map(|x| x.score).unwrap()
    };
    let gain = |keys: &str| english_score(&predictor(), keys) - english_score(&without, keys);
    // walrus (a noun lemma in no word list) gains; published (a verb form) doesn't.
    assert!(gain("KeyW KeyA KeyL KeyR KeyU KeyS") > 1.0, "noun gain {}", gain("KeyW KeyA KeyL KeyR KeyU KeyS"));
    assert_eq!(gain("KeyP KeyU KeyB KeyL KeyI KeyS KeyH KeyE KeyD"), 0.0);
}

#[test]
fn implied_weight_moves_the_bare_vowel_row_both_ways() {
    // ㅏㅣ is both the bare-vowel and the 2-set reading: one row, moved by the implied ㅇ weight.
    let ranked = |weight: f64| {
        let mut e = typed("KeyK KeyL");
        e.set_prediction_weight(jis_korean::predict::Weight::Implied, weight);
        e.predictions().into_iter().map(|p| p.text).collect::<Vec<_>>()
    };
    let at = |t: &[String], s: &str| t.iter().position(|x| x == s).unwrap();
    let (low, high) = (ranked(0.1), ranked(10.0));
    assert_eq!(low[0], "ㅏㅣ", "{low:?}");
    assert!(at(&high, "ㅏㅣ") > at(&high, "kl"), "{high:?}");
    assert_eq!(low.iter().filter(|x| *x == "ㅏㅣ").count(), 1);
}

#[test]
fn a_lone_vowel_key_is_its_implied_syllable_not_a_letter() {
    for (key, syllable) in [("KeyH", "오"), ("KeyU", "여"), ("KeyO", "애"), ("KeyI", "야"), ("KeyN", "우"), ("KeyM", "으"), ("KeyB", "유")] {
        assert_eq!(top(key), (syllable.into(), Kind::Korean), "{key}");
    }
}

// The travel ranker (spec §10.7, experimental)

fn top_by(ranker: jis_korean::predict::Ranker, keys: &str) -> String {
    let mut e = typed(keys);
    e.set_ranker(ranker);
    e.predictions()[0].text.clone()
}

#[test]
fn travel_ranker_keeps_chat_vowels_and_real_words() {
    use jis_korean::predict::{Effort, Ranker};
    let travel = Ranker::Travel(Effort::default());
    // ㅠㅠ is a chat word either way; 아이 stays a word.
    assert_eq!(top_by(travel, "KeyB KeyB"), "ㅠㅠ");
    assert_eq!(top_by(travel, "KeyK KeyL"), "아이");
    // A run of a vowel chat doesn't double as jamo stays syllables, for both.
    assert_eq!(top_by(Ranker::Rules, "KeyH KeyH KeyQ KeyO KeyR KeyS KeyU KeyS"), "오오백년");
    assert_eq!(top_by(travel, "KeyH KeyH KeyQ KeyO KeyR KeyS KeyU KeyS"), "오오백년");
    // ㅠㅠ is the chat word, by the lexicon's counts, for the rules too.
    assert_eq!(top_by(Ranker::Rules, "KeyB KeyB KeyB"), "ㅠㅠㅠ");
}

// The route-preference ranker (spec §10.8, experimental)

#[test]
fn route_ranker_keeps_words_chat_runs_and_typed_readings() {
    use jis_korean::predict::{Preference, Ranker};
    let route = Ranker::Route(Preference::default());
    // A run of one vowel is a chat word; 아이 and 오오백년 are words; jamo-only mixes aren't.
    assert_eq!(top_by(route, "KeyB KeyB"), "ㅠㅠ");
    assert_eq!(top_by(route, "KeyK KeyL"), "아이");
    assert_eq!(top_by(route, "KeyH KeyH KeyQ KeyO KeyR KeyS KeyU KeyS"), "오오백년");
    assert_eq!(top_by(route, "KeyJ KeyK KeyQ"), "어압");
    // 데이터의 typed with ㅡ ㅣ, not the dedicated ㅢ key, is still 데이터의.
    assert_eq!(top_by(route, "KeyE KeyP KeyL KeyX KeyJ KeyM KeyL"), "데이터의");
}

// Suboptimal routes (spec §10.10): every valid way to type a word gets it, as typed and on top

#[test]
fn every_route_the_owner_named_types_the_word_on_top() {
    let routes: [(&str, &[&str]); 2] = [
        ("의", &["Digit9", "KeyD Digit9", "KeyD KeyM KeyL"]),
        ("당신이", &["KeyE KeyK KeyD KeyT KeyL KeyS S-KeyL", "KeyE KeyK KeyD KeyT KeyL KeyS KeyL KeyL", "KeyE KeyK KeyD KeyT KeyL KeyS KeyD KeyL"]),
    ];
    for (word, ways) in routes {
        for keys in ways {
            let e = typed(keys);
            assert_eq!(e.preedit(), word, "{keys}");
            assert_eq!(e.predictions()[0].text, word, "{keys}: {:?}", e.predictions());
        }
    }
}

#[test]
fn a_word_started_with_shift_vowel_is_korean_and_names_stay_english() {
    // 있습니다 with Shift on ㅣ (and ㅆ by Shift): Korean, not LTtmqslek.
    assert_eq!(top("S-KeyL S-KeyT KeyT KeyM KeyQ KeyS KeyL KeyE KeyK"), ("있습니다".into(), Kind::Korean));
    // A known English word with the same kind of capital stays English for sure.
    assert_eq!(top("S-KeyH KeyE KeyL KeyL KeyO").1, Kind::English);
    // An acronym before a particle is still English + particle.
    assert_eq!(top("S-KeyK S-KeyP S-KeyI Digit0"), ("KPI와".into(), Kind::Mixed));
}

#[test]
fn the_web_demos_easy_words_type_their_word_on_top() {
    // web/src/App.tsx EASY_WORDS (from docs/easy-words.md).
    for (keys, word) in [("98", "의외"), ("09", "와의"), ("a7", "뭐"), ("3h", "또"), ("h1k", "오빠"), ("98fh", "의외로"), ("k4k7", "아까워"), ("l3k6", "이따위")] {
        let codes: Vec<String> = keys.chars().map(|c| if c.is_ascii_digit() { format!("Digit{c}") } else { format!("Key{}", c.to_ascii_uppercase()) }).collect();
        let e = typed(&codes.join(" "));
        assert_eq!(e.preedit(), word, "{keys}");
        assert_eq!(e.predictions()[0].text, word, "{keys}: {:?}", e.predictions());
    }
}

// Keeping a final before an implied ㅇ (spec §10.3; 한글 맞춤법 제14·15·19·22항): typed the lazy way
// (no double press, no ㅇ), the right spelling comes first; where both spellings are words, both
// are offered.

/// The keys for a word typed lazily: an ㅇ-initial syllable after a final gets no key of its own.
fn lazy_keys(word: &str) -> String {
    const I: [&str; 19] = ["KeyR", "S-KeyR", "KeyS", "KeyE", "S-KeyE", "KeyF", "KeyA", "KeyQ", "S-KeyQ", "KeyT", "S-KeyT", "KeyD", "KeyW", "S-KeyW", "KeyC", "KeyZ", "KeyX", "KeyV", "KeyG"];
    const M: [&str; 21] = ["KeyK", "KeyO", "KeyI", "S-KeyO", "KeyJ", "KeyP", "KeyU", "S-KeyP", "KeyH", "KeyH KeyK", "KeyH KeyO", "KeyH KeyL", "KeyY", "KeyN", "KeyN KeyJ", "KeyN KeyP", "KeyN KeyL", "KeyB", "KeyM", "KeyM KeyL", "KeyL"];
    const F: [&str; 28] = ["", "KeyR", "S-KeyR", "KeyR KeyT", "KeyS", "KeyS KeyW", "KeyS KeyG", "KeyE", "KeyF", "KeyF KeyR", "KeyF KeyA", "KeyF KeyQ", "KeyF KeyT", "KeyF KeyX", "KeyF KeyV", "KeyF KeyG", "KeyA", "KeyQ", "KeyQ KeyT", "KeyT", "S-KeyT", "KeyD", "KeyW", "KeyC", "KeyZ", "KeyX", "KeyV", "KeyG"];
    let mut keys = Vec::new();
    let mut prev_final = false;
    for ch in word.chars() {
        let n = ch as usize - 0xAC00;
        let (i, m, f) = (n / 588, n % 588 / 28, n % 28);
        if !(i == 11 && (prev_final || keys.is_empty())) {
            keys.push(I[i]);
        }
        keys.push(M[m]);
        if f > 0 {
            keys.push(F[f]);
        }
        prev_final = f > 0;
    }
    keys.into_iter().filter(|k| !k.is_empty()).collect::<Vec<_>>().join(" ")
}

#[test]
fn kept_and_moved_finals_follow_the_spelling_rules() {
    let p = predictor();
    let dict = Arc::new(Dictionary::parse(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/dict/hanja.tsv")).unwrap()));
    let ranked = |word: &str| {
        let mut e = Engine::new();
        e.set_dictionary(dict.clone());
        e.set_predictor(Some(p.clone()));
        for token in lazy_keys(word).split(' ') {
            let (mods, code) = token.strip_prefix("S-").map_or((Mods::NONE, token), |c| (Mods::SHIFTED, c));
            e.key(Key::from_code(code), mods);
        }
        e.predictions().into_iter().map(|x| x.text).collect::<Vec<_>>()
    };
    // One right spelling: it comes first.
    let first = [
        "당신이", "기준입니다", "드리겠습니다", "지난주", "괜찮더라", "사람", "먹어", "웃음", "무덤", "마개", "마중", "걸어",
        "많이", "좋은", "앉아", "값이", "드러나다", "깍두기", "바깥", "지붕", "바둑이", "할아버지", "일요일", "국어",
        // A ㅇ final before a particle that only follows consonants (the owner's 소비량은).
        "소비량은", "생산량을", "공간을", "방법으로", "사용량이", "아이",
    ];
    let mut misses = Vec::new();
    for word in first {
        let list = ranked(word);
        if list.first().map(String::as_str) != Some(word) {
            misses.push(format!("{word}: {list:?}"));
        }
    }
    // Both spellings are words: both offered.
    let pairs = [("산이", "사니"), ("말이", "마리"), ("길이", "기리"), ("너머", "넘어"), ("어린이", "어리니"), ("놀이", "노리"), ("얼음", "어름")];
    for (kept, moved) in pairs {
        let list = ranked(kept);
        if !(list.contains(&kept.to_string()) && list.contains(&moved.to_string())) {
            misses.push(format!("{kept}/{moved}: {list:?}"));
        }
    }
    eprintln!("{} of {} edge cases miss:\n  {}", misses.len(), first.len() + pairs.len(), misses.join("\n  "));
    // The bar: about 1 in 10 needing a manual pick.
    assert!(misses.len() * 10 <= first.len() + pairs.len(), "{misses:#?}");
}

// An implied ㄹ after a ㄴ final (spec §10.5): 전력 typed lazily as 저녁's keys

#[test]
fn an_implied_l_after_n_is_offered_and_its_weight_0_turns_it_off() {
    use jis_korean::predict::Weight;
    let keys = "KeyW KeyJ KeyS KeyU KeyR"; // ㅈ ㅓ ㄴ ㅕ ㄱ: 저녁 as composed
    let p = typed(keys).predictions();
    assert_eq!(p[0].text, "저녁", "{p:?}");
    assert!(p.iter().any(|x| x.text == "전력" && x.kind == Kind::ImpliedL), "{p:?}");
    // Weight 0: gone entirely.
    let mut e = typed(keys);
    e.set_prediction_weight(Weight::ImpliedL, 0.0);
    assert!(e.predictions().iter().all(|x| x.kind != Kind::ImpliedL), "{:?}", e.predictions());
    // Korean and English can't be turned off: 0 is clamped up.
    let mut e = typed(keys);
    e.set_prediction_weight(Weight::Korean, 0.0);
    assert_eq!(e.effective_weights().get(Weight::Korean), jis_korean::predict::Weights::MIN);
    // Only after a ㄴ final: 사람 has none to offer.
    assert!(typed("KeyT KeyK KeyF KeyK KeyA").predictions().iter().all(|x| x.kind != Kind::ImpliedL));
}

/// Mode B (spec §3.6): predictions read the keys as US ANSI, so Shift+2 is @ there, never JIS's ".
#[test]
fn mode_b_predictions_read_us_symbols() {
    use jis_korean::Mode;
    let typed_in = |mode: Mode, keys: &str| {
        let mut e = Engine::new();
        e.set_mode(mode);
        e.set_predictor(Some(predictor()));
        for token in keys.split_whitespace() {
            let (mods, code) = match token.strip_prefix("S-") {
                Some(code) => (Mods::SHIFTED, code),
                None => (Mods::NONE, token),
            };
            e.key(Key::from_code(code), mods);
        }
        e.predictions().into_iter().map(|p| p.text).collect::<Vec<_>>()
    };
    // The standard 2-set reading (spec §10.2) types the stock keyboard's symbol: " on JIS, @ on US.
    assert_eq!(typed_in(Mode::A, "KeyG KeyK KeyS S-Digit2"), ["한2", "한\"", "gks2"]);
    let b = typed_in(Mode::B, "KeyG KeyK KeyS S-Digit2");
    assert_eq!(b, ["한2", "한@", "gks2"]);
}

/// Resolving keys (spec §8.3) follows what the shell shows with predictions on: the top prediction,
/// committed by Space.
#[test]
fn resolving_follows_predictions() {
    use jis_korean::resolve::{key_input_after, keys_for_output, type_keys};
    let hello = ["KeyH", "KeyE", "KeyL", "KeyL", "KeyO"];
    let plain = Engine::new();
    let mut predicting = Engine::new();
    predicting.set_predictor(Some(predictor()));
    assert_eq!(type_keys(&plain, &hello).as_deref(), Some("옫이애"));
    assert_eq!(type_keys(&predicting, &hello).as_deref(), Some("hello"));
    assert_eq!(key_input_after(&predicting, "hello", "Space").as_deref(), Some("hello "));
    assert_eq!(key_input_after(&plain, "옫이애", "Space").as_deref(), Some("옫이애 "));
    let found = keys_for_output(&predicting, "hello", 5, 5);
    assert!(found.iter().any(|r| r.join(" ") == "KeyH KeyE KeyL KeyL KeyO"), "{found:?}");
    for route in &found {
        assert_eq!(type_keys(&predicting, route).as_deref(), Some("hello"), "{route:?}");
    }
}
