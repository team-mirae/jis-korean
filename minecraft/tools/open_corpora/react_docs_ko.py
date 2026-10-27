"""Extracts the Korean React documentation into corpora/derived/react-docs-ko/ for the predictor.

Source: reactjs/ko.react.dev, the src/content/ tree (corpora/open/react-docs-ko/src/src/content,
commit in SOURCE.txt). License: CC BY 4.0 for the docs (LICENSE-DOCS.md). Only cleaned sentences
and token lists derived from it are written; the predictor ships word counts, not the text.

What and how: walks every .md file except versions.md, errors/ and warnings/ (tiny, mostly
templates). Drops the YAML front matter (keeping its title and description values as lines),
fenced code, <TerminalBlock> bodies and lines that are JSX tags (<Sandpack>, <Note>, <Intro>, a
multi-line <YouTubeIframe ... /> up to its closing '>'); the prose inside <Note>, <Pitfall>,
<DeepDive>, <Solution> and the like is kept. Heading anchors {/*...*/} go, Markdown links become
their text, images go, inline JSX tags (<CodeStep step={1}>) are stripped. Inline `code` is
unwrapped only when it is a plain word or acronym (`key`, `ref`, `null`, `JSX`, `React`, or a
number); camelCase or punctuated code (`useState`, `roomId`, `? :`) poisons the sentence, which
is dropped rather than left with a hole. A particle the docs space off after a code span or a
Latin word ('`state` 를', 'React 는') is rejoined. Emphasis, headings, list bullets, blockquotes
and table pipes are stripped, hard-wrapped lines are joined into paragraphs and split into
sentences. Sentences without two Hangul syllables, with a Hangul share under 0.3 of their letters,
with URLs or with leftover markup/code are dropped, and the rest deduplicated in first-seen order.

Writes ko.txt (one sentence per line, Korean tech prose with English terms such as React, JSX,
API, props), mixed.txt (distinct Latin+Hangul tokens such as JSX를, props는, API를), en_words.tsv
(Latin words seen inside the kept Korean sentences, with counts: React, state, props, Hook),
terms.tsv (korean<TAB>english from 'X(Y)' glosses such as 순수 함수(pure function), most frequent
first) and meta.json. Moderate weight: the register is technical and words such as 컴포넌트,
렌더링 must not inflate the general lexicon.

Usage: python3 -I react_docs_ko.py
"""

import json
import os
import re
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
BASE = os.path.join(ROOT, "corpora", "open", "react-docs-ko")
SRC = os.path.join(BASE, "src", "src", "content")
OUT = os.path.join(ROOT, "corpora", "derived", "react-docs-ko")
SKIP_DIRS = {"errors", "warnings"}
SKIP_FILES = {"versions.md"}

HOLE = "\x00"  # marks something the sentence can't do without; such sentences are dropped
MARK = ""  # brackets text that a spaced-off particle should rejoin
HANGUL = re.compile(r"[가-힣]")
HANGUL2 = re.compile(r"[가-힣].*[가-힣]")
LETTER = re.compile(r"[A-Za-z가-힣ㄱ-ㅎㅏ-ㅣ]")

FENCE = re.compile(r"^\s*(```+|~~~+)")
COMMENT = re.compile(r"<!--.*?-->", re.S)
JSX_COMMENT = re.compile(r"\{/\*.*?\*/\}", re.S)
JSX_OPEN = re.compile(r"^\s*</?[A-Za-z][\w.]*")
DROP_BODY = {"TerminalBlock", "Sandpack", "SandpackWithHTMLOutput", "ConsoleBlock", "ConsoleBlockMulti", "CodeDiagram"}
IMAGE = re.compile(r"!\[[^\]]*\]\([^()]*(?:\([^()]*\)[^()]*)*\)")
LINK = re.compile(r"\[([^\[\]]*)\]\([^()]*(?:\([^()]*\)[^()]*)*\)")
REFLINK = re.compile(r"\[([^\[\]]*)\]\[[^\[\]]*\]")
REFDEF = re.compile(r"^\s*\[[^\]]+\]:\s*\S+")
CODESPAN = re.compile(r"`+([^`]*)`+")
AUTOLINK = re.compile(r"<(?:https?|ftp|mailto):[^<>]*>")
STRONG = re.compile(r"\*\*([^*]+)\*\*|__([^_]+)__")
EM = re.compile(r"(?<![\w*])\*([^*\s][^*]*?)\*|(?<![\w])_([^_\s][^_]*?)_(?![A-Za-z0-9])")
# A plain word or acronym: lower, UPPER or Capitalized, not camelCase; or a plain number.
PLAIN_CODE = re.compile(r"^(?:[a-z]{1,12}|[A-Z]{1,12}|[A-Z][a-z]{1,11}|\d{1,6})$")
TAG = re.compile(r"</?[A-Za-z][\w.]*(?:\s[^<>]*)?/?>|<>|</>")
URL = re.compile(r"(?:https?://|ftp://|www\.)\S+|\S+@\S+\.\S+|\S+\.(?:io|com|org|net|dev|sh|js|ts)\b")
HEADING = re.compile(r"^\s{0,3}#{1,6}\s*")
BULLET = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s+")
TABLE_SEP = re.compile(r"^\s*\|?\s*:?-{2,}:?\s*(?:\|\s*:?-{2,}:?\s*)*\|?\s*$")
EMPH = re.compile(r"\*\*|__|(?<![\w*])\*(?=\S)|(?<=\S)\*(?![\w*])|(?<![\w])_(?=[^\s_])|(?<=[^\s_])_(?![\w])")
PARTICLE = (
    "이|가|은|는|을|를|의|에|에서|에게|로|으로|와|과|도|만|이나|나|이다|이며|이고|이므로|라는|이라는|"
    "로서|으로서|로써|부터|까지|보다|처럼|만큼|란|이란|라고|이라고|이어야|여야|에는|에도|에서는|로는|"
    "으로는|와는|과는|이라|라|인|일|이면|면|입니다|이지만|로도|으로도|에만|만을|이든|든지"
)
SPACED_PARTICLE = re.compile("(" + MARK + "[^" + MARK + "]*" + MARK + r")[ \t ]+(" + PARTICLE + r")(?=[\s.,:;!?)\]]|$)")
LATIN_PARTICLE = re.compile(r"([A-Za-z0-9)\"])[ \t ]+(을|를|은|는|의|가|에|에서|에는|로|으로|와|과|도|만|라는|이라는)(?=[\s.,:;!?)]|$)")
SENT = re.compile(r"(?<=[.!?])\s+(?=\S)")
LEFTOVER = re.compile(r"\]\(|\[\[|\]\]|<|>|\\|`|\$\(|&&|\|\||==|=>|->|\{|\}|::|;|\(\)")
MIXED = re.compile(r"(?<![A-Za-z0-9가-힣.+\-])(?:[A-Za-z0-9][A-Za-z0-9.+\-]*[가-힣]+|[가-힣]+[A-Za-z][A-Za-z0-9]*)(?![A-Za-z0-9가-힣])")
LATIN_WORD = re.compile(r"(?<![A-Za-z0-9_.\-])[A-Za-z][A-Za-z]*(?:'[a-z]+)?(?![A-Za-z0-9_])")
# One or two Hangul words of 2+ syllables starting a word: '순수 함수(pure function)'.
GLOSS = re.compile(r"(?:^|(?<=[\s\"'“‘]))([가-힣]{2,10}(?: [가-힣]{2,10})?)\s?\(([A-Za-z][A-Za-z .\-]{1,40}?)\)")


def md_files():
    for dirpath, dirnames, filenames in os.walk(SRC):
        if os.path.normpath(dirpath) == os.path.normpath(SRC):
            dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        dirnames.sort()
        for name in sorted(filenames):
            if name.endswith(".md") and name not in SKIP_FILES:
                yield os.path.join(dirpath, name)


def front_matter(lines):
    """Splits off the YAML front matter; returns (title/description values, body lines)."""
    if not lines or lines[0].strip() != "---":
        return [], lines
    for end in range(1, len(lines)):
        if lines[end].strip() == "---":
            break
    else:
        return [], lines
    keep = []
    for line in lines[1:end]:
        m = re.match(r"^(title|description):\s*(.+)$", line)
        if m:
            keep.append(m.group(2).strip().strip("'\""))
    return keep, lines[end + 1:]


def strip_blocks(lines):
    """Blanks fenced code, JSX tag lines (also multi-line tags) and code-only JSX bodies."""
    out, fence, in_tag, drop_until = [], None, False, None
    for line in lines:
        m = FENCE.match(line)
        if fence:
            if m and line.strip().startswith(fence) and not line.strip()[len(fence):].strip():
                fence = None
            out.append("")
            continue
        if m:
            fence = m.group(1)
            out.append("")
            continue
        if in_tag:  # attribute lines of a tag opened on an earlier line
            if ">" in line:
                in_tag = False
            out.append("")
            continue
        if drop_until:
            if re.match(r"^\s*</" + drop_until + r"\s*>", line):
                drop_until = None
            out.append("")
            continue
        t = JSX_OPEN.match(line)
        if t:
            name = t.group(0).strip().lstrip("</")
            if name in DROP_BODY and not t.group(0).strip().startswith("</") and not line.rstrip().endswith("/>"):
                if not re.search(r"</" + name + r"\s*>", line):
                    drop_until = name
            if ">" not in line:
                in_tag = True
            out.append("")
            continue
        out.append(line)
    return out


def blocks(body):
    """Yields paragraphs (one logical line each) from cleaned markdown lines."""
    para = []

    def flush():
        if para:
            yield " ".join(para)
            para.clear()

    for raw in body.split("\n"):
        line = raw.rstrip()
        if not line.strip() or REFDEF.match(line) or TABLE_SEP.match(line):
            yield from flush()
            continue
        stripped = line.strip()
        if stripped.startswith("|"):  # a table row
            yield from flush()
            for cell in stripped.strip("|").split("|"):
                yield cell.strip()
            continue
        stripped = re.sub(r"^(?:>\s?)+", "", stripped)
        if HEADING.match(stripped):
            yield from flush()
            yield HEADING.sub("", stripped).rstrip("#").strip()
            continue
        if BULLET.match(stripped):
            yield from flush()
            stripped = BULLET.sub("", stripped)
        para.append(stripped)
    yield from flush()


def codespan(m):
    inner = m.group(1).strip()
    if PLAIN_CODE.match(inner):
        return MARK + inner + MARK
    return HOLE


def clean(paragraph):
    """One paragraph -> clean sentences."""
    text = AUTOLINK.sub(HOLE, paragraph)
    text = IMAGE.sub("", text)
    text = LINK.sub(lambda m: MARK + m.group(1) + MARK, text)
    text = REFLINK.sub(lambda m: MARK + m.group(1) + MARK, text)
    text = CODESPAN.sub(codespan, text)
    text = TAG.sub("", text)
    text = STRONG.sub(lambda m: MARK + (m.group(1) or m.group(2)) + MARK, text)
    text = EM.sub(lambda m: MARK + (m.group(1) or m.group(2)) + MARK, text)
    text = SPACED_PARTICLE.sub(r"\1\2", text)
    text = text.replace(MARK, "")
    text = LATIN_PARTICLE.sub(r"\1\2", text)
    text = EMPH.sub("", text).replace("**", "")
    text = text.replace("&nbsp;", " ").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
    text = re.sub(r"\s+", " ", text).strip()
    for sent in SENT.split(text):
        sent = sent.strip(" -:|")
        if not sent or HOLE in sent or URL.search(sent) or LEFTOVER.search(sent):
            continue
        if not HANGUL2.search(sent):
            continue
        letters = LETTER.findall(sent)
        if len(HANGUL.findall(sent)) < 0.3 * len(letters):
            continue
        yield sent


def main():
    seen, lines = set(), []
    nfiles = 0
    for path in md_files():
        nfiles += 1
        with open(path, encoding="utf-8", errors="replace") as f:
            raw = f.read().replace("\r\n", "\n").split("\n")
        meta, body = front_matter(raw)
        text = "\n".join(body)
        text = JSX_COMMENT.sub("", COMMENT.sub("\n", text))
        text = "\n".join(strip_blocks(text.split("\n")))
        for para in meta + list(blocks(text)):
            for sent in clean(para):
                if sent not in seen:
                    seen.add(sent)
                    lines.append(sent)

    terms, mixed, en = Counter(), set(), Counter()
    for sent in lines:
        for m in GLOSS.finditer(sent):
            terms[(m.group(1).strip(), m.group(2).strip())] += 1
        for tok in MIXED.findall(sent):
            tok = tok.rstrip(".-+")
            if re.search(r"[A-Za-z]", tok) and HANGUL.search(tok):
                mixed.add(tok)
        for w in LATIN_WORD.findall(sent):
            en[w] += 1

    os.makedirs(OUT, exist_ok=True)

    def write(name, rows):
        with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
            for row in rows:
                f.write(row + "\n")
        return len(rows)

    files = {
        "ko.txt": write("ko.txt", lines),
        "mixed.txt": write("mixed.txt", sorted(mixed)),
        "en_words.tsv": write("en_words.tsv", [f"{w}\t{n}" for w, n in sorted(en.items(), key=lambda kv: (-kv[1], kv[0]))]),
        "terms.tsv": write(
            "terms.tsv",
            [ko + "\t" + e for (ko, e), _ in sorted(terms.items(), key=lambda kv: (-kv[1], kv[0]))],
        ),
    }
    meta = {
        "source": "react-docs-ko",
        "tier": "moderate",
        "weight": 0.8,
        "uses": ["korean-key-model", "mixed-ko-en", "acronyms", "particles-endings", "korean-lexicon", "english-nouns"],
        "license": "CC BY 4.0",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(nfiles, "files ->", files)


if __name__ == "__main__":
    main()
