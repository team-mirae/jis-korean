//! Blind A/B/C typing for evaluating the three rankers (spec §10.7–10.8). Types a text word by
//! word into three engines, one per ranker behind the labels A, B and C, and reports where their
//! predictions differ. Which ranker is which comes from the file named by `ABC_MAP` (one line:
//! three of rules / travel / route, in A, B, C order); raters never see it.
//!
//! Usage: ABC_MAP=<map file> cargo run --release --example abc -- --state <file> [--all] <text file>
//!        cargo run --release --example abc -- --keys <word>   (the JIS Korean keys for a word)
//!
//! Text: words separated by spaces. Korean words are typed the way a fluent JIS Korean typist
//! would (fewest presses), jamo-only words (ㅋㅋ, ㅠㅠ) one key per jamo, Latin words letter by
//! letter. `{keys=같은}` types raw keys instead (letters; a capital is Shift) for the
//! word that's meant, e.g. `{rkxms=같은}` for 같은 typed without the double press.
//! Punctuation and digits count as presses but aren't evaluated.

use std::collections::HashMap;
use std::sync::Arc;

use jis_korean::plan::{Metric, Press, Robot, each_jamo, keys_of};
use jis_korean::predict::{Effort, Predictor, Preference, Ranker};
use jis_korean::{Dictionary, Engine, Key, Mods};

const LABELS: [&str; 3] = ["A", "B", "C"];

fn ranker(name: &str) -> Ranker {
    match name.trim() {
        "rules" => Ranker::Rules,
        "travel" => Ranker::Travel(Effort::default()),
        "route" => Ranker::Route(Preference::default()),
        other => panic!("unknown ranker {other:?} in ABC_MAP"),
    }
}

fn is_hangul(c: char) -> bool {
    ('가'..='힣').contains(&c) || ('ㄱ'..='ㅣ').contains(&c)
}

/// The keys for a word as a fluent typist presses them.
fn word_keys(word: &str) -> Option<Vec<Press>> {
    let mut keys = Vec::new();
    let chars: Vec<char> = word.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let latin = chars[i].is_ascii_alphabetic();
        let end = chars[i..].iter().position(|&c| c.is_ascii_alphabetic() != latin).map_or(chars.len(), |n| i + n);
        let run: String = chars[i..end].iter().collect();
        if latin {
            keys.extend(keys_of(&run));
        } else if run.chars().all(|c| ('ㄱ'..='ㅣ').contains(&c)) {
            keys.extend(each_jamo(&run)?);
        } else {
            let mut robot = Robot::new(None);
            robot.metric = Metric::Presses;
            robot.type_text(&run);
            if !robot.report.failures.is_empty() {
                return None;
            }
            keys.extend(robot.log.iter().copied().filter(|p| !matches!(p.key, Key::F6 | Key::F7 | Key::F8 | Key::F9 | Key::F10)));
        }
        i = end;
    }
    Some(keys)
}

fn spell(keys: &[Press]) -> String {
    keys.iter()
        .map(|p| {
            let code = p.key.code();
            let c = code.strip_prefix("Key").or_else(|| code.strip_prefix("Digit")).unwrap_or(code);
            if p.shift { c.to_string() } else { c.to_lowercase() }
        })
        .collect()
}

#[derive(Default)]
struct Tally {
    top: usize,
    listed: usize,
    total: usize,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(i) = args.iter().position(|a| a == "--keys") {
        let word = &args[i + 1];
        match word_keys(word) {
            Some(keys) => println!("{word}: {}", spell(&keys)),
            None => println!("{word}: can't be typed"),
        }
        return;
    }
    let all = args.iter().any(|a| a == "--all");
    let state_path = args.iter().position(|a| a == "--state").map(|i| args[i + 1].clone()).expect("--state <file>");
    let text_path = args.last().expect("text file");
    let map = std::fs::read_to_string(std::env::var("ABC_MAP").expect("ABC_MAP")).expect("read ABC_MAP");
    let rankers: Vec<Ranker> = map.trim().split(',').map(ranker).collect();
    assert_eq!(rankers.len(), 3);

    let root = env!("CARGO_MANIFEST_DIR");
    let read = |f: &str| std::fs::read_to_string(format!("{root}/predict/{f}")).unwrap();
    let model = std::fs::read(format!("{root}/predict/ranker.bin")).unwrap();
    let predictor = Arc::new(Predictor::parse(&model, &read("common-en.txt"), &read("common-ko.txt"), &read("english-nouns.txt")).unwrap());
    let dictionary = Arc::new(Dictionary::parse(&std::fs::read_to_string(format!("{root}/dict/hanja.tsv")).unwrap()));
    let mut engines: Vec<Engine> = rankers
        .iter()
        .map(|&r| {
            let mut e = Engine::new();
            e.set_dictionary(dictionary.clone());
            e.set_predictor(Some(predictor.clone()));
            e.set_ranker(r);
            e
        })
        .collect();

    // Running totals across runs, so a rater can type in several sittings.
    let mut state: HashMap<String, usize> = std::fs::read_to_string(&state_path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once(' '))
        .filter_map(|(k, v)| Some((k.to_string(), v.parse().ok()?)))
        .collect();
    let mut tallies: [Tally; 3] = Default::default();
    let mut presses = 0;
    let mut differing = 0;
    let text = std::fs::read_to_string(text_path).expect("read the text file");
    for token in text.split_whitespace() {
        presses += 1; // the space (or Enter) after it
        // {keys=word}: raw keys for the word meant.
        let (keys, want) = if let Some(inner) = token.strip_prefix('{').and_then(|t| t.split_once('}')).map(|(t, rest)| {
            presses += rest.chars().count();
            t
        }) {
            let Some((raw, want)) = inner.split_once('=') else { continue };
            (keys_of(raw), want.to_string())
        } else {
            // Punctuation and digits around and inside: counted, not evaluated.
            let core: String = token.chars().filter(|&c| is_hangul(c) || c.is_ascii_alphabetic()).collect();
            for c in token.chars().filter(|&c| !(is_hangul(c) || c.is_ascii_alphabetic())) {
                presses += if c.is_ascii_digit() || "!@#$%^&*()_+{}|:\"<>?".contains(c) { 2 } else { 1 };
            }
            if core.is_empty() {
                continue;
            }
            let Some(keys) = word_keys(&core) else {
                println!("  (skipped, can't be typed: {core})");
                continue;
            };
            (keys, core)
        };
        presses += keys.iter().map(|p| 1 + p.shift as usize).sum::<usize>();
        let mut lists = Vec::new();
        for (e, tally) in engines.iter_mut().zip(tallies.iter_mut()) {
            for p in &keys {
                e.key(p.key, if p.shift { Mods::SHIFTED } else { Mods::NONE });
            }
            let list: Vec<String> = e.predictions().into_iter().map(|p| p.text).collect();
            let at = list.iter().position(|t| *t == want);
            tally.total += 1;
            tally.top += (at == Some(0)) as usize;
            tally.listed += at.is_some() as usize;
            // Commit what was meant when it's listed (a click), else the top reading (Space).
            if list.is_empty() {
                e.flush();
            } else {
                e.commit_prediction(at.unwrap_or(0));
            }
            lists.push(list);
        }
        let tops: Vec<&str> = lists.iter().map(|l| l.first().map_or("", String::as_str)).collect();
        let interesting = tops.iter().any(|t| *t != want) || tops.windows(2).any(|w| w[0] != w[1]);
        differing += interesting as usize;
        if all || interesting {
            println!("{want}  [{}]", spell(&keys));
            for (label, list) in LABELS.iter().zip(&lists) {
                let mark = if list.first() == Some(&want) { "✓" } else if list.contains(&want) { "~" } else { "✗" };
                println!("  {label} {mark} {}", list.join(" · "));
            }
        }
    }
    *state.entry("keypresses".into()).or_default() += presses;
    *state.entry("words".into()).or_default() += tallies[0].total;
    *state.entry("differing".into()).or_default() += differing;
    for (label, t) in LABELS.iter().zip(&tallies) {
        *state.entry(format!("{label}.top")).or_default() += t.top;
        *state.entry(format!("{label}.listed")).or_default() += t.listed;
    }
    let mut lines: Vec<String> = state.iter().map(|(k, v)| format!("{k} {v}")).collect();
    lines.sort();
    std::fs::write(&state_path, lines.join("\n") + "\n").expect("write the state file");

    println!("\nThis run: {} words, {presses} key presses, {differing} where a label missed or the labels differ", tallies[0].total);
    for (label, t) in LABELS.iter().zip(&tallies) {
        println!("  {label}: top-1 {}/{}, listed {}/{}", t.top, t.total, t.listed, t.total);
    }
    let words = state["words"];
    println!("Session so far: {} words, {} key presses", words, state["keypresses"]);
    for label in LABELS {
        println!("  {label}: top-1 {}/{words}, listed {}/{words}", state[&format!("{label}.top")], state[&format!("{label}.listed")]);
    }
}
