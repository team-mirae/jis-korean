"""Extracts KoreaScience summarization (corpora/open/koreascience-summarization,
huggingface.co/datasets/nglaura/koreascience-summarization, Apache-2.0) into
corpora/derived/koreascience-summarization/ for build_ranker.py. Only derived counts and
sentences are used.

KoreaScience (from the LoRaLay paper) is ~37.5k Korean research papers from koreascience.or.kr
across engineering, medicine, agriculture, IT and social science. Each record has a Korean
"abstract" and, for ~80%, an English "abstract_en". Academic register ("본 연구에서는 ...
제안하였다"), dense with Latin acronyms and terms written inside Korean (CAD/CAM 기술을, TTA에서).

Only train.txt, validation.txt and test.txt (JSON Lines) are read. The zips hold the full
papers as OCR'd word + bounding-box sequences (3.8 GB unpacked, noisy layout text) and are
skipped.

- ko.txt: "abstract" split into sentences after Hangul or ')' followed by . ? ! and whitespace,
  NFC-normalized, whitespace-collapsed, leading list markers (1., 2), ①) removed, exact
  duplicates removed. Dropped: fewer than 3 eojeol, no final . ? ! (abstracts cut off where
  the source lost a '<', as in "(p"), empty parentheses left by stripped symbols,
  under 50% Hangul (of non-space characters), URLs/e-mails, LaTeX or formula-heavy text (2+
  of = < > ^ _ { } | \\ ∑ ∫), broken text (U+FFFD, stray compatibility or conjoining jamo),
  and sentences with a bare particle token (" 의 ", " 를 ") left behind where the source
  stripped a formula.
- en.txt: "abstract_en" split into sentences, NFKC-normalized; kept when at least 4 words,
  mostly Latin letters, no Hangul and no formula symbols; deduplicated.
- ko_words.tsv: counts of Hangul-only eojeol (edge punctuation stripped) over ko.txt.
- mixed.txt: distinct eojeol mixing Latin letters and Hangul (IT기술의, TTA에서, ABAQUS를),
  split at '/' (CAD/CAM을 gives CAM을), edge punctuation stripped. Units followed by a particle (mm의, kW로,
  MPa에서) are left out so they don't flood the acronym+particle counts, as are tokens
  whose Latin part is a single letter (p값, x축).

Usage: python3 -I koreascience_summarization.py
"""

import json
import os
import re
import unicodedata
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
SRC = os.path.join(ROOT, "corpora", "open", "koreascience-summarization")
OUT = os.path.join(ROOT, "corpora", "derived", "koreascience-summarization")
FILES = ["train.txt", "validation.txt", "test.txt"]
CAP = 3_000_000

KO_SPLIT = re.compile(r"(?<=[가-힣)][.?!])\s+")
EN_SPLIT = re.compile(r"(?<=[a-z0-9)%][.?!])\s+(?=[A-Z(])")
LIST_MARKER = re.compile(r"^(?:\(?\d{1,2}[.)]|[①-⑳]|[-*•])\s*")
EMPTY_PARENS = re.compile(r"\(\s*[,~]?\s*\)")
HANGUL = re.compile(r"[가-힣]")
LATIN = re.compile(r"[A-Za-z]")
URL = re.compile(r"https?://|www\.|@[A-Za-z0-9-]+\.|\.(com|net|org|kr)\b", re.I)
FORMULA = re.compile(r"[=<>^_{}|\\∑∫]")
BROKEN = re.compile("[�ᄀ-ᇿㄱ-ㆎ]")
BARE_PARTICLE = re.compile(r"(?:^|\s)(?:의|을|를|은|는|와|과|으로|에서|에는|에|로|이며|이고|인)(?=\s|[,.]|$)")
EDGE_PUNCT = re.compile(r"^[^0-9A-Za-z가-힣]+|[^0-9A-Za-z가-힣]+$")
TOKEN = re.compile(r"[0-9A-Za-z가-힣\-]+")
UNITS = {
    "mm", "cm", "m", "km", "nm", "um", "kg", "g", "mg", "ng", "t", "l", "L", "mL", "ml", "Pa",
    "MPa", "kPa", "GPa", "W", "kW", "MW", "GW", "mW", "V", "kV", "mV", "A", "mA", "dB", "Hz",
    "kHz", "MHz", "GHz", "K", "s", "ms", "min", "h", "hr", "rpm", "ppm", "ppb", "kWh", "MWh",
    "N", "kN", "J", "kJ", "MJ", "cal", "kcal", "mol", "M", "mM", "lux", "lx", "ha", "pH", "Mbps",
    "Gbps", "kbps", "bps", "MB", "GB", "KB", "TB", "fps", "dBm", "m2", "m3", "cm2", "cm3", "mm2",
}
LATIN_RUN = re.compile(r"[A-Za-z][A-Za-z0-9]*")


def write_lines(name, lines):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    return len(lines)


def records():
    for name in FILES:
        with open(os.path.join(SRC, name), encoding="utf-8") as f:
            for raw in f:
                raw = raw.strip()
                if not raw:
                    continue
                try:
                    rec = json.loads(raw)
                except ValueError:
                    continue
                if isinstance(rec, dict):
                    yield rec


def ko_ok(s):
    if len(s.split()) < 3 or URL.search(s) or BROKEN.search(s) or EMPTY_PARENS.search(s):
        return False
    if s[-1] not in ".?!" or len(FORMULA.findall(s)) >= 2 or BARE_PARTICLE.search(s):
        return False
    chars = s.replace(" ", "")
    return len(HANGUL.findall(chars)) >= 0.5 * len(chars)


def en_ok(s):
    if len(s.split()) < 4 or URL.search(s) or HANGUL.search(s) or BROKEN.search(s):
        return False
    if FORMULA.search(s) or not s.isascii() or s[-1] not in ".?!":
        return False
    chars = s.replace(" ", "")
    return len(LATIN.findall(chars)) >= 0.7 * len(chars)


def mixed_ok(tok):
    if not (TOKEN.fullmatch(tok) and LATIN.search(tok) and HANGUL.search(tok)):
        return False
    latin = LATIN_RUN.findall(tok)
    if all(len(run) == 1 for run in latin):
        return False
    # unit + particle (mm의, 10kW로) or a number+unit glued to a Hangul word
    m = re.fullmatch(r"[0-9.\-]*([A-Za-z][A-Za-z0-9]*)([가-힣]+)", tok)
    if m and m.group(1) in UNITS:
        return False
    return True


def sample(lines):
    if len(lines) <= CAP:
        return lines
    k = -(-len(lines) // CAP)
    return lines[::k]


def main():
    os.makedirs(OUT, exist_ok=True)
    ko_seen, en_seen = set(), set()
    ko, en = [], []
    words = Counter()
    mixed = set()
    for rec in records():
        abstract = rec.get("abstract")
        if isinstance(abstract, str):
            text = " ".join(unicodedata.normalize("NFC", abstract).split())
            for s in KO_SPLIT.split(text):
                s = LIST_MARKER.sub("", s.strip())
                if s in ko_seen or not ko_ok(s):
                    continue
                ko_seen.add(s)
                ko.append(s)
                for tok in s.split():
                    for part in tok.split("/"):
                        part = EDGE_PUNCT.sub("", part)
                        if not part:
                            continue
                        if re.fullmatch(r"[가-힣]+", part):
                            words[part] += 1
                        elif mixed_ok(part):
                            mixed.add(part)
        abstract_en = rec.get("abstract_en")
        if isinstance(abstract_en, str):
            text = " ".join(unicodedata.normalize("NFKC", abstract_en).split())
            for s in EN_SPLIT.split(text):
                s = s.strip()
                if s in en_seen or not en_ok(s):
                    continue
                en_seen.add(s)
                en.append(s)

    files = {
        "ko.txt": write_lines("ko.txt", sample(ko)),
        "en.txt": write_lines("en.txt", sample(en)),
        "ko_words.tsv": write_lines(
            "ko_words.tsv",
            [f"{w}\t{c}" for w, c in sorted(words.items(), key=lambda x: (-x[1], x[0]))][:CAP]),
        "mixed.txt": write_lines("mixed.txt", sorted(mixed)[:CAP]),
    }
    meta = {
        "source": "koreascience-summarization",
        "tier": "moderate",
        "weight": 0.7,
        "uses": ["korean-key-model", "korean-lexicon", "mixed-ko-en", "acronyms",
                 "particles-endings", "english-nouns"],
        "license": "Apache-2.0 (dataset card, huggingface.co/datasets/nglaura/koreascience-summarization)",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=2)
        f.write("\n")
    for name, n in files.items():
        print(f"{name}: {n}")


if __name__ == "__main__":
    main()
