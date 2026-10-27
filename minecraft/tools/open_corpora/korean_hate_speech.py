"""Extracts BEEP! (corpora/open/korean-hate-speech, github.com/kocohub/korean-hate-speech,
CC BY-SA 4.0) into corpora/derived/korean-hate-speech/ for build_ranker.py. Only cleaned
sentences and derived token lists are used; the hate/bias labels are ignored.

BEEP! is ~2M Naver entertainment-news comments (src/unlabeled/unlabeled_comments_{1..5}.txt,
one comment per line) plus ~9.3k labeled ones (src/labeled/{train,dev}.tsv and
src/test.no_label.tsv, column "comments"), and the news title each comment was posted under
(src/news_title/). Very colloquial: slang, typos, ㅋㅋ, missing spaces, and Latin names and
acronyms with particles (BTS도, YG랑, PD가). It is also full of profanity and slurs, so it feeds
only the key-sequence (key-trigram) model, never the suggestion vocabulary: no ko_words.tsv is
written, and mixed.txt is filtered through a blocklist.

Cleaning, per comment:
- Stripped: URLs (http..., www..., bare host/path), e-mail addresses, HTML entities, emoji and other symbols
  (Unicode category So/Sk/Co/Cs/Mn, variation selectors, zero-width characters), ^^ / ^_^ / ♡ / ♥.
- Squeezed: runs of 3+ of the same jamo (ㅋㅋㅋㅋ, ㅠㅠㅠ, ㅎㅎㅎ) and of ! ? ~ , to 2; runs of 4+
  dots to "...".
- Dropped: lines with fewer than 4 Hangul syllables; lines where Hangul syllables, Latin letters
  and digits are under half of the non-space characters (mostly ㅋ/ㅠ/ㆍ/punctuation); lines
  where Hangul syllables are under 30% of letters (mostly English); char-spaced spam, i.e. 3+
  tokens of which at least 60% are a single character (빛 효 신, 지 랄 하 네, 재 ㅜ 현 ㅜ, N E X T).
- Exact duplicates removed (the unlabeled files repeat comments).

News titles: every src/news_title/*.txt line is a CSV-quoted headline repeated once per comment;
they are deduplicated (~20k), unquoted ("" -> "), stripped of bracketed desk tags ([단독],
[종합], [공식입장 전문], (종합)) and ♥, then cleaned like comments and appended after the
comments in ko.txt (news register).

Outputs:
- ko.txt: the cleaned comments, then the cleaned titles, one per line, source order.
- mixed.txt: distinct words with both Latin letters and Hangul syllables (BTS도, YG랑, sm과,
  KBS연기대상), edge punctuation and trailing jamo stripped, seen at least twice, none matching
  the profanity/slur blocklist. Acronym+particle forms ([A-Za-z]{2,}[가-힣]{1,2}) are the bulk.

Usage: python3 -I korean_hate_speech.py
"""

import collections
import json
import os
import re
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "korean-hate-speech", "src")
OUT = os.path.join(ROOT, "corpora", "derived", "korean-hate-speech")

URL = re.compile(r"(?:https?://|www\.)\S*|[\w.+-]*@?[A-Za-z0-9.-]+\.(?:com|net|org|kr|co|ly|me|tv)"
                 r"(?![A-Za-z])(?:/\S*)?", re.I)
ENTITY = re.compile(r"&(?:[a-z]+|#\d+);", re.I)
SMILEY = re.compile(r"\^[_\-.;]*\^+|[♡♥❤]")
JAMO_RUN = re.compile(r"([ㄱ-ㅣ])\1{2,}")
PUNCT_RUN = re.compile(r"([!?~,])\1{2,}")
DOTS = re.compile(r"\.{4,}")
SYLLABLE = re.compile(r"[가-힣]")
LATIN = re.compile(r"[A-Za-z]")
LETTER = re.compile(r"[가-힣ㄱ-ㅣA-Za-z]")
CONTENT = re.compile(r"[가-힣A-Za-z0-9]")
TITLE_TAG = re.compile(r"\[[^\]]{1,20}\]|\((?:종합|단독|인터뷰|공식|포토|영상|화보)[^)]{0,10}\)")
EDGE = re.compile(r"^[^가-힣A-Za-z0-9]+|[^가-힣A-Za-z0-9]+$")
TRAILING_JAMO = re.compile(r"[ㄱ-ㅣ]+$")
WORD = re.compile(r"[가-힣A-Za-z0-9]*[가-힣][가-힣A-Za-z0-9]*")
# Profanity and slurs kept out of mixed.txt (substring match on the word).
BLOCK = ("년", "새끼", "새x", "새ㄲ", "병x", "개x", "ooo", "새기", "섀끼", "색기", "쌔끼", "지랄", "시발", "씨발", "씨빨", "십알", "ㅅㅂ",
         "병신", "븅신", "빙신", "좆", "졷", "존나", "졸라", "개같", "개새", "개돼지", "충", "놈",
         "창녀", "걸레", "보지", "자지", "꼴페", "한남", "김치녀", "된장녀", "메갈", "일베", "닥쳐",
         "꺼져", "뒤져", "뒈져", "미친", "또라이", "똘아이", "등신", "쓰레기", "fuck", "shit",
         "bitch", "sibal", "ssibal", "tlqkf", "qudtls", "wlfkf")


def drop_symbols(text):
    out = []
    for ch in text:
        cat = unicodedata.category(ch)
        if cat in ("So", "Sk", "Co", "Cs", "Cf", "Mn") or 0xFE00 <= ord(ch) <= 0xFE0F or ch == "ㅤ":
            out.append(" ")
        else:
            out.append(ch)
    return "".join(out)


def clean(text):
    text = URL.sub(" ", text)
    text = ENTITY.sub(" ", text)
    text = SMILEY.sub(" ", text)
    text = drop_symbols(text)
    text = JAMO_RUN.sub(r"\1\1", text)
    text = PUNCT_RUN.sub(r"\1\1", text)
    text = DOTS.sub("...", text)
    text = " ".join(text.split())
    syllables = len(SYLLABLE.findall(text))
    if syllables < 4:
        return None
    nonspace = len(text) - text.count(" ")
    if len(CONTENT.findall(text)) * 2 < nonspace:
        return None
    if syllables < 0.3 * len(LETTER.findall(text)):
        return None
    tokens = text.split()
    if len(tokens) >= 3:
        single = sum(1 for t in tokens if len(t) == 1)
        if single >= 0.6 * len(tokens):
            return None
    return text


def clean_title(line):
    line = line.strip()
    if len(line) >= 2 and line[0] == '"' and line[-1] == '"':
        line = line[1:-1].replace('""', '"')
    line = TITLE_TAG.sub(" ", line)
    return clean(line)


def comments():
    for split in ("labeled/train.tsv", "labeled/dev.tsv", "test.no_label.tsv"):
        with open(os.path.join(SRC, split), encoding="utf-8", errors="replace") as f:
            next(f, None)  # header
            for line in f:
                yield line.rstrip("\n").split("\t")[0]
    for i in range(1, 6):
        with open(os.path.join(SRC, "unlabeled", f"unlabeled_comments_{i}.txt"),
                  encoding="utf-8", errors="replace") as f:
            for line in f:
                yield line.rstrip("\n")


def titles():
    folder = os.path.join(SRC, "news_title")
    for name in sorted(os.listdir(folder)):
        if name.endswith(".txt"):
            with open(os.path.join(folder, name), encoding="utf-8", errors="replace") as f:
                for line in f:
                    yield line.rstrip("\n")


def words_of(line):
    for tok in line.split():
        tok = EDGE.sub("", TRAILING_JAMO.sub("", EDGE.sub("", tok)))
        if WORD.fullmatch(tok):
            yield tok


def write_lines(name, lines):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    return len(lines)


def main():
    seen = set()
    lines = []
    raw = 0
    for text in comments():
        raw += 1
        text = clean(text)
        if text and text not in seen:
            seen.add(text)
            lines.append(text)
    n_comments = len(lines)
    raw_titles = set()
    for title in titles():
        if title not in raw_titles:
            raw_titles.add(title)
            text = clean_title(title)
            if text and text not in seen:
                seen.add(text)
                lines.append(text)
    del seen

    mixed = collections.Counter()
    for line in lines:
        for w in words_of(line):
            if LATIN.search(w):
                mixed[w] += 1
    mixed_list = sorted(w for w, c in mixed.items()
                        if c >= 2 and not any(b in w.lower() for b in BLOCK))

    os.makedirs(OUT, exist_ok=True)
    files = {"ko.txt": write_lines("ko.txt", lines),
             "mixed.txt": write_lines("mixed.txt", mixed_list)}
    meta = {
        "source": "korean-hate-speech",
        "tier": "moderate",
        "weight": 0.5,
        "uses": ["korean-key-model", "mixed-ko-en", "acronyms", "particles-endings", "typos"],
        "license": "CC BY-SA 4.0",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(f"{raw} comments, {n_comments} kept; {len(raw_titles)} titles, "
          f"{len(lines) - n_comments} kept; " + ", ".join(f"{k}: {v}" for k, v in files.items()))


main()
