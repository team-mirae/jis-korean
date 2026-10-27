//! Types text files the trusting way (spec §10.10) with each ranker (the learned one first, when
//! predict/learned.bin is there) and reports the failures.
//! `{keys=word}` tokens (from examples/abc.rs) are read as the word.
//! Usage: cargo run --release --example trust -- [--failures] [--style optimal|every-jamo|mixed:<seed>] <text files…>

use std::sync::Arc;

use jis_korean::Dictionary;
use jis_korean::predict::{Effort, Predictor, Preference, Ranker};
use jis_korean::plan::Style;
use jis_korean::trust::type_text;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let show = args.iter().any(|a| a == "--failures");
    let style = match args.iter().position(|a| a == "--style").map(|i| args[i + 1].as_str()) {
        Some("every-jamo") => Style::EveryJamo,
        Some(s) if s.starts_with("mixed:") => Style::Mixed(s[6..].parse().expect("mixed:<seed>")),
        _ => Style::Optimal,
    };
    let root = env!("CARGO_MANIFEST_DIR");
    let read = |f: &str| std::fs::read_to_string(format!("{root}/predict/{f}")).unwrap();
    let model = std::fs::read(format!("{root}/predict/ranker.bin")).unwrap();
    let rules = Predictor::parse(&model, &read("common-en.txt"), &read("common-ko.txt"), &read("english-nouns.txt")).unwrap();
    // The learned ranker (spec §10.13), when its model is there and this isn't the legacy build.
    let learned = std::fs::read(format!("{root}/predict/learned.bin"))
        .ok()
        .and_then(|m| rules.with_learned(&m).ok())
        .filter(Predictor::is_learned)
        .map(Arc::new);
    let p = Arc::new(rules);
    let d = Arc::new(Dictionary::parse(&std::fs::read_to_string(format!("{root}/dict/hanja.tsv")).unwrap()));
    let text: String = args
        .iter()
        .enumerate()
        .filter(|(i, a)| !a.starts_with("--") && !(*i > 0 && args[i - 1] == "--style"))
        .map(|(_, a)| a)
        .map(|f| std::fs::read_to_string(f).expect("read"))
        .map(|t| without_keys(&t))
        .collect::<Vec<_>>()
        .join("\n");
    let rankers = [
        ("rules", p.clone(), Ranker::Rules),
        ("travel", p.clone(), Ranker::Travel(Effort::default())),
        ("route", p.clone(), Ranker::Route(Preference::default())),
    ];
    for (name, p, ranker) in learned.map(|l| ("learned", l, Ranker::Rules)).into_iter().chain(rankers) {
        let t = type_text(&text, p, Some(d.clone()), ranker, style, false, None).0;
        println!(
            "{name:7} {}/{} words failed ({:.2}%), {} chars wrong, {} hanja on top; effort {} vs {} without predictions ({:+.1}%)",
            t.failures.len(), t.words, 100.0 * t.failures.len() as f64 / t.words as f64, t.char_errors, t.hanja_on_top.len(),
            t.effort(), t.deterministic, 100.0 * (t.effort() as f64 / t.deterministic as f64 - 1.0)
        );
        if show && (name == "rules" || name == "learned") {
            for f in &t.failures {
                println!("  ✗ {:22} → {:22} [{}]  {}", f.want, f.got, f.keys, f.list.join(" · "));
            }
        }
    }
}

/// The text with each `{keys=word}` token replaced by its word.
fn without_keys(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('}') else { break };
        let inner = &rest[open + 1..open + close];
        out.push_str(inner.split_once('=').map_or(inner, |(_, word)| word));
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}
