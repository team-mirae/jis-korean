"""Extracts APEACH + BEEP! (corpora/open/apeach) into corpora/derived/apeach for the predictor.

Source: https://huggingface.co/datasets/jason9693/APEACH (revision e57e10cf), the Korean hate-speech
evaluation set: apeach.test.csv (crowd-written sentences) and beep.train.csv (BEEP!/kor_hate news
comments). License: CC BY-SA 4.0. Only derived counts/sentences are used for training; the text
itself is not shipped.

What it extracts (class == "0", i.e. non-offensive rows only; class-1 text is never used):
- ko.txt: casual Korean sentences. Rows are read with csv.DictReader (quoted fields carry commas
  and newlines). Lines with URLs lose them; lines with censoring placeholders (OO, OOO, oo) or
  censored jamo-only profanity (bare consonant runs like ㅈㄴ, ㅅㅂ) are dropped, while
  laughter/emotion runs (ㅋㅋ, ㅎㅎ, ㅠㅠ, ㅜㅜ, ㄷㄷ, ㅡㅡ) are kept, capped at 3 repeats.
  Repeated punctuation and tildes are collapsed, text is split on sentence punctuation and
  newlines, fragments with under 2 Hangul syllables are dropped, and the rest are deduplicated.
- mixed.txt: distinct Latin+Hangul tokens (MZ세대, sns에, jyp는), a small acronym+particle set.

Usage: python3 -I apeach.py
"""

import csv
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "apeach")
OUT = os.path.join(ROOT, "corpora", "derived", "apeach")
FILES = ["apeach.test.csv", "beep.train.csv"]

URL = re.compile(r"(?:https?://|www\.)\S+|\b[\w.-]+\.(?:com|net|me|kr|org)(?:/\S*)?", re.I)
PLACEHOLDER = re.compile(r"(?<![A-Za-z])(?:OO+|oo+|ㅇㅇㅇ+|○○+|XX+|xx+)(?![A-Za-z])")
JAMO_RUN = re.compile(r"[ㄱ-ㅣ]{2,}")
KEEP_JAMO = set("ㅋㅎㅠㅜㄷㅡ")
SPLIT = re.compile(r"(?<=[.!?])\s+|\n+|(?<=\.\.\.)(?=[가-힣])")
HANGUL = re.compile(r"[가-힣]")
MIXED = re.compile(r"[A-Za-z][A-Za-z0-9]*[가-힣]+|[가-힣]+[A-Za-z]{2,}[A-Za-z0-9]*")


def clean(text):
    text = URL.sub(" ", text)
    if PLACEHOLDER.search(text):
        return None
    for run in JAMO_RUN.findall(text):
        if not set(run) <= KEEP_JAMO:
            return None
    text = re.sub(r"([ㄱ-ㅣ])\1{3,}", r"\1\1\1", text)
    text = re.sub(r"\.{3,}|…+", "...", text)
    text = re.sub(r"([!?~^>,;])\1+", r"\1", text)
    text = re.sub(r"[!?]{2,}", lambda m: m.group(0)[0], text)
    text = re.sub(r"\.\.(?!\.)", ".", text)
    text = re.sub(r"[ \t\r ​]+", " ", text)
    return text


def main():
    os.makedirs(OUT, exist_ok=True)
    seen, lines, mixed = set(), [], set()
    for name in FILES:
        with open(os.path.join(SRC, name), encoding="utf-8", newline="") as f:
            for row in csv.DictReader(f):
                if (row.get("class") or "").strip() != "0":
                    continue
                text = clean(row.get("text") or "")
                if text is None:
                    continue
                for part in SPLIT.split(text):
                    part = part.strip(" \"'")
                    if len(HANGUL.findall(part)) < 2 or part in seen:
                        continue
                    seen.add(part)
                    lines.append(part)
                    mixed.update(MIXED.findall(part))
    files = {}
    with open(os.path.join(OUT, "ko.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.writelines(line + "\n" for line in lines)
    files["ko.txt"] = len(lines)
    mixed = sorted(mixed)
    with open(os.path.join(OUT, "mixed.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.writelines(token + "\n" for token in mixed)
    files["mixed.txt"] = len(mixed)
    meta = {
        "source": "apeach",
        "tier": "moderate",
        "weight": 0.4,
        "uses": ["korean-key-model", "mixed-ko-en", "typos", "particles-endings"],
        "license": "CC BY-SA 4.0",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(files)


if __name__ == "__main__":
    main()
