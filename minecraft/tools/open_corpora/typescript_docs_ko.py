"""Extracts the Korean TypeScript handbook into corpora/derived/typescript-docs-ko/ for the predictor.

Source: typescript-kr/typescript-kr.github.io, the pages/ tree (corpora/open/typescript-docs-ko/
src/pages, commit in SOURCE.txt). License: MIT (LICENSE.md). Only cleaned sentences and token lists
derived from it are written; the predictor ships word counts, not the text.

What and how: walks every .md file under src/pages (some names have spaces), drops the YAML front
matter (keeping title/oneline only when they contain Hangul), fenced code, indented code, HTML
comments and markdown table rows. Markdown links become their text, images and reference
definitions go, HTML tags (<br/>, <a>, ...) are removed. Inline `code` is unwrapped when it is a
plain identifier (`Animal`, `strictNullChecks`, `Array.prototype`), so '`Animal`의' becomes
'Animal의', the identifier+particle signal; any other code span (types, paths, expressions,
/// directives) poisons its sentence, which is dropped. The handbook's habit of spacing a particle
off a code span or emphasis ('*타입 변수* 를') is undone ('타입 변수를'). Emphasis, headings,
list bullets and blockquotes are stripped, hard-wrapped lines are joined into paragraphs and split
into sentences (after . ! ? and after a colon that ends a clause). Lines without Hangul (the
English-only release notes, tutorials), with URLs or with leftover markup/code are dropped, and
lines are deduplicated in first-seen order.

Writes ko.txt (one sentence per line), mixed.txt (distinct Latin+Hangul tokens such as
Animal의, JSDoc을, tsconfig.json에서), terms.tsv (korean<TAB>english from the 'X (Y)' headings,
'# 소개 (Introduction)', and 'X(Y)' glosses in the text, most frequent first) and meta.json.
Moderate weight: technical register, and TypeScript words (제네릭, 네임스페이스) must not inflate
the general lexicon.

Usage: python3 -I typescript_docs_ko.py
"""

import json
import os
import re
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
BASE = os.path.join(ROOT, "corpora", "open", "typescript-docs-ko")
SRC = os.path.join(BASE, "src", "pages")
OUT = os.path.join(ROOT, "corpora", "derived", "typescript-docs-ko")

HOLE = "\x00"  # marks something the sentence can't do without; such sentences are dropped
MARK = ""  # brackets a code span / link / emphasis so a spaced-off particle can rejoin it
HANGUL = re.compile(r"[가-힣]")
HANGUL2 = re.compile(r"[가-힣].*[가-힣]")

FENCE = re.compile(r"^\s*(```+|~~~+)")
COMMENT = re.compile(r"<!--.*?-->", re.S)
IMAGE = re.compile(r"!\[[^\]]*\]\([^()]*(?:\([^()]*\)[^()]*)*\)")
LINK = re.compile(r"\[([^\[\]]*)\]\([^()]*(?:\([^()]*\)[^()]*)*\)")
REFLINK = re.compile(r"\[([^\[\]]*)\]\[[^\[\]]*\]")
REFDEF = re.compile(r"^\s*\[[^\]]+\]:\s*\S+")
CODESPAN = re.compile(r"(`+)(.+?)\1")
AUTOLINK = re.compile(r"<(?:https?|ftp|mailto):[^<>]*>")
STRONG = re.compile(r"\*\*([^*]+)\*\*|__([^_]+)__")
EM = re.compile(r"(?<![\w*])\*([^*\s][^*]*?)\*|(?<![\w])_([^_\s][^_]*?)_(?![A-Za-z0-9])")
PLAIN_IDENT = re.compile(r"^[A-Za-z_$][A-Za-z0-9_$]*(?:\.[A-Za-z_$][A-Za-z0-9_$]*)*$")
TAG = re.compile(r"</?[A-Za-z][^<>]*>")
URL = re.compile(r"(?:https?://|ftp://|www\.)\S+|\S+@\S+\.\S+|\S+\.(?:io|com|org|net|dev|sh)\b")
HEADING = re.compile(r"^\s{0,3}#{1,6}\s*")
BULLET = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s+")
EMPH = re.compile(r"\*\*|__|(?<![\w*])\*(?=\S)|(?<=\S)\*(?![\w*])|(?<![\w])_(?=[^\s_])|(?<=[^\s_])_(?![\w])")
PARTICLE = (
    "이|가|은|는|을|를|의|에|에서|에게|로|으로|와|과|도|만|이나|나|이다|이며|이고|이므로|라는|이라는|"
    "로서|으로서|로써|부터|까지|보다|처럼|만큼|란|이란|라고|이라고|이어야|여야|에는|에도|에서는|로는|"
    "으로는|와는|과는|이라|라|인|일|이면|면|입니다|이지만|로도|으로도|에만|만을|이든|든지|입니다|"
    "이기|이기도|이거나|거나|에서도|에게는|으로써|이자|이며|처럼|같이"
)
SPACED_PARTICLE = re.compile("(" + MARK + "[^" + MARK + "]*" + MARK + ")[ \t ]+(" + PARTICLE + r")(?=[\s.,:;!?)\]]|$)")
LATIN_PARTICLE = re.compile(r"([A-Za-z0-9)\"])[ \t ]+(을|를|은|는|의|가|에|에서|에는|로|으로|와|과|도|만|라는|이라는)(?=[\s.,:;!?)]|$)")
# A sentence break: after . ! ? (not inside a number or dotted name), or after a colon ending a clause.
SENT = re.compile(r"(?<=[.!?])\s+(?=\S)|(?<=[가-힣]):\s+(?=\S)")
LEFTOVER = re.compile(r"\]\(|\[\[|\]\]|<|>|\\|`|\||==|=>|->|\{|\}|::|;|//|\$\{|\.\.?/|↥|\[\]")
MIXED = re.compile(r"(?<![A-Za-z0-9가-힣.+\-])(?:[A-Za-z0-9][A-Za-z0-9.+\-]*[가-힣]+|[가-힣]+[A-Za-z][A-Za-z0-9]*)(?![A-Za-z0-9가-힣])")
GLOSS = re.compile(r"(?<![가-힣])([가-힣]{2,12})\s?\(([A-Za-z][A-Za-z0-9 .\-/]{1,40}?)\)")
TITLE_TERM = re.compile(r"^([가-힣][가-힣0-9 ]{0,30}?)\s*\(([A-Za-z][A-Za-z0-9 .,'\-/]{1,60})\)\s*$")


def md_files():
    for dirpath, dirnames, filenames in os.walk(SRC):
        dirnames.sort()
        for name in sorted(filenames):
            if name.endswith(".md"):
                yield os.path.join(dirpath, name)


def front_matter(lines):
    """Splits off the YAML front matter; returns (Hangul values worth keeping, body lines)."""
    if not lines or lines[0].strip() != "---":
        return [], lines
    for end in range(1, len(lines)):
        if lines[end].strip() == "---":
            break
    else:
        return [], lines
    keep = []
    for line in lines[1:end]:
        m = re.match(r"^(title|oneline):\s*(.*)$", line)
        if m and HANGUL.search(m.group(2)):
            keep.append(m.group(2).strip().strip("'\""))
    return keep, lines[end + 1:]


def strip_code_blocks(lines):
    out, fence = [], None
    for line in lines:
        m = FENCE.match(line)
        if fence:
            if m and m.group(1)[0] == fence[0] and len(m.group(1)) >= len(fence) and not line.strip()[len(m.group(1)):].strip():
                fence = None
            out.append("")
            continue
        if m:
            fence = m.group(1)
            out.append("")
            continue
        # Indented code: 4+ spaces (or a tab) that isn't a list continuation of Korean prose.
        if re.match(r"^(?: {4,}|\t)", line) and (not HANGUL.search(line) or LEFTOVER.search(line)
                                                  or re.search(r"[=;(){}]\s*$|^\s*(?:let|const|var|function|return|class|interface|type)\b", line)):
            out.append("")
            continue
        out.append(line)
    return out


def inline(text):
    text = IMAGE.sub("", text)
    text = LINK.sub(lambda m: MARK + m.group(1) + MARK, text)
    text = REFLINK.sub(lambda m: MARK + m.group(1) + MARK, text)
    return text


def codespan(m):
    inner = m.group(2).strip()
    if PLAIN_IDENT.match(inner):
        return MARK + inner + MARK
    return HOLE


def blocks(body):
    """Yields paragraphs (already one logical line each) from cleaned markdown lines."""
    para = []

    def flush():
        if para:
            yield " ".join(para)
            para.clear()

    for raw in body.split("\n"):
        line = raw.rstrip()
        stripped = line.strip()
        if not stripped or REFDEF.match(line):
            yield from flush()
            continue
        if stripped.startswith("|") or " | " in stripped:  # a table row: skipped
            yield from flush()
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


def clean(paragraph):
    """One paragraph -> clean sentences."""
    text = AUTOLINK.sub(HOLE, paragraph)
    text = re.sub(r"<br\s*/?>", " ", text, flags=re.I)
    text = CODESPAN.sub(codespan, text)
    text = STRONG.sub(lambda m: MARK + (m.group(1) or m.group(2)) + MARK, text)
    text = EM.sub(lambda m: MARK + (m.group(1) or m.group(2)) + MARK, text)
    text = SPACED_PARTICLE.sub(r"\1\2", text)
    text = text.replace(MARK, "")
    # Same habit in plain text after a Latin word ('number 를'); not '이', which is also 'this'.
    text = LATIN_PARTICLE.sub(r"\1\2", text)
    text = TAG.sub("", text)
    text = EMPH.sub("", text).replace("**", "")
    text = text.replace("&nbsp;", " ").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
    text = re.sub(r"\s+", " ", text).strip()
    for sent in SENT.split(text):
        sent = sent.strip(" -:|")
        sent = re.sub(r"^\[\d+\]\s*", "", sent)  # footnote markers
        if not sent or HOLE in sent or URL.search(sent) or LEFTOVER.search(sent):
            continue
        if not HANGUL2.search(sent):
            continue
        if re.search(r"(?:^|\s)--?[a-z]", sent) or re.search(r"\b\w+=\S", sent):
            continue
        yield sent


def main():
    seen, lines, terms = set(), [], Counter()
    nfiles = 0
    for path in md_files():
        nfiles += 1
        with open(path, encoding="utf-8", errors="replace") as f:
            raw = f.read().replace("\r\n", "\n").split("\n")
        meta, body = front_matter(raw)
        text = COMMENT.sub("\n", "\n".join(strip_code_blocks(body)))
        for m in re.finditer(r"(?m)^\s{0,3}#{1,6}\s*(.+?)\s*#*\s*$", text):
            tm = TITLE_TERM.match(m.group(1).replace("`", "").replace("*", ""))
            if tm:
                terms[(tm.group(1).strip(), tm.group(2).strip())] += 1
        text = inline(text)
        for para in [inline(v) for v in meta] + list(blocks(text)):
            for sent in clean(para):
                if sent not in seen:
                    seen.add(sent)
                    lines.append(sent)

    mixed, mixed_seen = [], set()
    for sent in lines:
        for m in GLOSS.finditer(sent):
            # One Hangul word glosses one English word ('제네릭(Generics)'); '타입 단언(type
            # assertion)' would otherwise pair 단언 with the whole phrase. Headings carry those.
            if " " not in m.group(2).strip():
                terms[(m.group(1), m.group(2).strip())] += 1
        for tok in MIXED.findall(sent):
            tok = tok.rstrip(".-+")
            if re.search(r"[A-Za-z]", tok) and HANGUL.search(tok) and tok not in mixed_seen:
                mixed_seen.add(tok)
                mixed.append(tok)

    os.makedirs(OUT, exist_ok=True)

    def write(name, rows):
        with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
            for row in rows:
                f.write(row + "\n")
        return len(rows)

    files = {
        "ko.txt": write("ko.txt", lines),
        "mixed.txt": write("mixed.txt", sorted(mixed)),
        "terms.tsv": write(
            "terms.tsv",
            [ko + "\t" + en for (ko, en), _ in sorted(terms.items(), key=lambda kv: (-kv[1], kv[0]))],
        ),
    }
    meta = {
        "source": "typescript-docs-ko",
        "tier": "moderate",
        "weight": 0.8,
        "uses": ["mixed-ko-en", "korean-key-model", "korean-lexicon", "acronyms", "particles-endings"],
        "license": "MIT",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(nfiles, "files ->", files)


if __name__ == "__main__":
    main()
