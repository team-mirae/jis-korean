"""Extracts 3i4k (corpora/open/3i4k, github.com/warnikchow/3i4k, CC BY-SA 4.0) into
corpora/derived/3i4k/ for build_ranker.py. Only derived counts and sampled sentences are used.

3i4k is ~61k colloquial Korean utterances labeled by intention (0 fragment, 1 statement,
2 question, 3 command, 4 rhetorical question, 5 rhetorical command, 6 intonation-dependent).
Only src/data/fci.txt is read (label<TAB>text); fci-iu.txt and train_val_test/ are subsets or
splits of it, intodep.txt is small and in another format. The text is pure Hangul, with no
Latin or digits (numbers are spelled out: 공일육, 이십일), so there is no English or mixed output.

- ko.txt: labels 1-6, whitespace-normalized, exact duplicates removed, lines with any character
  outside Hangul syllables and spaces dropped. Assistant-command templates are capped: at most
  CAP_PER_ENDING sentences share a last token (알려줘, 있니, 확인해줘...), sampled with
  random.Random(20261027), so command phrasing doesn't dominate.
- ko_words.tsv: space-separated tokens over all deduplicated lines (fragments included), with
  their counts. Spelled-out digit strings (3+ syllables of 공영일이삼사오육칠팔구 only: phone
  numbers like 공일육, 팔사칠팔) and interjection noise (a syllable repeated 3+ times: 아구구구구)
  are left out.

Usage: python3 -I 3i4k.py
"""

import collections
import json
import os
import random
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "3i4k", "src", "data", "fci.txt")
OUT = os.path.join(ROOT, "corpora", "derived", "3i4k")

CAP_PER_ENDING = 500
HANGUL_LINE = re.compile(r"[가-힣 ]+")
DIGIT_WORD = re.compile(r"[공영일이삼사오육칠팔구]{3,}")
REPEAT = re.compile(r"(.)\1\1")


def write_lines(name, lines):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    return len(lines)


def main():
    seen = set()
    rows = []  # (label, text), first occurrence order
    with open(SRC, encoding="utf-8") as f:
        for raw in f:
            parts = raw.rstrip("\n").split("\t", 1)
            if len(parts) != 2:
                continue
            text = " ".join(parts[1].split())
            if not text or not HANGUL_LINE.fullmatch(text) or text in seen:
                continue
            seen.add(text)
            rows.append((parts[0].strip(), text))

    # Sentences: no fragments, templates capped by last token.
    by_ending = collections.defaultdict(list)
    for label, text in rows:
        if label != "0" and " " in text:
            by_ending[text.rsplit(" ", 1)[1]].append(text)
    rng = random.Random(20261027)
    keep = set()
    for ending in sorted(by_ending):
        group = by_ending[ending]
        if len(group) > CAP_PER_ENDING:
            group = rng.sample(group, CAP_PER_ENDING)
        keep.update(group)
    sentences = [text for _, text in rows if text in keep]

    words = collections.Counter()
    for _, text in rows:
        for tok in text.split():
            if DIGIT_WORD.fullmatch(tok) or REPEAT.search(tok):
                continue
            words[tok] += 1

    os.makedirs(OUT, exist_ok=True)
    files = {"ko.txt": write_lines("ko.txt", sentences)}
    ordered = sorted(words.items(), key=lambda kv: (-kv[1], kv[0]))
    files["ko_words.tsv"] = write_lines("ko_words.tsv", [f"{w}\t{c}" for w, c in ordered])
    meta = {
        "source": "3i4k",
        "tier": "moderate",
        "weight": 0.6,
        "uses": ["korean-key-model", "korean-lexicon", "particles-endings"],
        "license": "CC BY-SA 4.0",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(f"{len(rows)} unique lines; " + ", ".join(f"{k}: {v}" for k, v in files.items()))


main()
