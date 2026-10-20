//! Measures the engine for the docs (web playground's API docs, "Performance"): load times, time per
//! key press with and without predictions, and the resolve functions (spec §8.3).
//! Usage: cargo run --release --example bench [lite|full|resolve]   (run each under
//! `/usr/bin/time -l` for peak memory). Text: web/public/long-sample-text.txt, word by word.

use std::sync::Arc;
use std::time::Instant;

use jis_korean::plan::keys_for_word;
use jis_korean::{Dictionary, Engine, Mods};

fn main() {
    let what = std::env::args().nth(1).unwrap_or_else(|| "lite".into());
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/");
    let text = std::fs::read_to_string(format!("{root}../web/public/long-sample-text.txt")).unwrap();

    let t = Instant::now();
    let tsv = std::fs::read_to_string(format!("{root}dict/hanja.tsv")).unwrap();
    let dictionary = Arc::new(Dictionary::parse(&tsv));
    println!("dictionary: {} readings, parsed in {:.1} ms", dictionary.len(), t.elapsed().as_secs_f64() * 1e3);

    let mut engine = Engine::new();
    engine.set_dictionary(dictionary);
    #[cfg(feature = "predict")]
    if what != "lite" {
        let read = |f: &str| std::fs::read_to_string(format!("{root}predict/{f}")).unwrap();
        let model = std::fs::read(format!("{root}predict/ranker.bin")).unwrap();
        let t = Instant::now();
        let predictor = jis_korean::predict::Predictor::parse(&model, &read("common-en.txt"), &read("common-ko.txt"), &read("english-nouns.txt")).unwrap();
        println!("predictor: loaded in {:.1} ms", t.elapsed().as_secs_f64() * 1e3);
        engine.set_predictor(Some(Arc::new(predictor)));
    }

    if what == "resolve" {
        use jis_korean::resolve::{key_input_after, keys_for_output};
        for (label, run) in [
            ("keys_for_output(한, 3, 3)", Box::new(|e: &Engine| keys_for_output(e, "한", 3, 3).len()) as Box<dyn Fn(&Engine) -> usize>),
            ("keys_for_output(한글, 1, 8)", Box::new(|e: &Engine| keys_for_output(e, "한글", 1, 8).len())),
            ("keys_for_output(hello, 5, 5)", Box::new(|e: &Engine| keys_for_output(e, "hello", 5, 5).len())),
            ("keys_for_output(學校, 6, 6)", Box::new(|e: &Engine| keys_for_output(e, "學校", 6, 6).len())),
            ("key_input_after(대한민국의, KeyS)", Box::new(|e: &Engine| {
                println!("  → {:?}", key_input_after(e, "대한민국의", "KeyS"));
                1
            })),
        ] {
            let t = Instant::now();
            let n = run(&engine);
            println!("{label}: {n} in {:.2} ms", t.elapsed().as_secs_f64() * 1e3);
        }
        return;
    }

    // Time per key press: every word's keys, then Space, timing each press (and, with predictions,
    // ranking the readings as a shell does after each press).
    let mut times = Vec::new();
    for word in text.split_whitespace() {
        let Some(keys) = keys_for_word(word) else { continue };
        for p in keys.iter().copied().chain([jis_korean::plan::Press::new(jis_korean::Key::Space)]) {
            let t = Instant::now();
            engine.key(p.key, if p.shift { Mods::SHIFTED } else { Mods::NONE });
            #[cfg(feature = "predict")]
            std::hint::black_box(engine.predictions());
            times.push(t.elapsed().as_secs_f64() * 1e6);
        }
    }
    times.sort_by(f64::total_cmp);
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    let at = |q: f64| times[((times.len() - 1) as f64 * q) as usize];
    println!("{what}: {} key presses, mean {mean:.1} µs, median {:.1} µs, p99 {:.1} µs, max {:.1} µs", times.len(), at(0.5), at(0.99), at(1.0));
}
