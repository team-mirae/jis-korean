"""Extracts the Korean Kubernetes documentation into corpora/derived/kubernetes-docs-ko/ for the predictor.

Source: kubernetes/website, the content/ko/ tree (corpora/open/kubernetes-docs-ko/src/content/ko,
commit in SOURCE.txt). License: CC BY 4.0 (src/LICENSE). Only cleaned sentences and token lists
derived from it are written; the predictor ships word counts, not the text.

What and how: walks every .md file, drops the YAML front matter (keeping its title, linktitle,
description and short_description values as lines), fenced and indented code, HTML comments and
{{< comment >}} blocks. Hugo shortcodes: glossary_tooltip becomes its text="..." value, block
shortcodes (note, caution, tabs, heading, include, feature-state, ...) are removed, and value
shortcodes (skew, param, latest-version, ...) poison the sentence they sit in, which is dropped.
Markdown links become their text, images and reference definitions go, inline `code` is unwrapped
when it is a plain identifier (`spec`, `binaryData`) and otherwise poisons the sentence (paths,
flags, commands, key=value). The docs' habit of spacing a particle off a code span ('`spec` 이')
is undone ('spec이'). Emphasis, headings, list bullets, blockquotes and HTML tags are stripped,
table rows become one line per cell, hard-wrapped lines are joined into paragraphs and split into
sentences. Lines without Hangul, with URLs or with leftover markup/code are dropped, and lines are
deduplicated in first-seen order (includes/ and the glossary repeat text).

Writes ko.txt (one sentence per line), mixed.txt (distinct Latin+Hangul tokens such as API를,
kubelet은, 1MiB를), terms.tsv (korean<TAB>english from 'X (원문, Y)' glossary titles and 'X(Y)'
glosses in the text, most frequent first) and meta.json. Moderate weight: technical register, and
k8s-specific words (파드, 쿠버네티스, 레플리카셋) must not inflate the general lexicon.

Usage: python3 -I kubernetes_docs_ko.py
"""

import json
import os
import re
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
BASE = os.path.join(ROOT, "corpora", "open", "kubernetes-docs-ko")
SRC = os.path.join(BASE, "src", "content", "ko")
OUT = os.path.join(ROOT, "corpora", "derived", "kubernetes-docs-ko")

HOLE = "\x00"  # marks something the sentence can't do without; such sentences are dropped
HANGUL = re.compile(r"[가-힣]")
HANGUL2 = re.compile(r"[가-힣].*[가-힣]")

FENCE = re.compile(r"^\s*(```|~~~)")
COMMENT = re.compile(r"<!--.*?-->", re.S)
SC_COMMENT = re.compile(r"\{\{[<%]\s*comment\s*[>%]\}\}.*?\{\{[<%]\s*/comment\s*[>%]\}\}", re.S)
TOOLTIP = re.compile(r"\{\{[<%]\s*glossary_tooltip\b([^{}]*?)[>%]\}\}", re.S)
TEXT_ARG = re.compile(r"\btext\s*=\s*\"([^\"]*)\"")
SHORTCODE = re.compile(r"\{\{[<%]\s*(/?)\s*([A-Za-z_][\w-]*)?[^{}]*?[>%]\}\}", re.S)
# Shortcodes that only open/close a box or insert a whole block: removed, the sentence survives.
BLOCK = {
    "note", "caution", "warning", "tab", "tabs", "table", "heading", "include", "codenew",
    "code_sample", "code", "feature-state", "figure", "version-check", "thirdparty-content",
    "glossary_definition", "alert", "example", "details", "api-reference", "mermaid", "toc",
    "cncf-landscape", "dockershim-removal", "legacy-repos-deprecation", "highlight", "link",
    "blocks/section", "eol-releases", "doc-versions-list", "release-data",
}
IMAGE = re.compile(r"!\[[^\]]*\]\([^()]*(?:\([^()]*\)[^()]*)*\)")
LINK = re.compile(r"\[([^\[\]]*)\]\([^()]*(?:\([^()]*\)[^()]*)*\)")
REFLINK = re.compile(r"\[([^\[\]]*)\]\[[^\[\]]*\]")
REFDEF = re.compile(r"^\s*\[[^\]]+\]:\s*\S+")
CODESPAN = re.compile(r"`+([^`]*)`+")
AUTOLINK = re.compile(r"<(?:https?|ftp|mailto):[^<>]*>")
STRONG = re.compile(r"\*\*([^*]+)\*\*|__([^_]+)__")
EM = re.compile(r"(?<![\w*])\*([^*\s][^*]*?)\*|(?<![\w])_([^_\s][^_]*?)_(?![A-Za-z0-9])")
PLAIN_IDENT = re.compile(r"^[A-Za-z][A-Za-z0-9_]*(?:-[A-Za-z0-9_]+)*$")
TAG = re.compile(r"</?[A-Za-z][^<>]*>")
URL = re.compile(r"(?:https?://|ftp://|www\.)\S+|\S+@\S+\.\S+|\S+\.(?:io|com|org|net|dev|sh)\b")
ANCHOR = re.compile(r"\s*\{#[^}]*\}\s*$")
HEADING = re.compile(r"^\s{0,3}#{1,6}\s*")
BULLET = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s+")
TABLE_SEP = re.compile(r"^\s*\|?\s*:?-{2,}:?\s*(?:\|\s*:?-{2,}:?\s*)*\|?\s*$")
EMPH = re.compile(r"\*\*|__|(?<![\w*])\*(?=\S)|(?<=\S)\*(?![\w*])|(?<![\w])_(?=[^\s_])|(?<=[^\s_])_(?![\w])")
# A particle the docs space off after a code span: '`spec` 이' -> 'spec이'.
PARTICLE = (
    "이|가|은|는|을|를|의|에|에서|에게|로|으로|와|과|도|만|이나|나|이다|이며|이고|이므로|라는|이라는|"
    "로서|으로서|로써|부터|까지|보다|처럼|만큼|란|이란|라고|이라고|이어야|여야|에는|에도|에서는|로는|"
    "으로는|와는|과는|이라|라|인|일|이면|면|이므로|입니다|이지만|로도|으로도|에만|만을|이든|든지"
)
SPACED_PARTICLE = re.compile("(\uE000[^\uE000]*\uE000)[ \t\u00a0]+(" + PARTICLE + r")(?=[\s.,:;!?)\]]|$)")
LATIN_PARTICLE = re.compile(r"([A-Za-z0-9)\"])[ \t\u00a0]+(을|를|은|는|의|가|에|에서|에는|로|으로|와|과|도|만|라는|이라는)(?=[\s.,:;!?)]|$)")
# A shell line with a trailing comment that escaped the code-block filter.
COMMAND = re.compile(r"\s#\s|^(?:kubectl|kubeadm|sudo|docker|curl|helm|minikube|git|apt-get|yum|cat|echo)\s+[a-z]")
SENT = re.compile(r"(?<=[.!?])\s+(?=\S)")
LEFTOVER = re.compile(r"\{\{|\}\}|\]\(|\[\[|\]\]|<|>|\\|`|\$\(|&&|\|\||==|:=|=>|->|\{|\}|::|;")
MIXED = re.compile(r"(?<![A-Za-z0-9가-힣.+\-])(?:[A-Za-z0-9][A-Za-z0-9.+\-]*[가-힣]+|[가-힣]+[A-Za-z][A-Za-z0-9]*)(?![A-Za-z0-9가-힣])")
GLOSS = re.compile(r"(?<![가-힣])([가-힣]{2,12})\s?\(([A-Za-z][A-Za-z0-9 .\-/]{1,40}?)\)")
TITLE_TERM = re.compile(r"^([가-힣][가-힣 ]{0,20}?)\s*\((?:원문,\s*)?([A-Za-z][A-Za-z0-9 .\-/]{1,40})\)\s*$")


def md_files():
    for dirpath, dirnames, filenames in os.walk(SRC):
        dirnames.sort()
        for name in sorted(filenames):
            if name.endswith(".md"):
                yield os.path.join(dirpath, name)


def front_matter(lines):
    """Splits off the YAML front matter; returns (values worth keeping, body lines)."""
    if not lines or lines[0].strip() != "---":
        return [], lines
    for end in range(1, len(lines)):
        if lines[end].strip() == "---":
            break
    else:
        return [], lines
    keep, i = [], 1
    while i < end:
        m = re.match(r"^(title|linktitle|description|short_description|summary):\s*(.*)$", lines[i])
        i += 1
        if not m:
            continue
        value = m.group(2).strip()
        if value in (">", "|", ">-", "|-", ">+", "|+"):
            parts = []
            while i < end and (lines[i].startswith((" ", "\t")) or not lines[i].strip()):
                parts.append(lines[i].strip())
                i += 1
            value = " ".join(p for p in parts if p)
        keep.append((m.group(1), value.strip("'\"")))
    return keep, lines[end + 1:]


def strip_code_blocks(lines):
    out, fence = [], None
    for line in lines:
        m = FENCE.match(line)
        if fence:
            if m and m.group(1) == fence:
                fence = None
            out.append("")
            continue
        if m:
            fence = m.group(1)
            out.append("")
            continue
        # Indented code: 4+ spaces (or a tab) and no Hangul. Indented Korean is list continuation.
        if re.match(r"^(?: {4,}|\t)", line) and not HANGUL.search(line):
            out.append("")
            continue
        out.append(line)
    return out


def shortcode(m):
    closing, name = m.group(1), (m.group(2) or "").lower()
    if name in BLOCK or name.startswith("blocks/"):
        return "\n" if not closing else "\n"
    return HOLE


def inline(text):
    # Tooltip and link text is marked like a code span so a spaced-off particle rejoins it.
    def tip(m):
        t = TEXT_ARG.search(m.group(1))
        return "\uE000" + t.group(1) + "\uE000" if t else HOLE

    text = TOOLTIP.sub(tip, text)
    text = IMAGE.sub("", text)
    text = LINK.sub(lambda m: "\uE000" + m.group(1) + "\uE000", text)
    text = REFLINK.sub(lambda m: "\uE000" + m.group(1) + "\uE000", text)
    text = SHORTCODE.sub(shortcode, text)
    return text


def codespan(m):
    inner = m.group(1).strip()
    if PLAIN_IDENT.match(inner) and "--" not in inner:
        return "\uE000" + inner + "\uE000"
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
        if not line.strip() or REFDEF.match(line) or TABLE_SEP.match(line):
            yield from flush()
            continue
        stripped = line.strip()
        if "|" in stripped:  # a table row, with or without the outer pipes
            yield from flush()
            for cell in stripped.strip("|").split("|"):
                yield cell.strip()
            continue
        stripped = re.sub(r"^(?:>\s?)+", "", stripped)
        if HEADING.match(stripped):
            yield from flush()
            yield ANCHOR.sub("", HEADING.sub("", stripped).rstrip("#").strip())
            continue
        if BULLET.match(stripped):
            yield from flush()
            stripped = BULLET.sub("", stripped)
        para.append(stripped)
    yield from flush()


def clean(paragraph):
    """One paragraph -> clean sentences."""
    text = AUTOLINK.sub(HOLE, paragraph)
    text = CODESPAN.sub(codespan, text)
    # Emphasis is marked like a code span so a spaced-off particle rejoins it ('_앱_ 의' -> '앱의').
    text = STRONG.sub(lambda m: "\uE000" + (m.group(1) or m.group(2)) + "\uE000", text)
    text = EM.sub(lambda m: "\uE000" + (m.group(1) or m.group(2)) + "\uE000", text)
    text = SPACED_PARTICLE.sub(r"\1\2", text)
    text = text.replace("\uE000", "")
    # Same habit in plain text after a Latin word ('kubelet 은'); not '이', which is also 'this'.
    text = LATIN_PARTICLE.sub(r"\1\2", text)
    text = TAG.sub("", text)
    text = EMPH.sub("", text).replace("**", "")
    text = text.replace("&nbsp;", " ").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
    text = re.sub(r"\s+", " ", text).strip()
    for sent in SENT.split(text):
        sent = sent.strip(" -:|")
        if not sent or HOLE in sent or URL.search(sent) or LEFTOVER.search(sent):
            continue
        if not HANGUL2.search(sent):
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
            raw = f.read().replace("\r\n", "\n").split("\n")
        meta, body = front_matter(raw)
        for key, value in meta:
            if key in ("title", "linktitle"):
                m = TITLE_TERM.match(value)
                if m:
                    terms[(m.group(1).strip(), m.group(2).strip())] += 1
        text = "\n".join(strip_code_blocks(body))
        text = SC_COMMENT.sub("\n", COMMENT.sub("\n", text))
        # Glossary hover text names terms the way the docs write them: '시크릿(Secret)'.
        for m in TOOLTIP.finditer(text):
            t = TEXT_ARG.search(m.group(1))
            if t:
                tm = TITLE_TERM.match(t.group(1))
                if tm:
                    terms[(tm.group(1).strip(), tm.group(2).strip())] += 1
        text = inline(text)
        paragraphs = [inline(v) for _, v in meta] + list(blocks(text))
        for para in paragraphs:
            for sent in clean(para):
                if sent not in seen:
                    seen.add(sent)
                    lines.append(sent)

    mixed, mixed_seen = [], set()
    for sent in lines:
        for m in GLOSS.finditer(sent):
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
        "source": "kubernetes-docs-ko",
        "tier": "moderate",
        "weight": 0.7,
        "uses": ["korean-key-model", "mixed-ko-en", "acronyms", "particles-endings", "korean-lexicon"],
        "license": "CC BY 4.0",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(nfiles, "files ->", files)


if __name__ == "__main__":
    main()
