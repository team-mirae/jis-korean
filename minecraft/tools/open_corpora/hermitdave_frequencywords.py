"""Extracts hermitdave/FrequencyWords (content/2018) from corpora/open/hermitdave-frequencywords
into corpora/derived/hermitdave-frequencywords. Source: https://github.com/hermitdave/FrequencyWords,
word counts over OpenSubtitles 2018 (subtitle/colloquial register). License: content CC BY-SA 4.0,
code MIT (README). Tier moderate, weight 0.8.

Input lines are `word count` (count = last space-separated field). Only ko_full.txt and
en_full.txt are read; the *_50k files are subsets of them.

- ko_words.tsv: pure-Hangul tokens ([가-힣]+, so no bare jamo) with count >= 3, digit+Hangul
  tokens (1년, 30분: counters and units) with count >= 3, and the accepted mixed tokens below.
  Counts are the source's raw frequencies; damp them (e.g. log) when training, since subtitle
  chat would otherwise swamp the model.
- mixed.txt: Latin+Hangul tokens (^[a-z]{2,5}[가-힣]{1,3}$, count >= 3) whose Hangul part is a
  known particle/ending and whose Latin stem is an acronym (known list, or no vowels: fbi, nzt),
  upper-cased (FBI가). Other stems are dropped: in subtitles they are almost all character names
  (brian이, finch가, mary가) or tokenizer junk (quot를), and a dictionary check doesn't remove names.
- particles.txt: the distinct endings seen after accepted Latin stems.
- en_words.tsv: ^[a-z]{2,}$ tokens (plus "a" and "i") with count >= 20, minus contraction
  fragments ('re, 'll, didn, isn, ...).
- meta.json: source, tier, weight, uses, license, line counts.

Usage: python3 -I hermitdave_frequencywords.py
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "hermitdave-frequencywords")
OUT = os.path.join(ROOT, "corpora", "derived", "hermitdave-frequencywords")

NAME = "hermitdave-frequencywords"
TIER = "moderate"
WEIGHT = 0.8
USES = ["korean-key-model", "english-key-model", "korean-lexicon", "english-lexicon",
        "particles-endings", "mixed-ko-en", "acronyms"]
LICENSE = ("CC BY-SA 4.0 (content), MIT (code); hermitdave/FrequencyWords word counts "
           "over OpenSubtitles 2018")

KO_MIN = 3
EN_MIN = 20
CAP = 3_000_000

HANGUL = re.compile(r"^[가-힣]+$")
DIGIT_HANGUL = re.compile(r"^[0-9]{1,4}[가-힣]{1,4}$")
MIXED = re.compile(r"^([a-z]{2,5})([가-힣]{1,3})$")
ENGLISH = re.compile(r"^[a-z]{2,}$")
VOWELS = set("aeiouy")

# Acronyms with vowels (vowel-less stems count as acronyms anyway).
ACRONYMS = {
    "tv", "dna", "la", "cd", "usb", "suv", "ceo", "ipo", "id", "cia", "fbi", "nasa", "ufo",
    "ai", "it", "pc", "ok", "dvd", "atm", "sos", "iq", "ip", "ui", "api", "cpu", "gpu", "ram",
    "rom", "os", "ios", "sat", "ged", "mba", "erp", "kb", "sk", "lg", "ev", "eu",
    "un", "uae", "usa", "uk", "nypd", "lapd", "fda", "irs", "ira", "mit", "ucla", "nba",
    "nfl", "mvp", "vip", "dea", "atf", "doe", "sec", "cfo", "cto", "coo", "hiv", "icu", "ekg",
    "eeg", "er", "or", "pr", "hr", "ir", "qa", "aa", "ad", "ed", "ep", "ost", "bgm", "mc",
    "pd", "md", "ocd", "adhd", "ptsd", "abc", "bbc", "cnn", "cbs", "nbc", "espn", "hbo",
    "ibm", "amd", "ssd", "led", "lcd", "ufc", "wwe", "aka", "eta", "diy", "faq",
}
PARTICLES = {
    "가", "이", "는", "은", "를", "을", "에", "에서", "의", "로", "으로", "와", "과", "도",
    "만", "랑", "이랑", "나", "이나", "에게", "한테", "께", "부터", "까지", "보다", "처럼",
    "같은", "라고", "이라고", "다", "이다", "야", "이야", "요", "에요", "이에요", "예요",
    "입니다", "죠", "지", "인", "라는", "이라는", "로서", "로써", "였", "였어", "였다",
    "하고", "든", "든지", "라도", "이라도", "마저", "조차", "밖에", "쯤", "뿐", "들",
    "들이", "들은", "들을", "들의", "에겐", "한텐", "에선", "엔", "로는", "와의", "과의",
}
# Halves of English contractions, not words on their own.
FRAGMENTS = {"re", "ll", "ve", "nt", "didn", "doesn", "isn", "wouldn", "couldn", "shouldn",
             "wasn", "weren", "hasn", "hadn", "aren", "mustn", "needn", "ain", "mightn"}


def read_counts(path):
    """Yields (word, count) from a `word count` file, skipping malformed lines."""
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            word, _, count = line.strip().rpartition(" ")
            if word and count.isdigit():
                yield word, int(count)


def write_lines(name, lines, files):
    lines = list(lines)
    if len(lines) > CAP:
        step = len(lines) / CAP
        lines = [lines[int(i * step)] for i in range(CAP)]
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    files[name] = len(lines)


def main():
    os.makedirs(OUT, exist_ok=True)
    files = {}

    ko = {}
    mixed = {}
    particles = set()
    dropped_mixed = 0
    for word, count in read_counts(os.path.join(SRC, "ko_full.txt")):
        if count < KO_MIN:
            continue
        word = word.lower()
        if HANGUL.match(word) or DIGIT_HANGUL.match(word):
            ko[word] = ko.get(word, 0) + count
            continue
        m = MIXED.match(word)
        if not m:
            continue
        stem, ending = m.groups()
        if ending not in PARTICLES:
            dropped_mixed += 1
            continue
        if stem in ACRONYMS or not (set(stem) & VOWELS):
            token = stem.upper() + ending
        else:
            dropped_mixed += 1
            continue
        mixed[token] = mixed.get(token, 0) + count
        particles.add(ending)
    for token, count in mixed.items():
        ko[token] = ko.get(token, 0) + count

    by_count = lambda d: sorted(d.items(), key=lambda kv: (-kv[1], kv[0]))
    write_lines("ko_words.tsv", (f"{w}\t{c}" for w, c in by_count(ko)), files)
    write_lines("mixed.txt", (w for w, _ in by_count(mixed)), files)
    write_lines("particles.txt", sorted(particles), files)

    en = {}
    for word, count in read_counts(os.path.join(SRC, "en_full.txt")):
        if count < EN_MIN:
            continue
        if word in ("a", "i") or (ENGLISH.match(word) and word not in FRAGMENTS):
            en[word] = en.get(word, 0) + count
    write_lines("en_words.tsv", (f"{w}\t{c}" for w, c in by_count(en)), files)

    meta = {"source": NAME, "tier": TIER, "weight": WEIGHT, "uses": USES,
            "license": LICENSE, "files": files}
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(json.dumps(files), "mixed dropped:", dropped_mixed)


if __name__ == "__main__":
    main()
