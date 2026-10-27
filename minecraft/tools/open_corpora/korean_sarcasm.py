"""Extracts Kocasm (corpora/open/korean-sarcasm, github.com/SpellOnYou/korean-sarcasm, MIT) into
corpora/derived/korean-sarcasm/ for build_ranker.py. Only cleaned sentences and derived counts
are used.

Kocasm is ~9.3k Korean tweets labeled for sarcasm. Only src/data/jiwon/train.csv and test.csv
are read (columns: index, tokens, label), with the csv module; the label is ignored. data/ghosh
(English sarcasm tweets) and the model code are skipped. The tweets are casual: slang, typos,
ㅋㅋ, emoticons, and the occasional Latin word or brand (SK텔레콤, 한미FTA는, SNS를).

Cleaning, per tweet:
- Dropped: rows starting with "RT ", rows containing #bot or "[ ★", and ad/giveaway rows
  (이벤트, 추첨, 당첨, 경품, 응모, 상품권, 쿠폰, 리트윗, 팔로우/팔로잉 하시...). A reply that quotes
  a retweet ("my comment RT @user : their tweet") keeps only the comment before "RT @user".
  Foursquare check-ins ("I'm at ...") are dropped.
  Pharmacy/casino spam, job ads and traffic-report bots (비아그라, 카지노, 직종 :, 【) are dropped.
- Stripped: @user mentions, URL placeholders and their leftover query strings
  ("URL ? id = 123"), pic.twitter.com links (also split as "pic .twitter.com"), other URLs
  (":/ / host", "www.x.com"), hashtags (also split as "# 씀"), "…", the U+3164 filler.
- The tokenizer split punctuation off; it is rejoined: no space before . , ? ! ~ and ), none
  after (, so "? ?" becomes "??" and ". ." becomes "..". "09 : 00" becomes "09:00".
- Kept if it has at least 2 Hangul syllables and Hangul is at least 30% of its letters
  (drops English tweets with a stray Korean word); exact duplicates removed.

Outputs:
- ko.txt: the cleaned tweets, one per line, in source order.
- ko_words.tsv: eojeol counts over ko.txt. Edge punctuation and trailing jamo runs (ㅋㅋ, ㅠㅠ)
  are stripped; a word must then be Hangul syllables, Latin letters and digits only, with at
  least one syllable. Jamo-only laughter is not a word here.
- mixed.txt: distinct words from ko_words.tsv that have both Latin letters and Hangul
  syllables (SK텔레콤, FTA를, D램). Words containing "RT" (RT로, RT이벤트, RT해주세요) are left
  out: they are a Twitter artifact and the list has no counts to downweight them with.

Usage: python3 -I korean_sarcasm.py
"""

import collections
import csv
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "korean-sarcasm", "src", "data", "jiwon")
OUT = os.path.join(ROOT, "corpora", "derived", "korean-sarcasm")

AD_MARKERS = ("#bot", "[ ★", "이벤트", "추첨", "당첨", "경품", "응모", "상품권", "쿠폰", "리트윗",
              "팔로우 하시", "팔로잉 하시", "팔로우한 뒤", "팔로한후", "트친 선물",
              # pharmacy/casino spam, job ads, traffic-report bots
              "비아그라", "시알리스", "씨알리스", "카지노", "정품취급", "직종 :", "【")
STRIP = [
    # quoted retweet ("RT @user :", "RT \" @user :", "ㅎㅎRT :"): keep only the comment before it
    (re.compile(r"\s*(?<![A-Za-z])RT\s*\.?\s*\"?\s*(?:@user|:).*$"), ""),
    (re.compile(r"\[ pic \]\s*—?"), " "),
    (re.compile(r":/ /\s*$"), " "),
    (re.compile(r"\S*?(?:pic\s*)?\.\s*twitter\s*\.\s*com\s*/\s*\S*"), " "),
    (re.compile(r":/ /\s*\S+"), " "),
    (re.compile(r"\S+\.(?:com|net|org|kr|be|ly)(?:/\S*)?\b"), " "),
    (re.compile(r"\bURL(?:\s*\?\s*[\w.]+\s*=\s*[\w.\-]+(?:\s*&\s*[\w.]+\s*=\s*[\w.\-]+)*)?"), " "),
    (re.compile(r"@use(?:r)?\b"), " "),
    (re.compile(r"#\s?\S*"), " "),
    (re.compile(r"[\u3164\u0300-\u036f\u200b-\u200f\ufeff]"), " "),
    (re.compile(r"…"), " "),
]
JOIN = [
    (re.compile(r"\s+([.,?!~)])"), r"\1"),
    (re.compile(r"\(\s+"), "("),
    (re.compile(r"(\d) : (\d)"), r"\1:\2"),
    (re.compile(r"\(\)"), " "),
]
SYLLABLE = re.compile(r"[가-힣]")
LETTER = re.compile(r"[가-힣ㄱ-ㅣA-Za-z]")
EDGE = re.compile(r"^[^가-힣A-Za-z0-9]+|[^가-힣A-Za-z0-9]+$")
TRAILING_JAMO = re.compile(r"[ㄱ-ㅣ]+$")
WORD = re.compile(r"[가-힣A-Za-z0-9]*[가-힣][가-힣A-Za-z0-9]*")
LATIN = re.compile(r"[A-Za-z]")


def clean(text):
    text = " ".join(text.split())
    if text.startswith("RT ") or text.startswith("I'm at ") or any(m in text for m in AD_MARKERS):
        return None
    for pat, rep in STRIP:
        text = pat.sub(rep, text)
    text = " ".join(text.split())
    for pat, rep in JOIN:
        text = pat.sub(rep, text)
    text = " ".join(text.split()).strip(" :")
    hangul = len(SYLLABLE.findall(text))
    if hangul < 2 or hangul < 0.3 * len(LETTER.findall(text)):
        return None
    return text


def words_of(line):
    for tok in line.split():
        tok = TRAILING_JAMO.sub("", EDGE.sub("", tok))
        tok = EDGE.sub("", tok)
        if WORD.fullmatch(tok):
            yield tok


def write_lines(name, lines):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    return len(lines)


def main():
    raw = 0
    seen = set()
    lines = []
    for split in ("train.csv", "test.csv"):
        with open(os.path.join(SRC, split), encoding="utf-8", newline="") as f:
            for row in csv.DictReader(f):
                raw += 1
                text = clean(row.get("tokens") or "")
                if text and text not in seen:
                    seen.add(text)
                    lines.append(text)

    words = collections.Counter()
    for line in lines:
        words.update(words_of(line))
    mixed = sorted(w for w in words if LATIN.search(w) and "RT" not in w)

    os.makedirs(OUT, exist_ok=True)
    files = {"ko.txt": write_lines("ko.txt", lines)}
    ordered = sorted(words.items(), key=lambda kv: (-kv[1], kv[0]))
    files["ko_words.tsv"] = write_lines("ko_words.tsv", [f"{w}\t{c}" for w, c in ordered])
    files["mixed.txt"] = write_lines("mixed.txt", mixed)
    meta = {
        "source": "korean-sarcasm",
        "tier": "moderate",
        "weight": 0.5,
        "uses": ["korean-key-model", "korean-lexicon", "mixed-ko-en", "particles-endings", "typos"],
        "license": "MIT",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(f"{raw} rows, {len(lines)} kept; " + ", ".join(f"{k}: {v}" for k, v in files.items()))


main()
