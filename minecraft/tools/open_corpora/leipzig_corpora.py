"""Extracts the Leipzig Corpora Collection downloads (corpora/open/leipzig-corpora,
downloads.wortschatz-leipzig.de/corpora/, CC BY; cite Goldhahn, Eckart & Quasthoff, LREC 2012)
into corpora/derived/leipzig-corpora/ for build_ranker.py. Only derived counts and sentences are
used.

Each corpus is <name>/<name>/<name>-sentences.txt (id<TAB>sentence, 1M sentences, sorted
alphabetically) and <name>-words.txt (id<TAB>token<TAB>count). Read: seven Korean sets
(kor_news_2019/2020/2022, kor_newscrawl_2016, kor_wikipedia_2021, kor-kr_web_2015/2020) and two
English ones (eng_news_2024, eng_wikipedia_2016). Co-occurrence, source and SQL files are not read.

Cleaning (sentences): NFC, whitespace normalized; parentheses holding another script (hanja,
kana, Cyrillic: 서울(漢陽)) are removed and any sentence still holding such characters is
dropped; dropped too: a URL or e-mail address, over MAX_LEN or under MIN_LEN characters, a first
character that is not a letter, digit or an opening quote/bracket closed later in the line (the
alphabetical sort puts fragments like "`생의 찬미’ 전은" first), more than 30% symbols. Korean
sentences need at least as much Hangul as Latin, and lose web gambling SEO spam (site names glued
to words: 레전드카지노, 바카라사이트); English ones lose Wikipedia's leftover "sfn" citations. Exact dedupe across all sets.

- ko.txt: Korean sentences (Latin words, acronyms and digits kept), at most ~3M lines, sampled by
  a hash of the sentence at a per-set rate: web 2015/2020 and news 2022 weigh 1.5, the other news
  sets 1, Wikipedia 0.5 (encyclopedic register, tokenisation noise).
- en.txt: English sentences from eng_news_2024 and eng_wikipedia_2016 (all that pass cleaning).
- ko_words.tsv: all-Hangul tokens with their counts summed over the Korean word lists, count >= 3,
  most frequent first.
- en_words.tsv: alphabetic tokens from eng_news_2024 plus half of eng_wikipedia_2016's counts,
  count >= 5, most frequent first.
- mixed.txt: distinct tokens mixing Latin letters and Hangul (A씨는, SNS에, MZ세대, KB국민카드)
  seen at least MIN_MIXED times in the Korean word lists, sorted.
- No particles.txt (Leipzig has no morphology) and no terms.tsv (no bilingual pairs); acronym +
  particle statistics come from splitting mixed.txt into its Latin head and Hangul tail.

Usage: python3 -I leipzig_corpora.py
"""

import collections
import hashlib
import json
import os
import re
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "leipzig-corpora")
OUT = os.path.join(ROOT, "corpora", "derived", "leipzig-corpora")

KO_SETS = {
    "kor-kr_web_2015_1M": 1.5,
    "kor-kr_web_2020_1M": 1.5,
    "kor_news_2022_1M": 1.5,
    "kor_news_2019_1M": 1.0,
    "kor_news_2020_1M": 1.0,
    "kor_newscrawl_2016_1M": 1.0,
    "kor_wikipedia_2021_1M": 0.5,
}
EN_WORD_SETS = {"eng_news_2024_1M": 1.0, "eng_wikipedia_2016_1M": 0.5}
EN_SETS = ["eng_news_2024_1M", "eng_wikipedia_2016_1M"]
# Keep rate per unit of weight: ~6.8M clean Korean lines, 7.7M weighted, x 0.37 ~ 2.85M.
BASE_RATE = 0.37
CAP = 3_000_000
MAX_LEN = 300
MIN_LEN = 8
MIN_KO_WORD = 3
MIN_EN_WORD = 5
MIN_MIXED = 2

FOREIGN = "Ͱ-ϿЀ-ӿ぀-ヿ㐀-䶿一-鿿豈-﫿"
FOREIGN_CHAR = re.compile(f"[{FOREIGN}]")
FOREIGN_PAREN = re.compile(rf"\s*\([^()]*[{FOREIGN}][^()]*\)")
URL = re.compile(r"https?://|www\.|\.(com|net|org|kr|co)\b|@[A-Za-z0-9_-]+(\.|\b)", re.I)
HANGUL = re.compile(r"[가-힣ㄱ-ㅣ]")
LATIN = re.compile(r"[A-Za-z]")
# Characters that are not symbols: letters, digits, whitespace, ordinary punctuation.
PLAIN = re.compile(r"[가-힣ㄱ-ㅣA-Za-z0-9\s.,?!'\"“”‘’()\-·…:;%~/]")
OPENERS = {'"': '"', "'": "'", "“": "”", "‘": "’", "(": ")", "[": "]", "「": "」", "『": "』",
           "〈": "〉", "《": "》", "<": ">"}
MIXED = re.compile(r"(?=.*[A-Za-z])(?=.*[가-힣])[A-Za-z0-9가-힣&+.\-]+")
# Web-set gambling SEO spam glues site names onto sentences (레전드카지노, 생중계바카라싸이트).
SPAM = re.compile(r"[가-힣A-Za-z]{2,}(카지노|바카라)|(카지노|바카라|토토)\s?(사이트|싸이트|주소|바로가기)")
# Wikipedia citation templates left as a glued "sfn" (Islam.sfn Emperor).
WIKI_JUNK = re.compile(r"[a-z.)]sfn\b|\bsfn\b")
ALL_HANGUL = re.compile(r"[가-힣]+")
ALPHA = re.compile(r"[A-Za-z]+(?:'[A-Za-z]+)?")


def files(name, kind):
    return os.path.join(SRC, name, name, f"{name}-{kind}.txt")


def column(path, col):
    """Yields column col of a tab-separated Leipzig file, streamed."""
    with open(path, encoding="utf-8", errors="replace", newline="\n") as f:
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if len(parts) > col:
                yield parts if col < 0 else parts[col]


def clean(s, korean):
    s = unicodedata.normalize("NFC", s)
    s = " ".join(s.split())
    s = FOREIGN_PAREN.sub("", s)
    if not (MIN_LEN <= len(s) <= MAX_LEN) or FOREIGN_CHAR.search(s) or URL.search(s):
        return None
    first = s[0]
    if not first.isalnum():
        closer = OPENERS.get(first)
        if closer is None or closer not in s[1:]:
            return None
    if len(PLAIN.findall(s)) < 0.7 * len(s):
        return None
    hangul = len(HANGUL.findall(s))
    latin = len(LATIN.findall(s))
    if korean:
        if SPAM.search(s) or hangul == 0 or hangul < latin:
            return None
    elif hangul or WIKI_JUNK.search(s) or latin < 0.5 * len(s):
        return None
    return s


def key(s):
    return int.from_bytes(hashlib.blake2b(s.encode("utf-8"), digest_size=8).digest(), "big")


def korean_sentences():
    seen = set()
    kept = 0
    stats = {}
    with open(os.path.join(OUT, "ko.txt"), "w", encoding="utf-8", newline="\n") as out:
        for name, weight in KO_SETS.items():
            threshold = min(1.0, weight * BASE_RATE) * 2**64
            total = clean_n = n = 0
            for raw in column(files(name, "sentences"), 1):
                total += 1
                s = clean(raw, True)
                if s is None:
                    continue
                k = key(s)
                if k in seen:
                    continue
                seen.add(k)
                clean_n += 1
                # Sampling by a second hash keeps the choice independent of the dedupe key's bits.
                if key("#" + s) < threshold and kept < CAP:
                    out.write(s + "\n")
                    kept += 1
                    n += 1
            stats[name] = (total, clean_n, n)
            print(f"  {name}: {total} read, {clean_n} clean+new, {n} kept")
    return kept


def english_sentences():
    seen = set()
    n = 0
    with open(os.path.join(OUT, "en.txt"), "w", encoding="utf-8", newline="\n") as out:
        for name in EN_SETS:
            for raw in column(files(name, "sentences"), 1):
                s = clean(raw, False)
                if s is None:
                    continue
                k = key(s)
                if k in seen:
                    continue
                seen.add(k)
                if n < CAP:
                    out.write(s + "\n")
                    n += 1
    return n


def word_counts(sets, accept):
    counts = collections.Counter()
    for name, weight in sets.items():
        for parts in column(files(name, "words"), -1):
            if len(parts) < 3:
                continue
            tok = unicodedata.normalize("NFC", parts[1].strip())
            try:
                c = int(parts[2])
            except ValueError:
                continue
            if tok and accept(tok):
                counts[tok] += int(c * weight)
    return counts


def write_tsv(name, counts, minimum):
    rows = [(w, c) for w, c in counts.items() if c >= minimum]
    rows.sort(key=lambda wc: (-wc[1], wc[0]))
    rows = rows[:CAP]
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for w, c in rows:
            f.write(f"{w}\t{c}\n")
    return len(rows)


def main():
    os.makedirs(OUT, exist_ok=True)
    lines = {}
    print("Korean sentences")
    lines["ko.txt"] = korean_sentences()
    print("English sentences")
    lines["en.txt"] = english_sentences()
    print("word lists")
    ones = {name: 1.0 for name in KO_SETS}
    ko = word_counts(ones, lambda t: ALL_HANGUL.fullmatch(t) is not None)
    lines["ko_words.tsv"] = write_tsv("ko_words.tsv", ko, MIN_KO_WORD)
    del ko
    en = word_counts(EN_WORD_SETS, lambda t: ALPHA.fullmatch(t) is not None)
    lines["en_words.tsv"] = write_tsv("en_words.tsv", en, MIN_EN_WORD)
    del en
    mixed = word_counts(ones, lambda t: MIXED.fullmatch(t) is not None
                        and t[0] not in ".-" and t[-1] not in ".-&+")
    rows = sorted(w for w, c in mixed.items() if c >= MIN_MIXED)[:CAP]
    with open(os.path.join(OUT, "mixed.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.writelines(w + "\n" for w in rows)
    lines["mixed.txt"] = len(rows)
    meta = {
        "source": "leipzig-corpora",
        "tier": "high",
        "weight": 2,
        "uses": ["korean-key-model", "english-key-model", "korean-lexicon", "english-lexicon",
                 "mixed-ko-en", "acronyms", "particles-endings", "business-terms"],
        "license": "CC BY (Leipzig Corpora Collection downloads; cite Goldhahn, Eckart & "
                   "Quasthoff, LREC 2012)",
        "files": lines,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    for name, n in lines.items():
        print(f"{name}: {n}")


main()
