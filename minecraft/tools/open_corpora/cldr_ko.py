"""Extracts a small Korean lexicon from Unicode CLDR (corpora/open/cldr-ko) into
corpora/derived/cldr-ko. License: Unicode License v3 (corpora/open/cldr-ko/LICENSE). Only word
counts derived from it are used.

Source: unicode-org/cldr-json, the Korean emoji annotations (cldr-annotations-full and
cldr-annotations-derived-full) and the Korean locale display names (cldr-localenames-full,
languages / territories / scripts). CLDR has no running text and no frequencies, so this yields:

- ko_words.tsv: every all-Hangul token of two or more syllables found in the emoji names and
  keywords (default[] + tts[]) and in the language, territory and script names, split on
  whitespace and punctuation. Math-operator annotations (U+2200..U+22FF, U+2A00..U+2AFF) are
  stilted and dropped; derived annotations carrying a skin-tone modifier (U+1F3FB..U+1F3FF) only
  repeat their base phrase plus a skin colour and are skipped. Count is flat and low: the number
  of the three sources (annotations, derived annotations, locale names) the word appears in, 1..3.
- mixed.txt: distinct tokens mixing Latin letters and Hangul (B형, N항).
- meta.json.

Usage: python3 -I cldr_ko.py
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "cldr-ko", "src", "cldr-json")
OUT = os.path.join(ROOT, "corpora", "derived", "cldr-ko")

HANGUL_WORD = re.compile("^[가-힣]{2,}$")
MIXED = re.compile("^(?=.*[A-Za-z])(?=.*[가-힣])[A-Za-z가-힣]+$")
SPLIT = re.compile(r"[\s,()\[\]:?!.·/\"'“”‘’]+")
SKIN = re.compile("[\U0001F3FB-\U0001F3FF]")


def load(*parts):
    with open(os.path.join(SRC, *parts), encoding="utf-8") as f:
        return json.load(f)


def is_math(key):
    return any(0x2200 <= ord(c) <= 0x22FF or 0x2A00 <= ord(c) <= 0x2AFF for c in key)


def phrases_from_annotations(annotations, skip_skin):
    for key, entry in annotations.items():
        if is_math(key) or (skip_skin and SKIN.search(key)):
            continue
        for phrase in entry.get("default", []) + entry.get("tts", []):
            yield phrase


def tokens(phrases):
    words, mixed = set(), set()
    for phrase in phrases:
        for tok in SPLIT.split(phrase):
            if HANGUL_WORD.match(tok):
                words.add(tok)
            elif MIXED.match(tok):
                mixed.add(tok)
    return words, mixed


def main():
    ann = load("cldr-annotations-full", "annotations", "ko", "annotations.json")
    der = load("cldr-annotations-derived-full", "annotationsDerived", "ko", "annotations.json")
    names = []
    for kind in ("languages", "territories", "scripts"):
        names.extend(load("cldr-localenames-full", "main", "ko", kind + ".json")
                     ["main"]["ko"]["localeDisplayNames"][kind].values())

    sources = [
        tokens(phrases_from_annotations(ann["annotations"]["annotations"], False)),
        tokens(phrases_from_annotations(der["annotationsDerived"]["annotations"], True)),
        tokens(names),
    ]
    counts, mixed = {}, set()
    for words, mx in sources:
        for w in words:
            counts[w] = counts.get(w, 0) + 1
        mixed |= mx

    os.makedirs(OUT, exist_ok=True)
    files = {}
    ranked = sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))
    with open(os.path.join(OUT, "ko_words.tsv"), "w", encoding="utf-8", newline="\n") as f:
        for w, c in ranked:
            f.write(f"{w}\t{c}\n")
    files["ko_words.tsv"] = len(ranked)
    if mixed:
        with open(os.path.join(OUT, "mixed.txt"), "w", encoding="utf-8", newline="\n") as f:
            for t in sorted(mixed):
                f.write(t + "\n")
        files["mixed.txt"] = len(mixed)
    meta = {
        "source": "cldr-ko",
        "tier": "moderate",
        "weight": 0.3,
        "uses": ["korean-lexicon"],
        "license": "Unicode License v3",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(files)


if __name__ == "__main__":
    main()
