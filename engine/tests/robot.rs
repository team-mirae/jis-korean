//! The robot typing test: a robot types real text through the engine, key by key, and the screen
//! must match the text after every character. Run with `--nocapture` to see the report.
//!
//! Corpus: Samsung Electronics' latest "II. 사업의 내용" (fetch it with
//! `python3 -I tests/data/fetch_corpus.py` from `tests/data`). Without it, only the built-in
//! sample runs.

#[path = "support/robot.rs"]
mod robot;

use robot::Robot;

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
    let mut robot = Robot::new(&dictionary());
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
