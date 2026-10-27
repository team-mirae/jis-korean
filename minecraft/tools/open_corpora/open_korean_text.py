"""Extracts a Korean lexicon whitelist, particles/endings and Latin+Hangul tokens from the
open-korean-text tokenizer's dictionaries (corpora/open/open-korean-text, commit 74cc4ae) into
corpora/derived/open-korean-text. License: Apache-2.0 (corpora/open/open-korean-text/LICENSE).

The useful part is src/src/main/resources/org/openkoreantext/processor/util/: plain one-entry-
per-line word lists by part of speech. There are no frequencies (freq/entity-freq.txt.gz holds
1.4k 0..1 scores, not counts) and no running text worth keeping (example_tweets.txt is 1k lines),
so nothing here goes to the key-trigram ranker as sentences; the words are a membership list with
a flat pseudo-count of 1.

- ko_words.tsv: word<TAB>1 from noun/nouns.txt, noun/entities.txt, the small curated noun lists
  (foreign, brand, company_names, fashion, slangs, neologism, twitter), adverb/adverb.txt,
  auxiliary/*.txt, substantives/modifier.txt and example_chunks.txt (inflected chunks such as
  각오를, 가거든요). verb/verb.txt and adjective/adjective.txt hold bare stems (가꾸, 가깝), which
  are never typed alone, so they are written in dictionary form (가꾸다, 가깝다). Only all-syllable
  forms of at most 8 syllables are kept, plus runs of 2+ compatibility jamo (ㅋㅋ, ㅠㅠ).
  Skipped: wikipedia_title_nouns.txt (158k long-tail titles), people and fandom lists (congress,
  kpop, names, bible, lol, pokemon, substantives family/given names), suffix fragments,
  profane/spam, geolocations (JSON), typos (intentional orthographic play, not key slips).
- particles.txt: josa/josa.txt, then verb/eomi.txt and verb/pre_eomi.txt, deduplicated in that
  order, all-syllable entries only.
- mixed.txt: the acronym+Hangul tokens of noun/entities.txt (곰TV). Lowercase junk such as
  진중권unheim and Hangul+digit names like 갤노트3 stay out.
- meta.json.

There is no English, so no en.txt / en_words.tsv; no translations, so no terms.tsv.

Usage: python3 -I open_korean_text.py
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "open-korean-text", "src", "src", "main", "resources",
                   "org", "openkoreantext", "processor", "util")
OUT = os.path.join(ROOT, "corpora", "derived", "open-korean-text")

META = {
    "source": "open-korean-text",
    "tier": "moderate",
    "weight": 0.7,
    "uses": ["korean-lexicon", "particles-endings", "korean-key-model"],
    "license": "Apache-2.0",
}

WORD_LISTS = [
    "noun/nouns.txt", "noun/entities.txt", "noun/foreign.txt", "noun/brand.txt",
    "noun/company_names.txt", "noun/fashion.txt", "noun/slangs.txt", "noun/neologism.txt",
    "noun/twitter.txt", "adverb/adverb.txt", "auxiliary/conjunctions.txt",
    "auxiliary/determiner.txt", "auxiliary/exclamation.txt", "substantives/modifier.txt",
    "example_chunks.txt",
]
STEM_LISTS = ["verb/verb.txt", "adjective/adjective.txt"]
PARTICLE_LISTS = ["josa/josa.txt", "verb/eomi.txt", "verb/pre_eomi.txt"]
MIXED_LISTS = ["noun/entities.txt"]

MAX_SYLLABLES = 8
SYLLABLES = re.compile("^[가-힣]+$")
LAUGH = re.compile("^[ㄱ-ㅣ]{2,}$")
MIXED = re.compile("^(?=.*[가-힣])(?=.*[A-Z])[가-힣A-Z0-9]+$")  # acronym + Hangul: 곰TV
CAP = 3_000_000


def entries(rel):
    """The stripped, non-empty lines of a dictionary file (one entry per line)."""
    with open(os.path.join(SRC, rel), encoding="utf-8", errors="replace") as f:
        for line in f:
            w = line.strip().lstrip("﻿")
            if w:
                yield w


def is_word(w):
    return (SYLLABLES.match(w) and len(w) <= MAX_SYLLABLES) or LAUGH.match(w)


def write_lines(name, lines, files):
    lines = lines[:CAP]
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    files[name] = len(lines)


def main():
    os.makedirs(OUT, exist_ok=True)
    files = {}

    words = {}
    for rel in WORD_LISTS:
        for w in entries(rel):
            if is_word(w):
                words.setdefault(w, 1)
    for rel in STEM_LISTS:
        for stem in entries(rel):
            w = stem + "다"
            if SYLLABLES.match(w) and len(w) <= MAX_SYLLABLES:
                words.setdefault(w, 1)
    write_lines("ko_words.tsv", [f"{w}\t{c}" for w, c in sorted(words.items())], files)

    particles = {}
    for rel in PARTICLE_LISTS:
        for p in entries(rel):
            if SYLLABLES.match(p):
                particles.setdefault(p, None)
    write_lines("particles.txt", list(particles), files)

    mixed = {}
    for rel in MIXED_LISTS:
        for w in entries(rel):
            if MIXED.match(w):
                mixed.setdefault(w, None)
    if mixed:
        write_lines("mixed.txt", sorted(mixed), files)

    meta = dict(META, files=files)
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=2)
        f.write("\n")
    for name, n in files.items():
        print(f"{name}: {n}")


if __name__ == "__main__":
    main()
