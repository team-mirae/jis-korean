"""Extracts the Korean Parallel Corpora (corpora/open/korean-parallel-corpora,
github.com/jungyeul/korean-parallel-corpora, CC BY-SA 3.0) into
corpora/derived/korean-parallel-corpora/ for build_ranker.py. Only derived counts and sentences
are used.

Korean-English news sentence pairs (Park, Hong and Cha, PACLIC 30, 2016), scraped from the JoongAng
Ilbo's CNN/AP translation pages around 2005-2009: ~97k raw lines per side, news register with many
acronyms and Latin names inside Korean text (IAEA를, MD기지를, PS3의).

Read (tarballs streamed with tarfile, never unpacked to disk):
- src/korean-english-news-v1/korean-english-park.{train,dev,test}.tar.gz: the *.ko and *.en
  members (UTF-8, one sentence per line).
- src/northkorean-english-news-v1/*.nk and *.en, src/korean-english-jhe/jhe-koen-{dev,eval}.ko
  and .en: small extras (~4.4k lines per side).
Skipped: bible/ (archaic register), the *.ko.pos files (tagged), the Korean-French set.

The scrape carries a lot of page chrome, so lines are cleaned before use:
- NFC; non-breaking/ideographic spaces become spaces; curly quotes, U+02DD and U+00B4 become ASCII
  quotes; U+2026 becomes "..."; whitespace collapsed.
- LaTeX-style quotes (`` and '') become ASCII double quotes.
- Leading datelines ("SYDNEY, Australia (CNN)", "(AP) _") and trailing chrome stripped: bylines ("(CNN) / 이수지(JOINS)", "SEOUL, South Korea (AP)/..."),
  "원문기사보기", "원문 & 독해설명 인쇄", "내 블로그에 저장".
- A line that does not end in sentence-final punctuation is cut after its last sentence end, if it
  has one (drops appended vocabulary glosses like "... 보도했다. modalities 양식"); otherwise it
  is dropped (headlines, glossary entries, truncated lines).
- Dropped: lines that occur 20+ times in the raw files (site chrome and pinned headlines that repeat
  ~550x), lines ending in "..", lines under 10 characters or 2 words, HTML entities or tags, URLs,
  script code, "##" links, characters outside the allowed set (Korean: Hangul syllables, printable
  ASCII, U+00B7; English: printable ASCII, which also drops EUC-KR mojibake like U+00A1 U+00AF),
  Korean lines whose letters are under 40% Hangul (English lines in the .ko files), and English
  boilerplate ("All rights reserved", "may not be published"). Exact duplicates removed.

Outputs:
- ko.txt: Korean sentences (with their Latin acronyms and digits) for the key model, lexicon and
  particle statistics.
- en.txt: English sentences from the parallel side, for English sentence trigrams.
- mixed.txt: distinct eojeol mixing Latin letters and Hangul (IAEA를, CNN한글뉴스), edge
  punctuation stripped, tokens with inner symbols skipped.

Usage: python3 -I korean_parallel_corpora.py
"""

import collections
import json
import os
import re
import tarfile
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
SRC = os.path.join(ROOT, "corpora", "open", "korean-parallel-corpora", "src")
OUT = os.path.join(ROOT, "corpora", "derived", "korean-parallel-corpora")
TARBALLS = [os.path.join("korean-english-news-v1", f"korean-english-park.{s}.tar.gz")
            for s in ("train", "dev", "test")]
PLAIN = {
    "ko": ["northkorean-english-news-v1/northkorean-english.dev.nk",
           "northkorean-english-news-v1/northkorean-english.test.nk",
           "korean-english-jhe/jhe-koen-dev.ko",
           "korean-english-jhe/jhe-koen-eval.ko"],
    "en": ["northkorean-english-news-v1/northkorean-english.dev.en",
           "northkorean-english-news-v1/northkorean-english.test.en",
           "korean-english-jhe/jhe-koen-dev.en",
           "korean-english-jhe/jhe-koen-eval.en"],
}
MAX_LINES = 3_000_000
REPEAT_LIMIT = 20

TRANSLATE = str.maketrans({
    " ": " ", "　": " ", "\t": " ",
    "“": '"', "”": '"', "˝": '"',
    "‘": "'", "’": "'", "´": "'",
    "…": "...",
})
ALLOWED = {
    "ko": re.compile(r"[가-힣\x20-\x7e·]+"),
    "en": re.compile(r"[\x20-\x7e]+"),
}
CHROME_TAIL = re.compile(
    r"\s*(?:"
    r"(?:[A-Z][A-Za-z .,'-]*)?\((?:CNN|AP|JOINS)\)\s*/.*"  # bylines: (CNN) / 이수지(JOINS) ...
    r"|[^\s()]*\(JOINS\).*"
    r"|원문기사보기|원문기사|원문 & 독해설명 인쇄|내 블로그에 저장"
    r")\s*$")
CHROME_HEAD = re.compile(r"^(?:[A-Z][A-Za-z .,'-]*)?\((?:CNN|AP)\)\s*[-_]*\s*")
SENTENCE_END = re.compile(r"""[.?!]["')]*$""")
LAST_END = re.compile(r"""^(.*[가-힣a-z0-9)][.?!]["')]*)\s""")
JUNK = re.compile(r"&#?[A-Za-z0-9]+;|<[A-Za-z/!]|https?://|www\.|##|function\s*\(|window\.|"
                  r"\{|\}|\.(?:com|net|org|co\.kr)\b", re.I)
EN_BOILER = re.compile(r"all rights reserved|may not be published|copyright", re.I)
HANGUL = re.compile(r"[가-힣]")
LATIN = re.compile(r"[A-Za-z]")
EDGE_PUNCT = re.compile(r"^[^0-9A-Za-z가-힣]+|[^0-9A-Za-z가-힣]+$")
MIXED_TOKEN = re.compile(r"[0-9A-Za-z가-힣\-]+")


def raw_lines():
    """Yields (lang, line) for every raw line of the chosen files."""
    for rel in TARBALLS:
        with tarfile.open(os.path.join(SRC, rel), "r:gz") as tar:
            for member in tar:
                if not member.isfile():
                    continue
                lang = {".ko": "ko", ".en": "en"}.get(os.path.splitext(member.name)[1])
                if lang is None:
                    continue
                f = tar.extractfile(member)
                for raw in f:
                    yield lang, raw.decode("utf-8", "replace")
    for lang, rels in PLAIN.items():
        for rel in rels:
            with open(os.path.join(SRC, rel), encoding="utf-8", errors="replace") as f:
                for raw in f:
                    yield lang, raw


def normalize(raw):
    text = unicodedata.normalize("NFC", raw).translate(TRANSLATE)
    text = text.replace("``", '"').replace("''", '"')
    return " ".join(text.split())


def clean(lang, text):
    """Returns the cleaned sentence, or None if the line is dropped."""
    if JUNK.search(text):
        return None
    prev = None
    while prev != text:
        prev = text
        text = CHROME_HEAD.sub("", CHROME_TAIL.sub("", text)).strip()
    if not SENTENCE_END.search(text):
        m = LAST_END.match(text)
        if not m:
            return None
        text = m.group(1)
    if (text.endswith("..") or len(text) < 10 or len(text.split()) < 2
            or not ALLOWED[lang].fullmatch(text)):
        return None
    if lang == "ko":
        hangul = len(HANGUL.findall(text))
        if hangul == 0 or hangul < 0.4 * (hangul + len(LATIN.findall(text))):
            return None
    else:
        if EN_BOILER.search(text) or HANGUL.search(text) or len(text.split()) < 3:
            return None
    return text


def sample(lines):
    if len(lines) <= MAX_LINES:
        return lines
    step = len(lines) / MAX_LINES
    return [lines[int(i * step)] for i in range(MAX_LINES)]


def write_lines(name, lines):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    return len(lines)


def main():
    os.makedirs(OUT, exist_ok=True)
    counts = {"ko": collections.Counter(), "en": collections.Counter()}
    order = {"ko": [], "en": []}
    for lang, raw in raw_lines():
        text = normalize(raw)
        if not text:
            continue
        if text not in counts[lang]:
            order[lang].append(text)
        counts[lang][text] += 1

    out = {"ko": [], "en": []}
    mixed = set()
    for lang in ("ko", "en"):
        seen = set()
        for text in order[lang]:
            if counts[lang][text] >= REPEAT_LIMIT:
                continue
            sentence = clean(lang, text)
            if sentence is None or sentence in seen:
                continue
            seen.add(sentence)
            out[lang].append(sentence)
            if lang == "ko" and LATIN.search(sentence):
                for tok in sentence.split():
                    tok = EDGE_PUNCT.sub("", tok)
                    if (MIXED_TOKEN.fullmatch(tok) and LATIN.search(tok)
                            and HANGUL.search(tok)):
                        mixed.add(tok)

    files = {
        "ko.txt": write_lines("ko.txt", sample(out["ko"])),
        "en.txt": write_lines("en.txt", sample(out["en"])),
        "mixed.txt": write_lines("mixed.txt", sorted(mixed)),
    }
    meta = {
        "source": "korean-parallel-corpora",
        "tier": "moderate",
        "weight": 0.6,
        "uses": ["korean-key-model", "korean-lexicon", "acronyms", "particles-endings",
                 "mixed-ko-en"],
        "license": "CC BY-SA 3.0 (Park, Hong and Cha 2016, Korean Language Resources for Everyone)",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=2)
        f.write("\n")
    for name, n in files.items():
        print(f"{name}: {n}")


if __name__ == "__main__":
    main()
