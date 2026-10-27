"""Extracts paraKQC (corpora/open/paraKQC, github.com/warnikchow/paraKQC, CC BY-SA 4.0) into
corpora/derived/paraKQC/ for build_ranker.py. Only derived counts and cleaned sentences are used.

paraKQC is 10,000 Korean questions and commands for a smart-home/assistant setting: 1,000 sets of
10 paraphrases, written by hand (colloquial, with spacing slips and casual spellings like 지울꺼야).
Only src/data/paraKQC_v1.txt is read (two label columns<TAB>text); the label columns are dropped.
src/*.py, *.npy and the model *.hdf5 are ignored (never imported or executed).

- ko.txt: the text column, whitespace-normalized (double/trailing spaces collapsed), exact
  duplicates removed. Two typos are fixed rather than dropped: 떙볕 -> 땡볕 (7 lines) and
  바랍ㄴ다 -> 바랍니다 (1 line). The paraphrase sets are kept whole: each set is ten different
  surface forms of one intent, which is the variety the key model wants, and the corpus is too
  small for a topic to dominate. Digits and Latin inside the text are kept (2차나, usb로).
- ko_words.tsv: eojeol (space-separated tokens) over ko.txt with surrounding punctuation
  stripped, and their counts. Tokens without Hangul are left out.
- mixed.txt: distinct tokens that mix Latin letters and Hangul (usb로).

There is no English side and no morphology annotation, so no en.txt, terms.tsv or particles.txt.

Usage: python3 -I parakqc.py
"""

import collections
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "paraKQC", "src", "data", "paraKQC_v1.txt")
OUT = os.path.join(ROOT, "corpora", "derived", "paraKQC")

FIXES = [("떙볕", "땡볕"), ("바랍ㄴ다", "바랍니다")]
HANGUL = re.compile(r"[가-힣]")
LATIN = re.compile(r"[A-Za-z]")
EDGE_PUNCT = re.compile(r"^[^0-9A-Za-z가-힣]+|[^0-9A-Za-z가-힣]+$")


def write_lines(name, lines):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    return len(lines)


def main():
    seen = set()
    sentences = []
    with open(SRC, encoding="utf-8") as f:
        for raw in f:
            parts = raw.rstrip("\r\n").split("\t")
            if len(parts) != 3:
                continue
            text = " ".join(parts[2].split())
            for bad, good in FIXES:
                text = text.replace(bad, good)
            if not text or not HANGUL.search(text) or text in seen:
                continue
            seen.add(text)
            sentences.append(text)

    words = collections.Counter()
    mixed = set()
    for text in sentences:
        for tok in text.split():
            tok = EDGE_PUNCT.sub("", tok)
            # "삼촌,라디오" style slips: split on inner punctuation too.
            for w in re.split(r"[^0-9A-Za-z가-힣]+", tok):
                if not HANGUL.search(w):
                    continue
                words[w] += 1
                if LATIN.search(w):
                    mixed.add(w)

    os.makedirs(OUT, exist_ok=True)
    files = {"ko.txt": write_lines("ko.txt", sentences)}
    ordered = sorted(words.items(), key=lambda kv: (-kv[1], kv[0]))
    files["ko_words.tsv"] = write_lines("ko_words.tsv", [f"{w}\t{c}" for w, c in ordered])
    if mixed:
        files["mixed.txt"] = write_lines("mixed.txt", sorted(mixed))
    meta = {
        "source": "paraKQC",
        "tier": "moderate",
        "weight": 0.5,
        "uses": ["korean-key-model", "particles-endings", "korean-lexicon"],
        "license": "CC BY-SA 4.0",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(f"{len(sentences)} unique sentences; " + ", ".join(f"{k}: {v}" for k, v in files.items()))


main()
