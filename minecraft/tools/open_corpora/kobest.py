"""Extracts KoBEST (corpora/open/kobest, huggingface.co/datasets/skt/kobest_v1, CC BY-SA 4.0) into
corpora/derived/kobest/ for build_ranker.py. Only derived counts and sentences are used.

KoBEST is five Korean benchmark tasks (BoolQ, COPA, HellaSwag, SentiNeg, WiC), each split into
train/validation/test JSON Lines. Labels are ignored; only the Korean text fields are read:

- boolq: paragraph (Wikipedia-style prose) and question.
- copa: premise, alternative_1, alternative_2 (question is only 원인/결과, skipped).
- hellaswag: context and ending_1..ending_4.
- sentineg: sentence (product reviews). test_originated.jsonl is skipped: it is test.jsonl with
  the negations undone, so it would double the review templates.
- wic: context_1 and context_2, with the [ ] brackets around the target word removed (the
  brackets are kept as plain text elsewhere, e.g. 1[2]).

Cleaning: citation markers ([1], [*], [주해 1], [출처 필요]) are removed; parentheses holding
another script (Arabic, Khmer, Cyrillic, IPA, kana, hanja: 아랍어: مطبخ عماني) are removed, and any
sentence still holding such characters is dropped. Text is split into sentences on . ? ! followed
by whitespace, whitespace-normalized, and exactly deduplicated (HellaSwag contexts and WiC
sentences repeat across rows). Sentences without Hangul or shorter than 2 characters are dropped.

- ko.txt: the deduplicated sentences, in file order (Latin words, acronyms and digits kept).
- ko_words.tsv: whitespace tokens over ko.txt, edge punctuation stripped, Hangul-bearing only,
  with their counts.
- mixed.txt: distinct tokens mixing Latin letters and Hangul (TV를, SNS에서, KTX가), sorted.

Usage: python3 -I kobest.py
"""

import collections
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "kobest")
OUT = os.path.join(ROOT, "corpora", "derived", "kobest")

SPLITS = ("train", "validation", "test")
FIELDS = {
    "boolq": ("paragraph", "question"),
    "copa": ("premise", "alternative_1", "alternative_2"),
    "hellaswag": ("context", "ending_1", "ending_2", "ending_3", "ending_4"),
    "sentineg": ("sentence",),
    "wic": ("context_1", "context_2"),
}

CITATION = re.compile(r"\[(\d+|\*|주해 ?\d+|출처 필요)\]")
WIC_BRACKET = re.compile(r"\[([^\[\]]{1,30})\]")
# Letters the key model can't type, which only appear as glosses (Arabic, Khmer, Cyrillic, IPA,
# kana, hanja...): anything alphabetic outside ASCII, Latin-1/Extended-A/B and Hangul.
FOREIGN = "^\\W\\d_A-Za-z\u00c0-\u024f\uac00-\ud7a3\u1100-\u11ff\u3130-\u318f"
FOREIGN_CHAR = re.compile(f"[{FOREIGN}]")
FOREIGN_PAREN = re.compile(rf"\s*\([^()]*[{FOREIGN}][^()]*\)")
SENT_SPLIT = re.compile(r"(?<=[.?!])\s+")
HANGUL = re.compile(r"[가-힣]")
EDGE = "\"'“”‘’()[]{}<>《》〈〉「」『』.,;:!?…·~-"
MIXED = re.compile(r"(?=.*[A-Za-z])(?=.*[가-힣])[A-Za-z0-9가-힣&+.\-]+")


def texts():
    for task, fields in FIELDS.items():
        for split in SPLITS:
            path = os.path.join(SRC, task, split + ".jsonl")
            with open(path, encoding="utf-8") as f:
                for raw in f:
                    raw = raw.strip()
                    if not raw:
                        continue
                    try:
                        row = json.loads(raw)
                    except ValueError:
                        continue
                    for field in fields:
                        s = row.get(field)
                        if not isinstance(s, str):
                            continue
                        if task == "wic":
                            s = WIC_BRACKET.sub(r"\1", s)
                        yield s


def clean(s):
    s = CITATION.sub("", s)
    s = FOREIGN_PAREN.sub("", s)
    return " ".join(s.split())


def write_lines(name, lines):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    return len(lines)


def main():
    seen = set()
    sentences = []
    dropped = 0
    for text in texts():
        for sent in SENT_SPLIT.split(clean(text)):
            sent = sent.strip()
            if len(sent) < 2 or not HANGUL.search(sent) or sent in seen:
                continue
            seen.add(sent)
            if FOREIGN_CHAR.search(sent):
                dropped += 1
                continue
            sentences.append(sent)

    words = collections.Counter()
    mixed = set()
    for sent in sentences:
        for tok in sent.split():
            tok = tok.strip(EDGE)
            if not tok or not HANGUL.search(tok):
                continue
            words[tok] += 1
            if MIXED.fullmatch(tok):
                mixed.add(tok)

    os.makedirs(OUT, exist_ok=True)
    files = {"ko.txt": write_lines("ko.txt", sentences)}
    ordered = sorted(words.items(), key=lambda kv: (-kv[1], kv[0]))
    files["ko_words.tsv"] = write_lines("ko_words.tsv", [f"{w}\t{c}" for w, c in ordered])
    files["mixed.txt"] = write_lines("mixed.txt", sorted(mixed))
    meta = {
        "source": "kobest",
        "tier": "moderate",
        "weight": 0.6,
        "uses": ["korean-key-model", "korean-lexicon", "particles-endings", "mixed-ko-en", "acronyms"],
        "license": "CC BY-SA 4.0",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    hangul = sum(len(HANGUL.findall(s)) for s in sentences)
    print(f"{len(sentences)} sentences, {hangul} Hangul syllables, {dropped} dropped (foreign script); "
          + ", ".join(f"{k}: {v}" for k, v in files.items()))


main()
