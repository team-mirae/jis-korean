//! Prints typing runs that switch language twice (English → Korean → English, or Korean →
//! English → Korean), taken from a text and repeated until there are `count` of them, as JSON lines
//! for a driver that types them for real (the lost-switch test in hhkb-ekj-trilingual,
//! docs/04-hammerspoon.md). Each chunk is one word with the keys that type it.
//! Usage: cargo run --release --example switch_runs -- <text file> <count>

use jis_korean::plan::keys_for_word;

#[derive(Clone, Copy, PartialEq)]
enum Lang {
    En,
    Ko,
}

fn lang(word: &str) -> Option<Lang> {
    if word.chars().all(|c| c.is_ascii_alphabetic()) {
        Some(Lang::En)
    } else if word.chars().all(|c| ('가'..='힣').contains(&c)) {
        Some(Lang::Ko)
    } else {
        None
    }
}

fn json(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: switch_runs <text file> <count>");
    let count: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(1000);
    let text = std::fs::read_to_string(&path).expect("read the text file");

    // Words stripped of surrounding punctuation; anything mixed (모바일AP) breaks a run.
    let words: Vec<(String, Option<Lang>)> = text
        .split_whitespace()
        .map(|w| {
            let w = w.trim_matches(|c: char| !c.is_alphanumeric());
            (w.to_string(), if w.is_empty() { None } else { lang(w) })
        })
        .collect();

    // [en-ko-en, ko-en-ko], interleaved below so both kinds get half the runs.
    let mut runs: [Vec<String>; 2] = [Vec::new(), Vec::new()];
    for window in words.windows(3) {
        let [(a, Some(la)), (b, Some(lb)), (c, Some(lc))] = window else { continue };
        if la == lb || la != lc {
            continue;
        }
        let chunks: Option<Vec<String>> = [(a, *la), (b, *lb), (c, *lc)]
            .iter()
            .map(|(word, l)| {
                let keys = keys_for_word(word)?;
                let keys: Vec<String> = keys.iter().map(|p| p.to_string()).collect();
                let l = if *l == Lang::En { "en" } else { "ko" };
                Some(format!(r#"{{"lang":"{l}","text":{},"keys":{}}}"#, json(word), json(&keys.join(" "))))
            })
            .collect();
        if let Some(chunks) = chunks {
            let (kind, list) = if *la == Lang::En { ("en-ko-en", 0) } else { ("ko-en-ko", 1) };
            runs[list].push(format!(r#"{{"kind":"{kind}","chunks":[{}]}}"#, chunks.join(",")));
        }
    }
    assert!(runs.iter().all(|r| !r.is_empty()), "the text needs both kinds of switching run");
    eprintln!("{} en-ko-en and {} ko-en-ko distinct runs", runs[0].len(), runs[1].len());
    for i in 0..count {
        let list = &runs[i % 2];
        println!("{}", list[(i / 2) % list.len()]);
    }
}
