"""Extracts Firefox's Korean UI strings into corpora/derived/firefox-l10n-ko/ for the predictor.

Source: mozilla-l10n/firefox-l10n, the ko/ tree (corpora/open/firefox-l10n-ko/src/ko, commit in
SOURCE.txt). License: MPL-2.0 (weak copyleft). Only cleaned strings and token lists derived from
it are written; the predictor ships word counts, not the text.

What and how: walks every .ftl (Fluent) and .properties file (skipping .js/.css/.ini/.md), drops
comments, access keys, style/size attributes and Fluent term definitions, and takes each value
(after ' = ', indented continuations, select variants without their '[x]' selector). Simple
Fluent terms ({ -brand-short-name } -> Firefox) and NSIS $Brand* variables are substituted with
their brand text. Variables ({ $x }, %S, %1$S, %d, #1, $0, ...) are deleted together with any
Hangul run attached right after them (particle or counter), so 'S를' or '개를' never surfaces.
Tags, access-key markers '(&X)', escapes and smart quotes go; strings that still look like code,
or have fewer than 2 Hangul syllables, are dropped. Multi-sentence strings are split, then lines
are deduplicated in first-seen order.

Writes ko.txt (one string or sentence per line), mixed.txt (distinct Latin+Hangul tokens such as
Firefox를, VPN에) and meta.json. Low weight: formal UI register, much repetition.

Usage: python3 -I firefox_l10n_ko.py
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
SRC = os.path.join(ROOT, "corpora", "open", "firefox-l10n-ko", "src", "ko")
OUT = os.path.join(ROOT, "corpora", "derived", "firefox-l10n-ko")

SKIP_KEY = re.compile(r"\.(accesskey|style|width|height|key|commandkey)\s*$", re.I)
HANGUL = re.compile(r"[\uac00-\ud7a3]")
HOLE = "\x00"  # stands in for a deleted variable until its attached Hangul run is removed

TERM_REF = re.compile(r"\{\s*(-[a-z0-9-]+)\s*\}")
PLACEABLE = re.compile(r"\{[^{}]*\}")
PRINTF = re.compile(r"%(?:\d+\$)?[-#+ 0]*\d*(?:\.\d+)?(?:l{0,2})[SsdiufxXc@]|%%")
NSIS_BRAND = re.compile(r"\$Brand[A-Za-z]*")
NSIS_VAR = re.compile(r"\$(?:\{[^}]*\}|[A-Za-z_][A-Za-z0-9_]*|\d+)")
PLURAL_NUM = re.compile(r"#\d+")
TAG = re.compile(r"</?[A-Za-z][^<>]*>")
URL = re.compile(r"(?:https?://|ftp://|www\.)\S+|\S+@\S+\.\S+")
ACCESS = re.compile(r"\(&\w\)|&(?=\w)")
SMART = re.compile(r"[\u201c\u201d\u2018\u2019\u00ab\u00bb\u300c\u300d]")
CODEY = re.compile(r"[<>{}\\=|`]|&\w+;|->|\b\w+\(\)|::")
# Counters and units that measure a deleted number ('{ $n } 번의' -> '').
COUNTER = r"(?:개|번|분|초|일|시간|주|개월|달|년|명|장|건|회|자|바이트|비트|줄|단계|배|퍼센트|%|[KMGT]i?B|kB|ms|px)[\uac00-\ud7a3]*"
# Developer identifiers (postMessage, mozGetAll, Element.setHTML, foo()) mark API messages, not prose.
IDENT = re.compile(r"\b[a-z]{2,}[A-Z]|[A-Za-z]\.[a-z]|\w\(|_\w|(?:[A-Z][a-z]+){3,}")
SENT = re.compile(r"(?<=[.!?])\s+(?=\S)")
MIXED = re.compile(r"[A-Za-z0-9][A-Za-z0-9.+\-]*[\uac00-\ud7a3]+|[\uac00-\ud7a3]+[A-Za-z][A-Za-z0-9]*")


def files():
    for dirpath, dirnames, filenames in os.walk(SRC):
        dirnames.sort()
        for name in sorted(filenames):
            if name.endswith((".ftl", ".properties")):
                yield os.path.join(dirpath, name)


def read(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        return f.read().replace("\r\n", "\n").split("\n")


def simple_terms():
    """Fluent terms whose value is one plain line: -brand-short-name -> Firefox."""
    terms = {}
    for path in files():
        if not path.endswith(".ftl"):
            continue
        for line in read(path):
            m = re.match(r"^(-[a-z0-9-]+)\s*=\s*(\S.*)$", line)
            if m and "{" not in m.group(2):
                terms.setdefault(m.group(1), m.group(2).strip())
    return terms


def ftl_values(lines):
    skip = False  # inside an attribute or term we don't want
    term = False
    for line in lines:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if not line[0].isspace():  # a new message or term
            term = line.startswith("-")
            if term:
                skip = True
                continue
            key, sep, value = line.partition("=")
            if not sep:
                continue
            skip = bool(SKIP_KEY.search(key))
            if not skip:
                yield value.strip()
            continue
        s = line.strip()
        m = re.match(r"^\.([A-Za-z0-9_-]+)\s*=\s*(.*)$", s)
        if m:
            skip = term or bool(SKIP_KEY.search("." + m.group(1)))
            if not skip:
                yield m.group(2).strip()
            continue
        if skip:
            continue
        s = re.sub(r"^\*?\[[^\]]*\]\s*", "", s)
        yield s


def properties_values(lines):
    for line in lines:
        s = line.strip()
        if not s or s[0] in "#!":
            continue
        key, sep, value = s.partition("=")
        if not sep or SKIP_KEY.search(key):
            continue
        value = value.strip()
        if len(value) >= 2 and value[0] == value[-1] == '"':
            value = value[1:-1]
        value = re.sub(r"\\u([0-9a-fA-F]{4})", lambda m: chr(int(m.group(1), 16)), value)
        value = value.replace("\\r", " ").replace("\\n", " ").replace("\\t", " ").replace("\\", "")
        for part in value.split(";"):  # plural forms
            yield part


def clean(value, terms):
    if not HANGUL.search(value):
        return []
    s = TERM_REF.sub(lambda m: terms.get(m.group(1), HOLE), value)
    s = re.sub(r"\{\s*[^{}]*->\s*$", HOLE, s)  # opening of a select expression
    s = PLACEABLE.sub(HOLE, s)
    s = s.replace("{", HOLE).replace("}", HOLE)
    s = NSIS_BRAND.sub("Firefox", s)
    s = NSIS_VAR.sub(HOLE, s)
    s = PRINTF.sub(HOLE, s)
    s = PLURAL_NUM.sub(HOLE, s)
    s = TAG.sub("", s)
    s = re.sub(r"([.!?])(?=[\uac00-\ud7a3])", r"\1 ", s)  # "</p><p>" glued two sentences
    s = URL.sub(HOLE, s)
    s = ACCESS.sub("", s)
    s = SMART.sub("", s)
    s = s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", '"').replace("&amp;", "&")
    s = s.replace("&nbsp;", " ").replace("\u00a0", " ").replace("\u200b", "")
    # A Hangul run glued to a deleted variable is its particle or counter: drop it too.
    s = re.sub(HOLE + r"(?:[\uac00-\ud7a3]|\([\uac00-\ud7a3/]+\))+", HOLE, s)
    s = re.sub(r"[\uac00-\ud7a3]+" + HOLE, HOLE, s)
    s = re.sub(HOLE + r"[\s\-(]*" + COUNTER + r"\)?", HOLE, s)
    # Wrapping brackets or quotes around a deleted variable go with it.
    s = re.sub(r"[\"'(\[]\s*" + HOLE + r"\s*[\"')\]]", HOLE, s)
    s = s.replace(HOLE, " ")
    s = re.sub(r"\s+", " ", s).strip()
    s = re.sub(r"\s+([.,!?:;)])", r"\1", s)
    s = re.sub(r"\([\s\-,]*\)|\[\s*\]|\"\s*\"", "", s)
    s = re.sub(r"^[\s,.:;\-\u2013\u2014/]+", "", s)
    s = re.sub(r"\s+", " ", s).strip()
    if CODEY.search(s) or IDENT.search(s):
        return []
    out = []
    for sent in SENT.split(s):
        sent = sent.strip()
        if len(HANGUL.findall(sent)) >= 2:
            out.append(sent)
    return out


def main():
    terms = simple_terms()
    seen, lines = set(), []
    for path in files():
        raw = read(path)
        values = ftl_values(raw) if path.endswith(".ftl") else properties_values(raw)
        for value in values:
            for sent in clean(value, terms):
                if sent not in seen:
                    seen.add(sent)
                    lines.append(sent)
    mixed = []
    mseen = set()
    for line in lines:
        for tok in MIXED.findall(line):
            tok = tok.strip(".-+")
            latin = re.sub(r"[^A-Za-z]", "", tok)
            if len(latin) < 2 or tok.count("-") >= 2 or IDENT.search(tok):
                continue  # one stray letter, or an HTTP header name like Access-Control-Allow-Origin
            if HANGUL.search(tok) and tok not in mseen:
                mseen.add(tok)
                mixed.append(tok)
    os.makedirs(OUT, exist_ok=True)
    outputs = {"ko.txt": lines, "mixed.txt": sorted(mixed)}
    for name, rows in outputs.items():
        with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
            for row in rows:
                f.write(row + "\n")
    meta = {
        "source": "firefox-l10n-ko",
        "tier": "moderate",
        "weight": 0.7,
        "uses": ["korean-lexicon", "mixed-ko-en", "acronyms", "particles-endings", "korean-key-model"],
        "license": "MPL-2.0 (mozilla-l10n/firefox-l10n, ko/)",
        "files": {name: len(rows) for name, rows in outputs.items()},
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(json.dumps(meta["files"]))


if __name__ == "__main__":
    main()
