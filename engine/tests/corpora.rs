//! The owner's English–Korean texts (tests/corpora-en-ko/, verbatim), typed the trusting way
//! (spec §10.10): the fewest presses, predictions assumed right, Space commits the top one.
//! Failures are ratcheted: they may go down, never up. See the report with
//! `cargo test --test corpora -- --nocapture`.
//!
//! tests/corpora-en-ko/extended/ holds the same texts continued by AI agents (marked "human
//! corpus extended by AI" on their first line, which isn't typed). They weigh AI_WEIGHT of the
//! owner's text in the weighted failure rate, and have their own ratchet.
#![cfg(feature = "predict")]

use std::sync::Arc;

use jis_korean::Dictionary;
use jis_korean::predict::{Effort, Predictor, Preference, Ranker};
use jis_korean::plan::Style;
use jis_korean::trust::{Trusting, type_text, type_trusting};

/// Failed words allowed per text, with the default ranker. Lower these when they improve.
const MAX_FAILURES: &[(&str, usize)] = &[("mixed-script-1-business.txt", 0), ("korean-1-business.txt", 0)];

/// How much a word of AI-extended text counts against a word of the owner's own.
const AI_WEIGHT: f64 = 0.25;

/// Failed words allowed in all of extended/ together, with the default ranker.
const MAX_AI_FAILURES: usize = 3;

fn predictor() -> Arc<Predictor> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/predict/");
    let read = |f: &str| std::fs::read_to_string(format!("{dir}{f}")).unwrap();
    let model = std::fs::read(format!("{dir}ranker.bin")).unwrap();
    Arc::new(Predictor::parse(&model, &read("common-en.txt"), &read("common-ko.txt"), &read("english-nouns.txt")).unwrap())
}

fn dictionary() -> Arc<Dictionary> {
    Arc::new(Dictionary::parse(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/dict/hanja.tsv")).unwrap()))
}

fn summary(name: &str, t: &Trusting) -> String {
    format!(
        "{name}: {}/{} words failed ({:.1}%), {} of {} characters wrong, {} hanja words picked with {} ↓; {} presses + {} to correct = {} to the right text, against {} without predictions ({:+.1}%)",
        t.failures.len(),
        t.words,
        100.0 * t.failures.len() as f64 / t.words as f64,
        t.char_errors,
        t.chars,
        t.hanja_picked,
        t.arrows,
        t.presses,
        t.corrections,
        t.effort(),
        t.deterministic,
        100.0 * (t.effort() as f64 / t.deterministic as f64 - 1.0),
    )
}

#[test]
fn owner_texts_typed_trusting_the_predictions() {
    let (p, d) = (predictor(), dictionary());
    for &(file, max) in MAX_FAILURES {
        let text = std::fs::read_to_string(format!("{}/tests/corpora-en-ko/{file}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let rules = type_trusting(&text, p.clone(), Some(d.clone()), Ranker::Rules);
        eprintln!("\n{}", summary(&format!("{file} (rules)"), &rules));
        for f in &rules.failures {
            eprintln!("  ✗ {:24} → {:24} [{}]  shown: {}", f.want, f.got, f.keys, f.list.join(" · "));
        }
        for (name, ranker) in [("travel", Ranker::Travel(Effort::default())), ("route", Ranker::Route(Preference::default()))] {
            eprintln!("{}", summary(&format!("{file} ({name})"), &type_trusting(&text, p.clone(), Some(d.clone()), ranker)));
        }
        assert!(rules.failures.len() <= max, "{file}: {} failures, at most {max} allowed", rules.failures.len());
        assert!(rules.hanja_on_top.is_empty(), "{file}: hanja ranked first for {:?}", rules.hanja_on_top);
    }
}

/// The AI-extended texts, without their marker line.
fn extended() -> Vec<(String, String)> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpora-en-ko/extended");
    let mut files: Vec<(String, String)> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|x| x == "txt"))
                .map(|e| {
                    let text = std::fs::read_to_string(e.path()).unwrap();
                    let body: Vec<&str> = text.lines().filter(|l| !l.starts_with("# ")).collect();
                    (e.file_name().to_string_lossy().into_owned(), body.join("\n").trim().to_string())
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

#[test]
fn ai_extended_texts_weigh_less() {
    let (p, d) = (predictor(), dictionary());
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpora-en-ko/");
    let human: Vec<Trusting> = MAX_FAILURES
        .iter()
        .map(|(file, _)| type_trusting(&std::fs::read_to_string(format!("{dir}{file}")).unwrap(), p.clone(), Some(d.clone()), Ranker::Rules))
        .collect();
    let (mut ai_words, mut ai_failures) = (0, 0);
    for (name, text) in extended() {
        let t = type_trusting(&text, p.clone(), Some(d.clone()), Ranker::Rules);
        eprintln!("{}", summary(&format!("{name} (rules)"), &t));
        for f in &t.failures {
            eprintln!("  ✗ {:24} → {:24} [{}]  shown: {}", f.want, f.got, f.keys, f.list.join(" · "));
        }
        assert!(t.hanja_on_top.is_empty(), "{name}: hanja ranked first for {:?}", t.hanja_on_top);
        ai_words += t.words;
        ai_failures += t.failures.len();
    }
    let (human_words, human_failures): (usize, usize) = human.iter().fold((0, 0), |(w, f), t| (w + t.words, f + t.failures.len()));
    let weighted = (human_failures as f64 + AI_WEIGHT * ai_failures as f64) / (human_words as f64 + AI_WEIGHT * ai_words as f64);
    eprintln!(
        "\nweighted failure rate {:.2}%: owner's text {human_failures}/{human_words}, AI-extended {ai_failures}/{ai_words} at ×{AI_WEIGHT}",
        100.0 * weighted
    );
    assert!(ai_failures <= MAX_AI_FAILURES, "AI-extended texts: {ai_failures} failures, at most {MAX_AI_FAILURES} allowed");
}

/// Typing styles real typists use besides the cheapest (spec §10.10): every jamo (ㅇ typed, Shift
/// for doubles, two keys for compound vowels), and mixes of the valid ways per syllable (의 as 9,
/// d9 or dml; 당신이 as ekdtlsL, ekdtlsll or ekdtlsdl). Failed words allowed in extended/ per
/// style, with the default ranker; the owner's texts allow none in any style.
const STYLES: &[(Style, usize)] = &[(Style::EveryJamo, 2), (Style::Mixed(1), 5), (Style::Mixed(2), 5), (Style::Mixed(3), 5)];

#[test]
fn every_typing_style_works() {
    let (p, d) = (predictor(), dictionary());
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpora-en-ko/");
    let owner: String = MAX_FAILURES.iter().map(|(f, _)| std::fs::read_to_string(format!("{dir}{f}")).unwrap()).collect::<Vec<_>>().join("\n");
    let ai: String = extended().into_iter().map(|(_, t)| t).collect::<Vec<_>>().join("\n");
    for &(style, max_ai) in STYLES {
        let o = type_text(&owner, p.clone(), Some(d.clone()), Ranker::Rules, style, false, None).0;
        let a = type_text(&ai, p.clone(), Some(d.clone()), Ranker::Rules, style, false, None).0;
        eprintln!("{style:?}: owner's texts {}/{} failed, AI-extended {}/{}", o.failures.len(), o.words, a.failures.len(), a.words);
        for f in o.failures.iter().chain(&a.failures) {
            eprintln!("  ✗ {:24} → {:24} [{}]", f.want, f.got, f.keys);
        }
        assert!(o.failures.is_empty(), "{style:?}: the owner's texts failed {:?}", o.failures.iter().map(|f| &f.want).collect::<Vec<_>>());
        assert!(a.failures.len() <= max_ai, "{style:?}: {} AI-extended failures, at most {max_ai}", a.failures.len());
        assert!(o.hanja_on_top.is_empty() && a.hanja_on_top.is_empty());
    }
}
