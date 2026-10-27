"""Extracts an English seed lexicon, common nouns and acronyms from the Moby word lists into
corpora/derived/moby/ (gitignored) for training the predictor's English side (docs/spec.md §10.3;
minecraft/tools/build_ranker.py).

Source: Grady Ward's Moby project via Project Gutenberg: https://www.gutenberg.org/ebooks/3201
(Moby Word Lists, corpora/open/moby/3201/files/) and https://www.gutenberg.org/ebooks/3203 (Moby
Part-of-Speech, corpora/open/moby/3203/mobypos.txt). Public domain (README-3201.txt, LICENSE.txt).
Tier moderate, weight 0.5. Files are read as latin-1 with CR stripped.

What it writes (en_words.tsv, word<TAB>count; nothing Korean, so no ko/mixed/particles files):
- Seed words from freq.txt (1,000 most frequent words of general text, ranked, no counts) and
  freq-int.txt (1,000 most frequent words on Usenet in 1992, with % of tokens). Counts are per
  million tokens: freq-int's % times 10,000 (case variants summed: The + the), freq.txt's rank r
  as a Zipf estimate SEED_ZIPF / r; a word in both gets the larger. Only alphabetic words are
  kept (no contractions, no single letters but a and i). From freq-int, all-caps tokens (COM,
  DAILY, UUCP), capitalized words with no lowercase form in the list (John, Berkeley) and Usenet
  path/host tokens (edu, uucp, hplabs, rutgers) are dropped.
- Common nouns from mobypos.txt (`word\\POS`): POS contains N, word matches ^[a-z]{3,}$, kept
  only when a frequency list knows them (hermitdave FrequencyWords en_full.txt count >=
  MIN_NOUN_FREQ, or engine/predict/common-en.txt), then the NOUN_CAP most frequent. Count 1 (Moby has no
  frequencies); the frequency list only filters and ranks, its counts are not copied (CC BY-SA,
  shipped by its own extractor). Nouns already among the seeds keep the seed count.
- Acronyms from acronyms.txt matching ^[A-Z0-9&]{2,6}$, kept only when the frequency list knows
  them in lowercase (count >= MIN_ACRONYM_FREQ) or common-en.txt has them, written as in the
  source (CEO, PDF, 3D), count 1. Skipped: entries whose lowercase (or its singular, -s) is an
  ordinary word in common.txt or freq.txt (TEMPO, ACTS, CAT, HE, AS) unless listed in
  KEEP_ACRONYMS (IT, US, OK, CD); entries that are names or places in names*.txt/places.txt
  (TINA, ESSEX); and one letter repeated (AAAAAA).
common.txt, names*.txt and places.txt are read only to filter acronyms. Skipped (rare or
archaic words, phrases, affixes): single.txt, compound.txt, crosswd*.txt, KJVfreq.txt,
usaconst.txt, fiction.txt, oftenmis.txt.

Usage: python3 -I moby.py
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
SRC = os.path.join(ROOT, "corpora", "open", "moby")
FILES = os.path.join(SRC, "3201", "files")
POS = os.path.join(SRC, "3203", "mobypos.txt")
FREQ = os.path.join(ROOT, "corpora", "open", "hermitdave-frequencywords", "en_full.txt")
COMMON_EN = os.path.join(ROOT, "engine", "predict", "common-en.txt")
OUT = os.path.join(ROOT, "corpora", "derived", "moby")

SEED_ZIPF = 40_000  # per-million estimate for rank 1 of freq.txt (freq-int has "the" at 40,184)
MIN_NOUN_FREQ = 20
NOUN_CAP = 20_000
MIN_ACRONYM_FREQ = 20
KEEP_ACRONYMS = {"IT", "US", "OK", "ID", "TV", "PC", "AI", "UN", "EU", "UK", "AM", "PM", "CD", "DOS", "CM"}
# Usenet 1992 header/path/host tokens in freq-int.txt (not words people type in prose).
USENET_JUNK = {
    "edu", "uucp", "uunet", "hplabs", "ucbvax", "decwrl", "rutgers", "usr", "att", "cmu", "mit",
    "com", "bitnet", "arpa", "rec", "misc", "soc", "sci", "comp", "mod", "hp", "cs", "cc", "ca",
    "ac", "uk", "gov", "int", "pm", "ve", "ll", "re", "etc", "info", "postmaster", "newsgroup",
    "char", "btw", "sys",
}

WORD = re.compile(r"[A-Za-z]+")
NOUN = re.compile(r"[a-z]{3,}")
ACRONYM = re.compile(r"[A-Z0-9&]{2,6}")


def lines(path):
    with open(path, encoding="latin-1", newline="") as f:
        for line in f:
            yield line.replace("\r", "").rstrip("\n")


def seed_words():
    counts = {}
    # freq.txt: header line, blank line, then one word per line in rank order.
    rank = 0
    zipf = {}
    for line in list(lines(os.path.join(FILES, "freq.txt")))[1:]:
        w = line.strip()
        if not w:
            continue
        rank += 1
        if not WORD.fullmatch(w):
            continue
        w = w.lower()
        if len(w) == 1 and w not in ("a", "i"):
            continue
        zipf.setdefault(w, SEED_ZIPF / rank)
    # freq-int.txt: "pct word" in decreasing order.
    raw = {}
    for line in list(lines(os.path.join(FILES, "freq-int.txt")))[1:]:
        parts = line.split()
        if len(parts) != 2:
            continue
        try:
            pct = float(parts[0])
        except ValueError:
            continue
        raw[parts[1]] = raw.get(parts[1], 0.0) + pct
    lower = {w for w in raw if w.islower()}
    usenet = {}
    for w, pct in raw.items():
        if not WORD.fullmatch(w):
            continue
        lw = w.lower()
        if w != lw:
            if w == "I":
                pass
            elif w.isupper() or lw not in lower:
                continue  # COM, DAILY; John, Berkeley
        if lw in USENET_JUNK or (len(lw) == 1 and lw not in ("a", "i")):
            continue
        usenet[lw] = usenet.get(lw, 0.0) + pct * 10_000
    for w in set(zipf) | set(usenet):
        counts[w] = max(zipf.get(w, 0.0), usenet.get(w, 0.0))
    return counts


def frequency_list():
    freq = {}
    with open(FREQ, encoding="utf-8", errors="replace") as f:
        for line in f:
            word, _, n = line.rstrip("\n").rpartition(" ")
            try:
                freq[word] = int(n)
            except ValueError:
                continue
    common = set()
    if os.path.isfile(COMMON_EN):
        with open(COMMON_EN, encoding="utf-8") as f:
            common = {l.strip() for l in f if l.strip() and not l.startswith("#")}
    return freq, common


def nouns():
    found = set()
    for line in lines(POS):
        word, sep, pos = line.partition("\\")
        if not sep:
            continue
        if "N" in pos and NOUN.fullmatch(word):
            found.add(word)
    return found


def main():
    seeds = seed_words()
    freq, common = frequency_list()
    all_nouns = nouns()

    known = [w for w in all_nouns if freq.get(w, 0) >= MIN_NOUN_FREQ or w in common]
    known.sort(key=lambda w: (-freq.get(w, 0), w))
    kept_nouns = known[:NOUN_CAP]

    freq_top = set()
    for line in list(lines(os.path.join(FILES, "freq.txt")))[1:]:
        if line.strip():
            freq_top.add(line.strip().lower())
    # common.txt (74k common words) tells ordinary words (tempo, cat) from abbreviations (atm, gps),
    # which mobypos.txt and single.txt also list in lowercase.
    plain = {l.strip() for l in lines(os.path.join(FILES, "common.txt")) if l.strip().islower()}
    names = set()
    for name in ("names.txt", "names-f.txt", "names-m.txt", "places.txt"):
        names.update(l.strip().upper() for l in lines(os.path.join(FILES, name)) if l.strip())
    acronyms, dropped = [], []
    seen = set()
    for line in lines(os.path.join(FILES, "acronyms.txt")):
        a = line.strip()
        if not ACRONYM.fullmatch(a) or a in seen:
            continue
        seen.add(a)
        la = a.lower()
        if not (freq.get(la, 0) >= MIN_ACRONYM_FREQ or la in common):
            continue
        ordinary = la in plain or (la.endswith("s") and la[:-1] in plain) or la in freq_top
        if (ordinary or a in names or (len(a) >= 4 and len(set(a)) == 1)) and a not in KEEP_ACRONYMS:
            dropped.append(a)
            continue
        acronyms.append(a)

    rows = {}
    for w, n in seeds.items():
        rows[w] = max(1, round(n))
    for w in kept_nouns:
        rows.setdefault(w, 1)
    for a in acronyms:
        rows.setdefault(a, 1)
    ordered = sorted(rows.items(), key=lambda kv: (-kv[1], kv[0]))

    os.makedirs(OUT, exist_ok=True)
    with open(os.path.join(OUT, "en_words.tsv"), "w", encoding="utf-8", newline="\n") as f:
        for w, n in ordered:
            f.write(f"{w}\t{n}\n")
    meta = {
        "source": "moby",
        "tier": "moderate",
        "weight": 0.5,
        "uses": ["acronyms", "english-nouns", "english-lexicon"],
        "license": "Public domain (Grady Ward's Moby Word Lists and Moby Part-of-Speech, Project Gutenberg #3201/#3203)",
        "files": {"en_words.tsv": len(ordered)},
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(f"seeds {len(seeds)}, nouns {len(kept_nouns)} of {len(all_nouns)} (known {len(known)}), "
          f"acronyms {len(acronyms)} (dropped {len(dropped)}: {' '.join(dropped[:200])})")
    print(f"en_words.tsv: {len(ordered)} lines -> {OUT}")


if __name__ == "__main__":
    main()
