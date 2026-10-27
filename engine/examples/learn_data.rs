//! Training data for the learned ranker (spec §10.13): types text through the engine the trusting
//! way (`trust.rs`, the planner's styles), with the predictor ranking by the rules, and records for
//! each word the distinct readings with their features and which one is the word. Words where no
//! reading is the word, or only one reading is offered, are skipped, and so are words with hanja
//! (picked from the list, never on top) and every word of the evaluation texts (`--exclude`).
//!
//! Usage: cargo run --release --example learn_data -- --plan <plan.tsv> --out <dir>
//!        [--exclude <text file>]… [--threads N] [--bits B]
//!
//! The plan (written by `tools/train_learned.py`): lines of `style<TAB>weight<TAB>source<TAB>text`,
//! style `o` (cheapest), `j` (every jamo) or `m<seed>` (a mix of the valid ways). Consecutive lines
//! with the same style and weight are typed as one text, one line after another.
//!
//! Output (little-endian arrays; one row per recorded word, identical rows merged with their
//! weights summed): `row_ptr.u32` (each row's first candidate, then the total), `row_weight.f32`,
//! `row_valid.u8` (1: validation, by a hash of the word), `row_group.u8` (the word's script:
//! k Korean, e Latin, m mixed, o other), `row_style.u8` (0 cheapest, 1 every jamo, 2 mixed, 3 lazy), `cand_ptr.u64` (each candidate's first
//! feature, then the total), `cand_label.u8`, `cand_rank.u8` (the rules' rank), `cand_legacy.f32`,
//! `cand_origin.u8`, `feat_idx.u32`, `feat_val.f32`, `meta.json`.

#[cfg(feature = "legacy-predict")]
fn main() {
    eprintln!("learn_data needs the learned ranker: build without the legacy-predict feature");
    std::process::exit(2);
}

#[cfg(not(feature = "legacy-predict"))]
fn main() {
    imp::main();
}

#[cfg(not(feature = "legacy-predict"))]
mod imp {
    use std::collections::{HashMap, HashSet};
    use std::io::Write;
    use std::sync::Arc;

    use jis_korean::{Dictionary, Key, Mods};
    use jis_korean::learned::{self, DEFAULT_BITS};
    use jis_korean::plan::{Style, normalize};
    use jis_korean::predict::{Predictor, Ranker};
    use jis_korean::trust::type_text_observed;

    /// Lines typed as one text: they share an engine and a robot.
    const BLOCK: usize = 400;

    struct Line {
        style: Style,
        style_code: u8,
        weight: f32,
        text: String,
    }

    /// A reading: (label, the rules' rank, the rules' score, origin, features).
    type Cand = (u8, u8, f32, u8, Vec<(u32, f32)>);

    #[derive(Default)]
    struct Row {
        weight: f32,
        valid: bool,
        group: u8,
        style: u8,
        /// (label, rules' rank, rules' score, origin, features)
        cands: Vec<Cand>,
    }

    pub fn main() {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
        let plan = value("--plan").expect("--plan <plan.tsv>");
        let out = value("--out").expect("--out <dir>");
        let bits: u32 = value("--bits").map_or(DEFAULT_BITS, |b| b.parse().expect("--bits"));
        let threads: usize = value("--threads").map_or_else(
            || std::thread::available_parallelism().map_or(4, |n| n.get()),
            |t| t.parse().expect("--threads"),
        );
        let excluded = excluded_words(&args);
        eprintln!("excluding {} evaluation words", excluded.len());

        let root = env!("CARGO_MANIFEST_DIR");
        let read = |f: &str| std::fs::read_to_string(format!("{root}/predict/{f}")).unwrap();
        let model = std::fs::read(format!("{root}/predict/ranker.bin")).unwrap();
        let predictor =
            Arc::new(Predictor::parse(&model, &read("common-en.txt"), &read("common-ko.txt"), &read("english-nouns.txt")).unwrap());
        let dictionary = Arc::new(Dictionary::parse(&std::fs::read_to_string(format!("{root}/dict/hanja.tsv")).unwrap()));

        let lines: Vec<Line> = std::fs::read_to_string(&plan)
            .expect("read the plan")
            .lines()
            .filter_map(|l| {
                let mut f = l.splitn(4, '\t');
                let (style, weight, _source, text) = (f.next()?, f.next()?, f.next()?, f.next()?);
                let (style, code) = match style {
                    "o" => (Style::Optimal, 0),
                    "j" => (Style::EveryJamo, 1),
                    s => (Style::Mixed(s.strip_prefix('m')?.parse().ok()?), 2),
                };
                Some(Line { style, style_code: code, weight: weight.parse().ok()?, text: text.to_string() })
            })
            .collect();
        // Blocks of consecutive lines of one style and weight.
        let mut blocks: Vec<(usize, usize)> = Vec::new();
        let mut start = 0;
        for i in 1..=lines.len() {
            let split = i == lines.len()
                || i - start >= BLOCK
                || lines[i].style != lines[start].style
                || lines[i].weight != lines[start].weight;
            if split {
                blocks.push((start, i));
                start = i;
            }
        }
        eprintln!("{} lines in {} blocks, {threads} threads", lines.len(), blocks.len());

        // Contiguous runs of blocks per thread, merged in order: the same output every run.
        let per = blocks.len().div_ceil(threads.max(1)).max(1);
        let done = std::sync::atomic::AtomicUsize::new(0);
        let parts: Vec<(Vec<Row>, usize, usize)> = std::thread::scope(|scope| {
            let handles: Vec<_> = blocks
                .chunks(per)
                .map(|chunk| {
                    let (predictor, dictionary, lines, excluded, done) = (&predictor, &dictionary, &lines, &excluded, &done);
                    scope.spawn(move || {
                        let mut rows: Vec<Row> = Vec::new();
                        let mut index: HashMap<u128, usize> = HashMap::new();
                        let (mut words, mut skipped) = (0usize, 0usize);
                        for &(a, b) in chunk {
                            let (style, code, weight) = (lines[a].style, lines[a].style_code, lines[a].weight);
                            let text: String = lines[a..b].iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n");
                            let mut observe = |engine: &jis_korean::Engine, word: &str, staged: &str| {
                                words += 1;
                                if excluded.contains(word) || word.chars().any(is_hanja) || staged.is_empty() {
                                    skipped += 1;
                                    return;
                                }
                                let list = engine.prediction_features(bits);
                                if list.len() < 2 || !list.iter().any(|f| f.text == staged) {
                                    skipped += 1;
                                    return;
                                }
                                let cands: Vec<Cand> = list
                                    .into_iter()
                                    .map(|f| ((f.text == staged) as u8, f.legacy_rank.min(255) as u8, f.legacy as f32, f.origin as u8, f.features))
                                    .collect();
                                let key = fingerprint(&cands);
                                match index.get(&key) {
                                    Some(&i) => rows[i].weight += weight,
                                    None => {
                                        index.insert(key, rows.len());
                                        rows.push(Row { weight, valid: is_validation(staged), group: group(staged), style: code, cands });
                                    }
                                }
                                // The lazy typist (spec §10.3 rule 4): some of the words with a final
                                // before a syllable on ㅇ are typed leaving the ㅇ out, trusting the
                                // predictions to keep the final (당신이 typed as 당시니).
                                if let Some(keys) = lazy_keys(staged).filter(|_| hash(staged) % LAZY == 1) {
                                    let mut lazy = engine.clone();
                                    lazy.flush();
                                    for (key, shift) in keys {
                                        lazy.key(key, if shift { Mods::SHIFTED } else { Mods::NONE });
                                    }
                                    let list = lazy.prediction_features(bits);
                                    if list.len() >= 2 && list.iter().any(|f| f.text == staged) {
                                        let cands: Vec<_> = list
                                            .into_iter()
                                            .map(|f| ((f.text == staged) as u8, f.legacy_rank.min(255) as u8, f.legacy as f32, f.origin as u8, f.features))
                                            .collect();
                                        let key = fingerprint(&cands);
                                        match index.get(&key) {
                                            Some(&i) => rows[i].weight += weight,
                                            None => {
                                                index.insert(key, rows.len());
                                                rows.push(Row { weight, valid: is_validation(staged), group: group(staged), style: 3, cands });
                                            }
                                        }
                                    }
                                }
                            };
                            type_text_observed(
                                &text,
                                predictor.clone(),
                                Some(dictionary.clone()),
                                Ranker::Rules,
                                style,
                                true,
                                None,
                                &mut observe,
                            );
                            let n = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                            if n % 200 == 0 {
                                eprint!("\r{n} blocks");
                            }
                        }
                        (rows, words, skipped)
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        eprintln!();

        // Merge, in order.
        let mut rows: Vec<Row> = Vec::new();
        let mut index: HashMap<u128, usize> = HashMap::new();
        let (mut words, mut skipped) = (0, 0);
        for (part, w, s) in parts {
            words += w;
            skipped += s;
            for row in part {
                let key = fingerprint(&row.cands);
                match index.get(&key) {
                    Some(&i) => rows[i].weight += row.weight,
                    None => {
                        index.insert(key, rows.len());
                        rows.push(row);
                    }
                }
            }
        }
        write(&out, &rows, bits, words, skipped);
    }

    /// One word in this many with a final before a syllable on ㅇ is also typed the lazy way.
    const LAZY: u64 = 3;

    fn hash(word: &str) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in word.bytes() {
            h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
        h >> 20
    }

    /// The keys of a word of syllables typed every jamo, but leaving out the ㅇ of a syllable after
    /// a final (and at the start: the implied ㅇ): None unless some final is followed by one.
    fn lazy_keys(word: &str) -> Option<Vec<(Key, bool)>> {
        const I: [&str; 19] = ["r", "R", "s", "e", "E", "f", "a", "q", "Q", "t", "T", "d", "w", "W", "c", "z", "x", "v", "g"];
        const M: [&str; 21] = ["k", "o", "i", "O", "j", "p", "u", "P", "h", "hk", "ho", "hl", "y", "n", "nj", "np", "nl", "b", "m", "ml", "l"];
        const F: [&str; 28] = [
            "", "r", "R", "rt", "s", "sw", "sg", "e", "f", "fr", "fa", "fq", "ft", "fx", "fv", "fg", "a", "q", "qt", "t", "T", "d", "w", "c", "z", "x",
            "v", "g",
        ];
        let mut keys = String::new();
        let (mut prev_final, mut lazy) = (false, false);
        for ch in word.chars() {
            let n = (ch as u32).checked_sub(0xAC00).filter(|&n| n < 11172)? as usize;
            let (i, m, f) = (n / 588, n % 588 / 28, n % 28);
            if i == 11 && (prev_final || keys.is_empty()) {
                lazy |= prev_final;
            } else {
                keys.push_str(I[i]);
            }
            keys.push_str(M[m]);
            keys.push_str(F[f]);
            prev_final = f > 0;
        }
        lazy.then(|| keys.chars().map(|c| (Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), c.is_ascii_uppercase())).collect())
    }

    /// Every word of the evaluation texts, split as the trusting typist splits them.
    fn excluded_words(args: &[String]) -> HashSet<String> {
        let mut words = HashSet::new();
        for (i, a) in args.iter().enumerate() {
            if a == "--exclude"
                && let Some(path) = args.get(i + 1)
            {
                let text = std::fs::read_to_string(path).unwrap_or_else(|_| panic!("read {path}"));
                let (text, _) = normalize(&text);
                words.extend(text.split(['\n', ' ']).filter(|w| !w.is_empty()).map(str::to_string));
            }
        }
        words
    }

    fn is_hanja(c: char) -> bool {
        ('\u{4E00}'..='\u{9FFF}').contains(&c) || ('\u{3400}'..='\u{4DBF}').contains(&c) || ('\u{F900}'..='\u{FAFF}').contains(&c)
    }

    /// FNV-1a over the word: 1 in 20 words is for validation, whatever their context.
    fn is_validation(word: &str) -> bool {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in word.bytes() {
            h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
        (h >> 32).is_multiple_of(20)
    }

    fn group(word: &str) -> u8 {
        let latin = word.chars().any(|c| c.is_ascii_alphabetic());
        let hangul = word.chars().any(|c| ('\u{AC00}'..='\u{D7A3}').contains(&c) || ('\u{3131}'..='\u{318E}').contains(&c));
        match (latin, hangul) {
            (false, true) => b'k',
            (true, false) => b'e',
            (true, true) => b'm',
            _ => b'o',
        }
    }

    /// Two independent 64-bit hashes of a row's candidates: identical rows merge.
    fn fingerprint(cands: &[Cand]) -> u128 {
        let (mut a, mut b): (u64, u64) = (0xcbf2_9ce4_8422_2325, 0x9e37_79b9_7f4a_7c15);
        let mut eat = |x: u64| {
            for byte in x.to_le_bytes() {
                a = (a ^ byte as u64).wrapping_mul(0x0000_0100_0000_01b3);
                b = (b ^ byte as u64).wrapping_mul(0xff51_afd7_ed55_8ccd).rotate_left(29);
            }
        };
        for (label, rank, legacy, origin, features) in cands {
            eat(((*label as u64) << 48) | ((*rank as u64) << 40) | ((*origin as u64) << 32) | legacy.to_bits() as u64);
            eat(features.len() as u64);
            for (i, v) in features {
                eat(((*i as u64) << 32) | v.to_bits() as u64);
            }
        }
        ((a as u128) << 64) | b as u128
    }

    fn write(dir: &str, rows: &[Row], bits: u32, words: usize, skipped: usize) {
        std::fs::create_dir_all(dir).unwrap();
        let file = |name: &str| std::io::BufWriter::new(std::fs::File::create(format!("{dir}/{name}")).unwrap());
        let (mut row_ptr, mut row_weight, mut row_valid, mut row_group, mut row_style) =
            (file("row_ptr.u32"), file("row_weight.f32"), file("row_valid.u8"), file("row_group.u8"), file("row_style.u8"));
        let (mut cand_ptr, mut cand_label, mut cand_rank, mut cand_legacy, mut cand_origin) =
            (file("cand_ptr.u64"), file("cand_label.u8"), file("cand_rank.u8"), file("cand_legacy.f32"), file("cand_origin.u8"));
        let (mut feat_idx, mut feat_val) = (file("feat_idx.u32"), file("feat_val.f32"));
        let (mut cands, mut feats) = (0u32, 0u64);
        for row in rows {
            row_ptr.write_all(&cands.to_le_bytes()).unwrap();
            row_weight.write_all(&row.weight.to_le_bytes()).unwrap();
            row_valid.write_all(&[row.valid as u8]).unwrap();
            row_group.write_all(&[row.group]).unwrap();
            row_style.write_all(&[row.style]).unwrap();
            for (label, rank, legacy, origin, features) in &row.cands {
                cand_ptr.write_all(&feats.to_le_bytes()).unwrap();
                cand_label.write_all(&[*label]).unwrap();
                cand_rank.write_all(&[*rank]).unwrap();
                cand_legacy.write_all(&legacy.to_le_bytes()).unwrap();
                cand_origin.write_all(&[*origin]).unwrap();
                for (i, v) in features {
                    feat_idx.write_all(&i.to_le_bytes()).unwrap();
                    feat_val.write_all(&v.to_le_bytes()).unwrap();
                }
                feats += features.len() as u64;
                cands += 1;
            }
        }
        row_ptr.write_all(&cands.to_le_bytes()).unwrap();
        cand_ptr.write_all(&feats.to_le_bytes()).unwrap();
        // Features the trainer leaves unregularized: none (the model is a correction to the rules'
        // score, which it starts from: `learned::base`).
        let free: Vec<String> = Vec::new();
        let _ = learned::index_of;
        let meta = format!(
            "{{\"format\": {}, \"features\": {}, \"bits\": {bits}, \"rows\": {}, \"candidates\": {cands}, \"nonzeros\": {feats}, \"words_seen\": {words}, \"words_skipped\": {skipped}, \"legacy_clip\": {}, \"unregularized\": [{}]}}\n",
            learned::FORMAT,
            learned::FEATURES,
            rows.len(),
            learned::LEGACY_CLIP,
            free.join(", ")
        );
        std::fs::write(format!("{dir}/meta.json"), meta).unwrap();
        eprintln!("{words} words seen, {skipped} skipped; {} rows, {cands} candidates, {feats} features → {dir}", rows.len());
    }
}
