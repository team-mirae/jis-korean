"""Extracts the Korean Vue.js documentation into corpora/derived/vue-docs-ko/ for the predictor.

Source: vuejs-translations/docs-ko, the src/ tree (corpora/open/vue-docs-ko/src/src, commit in
SOURCE.txt). License: CC BY 4.0 (src/LICENSE). Only cleaned sentences and token lists derived
from it are written; the predictor ships word counts, not the text.

What and how: walks every .md file (VitePress), drops the YAML front matter, fenced and indented
code, HTML comments, <script>/<style> blocks and the ::: container fences (the text inside them
is kept; a container title such as '::: tip 참고' is dropped). Heading anchors {#...} and the
docs' '\\*' / '\\*\\*' API-style markers go, Markdown links become their text, images and
reference definitions go. Inline `code` is unwrapped when it is a plain identifier (`ref()` ->
ref, `v-if`, `<KeepAlive>` -> KeepAlive) so '`ref()`는' becomes 'ref는'; any other code span
(app.mount(), $emit, expressions) poisons the sentence, which is dropped. A particle spaced off a
code span or Latin word is rejoined. Plain HTML tags (span, div, a, br, ...) are stripped, but
side-by-side options-api/composition-api spans and VitePress {{ }} interpolations poison the
sentence, as do Vue component tags left in running text. Emphasis, headings, list bullets,
blockquotes are stripped, tables become one line per cell, hard-wrapped lines are joined into
paragraphs and split into sentences after ./!/? and after a ':' that ends a clause ('...됩니다:'). Lines
with fewer than two Hangul syllables or under 30% Hangul (of letters), with URLs or with leftover
markup/code are dropped, and lines are deduplicated in first-seen order.

Writes ko.txt (one sentence per line, ~4K), mixed.txt (distinct Latin+Hangul tokens such as
ref는, API를, SFC에서), terms.tsv (korean<TAB>english from 'X(Y)' glosses such as
컴포넌트(component) and 서버 사이드 렌더링(SSR), most frequent first) and meta.json. Moderate weight: narrow front-end
register, so Vue-specific words must not inflate the general lexicon.

Usage: python3 -I vue_docs_ko.py
"""

import json
import os
import re
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
BASE = os.path.join(ROOT, "corpora", "open", "vue-docs-ko")
SRC = os.path.join(BASE, "src", "src")
OUT = os.path.join(ROOT, "corpora", "derived", "vue-docs-ko")

HOLE = "\x00"  # marks something the sentence can't do without; such sentences are dropped
MARK = ""  # brackets unwrapped code/link/emphasis text so a spaced-off particle rejoins it
HANGUL = re.compile(r"[가-힣]")
HANGUL2 = re.compile(r"[가-힣].*[가-힣]")

FENCE = re.compile(r"^\s*(```+|~~~+)")
CONTAINER = re.compile(r"^\s*:::")
COMMENT = re.compile(r"<!--.*?-->", re.S)
SCRIPT_STYLE = re.compile(r"<(script|style)\b[^>]*>.*?</\1\s*>", re.S | re.I)
SIDE_BY_SIDE = re.compile(r"</span>\s*<span\b", re.I)
INTERP = re.compile(r"\{\{.*?\}\}", re.S)
IMAGE = re.compile(r"!\[[^\]]*\]\([^()]*(?:\([^()]*\)[^()]*)*\)")
LINK = re.compile(r"\[([^\[\]]*)\]\([^()]*(?:\([^()]*\)[^()]*)*\)")
REFLINK = re.compile(r"\[([^\[\]]*)\]\[[^\[\]]*\]")
REFDEF = re.compile(r"^\s*\[[^\]]+\]:\s*\S+")
CODESPAN = re.compile(r"`+([^`]*)`+")
AUTOLINK = re.compile(r"<(?:https?|ftp|mailto):[^<>]*>")
STRONG = re.compile(r"\*\*([^*]+)\*\*|__([^_]+)__")
EM = re.compile(r"(?<![\w*])\*([^*\s][^*]*?)\*|(?<![\w])_([^_\s][^_]*?)_(?![A-Za-z0-9])")
# `ref()`, `v-if`, `data`, `<KeepAlive>`: unwrapped (without the () or <>).
PLAIN_IDENT = re.compile(r"^<?([A-Za-z][A-Za-z0-9_]*(?:-[A-Za-z0-9_]+)*)(?:\(\)|\s*/?>)?$")
HTML_TAG = re.compile(
    r"</?(?:span|div|a|br|p|b|i|em|strong|small|sup|sub|code|kbd|u|s|img|hr|abbr|mark)\b[^<>]*>",
    re.I,
)
URL = re.compile(r"(?:https?://|ftp://|www\.)\S+|\S+@\S+\.\S+|\S+\.(?:io|com|org|net|dev|sh|js)\b")
ANCHOR = re.compile(r"\s*\{#[^}]*\}\s*$")
API_MARK = re.compile(r"\s*(?:\\\*)+\s*")
HEADING = re.compile(r"^\s{0,3}#{1,6}\s*")
BULLET = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s+")
TABLE_SEP = re.compile(r"^\s*\|?\s*:?-{2,}:?\s*(?:\|\s*:?-{2,}:?\s*)*\|?\s*$")
EMPH = re.compile(r"\*\*|__|(?<![\w*])\*(?=\S)|(?<=\S)\*(?![\w*])|(?<![\w])_(?=[^\s_])|(?<=[^\s_])_(?![\w])")
PARTICLE = (
    "이|가|은|는|을|를|의|에|에서|에게|로|으로|와|과|도|만|이나|나|이다|이며|이고|이므로|라는|이라는|"
    "로서|으로서|로써|부터|까지|보다|처럼|만큼|란|이란|라고|이라고|이어야|여야|에는|에도|에서는|로는|"
    "으로는|와는|과는|이라|라|인|일|이면|면|입니다|이지만|로도|으로도|에만|만을|이든|든지"
)
SPACED_PARTICLE = re.compile("(" + MARK + "[^" + MARK + "]*" + MARK + ")[ \t ]+(" + PARTICLE + r")(?=[\s.,:;!?)\]]|$)")
LATIN_PARTICLE = re.compile(r"([A-Za-z0-9)\"])[ \t ]+(을|를|은|는|의|가|에|에서|에는|로|으로|와|과|도|만|라는|이라는)(?=[\s.,:;!?)]|$)")
COMMAND = re.compile(r"^(?:npm|npx|yarn|pnpm|vue|git|cd|node)\s+[a-z]")
# Sentence ends: after . ! ? and after a ':' that closes a Hangul clause ('...됩니다:').
SENT = re.compile(r"(?<=[.!?])\s+(?=\S)|(?<=[다요음함임]:)\s+(?=\S)")
LEFTOVER = re.compile(r"\{\{|\}\}|\]\(|\[\[|\]\]|<|>|\\|`|\$|&&|\|\||==|=>|->|\{|\}|::|;|\(\)")
MIXED = re.compile(r"(?<![A-Za-z0-9가-힣.+\-])(?:[A-Za-z0-9][A-Za-z0-9.+\-]*[가-힣]+|[가-힣]+[A-Za-z][A-Za-z0-9]*)(?![A-Za-z0-9가-힣])")
# 'X(Y)' glosses: the Korean side is as many words as the English has ('계산된 속성(computed
# property)'), or one word per letter of an acronym ('서버 사이드 렌더링(SSR)', at most 4).
GLOSS = re.compile(r"((?:[가-힣]+ ){0,3}[가-힣]{2,12})\s?\(([A-Za-z][A-Za-z0-9 .\-/]{1,40}?)\)")
ACRONYM = re.compile(r"^[A-Z][A-Z0-9]{1,3}$")


def md_files():
    for dirpath, dirnames, filenames in os.walk(SRC):
        dirnames.sort()
        for name in sorted(filenames):
            if name.endswith(".md"):
                yield os.path.join(dirpath, name)


def drop_front_matter(lines):
    if not lines or lines[0].strip() != "---":
        return lines
    for end in range(1, len(lines)):
        if lines[end].strip() == "---":
            return lines[end + 1:]
    return lines


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
        if CONTAINER.match(line):
            out.append("")
            continue
        # Indented code: 4+ spaces (or a tab) and no Hangul. Indented Korean is list continuation.
        if re.match(r"^(?: {4,}|\t)", line) and not HANGUL.search(line):
            out.append("")
            continue
        out.append(line)
    return out


def codespan(m):
    inner = m.group(1).strip()
    p = PLAIN_IDENT.match(inner)
    if p:
        return MARK + p.group(1) + MARK
    return HOLE


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
        if stripped.startswith("|") or stripped.count("|") >= 2 and "`" not in stripped:
            yield from flush()
            for cell in stripped.strip("|").split("|"):
                yield cell.strip()
            continue
        stripped = re.sub(r"^(?:>\s?)+", "", stripped)
        if HEADING.match(stripped):
            yield from flush()
            h = ANCHOR.sub("", HEADING.sub("", stripped).rstrip("#").strip())
            yield API_MARK.sub(" ", h).strip()
            continue
        if BULLET.match(stripped):
            yield from flush()
            stripped = BULLET.sub("", stripped)
        para.append(stripped)
    yield from flush()


def hangul_share(s):
    letters = [c for c in s if c.isalpha()]
    if not letters:
        return 0.0
    return sum(1 for c in letters if "가" <= c <= "힣") / len(letters)


def clean(paragraph):
    """One paragraph -> clean sentences."""
    text = AUTOLINK.sub(HOLE, paragraph)
    text = IMAGE.sub("", text)
    text = CODESPAN.sub(codespan, text)
    text = LINK.sub(lambda m: MARK + m.group(1) + MARK, text)
    text = REFLINK.sub(lambda m: MARK + m.group(1) + MARK, text)
    text = STRONG.sub(lambda m: MARK + (m.group(1) or m.group(2)) + MARK, text)
    text = EM.sub(lambda m: MARK + (m.group(1) or m.group(2)) + MARK, text)
    text = SPACED_PARTICLE.sub(r"\1\2", text)
    text = text.replace(MARK, "")
    text = LATIN_PARTICLE.sub(r"\1\2", text)
    text = SIDE_BY_SIDE.sub(HOLE, text)
    text = INTERP.sub(HOLE, text)
    text = HTML_TAG.sub("", text)
    text = EMPH.sub("", text).replace("**", "")
    text = text.replace("&nbsp;", " ").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
    text = re.sub(r"\s+", " ", text).strip()
    for sent in SENT.split(text):
        sent = sent.strip(" -|")
        if sent.endswith(":"):
            sent = sent[:-1].rstrip()
        if not sent or HOLE in sent or URL.search(sent) or LEFTOVER.search(sent):
            continue
        if not HANGUL2.search(sent) or hangul_share(sent) < 0.3:
            continue
        if re.search(r"(?:^|\s)--?[a-z]", sent) or re.search(r"\b\w+=\S", sent) or COMMAND.search(sent):
            continue
        yield sent


def main():
    seen, lines, terms = set(), [], Counter()
    nfiles = 0
    for path in md_files():
        nfiles += 1
        with open(path, encoding="utf-8", errors="replace") as f:
            raw = f.read().replace("\r\n", "\n")
        raw = SCRIPT_STYLE.sub("\n", COMMENT.sub("\n", raw))
        body = drop_front_matter(raw.split("\n"))
        text = "\n".join(strip_code_blocks(body))
        for para in blocks(text):
            for sent in clean(para):
                if sent not in seen:
                    seen.add(sent)
                    lines.append(sent)

    mixed, mixed_seen = [], set()
    for sent in lines:
        for m in GLOSS.finditer(sent):
            ko_words, en = m.group(1).split(), m.group(2).strip()
            n = len(ko_words[-1]) and (len(en) if ACRONYM.match(en) else len(en.split()))
            if n <= len(ko_words) and n <= 4:
                terms[(" ".join(ko_words[-n:]), en)] += 1
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
        "source": "vue-docs-ko",
        "tier": "moderate",
        "weight": 0.7,
        "uses": ["mixed-ko-en", "acronyms", "particles-endings", "korean-key-model", "korean-lexicon"],
        "license": "CC BY 4.0",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(nfiles, "files ->", files)


if __name__ == "__main__":
    main()
