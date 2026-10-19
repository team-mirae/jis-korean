//! The robot typing test: a robot types real text through the engine, key by key, and the screen
//! must match the text after every character. Run with `--nocapture` to see the report.
//!
//! Corpus: Samsung Electronics' latest "II. 사업의 내용" (fetch it with
//! `python3 -I tests/data/fetch_corpus.py` from `tests/data`). Without it, only the built-in
//! sample runs.

use std::sync::Arc;

use jis_korean::Dictionary;
use jis_korean::plan::Robot;

/// Covers every part of the layout: dedicated keys, the implied ㅇ and the double press, F8
/// symbols, Latin words in all cases, digits, hanja and a bare jamo.
const SAMPLE: &str = "\
말이 많은 사람은 아이와 오이를 샀다. 닭이 울면 갈아입고 앉아서 읽어 봐요.
2026년 상반기 DRAM과 NAND Flash 가격은 3~5% 올랐다(예상). 全 세계 HBM3E 수요—「AI」·클라우드!
문의: help@example.com, 값은 1,234원; 됐어? 빠르게 꽤 왜 웬 쐐기 뾰족한 iPhone과 Galaxy를 ㄱ자로 놓았다.
";

fn dictionary() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/dict/hanja.tsv")).expect("dict/hanja.tsv")
}

fn run(name: &str, text: &str) -> Robot {
    let mut robot = Robot::new(Some(Arc::new(Dictionary::parse(&dictionary()))));
    let expected = robot.type_text(text);
    eprintln!("── robot: {name}\n{}", robot.report);
    assert!(robot.report.failures.is_empty(), "{name}: {} failures", robot.report.failures.len());
    assert_eq!(robot.document(), expected, "{name}: document differs from the text");
    robot
}

#[test]
fn robot_types_the_sample() {
    let robot = run("sample", SAMPLE);
    let r = &robot.report;
    assert!(r.dropped.is_empty(), "sample should be fully typable: {:?}", r.dropped);
    // Each feature actually got used.
    assert!(r.dedicated_keys > 0 && r.implied_ieung > 0 && r.double_presses > 0);
    assert!(r.hanja == 1 && r.latin_words >= 7 && r.escapes > 0);
}

#[test]
fn robot_types_samsung_business() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/samsung-business.txt");
    let Ok(text) = std::fs::read_to_string(path) else {
        eprintln!("skipped: {path} missing; run tests/data/fetch_corpus.py");
        return;
    };
    let robot = run("Samsung Electronics, 사업의 내용", &text);
    assert!(robot.report.chars > 10_000);
}

#[test]
fn count_compares_jis_korean_with_conventional() {
    let counts = jis_korean::plan::count("말이 오이 같이 하자", None);
    assert_eq!(counts.chars, 11);
    assert_eq!(counts.dropped, 0);
    assert!(counts.jis < counts.conventional, "{counts:?}");
    // 말이: a k f ⇧l (5 with Shift) vs a k f d l (5); 오이: h ⇧l (3) vs d h d l (4) …
    assert_eq!(jis_korean::plan::conventional("까"), 3);
    assert_eq!(jis_korean::plan::count("까", None).jis, 2);
}

#[test]
fn symbols_off_the_legends_cost_three_on_both_layouts() {
    // ( ) are on stock JIS but not on the JIS Korean legends; · is the reverse.
    assert_eq!(jis_korean::plan::count("(", None).jis, 3);
    assert_eq!(jis_korean::plan::conventional("("), 2);
    assert_eq!(jis_korean::plan::count("·", None).jis, 1);
    assert_eq!(jis_korean::plan::conventional("·"), 3);
    // Digits after a ( still cost Shift+digit: the escape doesn't linger.
    assert_eq!(jis_korean::plan::count("(1", None).jis, 5);
}

#[test]
fn characters_no_key_types_cost_three_on_both_layouts() {
    let counts = jis_korean::plan::count("①", None);
    assert_eq!((counts.jis, counts.conventional, counts.off_keyboard, counts.dropped), (3, 3, 1, 0));
    // A hanja without a dictionary is entered from outside too.
    assert_eq!(jis_korean::plan::count("學", None).jis, 3);
    // Spaces and line breaks are one press each, both ways.
    assert_eq!(jis_korean::plan::count(" \n", None).jis, 2);
    assert_eq!(jis_korean::plan::conventional(" \n"), 2);
}
