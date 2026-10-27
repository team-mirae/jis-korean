"""Extracts the Wikimedia dumps (corpora/open/wikimedia-dumps, dumps.wikimedia.org, dumps dated
2026-10-01; CC BY-SA 4.0 and GFDL 1.3, texts saved alongside) into
corpora/derived/wikimedia-dumps/ for build_ranker.py. Only derived sentences and tokens are used.

Read:
- kowiki-latest-pages-articles.xml.bz2 (Korean Wikipedia) and simplewiki-latest-pages-articles
  .xml.bz2 (Simple English Wikipedia), streamed through bz2 and xml.etree.iterparse. Only
  namespace-0 pages that are not redirects (#넘겨주기 / #REDIRECT) are kept.
- kowiki-latest-all-titles-in-ns0.gz: titles that are an acronym plus a short Korean noun
  (AA전지, KB국민은행) join mixed.txt.
Not read: kowiktionary (glosses), kowikisource (archaic and literary text), kowikibooks /
kowikinews / kowikiquote (tiny, many bot and category pages), and enwiki's title list (names, not
typed text; acronyms already come from kowiki's prose).

Wikitext stripping (stdlib regexes, run in a process pool): comments, <ref>, <gallery> and
similar blocks, nested {{templates}} and {| tables |}, file / category / interwiki links are
removed; <math>, <chem>, <code>, <pre> and <syntaxhighlight> leave a marker that drops their
sentence (it would read "허수란 형태, 즉 ..." without them); [[a|b]] becomes b and [[a]] a;
[url label] becomes label; bold and italic quotes and leftover HTML tags go, entities are
unescaped. Headings, list, indent and table lines are dropped (fragments, not prose). Parentheses
holding hanja, kana or another non-Latin script (서울(漢陽)), or a language label ((영어: ...)),
are removed, as are parentheses left empty; a sentence whose parenthesis
still shows a hole ((; Ulster-Scots: )) is dropped. Paragraphs are split into sentences after . ! ?
followed by a space, except after an initial (에드워드 M. 케네디) or a common abbreviation.

Sentence filters: NFC, whitespace normalized, 10 to 200 characters (English: 20 to 300), no
leftover markup characters ({ } | [ ] < > =), no URL or e-mail, no remaining hanja/kana/Cyrillic/
Greek/Arabic/Hebrew/Thai. Korean sentences need Hangul to make up at least 60% of their letters,
must end in . ! ? (closing quotes allowed) and must not start with a bare particle (에 따라 ...,
left by a removed template). English ones: no Hangul, mostly Latin letters, a capital or digit
first and . ! ? last.

- ko.txt: kowiki sentences (Latin words, acronyms and digits kept), exact-deduped, sampled down to
  CAP lines by a hash of the sentence (the same sentence is always kept or always dropped, so the
  sample stays deduplicated and the run is deterministic).
- en.txt: simplewiki sentences, exact-deduped, sampled the same way if over CAP.
- mixed.txt: distinct tokens of a Latin letter run followed by Hangul (GPU를, KB국민카드의,
  Korea라고) seen at least MIN_MIXED times in all of kowiki's cleaned prose, plus the title list's
  acronym + short noun titles; sorted.
- No ko_words.tsv / en_words.tsv (the sentences carry the frequencies), no particles.txt (no
  morphology) and no terms.tsv (no bilingual pairs).

Usage: python3 -I wikimedia_dumps.py
"""

import bz2
import collections
import gzip
import hashlib
import html
import json
import multiprocessing
import os
import re
import unicodedata
import xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "wikimedia-dumps")
OUT = os.path.join(ROOT, "corpora", "derived", "wikimedia-dumps")

KOWIKI = "kowiki-latest-pages-articles.xml.bz2"
SIMPLEWIKI = "simplewiki-latest-pages-articles.xml.bz2"
KO_TITLES = "kowiki-latest-all-titles-in-ns0.gz"

CAP = 3_000_000
# Korean candidates are kept by hash below this share before the final trim to CAP, so the
# temporary file stays small (kowiki yields far more than CAP clean sentences).
KO_PREKEEP = 0.5
MIN_MIXED = 2
BATCH = 200  # pages per worker task
WORKERS = max(1, min(8, (os.cpu_count() or 2) - 2))

# --- Wikitext stripping ---

COMMENT = re.compile(r"<!--.*?(?:-->|$)", re.S)
# Formulas and code leave a hole in the sentence: they become NUL so the sentence is dropped.
HOLES = re.compile(r"<(math|chem|ce|code|pre|syntaxhighlight|source|score)\b[^>]*?(?:/>|>.*?</\1\s*>)",
                   re.S | re.I)
BLOCKS = re.compile(
    r"<(ref|gallery|timeline|graph|mapframe|"
    r"imagemap|templatedata|templatestyles|hiero|poem|table|div|nowiki|references)"
    r"\b[^>]*?(?:/>|>.*?</\1\s*>)", re.S | re.I)
SELF_TAG = re.compile(r"<(ref|references|br|hr)\b[^>]*/?>", re.I)
TEMPLATE = re.compile(r"\{\{[^{}]*\}\}")
TABLE = re.compile(r"\{\|(?:(?!\{\|).)*?\|\}", re.S)
FILE_LINK = re.compile(
    r"\[\[\s*(?:파일|File|그림|Image|이미지|분류|Category|미디어|Media)\s*:[^\[\]]*\]\]", re.I)
INTERWIKI = re.compile(r"\[\[\s*:?[a-z]{2,3}(?:-[a-z]+)?\s*:[^\[\]]*\]\]")
LINK = re.compile(r"\[\[([^\[\]|]*)(?:\|([^\[\]]*))?\]\]")
EXT_LINK = re.compile(r"\[(?:https?:)?//[^\s\]]+(?:\s+([^\]]*))?\]")
TAG = re.compile(r"</?[A-Za-z][^<>]*>")
QUOTES = re.compile(r"'{2,}")
MAGIC = re.compile(r"__[A-Z]+__")

FOREIGN = r"\u0370-\u04ff\u0590-\u06ff\u0e00-\u0e7f\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff"
FOREIGN_CHAR = re.compile(f"[{FOREIGN}]")
FOREIGN_PAREN = re.compile(rf"\s*\([^()]*[{FOREIGN}][^()]*\)")
LANG_PAREN = re.compile(r"\s*\((?:[가-힣]{1,8}어|[A-Za-z]{2,10})\s*:[^()]*\)")
EMPTY_PAREN = re.compile(r"\s*\([\s,;:·/\-–—]*\)")
PAREN_LEAD = re.compile(r"\(\s*(?:[,;:·]\s*)+")
# What a removed template leaves inside a parenthesis that still has text: (; Ulster-Scots: ).
PAREN_HOLE = re.compile(r"\([^()]*(?:[,;:–]|또는|\bor)\s*\)|\([^()]*[;:]\s*[;,]|\(\s*(?:또는|or\b)\s")
LEFTOVER = re.compile("[{}|\\[\\]<>=\0]")
URL = re.compile(r"https?://|www\.|\.(com|net|org|kr|co)\b|[\w.-]+@[\w-]+\.", re.I)
# Not after an initial (에드워드 M. 케네디) or a common abbreviation.
SENT_SPLIT = re.compile(r"(?<=[.!?])(?<![^A-Za-z][A-Z]\.)(?<!\b(?:Mr|Dr|St|vs|Jr|Sr|Ms|ed)\.)(?<!\be\.g\.)(?<!\bi\.e\.)\s+(?=\S)")
# A sentence that starts with a particle lost its subject to a removed template or formula.
PARTICLE_START = re.compile(r"(?:에|에서|에게|은|는|을|를|의|와|과|로|으로|으로서|으로써|도|만|이다|이며)[\s,]")
END = re.compile(r"[.!?][\"'”’)」』]*$")
HANGUL = re.compile(r"[가-힣ㄱ-ㅣ]")
LATIN = re.compile(r"[A-Za-z]")
MIXED = re.compile(r"[A-Za-z][A-Za-z0-9.]*[가-힣]+")
TITLE_MIXED = re.compile(r"[A-Z]{2,6}[가-힣]{1,4}")


def strip_wikitext(text):
    text = html.unescape(text)  # the XML layer already unescaped once; wikitext may hold &amp;
    text = COMMENT.sub("", text)
    text = HOLES.sub("\0", text)
    text = BLOCKS.sub("", text)
    text = SELF_TAG.sub(" ", text)
    for pattern in (TEMPLATE, TABLE):
        for _ in range(20):
            text, n = pattern.subn("", text)
            if not n:
                break
    for _ in range(5):
        text, n = FILE_LINK.subn("", text)
        if not n:
            break
    text = INTERWIKI.sub("", text)
    text = LINK.sub(lambda m: m.group(2) if m.group(2) is not None else m.group(1), text)
    text = EXT_LINK.sub(lambda m: m.group(1) or "", text)
    text = TAG.sub("", text)
    text = QUOTES.sub("", text)
    text = MAGIC.sub("", text)
    return html.unescape(text)


def paragraphs(text):
    for line in strip_wikitext(text).split("\n"):
        line = line.strip()
        if not line or line[0] in "=*#:;|!{}" or line.startswith("----"):
            continue
        yield line


def clean(s, korean):
    s = unicodedata.normalize("NFC", s)
    s = " ".join(s.split())
    s = FOREIGN_PAREN.sub("", s)
    s = LANG_PAREN.sub("", s)
    s = EMPTY_PAREN.sub("", s)
    s = PAREN_LEAD.sub("(", s)
    s = s.replace(" ,", ",").replace(" .", ".")
    lo, hi = (10, 200) if korean else (20, 300)
    if not (lo <= len(s) <= hi) or LEFTOVER.search(s) or PAREN_HOLE.search(s) or URL.search(s) or FOREIGN_CHAR.search(s):
        return None
    hangul = len(HANGUL.findall(s))
    latin = len(LATIN.findall(s))
    if korean:
        if hangul < 0.6 * (hangul + latin) or hangul < 5:
            return None
        if not END.search(s) or PARTICLE_START.match(s):
            return None
    else:
        if hangul or latin < 0.6 * len(s):
            return None
        if not (s[0].isupper() or s[0].isdigit()) or s[-1] not in ".!?":
            return None
    return s


def key(s):
    return int.from_bytes(hashlib.blake2b(s.encode("utf-8"), digest_size=8).digest(), "big")


def work(task):
    """(sentences with their hash, kept if under `threshold`; mixed-token counts; clean count)."""
    pages, korean, threshold = task
    kept = []
    mixed = collections.Counter()
    n = 0
    for text in pages:
        for para in paragraphs(text):
            for raw in SENT_SPLIT.split(para):
                s = clean(raw, korean)
                if s is None:
                    continue
                n += 1
                if korean:
                    mixed.update(t.rstrip(".") for t in MIXED.findall(s))
                k = key(s)
                if k < threshold:
                    kept.append((k, s))
    return kept, mixed, n


def pages(path):
    """Yields the wikitext of every namespace-0, non-redirect page, streamed."""
    with bz2.open(path, "rb") as f:
        context = ET.iterparse(f, events=("start", "end"))
        _, root = next(context)
        ns = None
        redirect = False
        for event, elem in context:
            if event != "end":
                continue
            tag = elem.tag.rpartition("}")[2]
            if tag == "ns":
                ns = elem.text
            elif tag == "redirect":
                redirect = True
            elif tag == "text":
                text = elem.text or ""
                if ns == "0" and not redirect:
                    head = text.lstrip()[:12].upper()
                    if not (head.startswith("#넘겨주기") or head.startswith("#REDIRECT")):
                        yield text
            elif tag == "page":
                ns = None
                redirect = False
                root.clear()


def batches(path, korean, threshold):
    batch = []
    for text in pages(path):
        batch.append(text)
        if len(batch) >= BATCH:
            yield batch, korean, threshold
            batch = []
    if batch:
        yield batch, korean, threshold


def extract(pool, name, korean, prekeep, out_name):
    """Writes the deduped, hash-sampled sentences of one dump; returns (lines, mixed counts)."""
    tmp = os.path.join(OUT, out_name + ".tmp")
    threshold = int(prekeep * 2**64) if prekeep < 1 else 2**64
    seen = set()
    mixed = collections.Counter()
    total = 0
    with open(tmp, "w", encoding="utf-8", newline="\n") as out:
        for i, (kept, m, n) in enumerate(pool.imap(work, batches(os.path.join(SRC, name), korean,
                                                                 threshold), chunksize=1)):
            total += n
            mixed.update(m)
            for k, s in kept:
                if k not in seen:
                    seen.add(k)
                    out.write(f"{k:016x}\t{s}\n")
            if i % 500 == 0:
                print(f"  {name}: {(i + 1) * BATCH} pages, {total} clean sentences, "
                      f"{len(seen)} kept", flush=True)
    print(f"  {name}: {total} clean sentences, {len(seen)} distinct kept before cap")
    # Trim to CAP by the smallest hashes: deterministic and deduplicated.
    cutoff = 2**64
    if len(seen) > CAP:
        cutoff = sorted(seen)[CAP]
    del seen
    lines = 0
    with open(tmp, encoding="utf-8") as f, \
            open(os.path.join(OUT, out_name), "w", encoding="utf-8", newline="\n") as out:
        for line in f:
            k, _, s = line.rstrip("\n").partition("\t")
            if int(k, 16) < cutoff:
                out.write(s + "\n")
                lines += 1
    os.remove(tmp)
    return lines, mixed


def title_mixed():
    found = set()
    with gzip.open(os.path.join(SRC, KO_TITLES), "rt", encoding="utf-8", errors="replace") as f:
        for line in f:
            for part in line.strip().split("_"):
                part = unicodedata.normalize("NFC", part)
                if TITLE_MIXED.fullmatch(part):
                    found.add(part)
    return found


def main():
    os.makedirs(OUT, exist_ok=True)
    lines = {}
    with multiprocessing.Pool(WORKERS) as pool:
        print("kowiki")
        lines["ko.txt"], mixed = extract(pool, KOWIKI, True, KO_PREKEEP, "ko.txt")
        print("simplewiki")
        lines["en.txt"], _ = extract(pool, SIMPLEWIKI, False, 1.0, "en.txt")
    tokens = {w for w, c in mixed.items() if c >= MIN_MIXED and len(w) <= 30}
    del mixed
    titles = title_mixed()
    print(f"mixed: {len(tokens)} from prose, {len(titles - tokens)} more from titles")
    rows = sorted(tokens | titles)[:CAP]
    with open(os.path.join(OUT, "mixed.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.writelines(w + "\n" for w in rows)
    lines["mixed.txt"] = len(rows)
    meta = {
        "source": "wikimedia-dumps",
        "tier": "high",
        "weight": 1.5,
        "uses": ["korean-key-model", "korean-lexicon", "mixed-ko-en", "acronyms",
                 "particles-endings", "english-key-model", "english-lexicon"],
        "license": "CC BY-SA 4.0 and GFDL 1.3 (Wikimedia dumps: kowiki, simplewiki)",
        "files": lines,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    for name, n in lines.items():
        print(f"{name}: {n}")


if __name__ == "__main__":
    main()
