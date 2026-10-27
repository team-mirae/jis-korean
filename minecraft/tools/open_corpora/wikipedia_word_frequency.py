"""Extracts IlyaSemenov/wikipedia-word-frequency (enwiki-2023-04-13.txt) from
corpora/open/wikipedia-word-frequency into corpora/derived/wikipedia-word-frequency. Source:
https://github.com/IlyaSemenov/wikipedia-word-frequency, word counts over the English Wikipedia
dump of 2023-04-13 (encyclopedic register; lower-cased, punctuation stripped, words with digits
dropped, only words used in at least 3 articles). License: MIT (LICENSE). Tier moderate,
weight 0.7.

Input lines are `word count` (count = last space-separated field).

- en_words.tsv: word<TAB>count for words matching ^[a-z]+(['-][a-z]+)*$ (drops non-ASCII, CJK
  and Hangul lines) with count >= 200, sorted by count. Single letters other than "a" and "i"
  (initials), contraction fragments (ll, didn, ...) and possessives (band's, women's) are
  dropped; contractions (don't, it's) and hyphenated words (long-term) are kept. Counts are the
  source's raw frequencies: filter to >= 1000 for the core ranker/lexicon, take log-scaled
  counts and cap the top function words when training.
- meta.json: source, tier, weight, uses, license, line counts.

Not done here (needs the Korean lexicon, so it belongs in the trainer): drop or down-weight
words whose 2-set key mapping spells a Korean word (gks = 한). The source is lower-cased, so
don't take acronym casing from it.

Usage: python3 -I wikipedia_word_frequency.py
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "wikipedia-word-frequency")
OUT = os.path.join(ROOT, "corpora", "derived", "wikipedia-word-frequency")

NAME = "wikipedia-word-frequency"
TIER = "moderate"
WEIGHT = 0.7
USES = ["english-key-model", "english-lexicon", "english-nouns"]
LICENSE = ("MIT (IlyaSemenov/wikipedia-word-frequency); word counts over the English "
           "Wikipedia dump of 2023-04-13")

EN_MIN = 200
CAP = 3_000_000

ENGLISH = re.compile(r"^[a-z]+(['-][a-z]+)*$")
# Halves of English contractions, not words on their own.
FRAGMENTS = {"re", "ll", "ve", "nt", "didn", "doesn", "isn", "wouldn", "couldn", "shouldn",
             "wasn", "weren", "hasn", "hadn", "aren", "mustn", "needn", "ain", "mightn",
             "don", "haven"}
# 's words that are contractions, not possessives.
S_CONTRACTIONS = {"it's", "he's", "she's", "that's", "there's", "what's", "who's", "let's",
                  "here's", "where's", "how's", "one's"}
KEEP_SHORT = {"a", "i"}


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


def keep(word):
    if not ENGLISH.match(word):
        return False
    if len(word) == 1:
        return word in KEEP_SHORT
    if word in FRAGMENTS:
        return False
    if word.endswith("'s") and word not in S_CONTRACTIONS:
        return False
    return True


def main():
    os.makedirs(OUT, exist_ok=True)
    files = {}

    en = {}
    for word, count in read_counts(os.path.join(SRC, "enwiki-2023-04-13.txt")):
        if count < EN_MIN:
            continue
        word = word.lower()
        if keep(word):
            en[word] = en.get(word, 0) + count
    ranked = sorted(en.items(), key=lambda kv: (-kv[1], kv[0]))
    write_lines("en_words.tsv", (f"{w}\t{c}" for w, c in ranked), files)

    meta = {"source": NAME, "tier": TIER, "weight": WEIGHT, "uses": USES,
            "license": LICENSE, "files": files}
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(json.dumps(files), ">=1000:", sum(1 for c in en.values() if c >= 1000))


if __name__ == "__main__":
    main()
