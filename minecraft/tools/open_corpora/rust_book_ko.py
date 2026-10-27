"""Extracts Korean programming prose, mixed Latin+Hangul tokens and Korean-English term pairs from
the Korean translation of The Rust Programming Language (corpora/open/rust-book-ko) into
corpora/derived/rust-book-ko. License: MIT OR Apache-2.0 (corpora/open/rust-book-ko/LICENSE-MIT,
LICENSE-APACHE).

Source: rinthel/rust-lang-book-ko (https://github.com/rinthel/rust-lang-book-ko), src/*.md at the
commit in SOURCE.txt. Inputs: every src/src/*.md (106 mdBook chapters, Markdown). SUMMARY.md,
title-page.md and appendix-06-translation.md are skipped, and of the two ch03-02 variants only
"ch03-02-data-types - 번역.md" (the translated one) is read; ch03-02-data-types.md is the English
original. About 60 chapters were never translated; their English prose fails the Hangul test below.

Cleaning, per chapter:

- Fenced code blocks (``` and ~~~, also inside > quotes), HTML comments (also multi-line), lines
  with {{#include}}/{{#rustdoc_include}} and <span ...> (listing captions and file names), and
  link-reference definitions ([x]: url) are dropped, as are table separator rows.
- Headings, > quotes and list markers are stripped; [text](url) and [text][ref] become text,
  images and URLs are removed, remaining HTML tags are stripped (text kept), entities unescaped,
  and * / _ emphasis removed.
- Inline code: a backticked token that is a real word or acronym (2+ ASCII letters, letters only:
  String, Cargo, API, Result) keeps its text without backticks (String을, Cargo는). A sentence with
  any other inline code (`x`, `s2`, `T`, `cargo new`, `Vec<T>`, `main()`, `std::io`), or a URL, is
  dropped rather than left with a hole.
- Hard-wrapped lines within a paragraph are joined; headings, list items and table cells are their
  own units. Units are split into sentences at . ! ? followed by a space.

Outputs:

- ko.txt: sentences with at least 30% of their letters Hangul (4+ characters, no leftover markup
  or code punctuation), deduplicated, in file order. Parenthetical English glosses are removed
  (소유권(ownership) -> 소유권, *푸시(push)* -> 푸시). An all-capitals acronym followed by a particle
  of the wrong allomorph for its letter-name reading (API을 for API를) has its particle corrected,
  as in mdn_ko.py.
- mixed.txt: distinct tokens mixing Latin letters and Hangul (String을, Cargo는, API를) with at
  least 2 Latin letters in a row (A가, x의 are skipped), counted over the sentences before glosses
  are removed, most frequent first.
- terms.tsv: Korean term<TAB>English gloss from the glosses: '*힙 공간 할당(allocating on the
  heap)*' gives the whole emphasised term; otherwise as many Hangul words before the parenthesis as
  the gloss has English words, up to 3 ('대여 검사기(borrow checker)', '소유권(ownership)'), and
  never when the Korean ends like a verb or particle (가능한(growable) is skipped). The most frequent
  English per Korean term wins.
- meta.json.

Usage: python3 -I rust_book_ko.py
"""

import html
import json
import os
import re
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "rust-book-ko", "src", "src")
OUT = os.path.join(ROOT, "corpora", "derived", "rust-book-ko")

META = {
    "source": "rust-book-ko",
    "tier": "moderate",
    "weight": 0.5,
    "uses": ["korean-key-model", "mixed-ko-en", "particles-endings", "korean-lexicon"],
    "license": "MIT OR Apache-2.0",
}

SKIP = {"SUMMARY.md", "title-page.md", "appendix-06-translation.md", "ch03-02-data-types.md"}
MAX_LINES = 3_000_000

HANGUL = re.compile("[가-힣]")
LATIN = re.compile(r"[A-Za-z]+")
FENCE = re.compile(r"^(```+|~~~+)")
LINKDEF = re.compile(r"^\[[^\]]+\]:\s*\S+")
HEADING = re.compile(r"^#{1,6}\s+")
LIST = re.compile(r"^(?:[-*+]|\d+[.)])\s+")
IMAGE = re.compile(r"!\[[^\]]*\]\([^)]*\)")
LINK = re.compile(r"\[([^\]]*)\]\((?:[^()]|\([^)]*\))*\)")
REFLINK = re.compile(r"\[([^\]]+)\]\[[^\]]*\]")
BRACKET = re.compile(r"\[([^\]]+)\]")
URL = re.compile(r"(?:https?://|ftp://|www\.)\S+", re.I)
COMMENT = re.compile(r"<!--.*?-->", re.S)
TAG = re.compile(r"</?[A-Za-z][A-Za-z0-9-]*(?:\s[^<>]*)?/?>")
CODE = re.compile(r"`+([^`]*)`+")
WORD = re.compile(r"^[A-Za-z]{2,}$")
EMPH = re.compile(r"\*\*|__|(?<![A-Za-z0-9])[*_](?=\S)|(?<=\S)[*_](?![A-Za-z0-9])")
SENT = re.compile(r"(?<=[.!?])\s+(?=\S)")
# '*힙 공간 할당(allocating on the heap)*' and '소유권(ownership)'.
GLOSS_EMPH = re.compile(r"\*{1,2}([가-힣][가-힣 ]*?)\s*\(([A-Za-z][A-Za-z '\-]*)\)\*{1,2}")
GLOSS = re.compile(r"((?:[가-힣]+ ){0,2}[가-힣]+)\s*\(([A-Za-z][A-Za-z '\-]*)\)")
# A Korean "term" ending like a verb, adjective or particle is a clause, not a term.
NOT_TERM = re.compile(r"(?:다|요|죠|한|된|는|은|을|를|의|이|가|에|도|로|적|게|고|서|지)$")
GLOSS_ANY = re.compile(r"\s*\([A-Za-z][A-Za-z '\-,./]*\)")
LEFTOVER = re.compile(r"\{\{|\}\}|\]\(|[<>{}\[\];`|\\]|::|=>|&[a-z]+;|^[:>#-]")
LATIN2 = re.compile(r"[A-Za-z]{2,}")
MIXED = re.compile(r"^(?=.*[A-Za-z])(?=.*[가-힣])[가-힣A-Za-z0-9&]+$")
ACRONYM = re.compile(r"(?<![A-Za-z0-9])([A-Z]{2,}[A-Z0-9]*)([가-힣]+)")
EDGE = "\"'()[]{}<>.,:;!?“”‘’「」『』·…/"

# Letter names ending in a consonant (ㄹ: L 엘, R 알; ㅁ: M 엠; ㄴ: N 엔) and Sino-Korean digits.
FINAL = {"L": "ㄹ", "R": "ㄹ", "M": "ㅁ", "N": "ㄴ",
         "0": "ㅇ", "1": "ㄹ", "3": "ㅁ", "6": "ㄱ", "7": "ㄹ", "8": "ㄹ"}
PAIRS = {"을": "를", "은": "는", "과": "와", "이": "가", "이나": "나", "이랑": "랑", "이며": "며"}
PAIRS_BACK = {v: k for k, v in PAIRS.items()}


def fix_particle(m):
    """API을 -> API를: the particle allomorph that fits the acronym's letter-name reading."""
    acr, tail = m.group(1), m.group(2)
    cons = acr[-1] in FINAL
    rieul = FINAL.get(acr[-1]) == "ㄹ"
    if tail in PAIRS and not cons:
        return acr + PAIRS[tail]
    if tail in PAIRS_BACK and cons:
        return acr + PAIRS_BACK[tail]
    if tail.startswith("으로") and (not cons or rieul):
        return acr + tail[1:]
    if tail.startswith("로") and cons and not rieul:
        return acr + "으" + tail
    return m.group(0)


def units(text):
    """Prose units (paragraphs, list items, headings, table cells) of one chapter."""
    text = COMMENT.sub(" ", text)
    fence, para = None, []
    in_comment = False
    for raw in text.split("\n"):
        line = raw.strip()
        body = line.lstrip("> ").strip() if line.startswith(">") else line
        if in_comment:
            if "-->" in body:
                in_comment = False
            continue
        if body.startswith("<!--"):
            in_comment = True
            continue
        if fence:
            if body.startswith(fence):
                fence = None
            continue
        f = FENCE.match(body)
        if f:
            fence = f.group(1)[:3]
            if para:
                yield " ".join(para)
                para = []
            continue
        if (not body or "{{#" in body or "<span" in body or LINKDEF.match(body)
                or re.match(r"^\|?[\s:|-]+\|?$", body) and "-" in body):
            if para:
                yield " ".join(para)
                para = []
            continue
        if body.startswith("|"):
            if para:
                yield " ".join(para)
                para = []
            for cell in body.strip("|").split("|"):
                yield cell.strip()
            continue
        if HEADING.match(body) or LIST.match(body):
            if para:
                yield " ".join(para)
            para = [LIST.sub("", HEADING.sub("", body))]
            if HEADING.match(body):
                yield para[0]
                para = []
            continue
        para.append(body)
    if para:
        yield " ".join(para)


def inline(text):
    """Markdown inline text -> plain text, inline code still in backticks."""
    text = IMAGE.sub(" ", text)
    text = LINK.sub(r"\1", text)
    text = REFLINK.sub(r"\1", text)
    text = TAG.sub("", text)
    text = html.unescape(text)
    text = BRACKET.sub(r"\1", text)
    return " ".join(text.split())


def sentence(s):
    """One sentence -> plain text, or None if it carries code or a URL."""
    for code in CODE.findall(s):
        if not WORD.match(code):
            return None
    if URL.search(s):
        return None
    return " ".join(EMPH.sub("", CODE.sub(r"\1", s)).split())


def glosses(text, terms):
    """Count Korean term -> English gloss pairs in one inline-cleaned unit."""
    def emph(m):
        k = " ".join(m.group(1).split())
        if not NOT_TERM.search(k):
            terms[k][" ".join(m.group(2).split())] += 1
        return " "
    text = GLOSS_EMPH.sub(emph, CODE.sub(" ", text))
    for k, e in GLOSS.findall(EMPH.sub("", text)):
        e = " ".join(e.split())
        n = min(len(e.split()), 3)
        k = " ".join(k.split()[-n:])
        if not NOT_TERM.search(k):
            terms[k][e] += 1


def keep(sentence):
    if len(sentence) < 4 or LEFTOVER.search(sentence):
        return False
    hangul = len(HANGUL.findall(sentence))
    latin = sum(len(w) for w in LATIN.findall(sentence))
    return hangul > 0 and hangul >= 0.3 * (hangul + latin)


def main():
    seen, ko = set(), []
    mixed = Counter()
    terms = defaultdict(Counter)
    fixed = dropped_code = chapters = 0
    for name in sorted(os.listdir(SRC)):
        if not name.endswith(".md") or name in SKIP:
            continue
        chapters += 1
        with open(os.path.join(SRC, name), encoding="utf-8", errors="replace") as f:
            text = f.read()
        for unit in units(text):
            if not HANGUL.search(unit):
                continue
            unit = inline(unit)
            glosses(unit, terms)
            for s in SENT.split(unit):
                s = sentence(s.strip())
                if s is None:
                    dropped_code += 1
                    continue
                if not keep(s):
                    continue
                for tok in s.split():
                    tok = tok.strip(EDGE)
                    if MIXED.match(tok) and LATIN2.search(tok):
                        mixed[ACRONYM.sub(fix_particle, tok)] += 1
                bare = " ".join(GLOSS_ANY.sub("", s).split())
                bare2 = ACRONYM.sub(fix_particle, bare)
                if bare2 != bare:
                    fixed += 1
                if not keep(bare2) or bare2 in seen:
                    continue
                seen.add(bare2)
                ko.append(bare2)

    if len(ko) > MAX_LINES:
        k = -(-len(ko) // MAX_LINES)
        ko = ko[::k]
    os.makedirs(OUT, exist_ok=True)

    def write(name, rows):
        with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
            for row in rows:
                f.write(row + "\n")
        return len(rows)

    term_rows = []
    for k in sorted(terms):
        c = terms[k]
        e, n = max(c.items(), key=lambda kv: (kv[1], -len(kv[0])))
        if n > 0 and len(k) >= 2:
            term_rows.append(f"{k}\t{e}")
    files = {
        "ko.txt": write("ko.txt", ko),
        "mixed.txt": write("mixed.txt", [w for w, _ in sorted(mixed.items(), key=lambda kv: (-kv[1], kv[0]))]),
        "terms.tsv": write("terms.tsv", term_rows),
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(dict(META, files=files), f, ensure_ascii=False, indent=1)
        f.write("\n")
    for name, n in files.items():
        print(f"{name}: {n}")
    print(f"chapters: {chapters}; acronym particles corrected: {fixed}; sentences dropped for code: {dropped_code}")


if __name__ == "__main__":
    main()
