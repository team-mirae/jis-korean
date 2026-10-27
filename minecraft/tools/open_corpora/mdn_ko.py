"""Extracts Korean web-development prose, mixed Latin+Hangul tokens, English API words and glossary
terms from the Korean MDN Web Docs (corpora/open/mdn-ko) into corpora/derived/mdn-ko. License:
prose CC BY-SA 2.5, code samples CC0/MIT (corpora/open/mdn-ko/src/LICENSE.md).

Source: mdn/translated-content (https://github.com/mdn/translated-content), files/ko/ at the commit
in SOURCE.txt. Inputs: every src/files/ko/**/index.md (about 3,350 Markdown pages: front matter,
KumaScript macros, Markdown, some raw HTML). _redirects.txt, _wikihistory.json and the SVGs are
skipped.

Cleaning, per page:

- The YAML front matter is dropped (its title/slug feed terms.tsv for glossary pages only), and so
  are fenced code blocks (``` and ~~~, also inside > quotes), <pre>/<script>/<style>/<svg>/<math>
  blocks, HTML comments, '> [!NOTE]'-style markers and table separator rows.
- Reference macros ({{domxref("CSS.supports()")}}, {{jsxref("Array/forEach", "forEach")}},
  {{glossary("function", "functions")}}, {{HTMLElement("div")}}, {{HTTPHeader(..)}} ...) become
  their display text: the second argument when there is one, else the first. Every other macro
  ({{Specifications}}, {{Compat}}, sidebars, {{optional_inline}}, live samples) is removed.
- [text](url) becomes text, images and bare/auto-linked URLs are removed, HTML tags are stripped
  (their text kept), entities unescaped, and **, _ emphasis and backticks are removed (inline code
  text kept). A unit (paragraph, list item, cell) whose inline code looks like code (operators,
  braces, quotes, over 30 characters) is dropped, as is any sentence with leftover markup.
- Wrapped paragraph lines are joined; list items, definitions ('- : '), headings and table cells
  are separate units. Units are split into sentences at . ! ? followed by a space.

Outputs:

- ko.txt: sentences containing Hangul (at least 30% of their letters Hangul, 4+ characters, no
  leftover markup), deduplicated, in file order. Latin API names inside them are kept (map()을,
  CSS 속성을). An all-capitals acronym followed by a particle of the wrong allomorph for its
  letter-name reading (UI을 for UI를: the last letter's name decides, L/R 엘/알 and M 엠, N 엔
  end in a consonant, every other letter in a vowel)
  has its particle corrected in place, but only where the reading is sure: letters only, at most 5,
  and spelled out (ending in a vowel letter, or with no vowel letter before the last: API, HTTP,
  CSS) or ending in L/M/N (HTML, DOM). GET, ACK, IMAP, TYPE, ERR, MP3, INTEGER may be read as words
  and are left as written.
- mixed.txt: distinct tokens mixing Latin letters and Hangul seen in ko.txt (DOM을, JavaScript의,
  CSS로), most frequent first, at least 2 occurrences.
- en_words.tsv: Latin words seen inside the Korean sentences with their counts (API names, CSS/HTML
  terms): letters only, 2+ letters, lower, Capitalized or ALL-CAPS (camelCase identifiers such as
  getElementById are skipped), at least 2 occurrences.
- terms.tsv: glossary page titles: 문자 집합 (Character set) gives 문자 집합<TAB>Character set; a
  Korean title without a parenthesis takes the English from the page slug (출처 -> Origin).
- meta.json.

Usage: python3 -I mdn_ko.py
"""

import html
import json
import os
import re
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "mdn-ko", "src", "files", "ko")
OUT = os.path.join(ROOT, "corpora", "derived", "mdn-ko")

META = {
    "source": "mdn-ko",
    "tier": "moderate",
    "weight": 0.8,
    "uses": ["mixed-ko-en", "acronyms", "particles-endings", "korean-lexicon", "english-lexicon",
             "korean-key-model"],
    "license": "CC BY-SA 2.5 (prose); code samples CC0/MIT",
}

MAX_LINES = 3_000_000

HANGUL = re.compile("[가-힣]")
MACRO = re.compile(r"\{\{\s*([A-Za-z_][A-Za-z0-9_-]*)\s*(?:\((.*?)\))?\s*\}\}")
MACRO_ARG = re.compile(r'"((?:[^"\\]|\\.)*)"|\'((?:[^\'\\]|\\.)*)\'')
IMAGE = re.compile(r"!\[[^\]]*\]\([^)]*\)")
LINK = re.compile(r"\[([^\]]*)\]\((?:[^()]|\([^)]*\))*\)")
REFLINK = re.compile(r"\[([^\]]+)\]\[[^\]]*\]")
AUTOLINK = re.compile(r"<(?:https?|mailto|ftp):[^>]*>", re.I)
URL = re.compile(r"(?:https?://|ftp://|www\.)\S+", re.I)
COMMENT = re.compile(r"<!--.*?-->", re.S)
TAG = re.compile(r"</?[A-Za-z][A-Za-z0-9-]*(?:\s[^<>]*)?/?>")
CODE = re.compile(r"`+([^`]*)`+")
CODEY = re.compile(r"[{};=<>\"'$\\|]|=>|\s{2,}")
BOLD = re.compile(r"\*\*|__|(?<![A-Za-z0-9])[*_](?=\S)|(?<=\S)[*_](?![A-Za-z0-9])")
SENT = re.compile(r"(?<=[.!?])\s+(?=\S)")
LIST = re.compile(r"^(?:[-*+]|\d+[.)])\s+(?::\s+)?")
HEADING = re.compile(r"^#{1,6}\s+")
BLOCK_OPEN = re.compile(r"^<(pre|script|style|svg|math|textarea)\b", re.I)
FENCE = re.compile(r"^(```+|~~~+)")
LEFTOVER = re.compile(r"[{}<>`\\^]|\]\(|\|\s|^[:>#-]|&[a-z]+;")
LATIN = re.compile(r"[A-Za-z]+")
EN_WORD = re.compile(r"^(?:[a-z]+|[A-Z][a-z]+|[A-Z]+)$")
# A token mixing Latin and Hangul (DOM을, CSS로, JavaScript의, 웹API).
MIXED = re.compile(r"^(?=.*[A-Za-z])(?=.*[가-힣])[가-힣A-Za-z0-9&]+$")
# An all-capitals acronym (digits allowed after the first two capitals) then Hangul.
ACRONYM = re.compile(r"(?<![A-Za-z0-9])([A-Z]{2,}[A-Z0-9]*)([가-힣]+)")
EDGE = "\"'()[]{}<>.,:;!?“”‘’「」『』·…/"

VOWELS = set("AEIOU")
# Whole-tail particle pairs (after a consonant, after a vowel).
PAIRS = {"을": "를", "은": "는", "과": "와", "이": "가", "이나": "나", "이랑": "랑", "이며": "며"}
PAIRS_BACK = {v: k for k, v in PAIRS.items()}


def macro(m):
    name, args = m.group(1), m.group(2)
    low = name.lower()
    if args is None or not (low.endswith("xref") or low in {
            "glossary", "htmlelement", "httpheader", "httpstatus", "httpmethod", "svgelement",
            "svgattr", "htmlattrxref", "mathmlelement", "cssxref", "jsxref", "domxref",
            "webextapiref", "rfc", "event", "csp", "httpheader", "jsxref", "interface",
            "element", "domxref", "glossarydisambiguation", "deprecated_header"}):
        return ""
    vals = [a or b for a, b in MACRO_ARG.findall(args)]
    if not vals or low == "deprecated_header":
        return ""
    if low == "rfc":
        return "RFC " + vals[0]
    shown = vals[1] if len(vals) > 1 and vals[1] else vals[0]
    if len(vals) == 1 and low.endswith("xref") and "/" in shown:
        shown = shown.rsplit("/", 1)[-1]
    return shown


def inline(text):
    """Markdown/HTML inline text -> plain text, or None if it carries code."""
    text = COMMENT.sub(" ", text)
    text = MACRO.sub(macro, text)
    text = IMAGE.sub(" ", text)
    text = LINK.sub(r"\1", text)
    text = REFLINK.sub(r"\1", text)
    text = AUTOLINK.sub(" ", text)
    text = TAG.sub("", text)
    text = html.unescape(text)
    for code in CODE.findall(text):
        if len(code) > 30 or CODEY.search(code):
            return None
    text = CODE.sub(r"\1", text)
    text = BOLD.sub("", text)
    if URL.search(text):
        return None
    return " ".join(text.split())


def units(lines):
    """Prose units (paragraphs, list items, headings, table cells) of one page, front matter off."""
    i = 0
    if lines and lines[0].strip() == "---":
        i = 1
        while i < len(lines) and lines[i].strip() != "---":
            i += 1
        i += 1
    fence, block, para = None, None, []
    for raw in lines[i:]:
        line = raw.strip()
        quoted = line.startswith(">")
        body = line.lstrip("> ").strip() if quoted else line
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
        if block:
            if re.search(r"</%s\s*>" % block, body, re.I):
                block = None
            continue
        b = BLOCK_OPEN.match(body)
        if b:
            if not re.search(r"</%s\s*>" % b.group(1), body, re.I):
                block = b.group(1)
            if para:
                yield " ".join(para)
                para = []
            continue
        if not body or re.match(r"^\[!\w+\]$", body) or re.match(r"^\|?[\s:|-]+\|?$", body) and "-" in body:
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
        starts = HEADING.match(body) or LIST.match(body) or body.startswith(": ")
        if starts:
            if para:
                yield " ".join(para)
            body = re.sub(r"^:\s+", "", LIST.sub("", HEADING.sub("", body)))
            para = [body]
            if HEADING.match(line):
                yield " ".join(para)
                para = []
            continue
        para.append(body)
    if para:
        yield " ".join(para)


def reading(acr):
    """How an acronym's reading ends: "rieul" (ㄹ), "cons", "vowel", or None when unsure."""
    if not acr.isalpha() or len(acr) > 5:
        return None  # digits (MP3 엠피쓰리) and long caps words (INTEGER, DOCTYPE) vary
    last = acr[-1]
    if last in "LMN":
        return "rieul" if last == "L" else "cons"  # 엘/엠/엔, and as a word too (HTML, DOM, SVN)
    if last == "R":
        # USER, ERR read as words (유저, 에러); XHR spelled out
        return None if any(c in VOWELS for c in acr[:-1]) else "rieul"
    if last == "E" and len(acr) >= 3 and acr[-3] in VOWELS and acr[-2] not in VOWELS:
        return None  # silent e: TYPE 타입, MIME 마임, DONE 던
    if last in VOWELS or not any(c in VOWELS for c in acr[:-1]):
        return "vowel"  # spelled out: API, UI, HTTP, CSS, TLS
    return None  # maybe read as a word: GET 겟, PUT 풋, ACK 액, IMAP 아이맵


def fix_particle(m):
    """UI을 -> UI를: the particle allomorph that fits the acronym's letter-name reading."""
    acr, tail = m.group(1), m.group(2)
    r = reading(acr)
    if r is None:
        return m.group(0)
    cons = r != "vowel"
    if tail in PAIRS and not cons:
        return acr + PAIRS[tail]
    if tail in PAIRS_BACK and cons:
        return acr + PAIRS_BACK[tail]
    if tail.startswith("으로") and r != "cons":
        return acr + tail[1:]
    if tail.startswith("로") and r == "cons":
        return acr + "으" + tail
    return m.group(0)


def keep(sentence):
    if len(sentence) < 4 or LEFTOVER.search(sentence):
        return False
    hangul = len(HANGUL.findall(sentence))
    latin = sum(len(w) for w in LATIN.findall(sentence))
    return hangul > 0 and hangul >= 0.3 * (hangul + latin)


def pages():
    for dirpath, dirnames, filenames in os.walk(SRC):
        dirnames.sort()
        for name in sorted(filenames):
            if name == "index.md":
                yield os.path.join(dirpath, name)


def front(lines):
    """title and slug from the front matter."""
    title = slug = ""
    if lines and lines[0].strip() == "---":
        for line in lines[1:]:
            if line.strip() == "---":
                break
            if line.startswith("title:"):
                title = line[6:].strip().strip("\"'")
            elif line.startswith("slug:"):
                slug = line[5:].strip().strip("\"'")
    return title, slug


def main():
    seen, ko = set(), []
    mixed, en = Counter(), Counter()
    terms = {}
    fixed = dropped_code = 0
    for path in pages():
        with open(path, encoding="utf-8", errors="replace") as f:
            lines = f.read().split("\n")
        title, slug = front(lines)
        if slug.lower().startswith("glossary/") and HANGUL.search(title):
            m = re.match(r"^(.*?[가-힣].*?)\s*\(([^()가-힣]+)\)\s*$", title)
            if m:
                k, e = m.group(1).strip(), m.group(2).strip()
            else:
                k, e = title.strip(), slug.split("/", 1)[1].replace("_", " ")
            if k and "(" not in k and k not in terms:
                terms[k] = e
        for unit in units(lines):
            if not HANGUL.search(unit):
                continue
            text = inline(unit)
            if text is None:
                dropped_code += 1
                continue
            for s in SENT.split(text):
                s = s.strip()
                if not keep(s):
                    continue
                s2 = ACRONYM.sub(fix_particle, s)
                if s2 != s:
                    fixed += 1
                    s = s2
                if s in seen:
                    continue
                seen.add(s)
                ko.append(s)
                for tok in s.split():
                    tok = tok.strip(EDGE)
                    if MIXED.match(tok):
                        mixed[tok] += 1
                for w in re.findall(r"(?<![A-Za-z0-9_.\u00c0-\u024f])[A-Za-z]+(?![A-Za-z0-9_(\u00c0-\u024f])", s):
                    if len(w) >= 2 and EN_WORD.match(w):
                        en[w] += 1

    if len(ko) > MAX_LINES:
        k = -(-len(ko) // MAX_LINES)
        ko = ko[::k]
    os.makedirs(OUT, exist_ok=True)

    def write(name, rows):
        with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
            for row in rows:
                f.write(row + "\n")
        return len(rows)

    by_count = lambda c: sorted(((w, n) for w, n in c.items() if n >= 2), key=lambda kv: (-kv[1], kv[0]))
    files = {
        "ko.txt": write("ko.txt", ko),
        "mixed.txt": write("mixed.txt", [w for w, _ in by_count(mixed)]),
        "en_words.tsv": write("en_words.tsv", [f"{w}\t{n}" for w, n in by_count(en)]),
        "terms.tsv": write("terms.tsv", [f"{k}\t{e}" for k, e in sorted(terms.items())]),
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(dict(META, files=files), f, ensure_ascii=False, indent=1)
        f.write("\n")
    for name, n in files.items():
        print(f"{name}: {n}")
    print(f"acronym particles corrected: {fixed}; units dropped for code: {dropped_code}")


if __name__ == "__main__":
    main()
