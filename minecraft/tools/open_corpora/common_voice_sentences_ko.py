"""Extracts Common Voice's Korean sentence-collector sentences into corpora/derived/common-voice-sentences-ko.

Source: corpora/open/common-voice-sentences-ko/sentence-collector.txt
(https://github.com/common-voice/common-voice/blob/main/server/data/ko/sentence-collector.txt).
License: CC0 1.0 (public domain); see LICENSE-CC0-1.0.txt beside it.

About 6,400 Korean sentences, one per line, largely taken from public-domain literature (so some
dialect and archaic speech: 아이구, 하구려). Each line has its quotation marks (straight, curly,
corner brackets), ellipses (U+2026) and stray Hangul fillers (U+115F) removed and its whitespace
collapsed. Lines with fewer than 3 Hangul syllables are dropped, and the rest are deduplicated in
source order. The few lines with Latin letters (K선생님, C샤프) are kept. Writes:
  ko.txt     the cleaned sentences, one per line
  mixed.txt  distinct Latin+Hangul tokens (K씨였었다), one per line
  meta.json  source, tier, weight, uses, license, line counts

Usage: python3 -I common_voice_sentences_ko.py
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "common-voice-sentences-ko", "sentence-collector.txt")
OUT = os.path.join(ROOT, "corpora", "derived", "common-voice-sentences-ko")

STRIP = re.compile("[\"'“”‘’『』「」…ᅟᅠ​﻿]")
HANGUL = re.compile("[가-힣]")
LATIN = re.compile("[A-Za-z]")
EDGE_PUNCT = ".,?!;:()[]·―–-"


def clean(line):
    line = STRIP.sub("", line)
    return " ".join(line.split())


def main():
    sentences = []
    seen = set()
    mixed = set()
    with open(SRC, encoding="utf-8", errors="replace") as f:
        for raw in f:
            line = clean(raw)
            if len(HANGUL.findall(line)) < 3 or line in seen:
                continue
            seen.add(line)
            sentences.append(line)
            for tok in line.split():
                tok = tok.strip(EDGE_PUNCT)
                if LATIN.search(tok) and HANGUL.search(tok):
                    mixed.add(tok)
    os.makedirs(OUT, exist_ok=True)
    files = {"ko.txt": sentences, "mixed.txt": sorted(mixed)}
    counts = {}
    for name, lines in files.items():
        if not lines:
            continue
        with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
            f.write("\n".join(lines) + "\n")
        counts[name] = len(lines)
    meta = {
        "source": "common-voice-sentences-ko",
        "tier": "moderate",
        "weight": 0.5,
        "uses": ["korean-key-model", "particles-endings", "korean-lexicon"],
        "license": "CC0-1.0",
        "files": counts,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    for name, n in counts.items():
        print(f"{name}: {n} lines")


main()
