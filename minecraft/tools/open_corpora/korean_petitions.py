"""Extracts korean-petitions (corpora/open/korean-petitions, huggingface.co/datasets/heegyu/korean-petitions,
from github.com/lovit/petitions_archive, MIT) into corpora/derived/korean-petitions/ for
build_ranker.py. Only derived counts and sampled sentences are used.

The source is ~437k 청와대 국민청원 petitions (2017-08 to 2019-08), one JSON object per line in
data/petitions_YYYY-MM.jsonl, with "title" and "content" (single-line, no embedded newlines) plus
metadata that is ignored. Long-form civic writing: politically skewed, hence weight 0.7.

Records are filtered before anything is counted:
- campaign floods are capped: at most TITLE_CAP petitions per title (compared without spaces, so
  "이명박 출국금지" and "이명박출국금지" are one title);
- content is deduplicated on a hash of its Hangul/Latin/digit characters only (whitespace and
  punctuation ignored), and content with under MIN_HANGUL Hangul syllables is dropped.

Outputs:
- ko.txt: titles and content sentences. URLs and e-mail addresses are removed, content is split
  after . ! ? (and 。…), separator rules (----, ㅡㅡㅡㅡ) are removed, runs of 4+ identical
  punctuation marks are cut to 3 and whitespace is normalized. A sentence is kept if it has
  4..300 characters, at least 4 Hangul syllables, Hangul making up at least 60% of its letters,
  no run of 7+ digits (phone, account and case numbers) and no markup/code characters
  ({ } < > \\ | ^ ` $ =).
  Exact duplicates are removed; if more than MAX_LINES remain, a sorted random.Random(20261027)
  sample of them is kept in source order.
- ko_words.tsv: eojeol (space-separated token) counts over all kept sentences before sampling.
  Surrounding punctuation is stripped; a token must be Hangul/Latin/digits only, contain Hangul
  and be at most 20 characters. Tokens seen fewer than MIN_WORD_COUNT times are left out, so
  one-off typos don't enter the lexicon.
- mixed.txt: distinct tokens mixing Latin letters and Hangul (CCTV를, SNS로, KEB하나은행, PC방),
  from the same tokens, seen at least MIN_WORD_COUNT times. Case variants are kept separately
  (cctv를 is what people type), ordered by count.

Usage: python3 -I korean_petitions.py
"""

import collections
import glob
import hashlib
import json
import os
import random
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "korean-petitions", "data")
OUT = os.path.join(ROOT, "corpora", "derived", "korean-petitions")

TITLE_CAP = 3
MIN_HANGUL = 10
MAX_LINES = 3_000_000
MIN_WORD_COUNT = 3
SEED = 20261027

URL = re.compile(r"https?:/+\S*|www\.\S+|\S+@\S+\.\w+|\S+\.(?:com|net|kr|org|html?|php|aspx?)(?:/\S*)?", re.I)
SPLIT = re.compile(r"(?<=[.!?。…])\s+")
HANGUL = re.compile(r"[가-힣]")
LATIN = re.compile(r"[A-Za-z]")
LONG_DIGITS = re.compile(r"\d[\d\-]{6,}")
RULE = re.compile(r"[-ㅡ_*~=#]{4,}")
REPEAT = re.compile(r"([^0-9A-Za-z가-힣\s])\1{3,}")
CODE = re.compile(r"[{}<>\\|^`$=]")
KEYCHARS = re.compile(r"[^0-9A-Za-z가-힣]+")
TOKEN = re.compile(r"[0-9A-Za-z가-힣]+")
STRIP = "\"'“”‘’()[]{}<>「」『』《》〈〉.,!?;:~·…-_*#&/"


def clean_sentences(text):
    text = REPEAT.sub(r"\1\1\1", RULE.sub(" ", URL.sub(" ", text)))
    text = " ".join(text.split())
    for s in SPLIT.split(text):
        s = s.strip()
        if not 4 <= len(s) <= 300:
            continue
        h = len(HANGUL.findall(s))
        if h < 4 or h < 0.6 * (h + len(LATIN.findall(s))):
            continue
        if LONG_DIGITS.search(s) or CODE.search(s):
            continue
        yield s


def write_lines(name, lines):
    n = 0
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
            n += 1
    return n


def main():
    titles = collections.Counter()
    content_seen = set()
    sent_seen = set()
    sentences = []
    words = collections.Counter()
    records = kept = 0

    def add(s):
        key = hashlib.blake2b(s.encode("utf-8"), digest_size=8).digest()
        if key in sent_seen:
            return
        sent_seen.add(key)
        sentences.append(s)
        for raw in s.split():
            tok = raw.strip(STRIP)
            if tok and len(tok) <= 20 and TOKEN.fullmatch(tok) and HANGUL.search(tok):
                words[tok] += 1

    for path in sorted(glob.glob(os.path.join(SRC, "petitions_*.jsonl"))):
        with open(path, encoding="utf-8") as f:
            for line in f:
                try:
                    rec = json.loads(line)
                except ValueError:
                    continue
                if not isinstance(rec, dict):
                    continue
                records += 1
                title = rec.get("title") if isinstance(rec.get("title"), str) else ""
                content = rec.get("content") if isinstance(rec.get("content"), str) else ""
                tkey = "".join(title.split())
                titles[tkey] += 1
                if tkey and titles[tkey] > TITLE_CAP:
                    continue
                if len(HANGUL.findall(content)) < MIN_HANGUL:
                    continue
                ckey = hashlib.blake2b(KEYCHARS.sub("", content).lower().encode("utf-8"),
                                       digest_size=8).digest()
                if ckey in content_seen:
                    continue
                content_seen.add(ckey)
                kept += 1
                for s in clean_sentences(title):
                    add(s)
                for s in clean_sentences(content):
                    add(s)
    del sent_seen, content_seen

    total = len(sentences)
    if total > MAX_LINES:
        pick = sorted(random.Random(SEED).sample(range(total), MAX_LINES))
        sentences = [sentences[i] for i in pick]

    common = sorted(((w, c) for w, c in words.items() if c >= MIN_WORD_COUNT),
                    key=lambda kv: (-kv[1], kv[0]))
    del words
    mixed = [w for w, _ in common if LATIN.search(w)]

    os.makedirs(OUT, exist_ok=True)
    files = {
        "ko.txt": write_lines("ko.txt", sentences),
        "ko_words.tsv": write_lines("ko_words.tsv", (f"{w}\t{c}" for w, c in common[:MAX_LINES])),
        "mixed.txt": write_lines("mixed.txt", mixed),
    }
    meta = {
        "source": "korean-petitions",
        "tier": "moderate",
        "weight": 0.7,
        "uses": ["korean-key-model", "korean-lexicon", "particles-endings", "mixed-ko-en",
                 "acronyms", "typos"],
        "license": "MIT",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(f"{records} records, {kept} kept, {total} unique sentences; "
          + ", ".join(f"{k}: {v}" for k, v in files.items()))


main()
