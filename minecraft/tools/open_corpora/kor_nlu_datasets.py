"""Extracts kor-nlu-datasets (corpora/open/kor-nlu-datasets, github.com/kakaobrain/kor-nlu-datasets,
CC BY-SA 4.0) into corpora/derived/kor-nlu-datasets/ for build_ranker.py. Only derived counts and
sampled sentences are used.

KorNLI and KorSTS are SNLI, MultiNLI, XNLI and STS-B translated into Korean (train sets by machine
translation, XNLI and STS dev/test sets post-edited by humans). The text is written Korean: news,
government reports, fiction, letters and telephone speech (MultiNLI), image captions (SNLI, STS).
Acronyms keep their Latin spelling and take Korean particles (GAO는, TV를, CEO들을).

Files read: src/KorNLI/{snli_1.0_train,multinli.train,xnli.dev,xnli.test}.ko.tsv (sentence1,
sentence2, gold_label; header skipped) and src/KorSTS/sts-{train,dev,test}.tsv (sentence1 and
sentence2 are columns 6 and 7).

- ko.txt: sentence1 and sentence2 of every pair, whitespace-normalized, exact duplicates removed
  (premises repeat once per hypothesis). Lines without Hangul are dropped, and so are lines with a
  run of 4+ lowercase Latin letters, a multi-word capitalized name (Place des Vosges) or a '?'
  inside a Latin word (Vel? Crose): those are proper nouns the machine translation left
  untransliterated, or encoding damage. SNLI captions are a narrow register, so SNLI is sampled
  to SNLI_KEEP of its lines (random.Random(20261027)), which weighs it about 0.3 against
  MultiNLI's 0.7. XNLI and KorSTS are kept whole.
- mixed.txt: distinct acronym+particle tokens (^[A-Z]{2,5}s?[가-힣]{1,3}$: EU의, CEO들을) seen
  in the kept lines, after stripping surrounding quotes and punctuation, sorted.

No ko_words.tsv: build_ranker.py counts words from ko.txt itself.

Usage: python3 -I kor_nlu_datasets.py
"""

import json
import os
import random
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "kor-nlu-datasets", "src")
OUT = os.path.join(ROOT, "corpora", "derived", "kor-nlu-datasets")

# (path under src, column of sentence1, column of sentence2, keep fraction)
SNLI_KEEP = 0.43  # ~0.3 / 0.7 of MultiNLI's weight
FILES = [
    ("KorNLI/multinli.train.ko.tsv", 0, 1, 1.0),
    ("KorNLI/snli_1.0_train.ko.tsv", 0, 1, SNLI_KEEP),
    ("KorNLI/xnli.dev.ko.tsv", 0, 1, 1.0),
    ("KorNLI/xnli.test.ko.tsv", 0, 1, 1.0),
    ("KorSTS/sts-train.tsv", 5, 6, 1.0),
    ("KorSTS/sts-dev.tsv", 5, 6, 1.0),
    ("KorSTS/sts-test.tsv", 5, 6, 1.0),
]
MAX_LINES = 3_000_000

HANGUL = re.compile(r"[가-힣]")
LOWER_RUN = re.compile(r"[a-z]{4,}")
CAP_NAME = re.compile(r"\b[A-Z][a-z]+(?:[ -](?:[a-z]{1,3} )?[A-Z][a-z]*)+")
BROKEN = re.compile(r"[A-Za-z]\?(?:[A-Za-z]| [a-z])")
CONTROL = re.compile(r"[\x00-\x08\x0b-\x1f\x7f​-‏﻿]")
ACRONYM = re.compile(r"[A-Z]{2,5}s?[가-힣]{1,3}")
STRIP = "\"'“”‘’()[]{}<>«»「」『』.,!?;:…·-–—/"


def clean(text):
    text = " ".join(CONTROL.sub(" ", text).split())
    if not text or not HANGUL.search(text):
        return None
    if LOWER_RUN.search(text) or CAP_NAME.search(text) or BROKEN.search(text):
        return None
    if "http" in text or "www." in text:
        return None
    return text


def write_lines(name, lines):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    return len(lines)


def main():
    rng = random.Random(20261027)
    seen = set()
    lines = []
    mixed = set()
    for rel, c1, c2, keep in FILES:
        kept = 0
        with open(os.path.join(SRC, rel), encoding="utf-8", errors="replace") as f:
            next(f, None)  # header
            for raw in f:
                cols = raw.rstrip("\n").split("\t")
                if len(cols) <= max(c1, c2):
                    continue
                for col in (c1, c2):
                    text = clean(cols[col])
                    if text is None or text in seen:
                        continue
                    seen.add(text)  # first occurrence decides, sampled or not
                    if keep < 1.0 and rng.random() >= keep:
                        continue
                    lines.append(text)
                    kept += 1
                    for tok in text.split():
                        tok = tok.strip(STRIP)
                        if ACRONYM.fullmatch(tok):
                            mixed.add(tok)
        print(f"  {rel}: {kept} lines")
    if len(lines) > MAX_LINES:
        picks = sorted(rng.sample(range(len(lines)), MAX_LINES))
        lines = [lines[i] for i in picks]

    os.makedirs(OUT, exist_ok=True)
    files = {"ko.txt": write_lines("ko.txt", lines)}
    files["mixed.txt"] = write_lines("mixed.txt", sorted(mixed))
    meta = {
        "source": "kor-nlu-datasets",
        "tier": "moderate",
        "weight": 0.6,
        "uses": ["korean-key-model", "korean-lexicon", "particles-endings", "acronyms", "mixed-ko-en"],
        "license": "CC BY-SA 4.0",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(", ".join(f"{k}: {v}" for k, v in files.items()))


main()
