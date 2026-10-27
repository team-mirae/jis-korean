"""Extracts the Korean pages of the Node.js website into corpora/derived/nodejs-site-ko/ for the predictor.

Source: nodejs/nodejs.org, the apps/site/pages/ko/ tree (corpora/open/nodejs-site-ko/src/apps/site/
pages/ko, commit in SOURCE.txt). License: MIT (src/LICENSE). Only cleaned sentences and token lists
derived from it are written; the predictor ships word counts, not the text.

What and how: walks every .md and .mdx file, drops the YAML front matter (keeping its title as a
line), fenced code, MDX import/export lines, HTML comments and <img>/<table> markup. Paired JSX
tags (<LinkWithArrow href=...>검증</LinkWithArrow>) are unwrapped to their text; a self-closing
component (<Release.VersionDropdown />) leaves a hole, so the sentence it sits in is dropped.
Markdown links ([text](url), [text][]) become their text, reference definitions go, inline `code`
is unwrapped when it is a plain identifier (`nodejs`, `libicu`) and otherwise poisons the sentence
(`child_process.fork()`, `<stream>`). Emphasis, headings, list bullets and blockquotes are stripped,
hard-wrapped lines are joined into paragraphs and split into sentences after 다./요.-style endings.
Lines without Hangul, with URLs or with leftover markup are dropped, and lines are deduplicated in
first-seen order.

Writes ko.txt (one sentence per line), mixed.txt (distinct Latin+Hangul tokens such as Node.js를,
TSC는, CVE가), terms.tsv (korean<TAB>english from 'X(Y)' glosses like 락(lock)) and meta.json.
Tiny (~200 sentences): moderate weight, best concatenated with the other tech-docs corpora or
held out as an eval set for mixed Latin+Hangul and acronym+particle prediction.

Usage: python3 -I nodejs_site_ko.py
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "nodejs-site-ko", "src", "apps", "site", "pages", "ko")
OUT = os.path.join(ROOT, "corpora", "derived", "nodejs-site-ko")

HANGUL = re.compile(r"[가-힣]")
POISON = "\x00"  # marks a sentence that lost content; dropped after splitting
URL = re.compile(r"https?://|www\.|\b[\w.-]+\.(?:com|org|io|net)/")
CODE_ID = re.compile(r"^[A-Za-z][A-Za-z0-9_.\-]*$")
SENT = re.compile(r"(?<=[가-힣][.!?])\s+|(?<=[가-힣][.!?][\"')\]])\s+")
MIXED = re.compile(r"(?<![A-Za-z0-9가-힣.+/\-])(?:[A-Za-z0-9][A-Za-z0-9./+\-]*[가-힣]+|[가-힣]+[A-Za-z][A-Za-z0-9]*)(?![A-Za-z0-9가-힣])")
GLOSS = re.compile(r"(?:([가-힣]+) )?([가-힣]+)\(([A-Za-z][A-Za-z .\-]*[A-Za-z])\)")
LEFTOVER = re.compile(r"[<>{}\[\]`|*_#=]|\]\(")


def md_files():
    for dirpath, _, names in sorted(os.walk(SRC)):
        for name in sorted(names):
            if name.endswith((".md", ".mdx")):
                yield os.path.join(dirpath, name)


def front_matter(text):
    """Returns (title or None, body)."""
    if not text.startswith("---\n"):
        return None, text
    end = text.find("\n---\n", 4)
    if end < 0:
        return None, text
    title = None
    for line in text[4:end].split("\n"):
        m = re.match(r"^title:\s*(.+)$", line)
        if m:
            title = m.group(1).strip().strip("'\"")
    return title, text[end + 5:]


def codespan(m):
    code = m.group(1).strip()
    return code if CODE_ID.match(code) else POISON


def inline(text):
    text = re.sub(r"<!--.*?-->", " ", text, flags=re.S)  # comments can span lines
    text = re.sub(r"<[A-Za-z][\w.]*\b[^<>]*/>", POISON, text)  # self-closing component: a hole
    text = re.sub(r"</?[A-Za-z][\w.]*\b[^<>]*>", "", text)  # paired tags: keep inner text
    text = re.sub(r"!\[[^\]]*\]\([^)]*\)", " ", text)  # images
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)  # [text](url)
    text = re.sub(r"\[([^\]]+)\]\[[^\]]*\]", r"\1", text)  # [text][] / [text][ref]
    text = re.sub(r"`([^`]*)`", codespan, text)
    text = re.sub(r"\*\*([^*]+)\*\*|__([^_]+)__", lambda m: m.group(1) or m.group(2), text)
    text = re.sub(r"(?<!\w)\*([^*\s][^*]*)\*(?!\w)", r"\1", text)
    text = text.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
    text = text.replace("&reg;", "").replace("®", "")  # nobody types the mark
    return re.sub(r"\s+", " ", text).strip()


def paragraphs(body):
    """Yields raw paragraphs (hard-wrapped lines joined) outside code, tables and markup blocks."""
    lines = body.replace("\r\n", "\n").split("\n")
    fence = None
    in_tag = 0  # inside a multi-line <img ... /> or <table>...</table>
    cur = []

    def flush():
        if cur:
            yield " ".join(cur)
            cur.clear()

    for line in lines:
        s = line.strip()
        if fence:
            if s.startswith(fence):
                fence = None
            continue
        m = re.match(r"^(`{3,}|~{3,})", s)
        if m:
            yield from flush()
            fence = m.group(1)
            continue
        if in_tag:
            if in_tag == 1 and s.endswith("/>"):
                in_tag = 0
            elif in_tag == 2 and s.startswith("</table>"):
                in_tag = 0
            continue
        if s.startswith("<table"):
            yield from flush()
            in_tag = 2
            continue
        if s.startswith("<img") and not s.endswith("/>") and not s.endswith(">"):
            yield from flush()
            in_tag = 1
            continue
        if (not s or re.match(r"^(?:import|export)\s", s) or re.match(r"^\[[^\]]+\]:\s", s)
                or re.match(r"^(?:-{3,}|\*{3,}|_{3,})$", s) or s.startswith("<img")):
            yield from flush()
            continue
        heading = re.match(r"^#{1,6}\s+(.*)$", s)
        bullet = re.match(r"^(?:[-*+]|\d+[.)])\s+(.*)$", s)
        s = re.sub(r"^(?:>\s*)+", "", s)
        if heading:
            yield from flush()
            yield heading.group(1)
            continue
        if bullet:
            yield from flush()
            s = bullet.group(1)
        # A standalone block tag line (<section>, </Release.Provider>) breaks a paragraph.
        if re.match(r"^</?[A-Za-z][\w.]*[^<>]*>$", s) and not re.search(r"/>$", s):
            yield from flush()
            continue
        cur.append(s)
    yield from flush()


def sentences(paragraph):
    text = inline(paragraph)
    for sent in SENT.split(text):
        sent = sent.strip()
        if not sent or POISON in sent or not HANGUL.search(sent):
            continue
        if URL.search(sent) or LEFTOVER.search(sent):
            continue
        sent = re.sub(r"\s+([.,!?)])", r"\1", sent)
        if len(sent) < 4:
            continue
        yield sent


def main():
    lines, seen = [], set()
    for path in md_files():
        with open(path, encoding="utf-8") as f:
            title, body = front_matter(f.read().replace("\r\n", "\n"))
        paras = ([title] if title else []) + list(paragraphs(body))
        for para in paras:
            for sent in sentences(para):
                if sent not in seen:
                    seen.add(sent)
                    lines.append(sent)

    mixed, mixed_seen = [], set()
    terms, term_seen = [], set()
    for sent in lines:
        for tok in MIXED.findall(sent):
            tok = tok.rstrip(".")
            if re.match(r"[가-힣]+[A-Z]?[a-z]{3}", tok):
                continue  # glued gloss (위원회Technical), not a typed token
            if re.search(r"[A-Za-z]", tok) and HANGUL.search(tok) and tok not in mixed_seen:
                mixed_seen.add(tok)
                mixed.append(tok)
        for prev, ko, en in GLOSS.findall(sent):
            if en.isupper():
                continue  # acronym expansions (FLOSS) aren't 1:1 with the word before them
            if " " in en and prev:
                ko = prev + " " + ko  # 차단 호출(blocking call)
            if (ko, en) not in term_seen:
                term_seen.add((ko, en))
                terms.append(ko + "\t" + en)

    os.makedirs(OUT, exist_ok=True)

    def write(name, rows):
        with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
            for row in rows:
                f.write(row + "\n")
        return len(rows)

    files = {"ko.txt": write("ko.txt", lines), "mixed.txt": write("mixed.txt", sorted(mixed))}
    if terms:
        files["terms.tsv"] = write("terms.tsv", terms)
    meta = {
        "source": "nodejs-site-ko",
        "tier": "moderate",
        "weight": 0.5,
        "uses": ["mixed-ko-en", "acronyms", "particles-endings", "eval-only"],
        "license": "MIT",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    for name, n in files.items():
        print(name, n)


if __name__ == "__main__":
    main()
