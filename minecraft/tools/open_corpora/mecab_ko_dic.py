"""Extracts a Korean lexicon whitelist, particles/endings and Hangul loanword spellings from the
mecab-ko-dic morpheme dictionary (corpora/open/mecab-ko-dic, 2.1.1-20180720) into
corpora/derived/mecab-ko-dic. License: Apache-2.0 (corpora/open/mecab-ko-dic/LICENSE).

The dictionary is CSV per part of speech (src/*.csv): column 1 is the surface, column 5 the POS,
column 7 the jongseong flag (T/F), column 9 the type (Compound/Inflect/*) and column 12 the
morpheme expression (가시/NNG/*+범/NNG/*+...). There are no frequencies and no running text, so
nothing here goes to the key-trigram ranker as sentences; the words are a membership list with a
flat pseudo-count of 1 (the dictionary's own cost is mostly the constant 2639).

- ko_words.tsv: word<TAB>1 from NNG, Foreign, CoinedWord, VV, VA, MAG, NNB, NR, NP, MM, IC and
  Inflect (inflected forms such as 모아요, 이뤄졌으면). Only all-syllable forms of 2..6 syllables
  (single syllables from the closed classes NNB/NP/NR/MM/IC/MAG are kept too), so single jamo
  (ㄱ, ㄴ) and jamo-led stems drop out; CoinedWord/IC laughter runs of 2+ compatibility jamo
  (ㅋㅋ, ㅎㅎ) are kept. NNG compounds of 3 or more parts (가시/범/하늘소) are dropped as rare.
  Hanja.csv (Hanja surfaces), Person*, Place*, NorthKorea, NNP, Wikipedia and Group are skipped.
- particles.txt: particles (J.csv) then endings (EC, EF, EP, ETM, ETN), deduplicated, keeping only
  the forms that start with a full syllable (을, 에서, 습니다). Endings led by a lone jamo
  (ᆫ다고, ㄴ가, ㅓ도) attach to the previous syllable's final and are not typed standalone.
- terms.tsv: Foreign.csv's Hangul loanword spellings (데이터, 플랫폼), korean<TAB> with an empty
  English column (the dictionary carries no source spelling). A seed for Hangul spellings of
  English business/tech words; it also contains non-business loanwords (가네샤).
- meta.json.

The dictionary has no Latin+Hangul surfaces, so there is no mixed.txt.

Usage: python3 -I mecab_ko_dic.py
"""

import csv
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "mecab-ko-dic", "src")
OUT = os.path.join(ROOT, "corpora", "derived", "mecab-ko-dic")

META = {
    "source": "mecab-ko-dic",
    "tier": "moderate",
    "weight": 0.5,
    "uses": ["korean-lexicon", "particles-endings", "business-terms"],
    "license": "Apache-2.0",
}

LEXICON = ["NNG", "Foreign", "CoinedWord", "VV", "VA", "MAG", "NNB", "NR", "NP", "MM", "IC",
           "Inflect"]
# Closed classes where a one-syllable word is a real typed word (것, 나, 셋, 그, 아, 꼭).
SHORT_OK = {"NNB", "NP", "NR", "MM", "IC", "MAG"}
PARTICLES = ["J", "EC", "EF", "EP", "ETM", "ETN"]
MAX_SYLLABLES = 6
SYLLABLES = re.compile("^[가-힣]+$")
LAUGH = re.compile("^[ㄱ-ㅎ]{2,}$")
CAP = 3_000_000


def rows(name):
    """The rows of src/<name>.csv. The files are plain CSV without quoting; csv handles both."""
    path = os.path.join(SRC, name + ".csv")
    with open(path, encoding="utf-8", errors="replace", newline="") as f:
        for row in csv.reader(f):
            if len(row) >= 8 and row[0]:
                yield row


def compound_parts(row):
    if len(row) >= 12 and row[8] == "Compound":
        return row[11].count("+") + 1
    return 1


def main():
    os.makedirs(OUT, exist_ok=True)

    words = {}
    for name in LEXICON:
        for row in rows(name):
            w = row[0].strip()
            if SYLLABLES.match(w):
                n = len(w)
                if n > MAX_SYLLABLES or (n == 1 and name not in SHORT_OK):
                    continue
                if name == "NNG" and compound_parts(row) >= 3:
                    continue
            elif not (name in ("CoinedWord", "IC") and LAUGH.match(w)):
                continue
            words[w] = 1

    particles = []
    seen = set()
    for name in PARTICLES:
        for row in rows(name):
            p = row[0].strip()
            if SYLLABLES.match(p) and p not in seen:
                seen.add(p)
                particles.append(p)

    terms = []
    tseen = set()
    for row in rows("Foreign"):
        t = row[0].strip()
        if SYLLABLES.match(t) and len(t) >= 2 and t not in tseen:
            tseen.add(t)
            terms.append(t)

    files = {}

    def write(fname, lines):
        lines = lines[:CAP]
        with open(os.path.join(OUT, fname), "w", encoding="utf-8", newline="\n") as f:
            for line in lines:
                f.write(line + "\n")
        files[fname] = len(lines)

    write("ko_words.tsv", [f"{w}\t{c}" for w, c in sorted(words.items())])
    write("particles.txt", particles)
    write("terms.tsv", [f"{t}\t" for t in sorted(terms)])

    meta = dict(META, files=files)
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=2)
        f.write("\n")
    for name, n in files.items():
        print(f"{name}\t{n}")


if __name__ == "__main__":
    main()
