"""Extracts a loanword lexicon, English words, acronyms and IT/business terms from
muik/transliteration into corpora/derived/muik-transliteration/ (gitignored) for training the
predictor (docs/spec.md §10; minecraft/tools/build_ranker.py).

Source: https://github.com/muik/transliteration (corpora/open/muik-transliteration/src),
Apache-2.0. English word -> Hangul transliteration pairs (외래어 표기), ~57k rows in
src/data/source/*.txt: korean-go.txt (국립국어원 외래어 표기 용례), doosan*.txt (Doosan
encyclopedia: world place names, Australia/NZ places, IT terms), wiktionary.txt, cities.txt, a
blog loanword dictionary (dictionary.txt), hand-entered pairs (self.txt) and user suggestions
(suggests.txt). Caution: the pairs were scraped from third-party sites whose own terms are
unclear. src/data/giga-fren.*, newstest2013.*, *.ids* and vocab* are training splits or numeric
ids and are ignored; no code from the repo is run.

Reading: lines starting with '#' are dropped and only rows of exactly two tab-separated fields
are kept; rows are deduplicated per file. The source has no frequencies, so counts are a flat
prior: COUNT_STRONG for words seen in the curated common-vocabulary files (doosan-it,
wiktionary, dictionary, self, suggests), COUNT_WEAK when seen only in the name/place/science
heavy ones (korean-go, doosan, doosan-at, doosan-nz, cities).

What it writes:
- ko_words.tsv: pure-Hangul tokens of the Korean column (split on spaces), word<TAB>count. The
  spelled-out reading of an acronym (CPI 시피아이) is skipped.
- en_words.tsv: English column split into words (on spaces, hyphens, slashes). Lowercase words
  are kept; Capitalized words are kept lowercased when dwyl words_alpha.txt or
  engine/predict/common-en.txt knows them, otherwise dropped as proper names, except brand
  names from doosan-it (iPad, KakaoTalk), kept as written. Acronyms are kept in UPPER case:
  ^[A-Z]{2,6}$ entries from every file except the place-name lists (cities, doosan*, where
  KABUL is a city); 4+ letter ones that are ordinary English words (FERN 펀) are skipped unless
  the Hangul spells the letters out.
- terms.tsv: korean<TAB>english for every doosan-it row, plus korean-go rows whose English
  contains a tech/business keyword (TERM_KEYWORDS) and no Capitalized word (Andrew Card is a
  person, not a card).
- mixed.txt: Hangul column entries that also contain Latin letters or digits (rare).

Usage: python3 -I muik_transliteration.py
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
SRC = os.path.join(ROOT, "corpora", "open", "muik-transliteration", "src", "data", "source")
WORDLIST = os.path.join(ROOT, "corpora", "open", "dwyl-english-words", "src", "words_alpha.txt")
COMMON_EN = os.path.join(ROOT, "engine", "predict", "common-en.txt")
OUT = os.path.join(ROOT, "corpora", "derived", "muik-transliteration")

STRONG = ("doosan-it", "wiktionary", "dictionary", "self", "suggests")
WEAK = ("korean-go", "doosan", "doosan-at", "doosan-nz", "cities")
PLACES = ("cities", "doosan", "doosan-at", "doosan-nz")
COUNT_STRONG = 3
COUNT_WEAK = 1

TERM_KEYWORDS = re.compile(
    r"\b(?:computer|data|database|network|internet|online|software|hardware|system|program"
    r"|digital|server|memory|chip|semiconductor|display|mobile|platform|cloud|code|signal"
    r"|electronic|device|interface|algorithm|web|virtual|sensor|robot|battery|laser|marketing"
    r"|market|management|business|finance|financial|bank|banking|fund|stock|bond|credit|loan"
    r"|insurance|account|accounting|audit|tax|price|pricing|cost|sales|brand|franchise|retail"
    r"|trade|trading|investment|invest|capital|asset|equity|economy|economic|consulting"
    r"|outsourcing|logistics|supply|leasing|license|merger|holding|index|rate|card|payment"
    r"|cash|budget|portfolio|dividend|venture|startup|commerce|service|project)s?\b",
    re.I,
)

HANGUL_ONLY = re.compile(r"^[가-힣]+$")
HAS_HANGUL = re.compile(r"[가-힣]")
HAS_LATIN_DIGIT = re.compile(r"[A-Za-z0-9]")
ACRONYM = re.compile(r"^[A-Z]{2,6}$")
NAME_WORD = re.compile(r"\b[A-Z][a-z]")  # a Capitalized word: a person or place name
EN_SPLIT = re.compile(r"[\s\-/]+")
EN_WORD = re.compile(r"^[A-Za-z][A-Za-z']*[A-Za-z]$|^[A-Za-z]$")
# Hangul readings of the Latin letters, longest first so the spelled-out check is greedy.
LETTER_NAMES = sorted(
    "에이 비 씨 시 디 이 에프 지 쥐 에이치 아이 제이 케이 엘 엠 엔 오 피 큐 아르 알 에스 티 유 "
    "브이 더블유 더블류 엑스 와이 제트".split(),
    key=len,
    reverse=True,
)


def spelled_out(hangul):
    """True when the Hangul is a run of Latin letter names (에이티엘 = A T L)."""
    s = hangul.replace(" ", "")
    i = 0
    while i < len(s):
        for name in LETTER_NAMES:
            if s.startswith(name, i):
                i += len(name)
                break
        else:
            return False
    return bool(s)


def load_wordlist():
    words = set()
    for path in (WORDLIST, COMMON_EN):
        if not os.path.exists(path):
            continue
        with open(path, encoding="utf-8", errors="replace") as f:
            for line in f:
                w = line.split("\t", 1)[0].strip().lower()
                if w and not w.startswith("#"):
                    words.add(w)
    return words


def read_pairs(name):
    rows = set()
    with open(os.path.join(SRC, name + ".txt"), encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.rstrip("\r\n")
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) != 2:
                continue
            en, ko = parts[0].strip(), parts[1].strip()
            if en and ko:
                rows.add((en, ko))
    return sorted(rows)


def bump(counter, key, value):
    if counter.get(key, 0) < value:
        counter[key] = value


def main():
    wordlist = load_wordlist()
    ko_words, en_words, terms, mixed = {}, {}, {}, set()

    for name in STRONG + WEAK:
        weight = COUNT_STRONG if name in STRONG else COUNT_WEAK
        for en, ko in read_pairs(name):
            is_acronym = bool(ACRONYM.match(en))

            if not is_acronym or not spelled_out(ko):
                for tok in ko.split():
                    if HANGUL_ONLY.match(tok):
                        bump(ko_words, tok, weight)
            if HAS_HANGUL.search(ko) and HAS_LATIN_DIGIT.search(ko):
                mixed.add(" ".join(ko.split()))

            if is_acronym:
                if name in PLACES:
                    continue
                if len(en) >= 4 and en.lower() in wordlist and not spelled_out(ko):
                    continue
                bump(en_words, en, weight)
            else:
                for tok in EN_SPLIT.split(en):
                    tok = tok.strip(".,()'\"")
                    if not EN_WORD.match(tok):
                        continue
                    if tok.islower():
                        bump(en_words, tok, weight)
                    elif tok.lower() in wordlist:
                        bump(en_words, tok.lower(), weight)
                    elif name == "doosan-it" and not tok.isupper():
                        bump(en_words, tok, weight)

            if name == "doosan-it" or (
                name == "korean-go" and TERM_KEYWORDS.search(en) and not NAME_WORD.search(en)
            ):
                if HAS_HANGUL.search(ko):
                    terms.setdefault(" ".join(ko.split()), " ".join(en.split()))

    os.makedirs(OUT, exist_ok=True)
    files = {}

    def write_lines(fname, lines):
        with open(os.path.join(OUT, fname), "w", encoding="utf-8", newline="\n") as f:
            for line in lines:
                f.write(line + "\n")
        files[fname] = len(lines)

    def by_count(d):
        return [f"{w}\t{c}" for w, c in sorted(d.items(), key=lambda kv: (-kv[1], kv[0]))]

    write_lines("ko_words.tsv", by_count(ko_words))
    write_lines("en_words.tsv", by_count(en_words))
    write_lines("terms.tsv", [f"{k}\t{v}" for k, v in sorted(terms.items())])
    if mixed:
        write_lines("mixed.txt", sorted(mixed))

    meta = {
        "source": "muik-transliteration",
        "tier": "moderate",
        "weight": 0.5,
        "uses": ["korean-lexicon", "english-lexicon", "english-nouns", "acronyms",
                 "business-terms"],
        "license": "Apache-2.0 (repo); pairs scraped from 국립국어원, Doosan encyclopedia, "
                   "Wiktionary and a blog, whose own terms are unclear",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    for k, v in files.items():
        print(f"{k}: {v}")


if __name__ == "__main__":
    main()
