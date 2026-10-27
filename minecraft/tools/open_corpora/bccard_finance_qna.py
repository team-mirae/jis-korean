"""Extracts BCCard-Finance-Kor-QnA (corpora/open/bccard-finance-qna) into corpora/derived/bccard-finance-qna.

Source: https://huggingface.co/datasets/BCCard/BCCard-Finance-Kor-QnA (revision in SOURCE.txt), one
JSONL file of ~31k Korean finance / card Q&A pairs ({"instruction", "output"}); the answers are
likely partly LLM-generated. License: Apache-2.0 (dataset card; LICENSE-Apache-2.0.txt).

What it writes (tier moderate, weight 0.8):
- ko.txt: Korean sentences, one per line. Questions come first (meta.json "question_lines" says how
  many), since they are closer to what people type and should be weighted above the answers. Rows
  with LaTeX are dropped, markdown (**, #, list markers) is stripped, text is split on ". ", "?",
  "!" and newlines, URLs and Hangul-poor lines are dropped, and exact duplicates removed.
- ko_words.tsv: pure-Hangul word counts, capped per word (CAP) so 있습니다/합니다 don't dominate.
- mixed.txt: distinct mixed Latin+Hangul tokens (BC카드, KB국민카드, ATM에서).
- particles.txt: Korean particles/endings seen right after an all-caps acronym (ATM에서 → 에서),
  limited to a known particle list, most frequent first.
- terms.tsv: business terms: Hangul nouns ending in 료/율/금/액/사/카드 (english empty), plus
  "한국어(English)" glosses (english filled).
The Latin tokens are not used as an English lexicon: they're acronyms, brands and noise.

Usage: python3 -I bccard_finance_qna.py
"""

import collections
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "bccard-finance-qna", "bccard-finance-qna.jsonl")
OUT = os.path.join(ROOT, "corpora", "derived", "bccard-finance-qna")

NAME = "bccard-finance-qna"
TIER = "moderate"
WEIGHT = 0.8
USES = ["korean-key-model", "korean-lexicon", "mixed-ko-en", "acronyms", "business-terms",
        "particles-endings"]
LICENSE = "Apache-2.0"
CAP = 500  # per-word count cap in ko_words.tsv

LATEX = ("\\(", "\\[", "\\frac", "\\times", "\\text")
HANGUL = re.compile(r"[가-힣]")
URL = re.compile(r"https?://|www\.|\.com\b|\.co\.kr\b", re.I)
SPLIT = re.compile(r"(?<=[.?!])\s+|\n+")
LIST_MARK = re.compile(r"^\s*(?:#+\s*|[-*•]\s+|\d+[.)]\s+)")
MIXED = re.compile(r"[A-Za-z0-9&]*[A-Za-z][A-Za-z0-9&]*[가-힣]+[A-Za-z0-9가-힣]*|[가-힣]+[A-Za-z][A-Za-z0-9&]*[가-힣]*")
ACRO = re.compile(r"(?<![A-Za-z])([A-Z]{2,})([가-힣]+)")
WORD = re.compile(r"^[가-힣]+$")
TERM_END = ("료", "율", "금", "액", "사", "카드")
GLOSS = re.compile(r"(?<![가-힣A-Za-z0-9])([가-힣]{2,12})\s?\(([A-Za-z][A-Za-z .&'-]{1,60}[A-Za-z])\)")
NOT_NOUN = ("한", "는", "은", "을", "를", "의", "된", "인", "적", "에", "로", "와", "과", "이", "가", "도")
PUNCT = "\"'“”‘’()[]{}<>.,!?;:·…~「」『』"
PARTICLES = {
    "이", "가", "을", "를", "은", "는", "의", "에", "에서", "에게", "께", "께서", "와", "과", "로", "으로",
    "도", "만", "까지", "부터", "보다", "처럼", "이나", "나", "이랑", "랑", "하고", "에는", "에서는",
    "으로는", "로는", "에도", "에서도", "로도", "으로도", "과의", "와의", "에의", "만의", "까지의",
    "이란", "란", "이라는", "라는", "이라고", "라고", "이며", "며", "이고", "고", "이다", "다", "입니다",
    "인", "이죠", "죠", "이요", "요", "이에요", "예요", "이야", "야", "에게서", "한테", "한테서", "인가요",
    "가요", "인지", "지", "이면", "면", "으로서", "로서", "으로써", "로써", "마다", "조차", "밖에",
    "뿐", "대로", "이든", "든", "이라도", "라도", "이나마", "나마", "을까요", "를까요", "에요", "이죠",
    "의한", "에의해", "를통해", "등", "들", "들은", "들이", "들을", "들의", "같은", "처럼의",
}


def clean(line):
    line = line.replace("**", "").replace("__", "").replace("`", "")
    line = LIST_MARK.sub("", line)
    return re.sub(r"\s+", " ", line).strip()


def sentences(text):
    for part in SPLIT.split(text.replace("\r", "")):
        s = clean(part)
        if not s or URL.search(s) or "\\" in s:
            continue
        hangul = len(HANGUL.findall(s))
        if hangul < 4 or hangul < 0.4 * len(s.replace(" ", "")):
            continue
        yield s


def main():
    os.makedirs(OUT, exist_ok=True)
    questions, answers, seen_q, seen = [], [], set(), set()
    with open(SRC, encoding="utf-8") as f:
        for raw in f:
            raw = raw.strip()
            if not raw:
                continue
            try:
                row = json.loads(raw)
            except ValueError:
                continue
            q, a = str(row.get("instruction") or ""), str(row.get("output") or "")
            if any(m in q or m in a for m in LATEX):
                continue
            if q in seen_q:
                q = ""  # duplicate question: keep its (possibly different) answer only
            seen_q.add(q)
            for text, sink in ((q, questions), (a, answers)):
                for s in sentences(text):
                    if s not in seen:
                        seen.add(s)
                        sink.append(s)
    lines = questions + answers

    words, mixed, particles, terms, glosses = (collections.Counter() for _ in range(5))
    for s in lines:
        for g in GLOSS.finditer(s):
            glosses[(g.group(1), g.group(2).strip())] += 1
        for tok in s.split():
            tok = tok.strip(PUNCT)
            if WORD.match(tok):
                words[tok] += 1
                if 2 <= len(tok) <= 8 and tok.endswith(TERM_END):
                    terms[tok] += 1
            for m in MIXED.findall(tok):
                if len(re.findall(r"[A-Za-z]", m)) >= 2:
                    mixed[m] += 1
            for acro, suffix in ACRO.findall(tok):
                if suffix in PARTICLES:
                    particles[suffix] += 1

    files = {}

    def write(name, rows):
        rows = list(rows)
        if not rows:
            return
        with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
            for r in rows:
                f.write(r + "\n")
        files[name] = len(rows)

    write("ko.txt", lines)
    write("ko_words.tsv", (f"{w}\t{min(c, CAP)}" for w, c in
                           sorted(words.items(), key=lambda kv: (-kv[1], kv[0]))))
    write("mixed.txt", sorted(m for m, c in mixed.items()))
    write("particles.txt", (p for p, c in sorted(particles.items(), key=lambda kv: (-kv[1], kv[0]))
                            if c >= 2))
    term_rows = {}
    for (ko, en), c in sorted(glosses.items(), key=lambda kv: (-kv[1], kv[0])):
        if ko in term_rows or len(en) < 2 or ko.endswith(NOT_NOUN) or (c < 2 and en.islower()):
            continue
        term_rows[ko] = en
    for t, c in sorted(terms.items(), key=lambda kv: (-kv[1], kv[0])):
        if c >= 3 and t not in term_rows:
            term_rows[t] = ""
    write("terms.tsv", (f"{k}\t{v}" for k, v in sorted(term_rows.items())))

    meta = {"source": NAME, "tier": TIER, "weight": WEIGHT, "uses": USES, "license": LICENSE,
            "files": files, "question_lines": len(questions)}
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(json.dumps(meta, ensure_ascii=False))


main()
