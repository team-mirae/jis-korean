"""Extracts Question_pair (corpora/open/question-pair, github.com/songys/Question_pair, MIT) into
corpora/derived/question-pair/ for build_ranker.py. Only derived counts and sentences are used.

Question_pair is a small Korean paraphrase set: pairs of short, casually typed questions and
statements (mostly about dating and breakups, from the same author as Chatbot_data) labelled as
paraphrase or not. About 13.9k unique sentences after merging both releases.

Read: src/kor_pair_train.csv and src/kor_Pair_test.csv (CSV with a UTF-8 BOM; question1 and
question2), and src/train.txt, validation.txt, test.txt (TSV with a header row; columns 0 and 1).
Labels and ids are ignored. data_image.png, config.json and README.md are skipped. No Chatbot_data
corpus is in the mix, so there is nothing to dedupe against beyond this source.

- ko.txt: every sentence as typed (casual endings, ㅋㅋ/ㅠㅠ kept), NFC-normalized and
  whitespace-collapsed (NBSP included), U+2026 spelled as "...", exact duplicates across all
  five files removed. Lines with URLs, without Hangul, or with characters outside Hangul syllables, Hangul compatibility
  jamo and printable ASCII are dropped. Sentence-level trigrams for the key model.
- ko_words.tsv: whitespace eojeol of ko.txt with edge punctuation stripped, counted (count =
  occurrences in the deduplicated sentences). Eojeol must contain Hangul and consist only of
  Hangul, Latin letters and digits.
- mixed.txt: distinct eojeol mixing Latin letters and Hangul (SNS에, SD카드, sns를). Tiny
  (~30 source lines contain Latin), but real acronym+particle usage.

Usage: python3 -I question_pair.py
"""

import collections
import csv
import json
import os
import re
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
SRC = os.path.join(ROOT, "corpora", "open", "question-pair", "src")
OUT = os.path.join(ROOT, "corpora", "derived", "question-pair")
CSV_FILES = ["kor_pair_train.csv", "kor_Pair_test.csv"]
TSV_FILES = ["train.txt", "validation.txt", "test.txt"]

ALLOWED = re.compile(r"[가-힣ㄱ-ㆎ\x20-\x7e]+")
URL = re.compile(r"https?://|www\.|\.(com|net|org|kr)\b", re.I)
LATIN = re.compile(r"[A-Za-z]")
HANGUL = re.compile(r"[가-힣]")
EDGE_PUNCT = re.compile(r"^[^0-9A-Za-z가-힣]+|[^0-9A-Za-z가-힣]+$")
WORD = re.compile(r"[0-9A-Za-z가-힣]+")


def write_lines(name, lines):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    return len(lines)


def raw_sentences():
    for name in CSV_FILES:
        with open(os.path.join(SRC, name), encoding="utf-8-sig", newline="") as f:
            for row in csv.DictReader(f):
                for key in ("question1", "question2"):
                    val = row.get(key)
                    if isinstance(val, str):
                        yield val
    for name in TSV_FILES:
        with open(os.path.join(SRC, name), encoding="utf-8-sig") as f:
            next(f, None)  # header
            for line in f:
                cols = line.rstrip("\r\n").split("\t")
                for val in cols[:2]:
                    yield val


def main():
    os.makedirs(OUT, exist_ok=True)
    seen = set()
    sentences = []
    words = collections.Counter()
    mixed = set()
    for raw in raw_sentences():
        text = " ".join(unicodedata.normalize("NFC", raw).replace("\u2026", "...").split())
        if (not text or text in seen or not ALLOWED.fullmatch(text)
                or URL.search(text) or not HANGUL.search(text)):
            continue
        seen.add(text)
        sentences.append(text)
        for tok in text.split():
            tok = EDGE_PUNCT.sub("", tok)
            if not WORD.fullmatch(tok) or not HANGUL.search(tok):
                continue
            words[tok] += 1
            if LATIN.search(tok):
                mixed.add(tok)

    word_lines = [f"{w}\t{c}" for w, c in sorted(words.items(), key=lambda x: (-x[1], x[0]))]
    files = {
        "ko.txt": write_lines("ko.txt", sentences),
        "ko_words.tsv": write_lines("ko_words.tsv", word_lines),
        "mixed.txt": write_lines("mixed.txt", sorted(mixed)),
    }
    meta = {
        "source": "question-pair",
        "tier": "moderate",
        "weight": 0.5,
        "uses": ["korean-key-model", "korean-lexicon", "particles-endings", "mixed-ko-en",
                 "acronyms"],
        "license": "MIT (Copyright (c) songys, github.com/songys/Question_pair)",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=2)
        f.write("\n")
    for name, n in files.items():
        print(f"{name}: {n}")


if __name__ == "__main__":
    main()
