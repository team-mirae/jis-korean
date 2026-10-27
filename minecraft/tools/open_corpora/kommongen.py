"""Extracts KommonGen (corpora/open/kommongen, github.com/nlpai-lab/KommonGen, MIT) into
corpora/derived/kommongen/ for build_ranker.py. Only derived counts and sentences are used.

KommonGen is a Korean commonsense-generation set: each record pairs a "concept-set" (a shuffled
list of lemmas) with a "scene", a short human-written Korean sentence using them, mostly in an
image-caption domain (기린, 얼룩말, 프리스비). About 90k scenes, ~450k eojeol, all unique.

Only src/dataset/kommongen_train.json, kommongen_dev.json and kommongen_test_1.1.json are read,
as JSON Lines, and only their "scene" field. "concept-set" is lemma soup, not text, and
src/examples/*.txt are model generations, so both are skipped.

- ko.txt: scenes, NFC-normalized and whitespace-collapsed, exact duplicates removed, lines with
  fewer than 2 eojeol, URLs, or characters outside Hangul syllables and printable ASCII dropped.
  Sentence-level trigrams and particle/ending statistics at low weight (caption domain skews
  word frequencies, so no ko_words.tsv).
- mixed.txt: distinct eojeol mixing Latin letters and Hangul (TV를, Wii와, GPS장치를), trailing
  punctuation stripped, tokens with inner quotes or symbols skipped. A tiny sample (~700 lines
  contain Latin); not trustworthy acronym data.

Usage: python3 -I kommongen.py
"""

import json
import os
import re
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
SRC = os.path.join(ROOT, "corpora", "open", "kommongen", "src", "dataset")
OUT = os.path.join(ROOT, "corpora", "derived", "kommongen")
FILES = ["kommongen_train.json", "kommongen_dev.json", "kommongen_test_1.1.json"]

ALLOWED = re.compile(r"[가-힣\x20-\x7e]+")
URL = re.compile(r"https?://|www\.|\.(com|net|org|kr)\b", re.I)
LATIN = re.compile(r"[A-Za-z]")
HANGUL = re.compile(r"[가-힣]")
EDGE_PUNCT = re.compile(r"^[^0-9A-Za-z가-힣]+|[^0-9A-Za-z가-힣]+$")
MIXED_TOKEN = re.compile(r"[0-9A-Za-z가-힣\-]+")


def write_lines(name, lines):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    return len(lines)


def scenes():
    for name in FILES:
        with open(os.path.join(SRC, name), encoding="utf-8") as f:
            for raw in f:
                raw = raw.strip()
                if not raw:
                    continue
                try:
                    rec = json.loads(raw)
                except ValueError:
                    continue
                scene = rec.get("scene") if isinstance(rec, dict) else None
                if isinstance(scene, str):
                    yield scene


def main():
    os.makedirs(OUT, exist_ok=True)
    seen = set()
    sentences = []
    mixed = set()
    for scene in scenes():
        text = " ".join(unicodedata.normalize("NFC", scene).split())
        if (not text or text in seen or len(text.split()) < 2
                or not ALLOWED.fullmatch(text) or URL.search(text)
                or not HANGUL.search(text)):
            continue
        seen.add(text)
        sentences.append(text)
        if LATIN.search(text):
            for tok in text.split():
                tok = EDGE_PUNCT.sub("", tok)
                if (MIXED_TOKEN.fullmatch(tok) and LATIN.search(tok)
                        and HANGUL.search(tok)):
                    mixed.add(tok)

    files = {
        "ko.txt": write_lines("ko.txt", sentences),
        "mixed.txt": write_lines("mixed.txt", sorted(mixed)),
    }
    meta = {
        "source": "kommongen",
        "tier": "moderate",
        "weight": 0.4,
        "uses": ["korean-key-model", "particles-endings"],
        "license": "MIT (Copyright (c) 2021 NLP & AI Lab)",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=2)
        f.write("\n")
    for name, n in files.items():
        print(f"{name}: {n}")


if __name__ == "__main__":
    main()
