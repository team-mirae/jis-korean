//! Side by side: the rules ranker, the travel ranker (spec §10.7) and the route-preference ranker
//! (§10.8) on the implied-ㅇ decisions and on held-out colloquial and business words, plus a grid
//! over the route ranker's parameters. With `/corpora` fetched and the model built with
//! `--corpora`, the held-out sets come from sentences the model never saw.
//! Usage: cargo run --release --example sxs [-- --no-grid]

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use jis_korean::predict::{Effort, Kind, Predictor, Preference, Ranker};
use jis_korean::{Engine, Key, Mods};

const I: [&str; 19] = ["r", "R", "s", "e", "E", "f", "a", "q", "Q", "t", "T", "d", "w", "W", "c", "z", "x", "v", "g"];
const M: [&str; 21] = ["k", "o", "i", "O", "j", "p", "u", "P", "h", "hk", "ho", "hl", "y", "n", "nj", "np", "nl", "b", "m", "ml", "l"];
const F: [&str; 28] = ["", "r", "R", "rt", "s", "sw", "sg", "e", "f", "fr", "fa", "fq", "ft", "fx", "fv", "fg", "a", "q", "qt", "t", "T", "d", "w", "c", "z", "x", "v", "g"];

/// The keys for a Hangul word, typed the JIS Korean way (implied ㅇ, the double press after a
/// final). `lazy`: the ㅇ after a final is left out too, as if it were implied there (말이 as 마리).
fn jis_keys(word: &str, lazy: bool) -> Option<String> {
    let combines = |prev_m: usize, next: char| matches!((prev_m, next), (8, 'k' | 'o' | 'l') | (13, 'j' | 'p' | 'l') | (18, 'l'));
    let mut keys = String::new();
    let mut prev: Option<(usize, usize)> = None;
    for ch in word.chars() {
        let n = (ch as u32).checked_sub(0xAC00).filter(|&n| n < 11172)? as usize;
        let (i, m, f) = (n / 588, n % 588 / 28, n % 28);
        let vowel = M[m];
        if i == 11 {
            match prev {
                Some((_, pf)) if pf > 0 && !lazy => keys.push_str(&vowel[..1]),
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
    Some(keys)
}

/// Whether the word has an ㅇ syllable right after a final (where lazy typing differs).
fn ieung_after_final(word: &str) -> bool {
    let parts: Vec<(usize, usize)> = word.chars().filter_map(|c| (c as u32).checked_sub(0xAC00).map(|n| n as usize)).map(|n| (n / 588, n % 28)).collect();
    parts.windows(2).any(|w| w[0].1 > 0 && w[1].0 == 11)
}

fn has_implied(word: &str) -> bool {
    jis_keys(word, false).is_some_and(|k| {
        // An ㅇ syllable typed without d: count ㅇ initials vs d keys.
        let ieung = word.chars().filter(|&c| ('가'..='힣').contains(&c) && (c as u32 - 0xAC00) / 588 == 11).count();
        ieung > k.matches('d').count()
    })
}

fn predict(p: &Arc<Predictor>, ranker: Ranker, keys: &str) -> Vec<(String, Kind)> {
    let mut e = Engine::new();
    e.set_predictor(Some(p.clone()));
    e.set_ranker(ranker);
    for c in keys.chars() {
        e.key(Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), if c.is_ascii_uppercase() { Mods::SHIFTED } else { Mods::NONE });
    }
    e.predictions().into_iter().map(|x| (x.text, x.kind)).collect()
}

struct Case {
    keys: String,
    want: String,
}

#[derive(Default, Clone, Copy)]
struct Score {
    top: usize,
    listed: usize,
    total: usize,
    /// Misses where an implied-ㅇ alternative (bare vowels, kept final) took the top.
    by_alternative: usize,
}

impl Score {
    fn pct(n: usize, d: usize) -> String {
        format!("{:6.2}%", n as f64 * 100.0 / d.max(1) as f64)
    }
    fn row(&self) -> String {
        format!(
            "top-1 {}  listed {}  lost to an ㅇ alternative {:4}  (n={})",
            Self::pct(self.top, self.total),
            Self::pct(self.listed, self.total),
            self.by_alternative,
            self.total
        )
    }
}

fn score(p: &Arc<Predictor>, ranker: Ranker, cases: &[Case]) -> Score {
    let mut s = Score { total: cases.len(), ..Score::default() };
    for c in cases {
        let ranked = predict(p, ranker, &c.keys);
        s.top += ranked.first().is_some_and(|r| r.0 == c.want) as usize;
        s.by_alternative += ranked
            .first()
            .is_some_and(|r| r.0 != c.want && matches!(r.1, Kind::BareVowels | Kind::KoreanKeepFinals | Kind::TwoSet)) as usize;
        s.listed += ranked.iter().any(|r| r.0 == c.want) as usize;
    }
    s
}

fn main() {
    let root = env!("CARGO_MANIFEST_DIR");
    let dir = format!("{root}/predict/");
    let corpora = format!("{root}/../corpora/");
    let read = |f: &str| std::fs::read_to_string(format!("{dir}{f}")).unwrap();
    let model = std::fs::read(format!("{dir}ranker.bin")).unwrap();
    let p = Arc::new(Predictor::parse(&model, &read("common-en.txt"), &read("common-ko.txt"), &read("english-nouns.txt")).unwrap());
    let grid = !std::env::args().any(|a| a == "--no-grid");

    let dict = std::fs::read_to_string(format!("{root}/dict/hanja.tsv")).unwrap();
    let readings: HashSet<String> = dict.lines().filter_map(|l| l.split('\t').next()).map(String::from).collect();
    let mut words: Vec<String> = dict.lines().filter_map(|l| l.split('\t').next()).filter(|r| r.chars().count() >= 2).step_by(40).map(String::from).collect();
    if let Ok(corpus) = std::fs::read_to_string(format!("{root}/tests/data/samsung-business.txt")) {
        words.extend(corpus.split(|c: char| !('가'..='힣').contains(&c)).filter(|w| w.chars().count() >= 2).map(String::from));
    }
    words.sort();
    words.dedup();
    // Held-out corpus words (word<TAB>count), most common first.
    let held = |name: &str| -> Vec<(String, usize)> {
        std::fs::read_to_string(format!("{corpora}{name}"))
            .map(|t| t.lines().filter_map(|l| l.split_once('\t')).map(|(w, n)| (w.to_string(), n.parse().unwrap_or(0))).collect())
            .unwrap_or_default()
    };
    let held_ko = held("heldout-ko.txt");
    let held_en = held("heldout-en.txt");
    // Generated business terms aren't marked in the lists; read them back from the corpora.
    let business_ko: Vec<String> = std::fs::read_to_string(format!("{corpora}std-words.csv"))
        .map(|t| t.lines().skip(1).filter_map(|l| l.split(',').next()).filter(|w| w.chars().count() >= 2 && w.chars().all(|c| ('가'..='힣').contains(&c))).map(String::from).collect())
        .unwrap_or_default();

    let syllables = |w: &str| w.chars().all(|c| ('가'..='힣').contains(&c));
    // A: words with an implied ㅇ, typed properly: the Korean reading should stay on top.
    let implied: Vec<Case> = words.iter().filter(|w| has_implied(w)).filter_map(|w| Some(Case { keys: jis_keys(w, false)?, want: w.clone() })).collect();
    // B: words with an ㅇ after a final, typed lazily (no double press).
    let lazy_all: Vec<(Case, bool)> = words
        .iter()
        .filter(|w| ieung_after_final(w))
        .filter_map(|w| {
            let keys = jis_keys(w, true)?;
            let mut e = Engine::new();
            for c in keys.chars() {
                e.key(Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), Mods::NONE);
            }
            let ambiguous = readings.contains(&e.preedit()) || words.binary_search(&e.preedit()).is_ok();
            Some((Case { keys, want: w.clone() }, ambiguous))
        })
        .collect();
    let (lazy, lazy_ambiguous): (Vec<_>, Vec<_>) = lazy_all.into_iter().partition(|(_, a)| !a);
    let lazy: Vec<Case> = lazy.into_iter().map(|(c, _)| c).collect();
    let lazy_ambiguous: Vec<Case> = lazy_ambiguous.into_iter().map(|(c, _)| c).collect();
    // C: bare-vowel chat words from held-out sentences (ㅠㅠ, ㅜㅜ, ㅡㅡ): the bare reading on top.
    let bare: Vec<Case> = held_ko
        .iter()
        .filter(|(w, _)| w.chars().count() >= 2 && w.chars().all(|c| ('ㅏ'..='ㅣ').contains(&c)))
        .map(|(w, _)| Case { keys: w.chars().map(|c| M[c as usize - 'ㅏ' as usize]).collect(), want: w.clone() })
        .collect();
    // D: held-out colloquial eojeols, typed properly, most common first.
    let colloquial: Vec<Case> = held_ko
        .iter()
        .filter(|(w, _)| w.chars().count() >= 2 && syllables(w))
        .filter_map(|(w, _)| Some(Case { keys: jis_keys(w, false)?, want: w.clone() }))
        .take(3000)
        .collect();
    // D': the held-out eojeols with an implied ㅇ in them.
    let colloquial_implied: Vec<Case> = colloquial.iter().filter(|c| has_implied(&c.want)).map(|c| Case { keys: c.keys.clone(), want: c.want.clone() }).collect();
    // E: held-out English words (3+ letters), typed as is.
    let english: Vec<Case> = held_en
        .iter()
        .filter(|(w, _)| w.len() >= 3 && w.chars().all(|c| c.is_ascii_lowercase()))
        .map(|(w, _)| Case { keys: w.clone(), want: w.clone() })
        .take(2000)
        .collect();
    // F: Korean business terms (standard words), typed properly.
    let business: Vec<Case> = business_ko.iter().step_by(4).filter_map(|w| Some(Case { keys: jis_keys(w, false)?, want: w.clone() })).collect();

    let sets: Vec<(&str, &[Case])> = vec![
        ("A  implied ㅇ words, typed properly → word", &implied),
        ("B  ㅇ after a final, typed lazily → word", &lazy),
        ("B' same, lazy keys also spell a word", &lazy_ambiguous),
        ("C  held-out bare-vowel chat words → bare", &bare),
        ("D  held-out chat eojeols → word", &colloquial),
        ("D' …of which with an implied ㅇ", &colloquial_implied),
        ("E  held-out English words → English", &english),
        ("F  Korean business terms → word", &business),
    ];
    let report = |name: &str, ranker: Ranker| {
        println!("\n{name}");
        let start = Instant::now();
        let mut n = 0;
        for (label, cases) in &sets {
            println!("  {label:44} {}", score(&p, ranker, cases).row());
            n += cases.len();
        }
        println!("  {:.1} µs per prediction", start.elapsed().as_secs_f64() * 1e6 / n.max(1) as f64);
    };
    let mut travel = Ranker::Travel(Effort::default());
    let mut route = Ranker::Route(Preference::default());
    report("Rules (retrained)", Ranker::Rules);
    report("Travel v1: skipped effort (retrained)", travel);
    report("Route v2: route preference, default parameters", route);

    // Errors where the implied-ㅇ decision matters, with clear ground truth: A, C and D'.
    let objective = |ranker: Ranker| {
        let (a, c, d) = (score(&p, ranker, &implied), score(&p, ranker, &bare), score(&p, ranker, &colloquial_implied));
        let b = score(&p, ranker, &lazy);
        ((a.total - a.top) + (c.total - c.top) + (d.total - d.top), b.top, b.total)
    };
    if grid {
        println!("\nGrid over the travel ranker (press, travel): errors on A + C + D', then B top-1");
        let mut best: Option<(usize, usize, Effort)> = None;
        for press in [-1.0, 0.0, 0.5, 1.0, 2.0, 3.0] {
            for travel in [-0.25, -0.1, 0.0, 0.1, 0.25, 0.5] {
                let costs = Effort { press, travel };
                let (errors, b_top, b_total) = objective(Ranker::Travel(costs));
                println!("  press {press:4.1} travel {travel:5.2}: errors {errors:4}  B top-1 {}", Score::pct(b_top, b_total));
                if best.is_none_or(|(e, bt, _)| (errors, std::cmp::Reverse(b_top)) < (e, std::cmp::Reverse(bt))) {
                    best = Some((errors, b_top, costs));
                }
            }
        }
        if let Some((_, _, costs)) = best {
            travel = Ranker::Travel(costs);
            report(&format!("Travel v1, best of grid: {costs:?}"), travel);
        }

        println!("\nGrid over the route ranker (press, travel, bias): errors on A + C + D', then B top-1");
        let mut best: Option<(usize, usize, Preference)> = None;
        for press in [0.0, 0.5, 1.0, 2.0, 4.0] {
            for travel in [-0.5, 0.0, 0.25, 0.5, 1.0] {
                for bias in [-2.0, -1.0, 0.0, 1.0, 2.0] {
                  for omission in [1.0, 2.0, 3.0, 5.0] {
                    let preference = Preference { press, travel, bias, omission };
                    let (errors, b_top, b_total) = objective(Ranker::Route(preference));
                    println!("  press {press:4.2} travel {travel:4.2} bias {bias:4.1} omission {omission:3.1}: errors {errors:4}  B top-1 {}", Score::pct(b_top, b_total));
                    if best.is_none_or(|(e, bt, _)| (errors, std::cmp::Reverse(b_top)) < (e, std::cmp::Reverse(bt))) {
                        best = Some((errors, b_top, preference));
                    }
                  }
                }
            }
        }
        if let Some((_, _, preference)) = best {
            route = Ranker::Route(preference);
            report(&format!("Route v2, best of grid: {preference:?}"), route);
        }
    }

    println!("\nWhere the three disagree on top-1 (first 40):");
    let mut shown = 0;
    for (label, cases) in &sets {
        for c in cases.iter() {
            let tops: Vec<String> = [Ranker::Rules, travel, route]
                .iter()
                .map(|&r| predict(&p, r, &c.keys).first().map(|x| x.0.clone()).unwrap_or_default())
                .collect();
            if (tops[0] != tops[1] || tops[1] != tops[2]) && shown < 40 {
                shown += 1;
                println!("  {:2} {:14} want {:10} rules {:10} travel {:10} route {:10}", &label[..2], c.keys, c.want, tops[0], tops[1], tops[2]);
            }
        }
    }
}
