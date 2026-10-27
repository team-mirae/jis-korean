"""Extracts an English lexicon (with inflections), common nouns and business/tech acronyms from
SCOWL into corpora/derived/scowl-esdb/ (gitignored) for training the predictor's English side
(docs/spec.md §10.3; minecraft/tools/build_ranker.py).

Source: SCOWL / English Speller Database, https://github.com/en-wl/wordlist (en-wl/wordlist v2,
commit in corpora/open/scowl-esdb/SOURCE.txt). License: the SCOWL permissive notice
(LICENSE-Copyright.txt: free use, copy and distribution with the notice kept; sizes above 60
also carry the UKACD and other notices, which is why nothing above size 60 is read). Tier
moderate, weight 0.7.

Read: src/data/scowl-pre.txt (the database export, one lemma per line: `SIZES [REGION] [TAGS]:
[VARIANT-INFO:] LEMMA <POS/CLASS> {defn} (usage): FORM, FORM, (FORM | v: FORM)`, grammar in
src/README.md) and the merge files src/data/hacker, extra and coca_llm (same format, no source
tags). `#` comments and `#!`/`#:` export notes are dropped; adjust-style lines (prefixed `? + - =
~ #` and a space) are skipped. A lemma's size is the smallest number in its SCOWL info.

Kept: US spelling only (no variant info, or spelling A/_ at variant level none or `.`: the default
American list; British/Canadian/Australian-only spellings and the v/V/~/@/-/x variants are
dropped), no GB/CA/AU region entries, no roman numerals, no lemmas or forms marked uncommon,
archaic, infrequent or inapplicable (`-` `@` `!` `~`), only plain ASCII letters (no possessives,
hyphens, spaces or dots). Inside `(a | b)` choices only the unmarked (preferred) form is taken.

What it writes (en_words.tsv, word<TAB>count; nothing Korean, so no ko/mixed/particles files).
SCOWL has no frequencies, so the size tier is the pseudo-frequency (TIER_COUNT: 35 is the core
of 11+ dictionaries, 50 of 5+, 60 of 2+; size-35 lines tagged [ukfreq], Kelk's frequency classes
2-16, count double); a word in several entries gets its best count, an inflection half its
line's (inflections often sit on their own `- <pos>: forms` line with their own size):
- Lexicon: lemmas of size <= 50 written in lowercase (any POS except names), plus their
  inflections; capitalized common words (POS class upper/demonym: Monday, English) also count,
  as the ranker lowercases them. Names, places, people, surnames and trademarks are dropped.
- Nouns: the <n> lemmas among them (already in the lexicon; listed in `uses`).
- Acronyms: 2-6 letter all-caps lemmas tagged abbr or upper at size <= 60, kept only when in
  BUSINESS_ACRONYMS (SCOWL has many obscure ones: WNW, VDT), written as in the source.

Usage: python3 -I scowl_esdb.py
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
SRC = os.path.join(ROOT, "corpora", "open", "scowl-esdb")
DATA = os.path.join(SRC, "src", "data")
FILES = ["scowl-pre.txt", "hacker", "extra", "coca_llm"]
OUT = os.path.join(ROOT, "corpora", "derived", "scowl-esdb")

LEXICON_MAX_SIZE = 50
ACRONYM_MAX_SIZE = 60
# Pseudo-frequency per SCOWL size (counts are relative: only the ratios matter to the ranker).
# SCOWL v2's smallest size is 35; within it, lines tagged [ukfreq] (Kelk's UK frequency classes
# 2-16, i.e. words seen often in real text) get UKFREQ_COUNT.
TIER_COUNT = {35: 15, 40: 8, 50: 3, 55: 2, 60: 1}
UKFREQ_COUNT = 30
DROP_CLASSES = {"name", "name?", "place", "person", "surname", "trademark"}
ACRONYM_TAGS = {"abbr", "abbr?", "upper", "upper?"}
# Acronyms people type in business, finance and tech writing (checked against SCOWL's list).
BUSINESS_ACRONYMS = set("""
AI API ATM B2B B2C BBC CBS CD CDC CEO CFO CIA CIO COO CPA CPI CPU CRM CSS CTO DNA DNS DVD EPA ERP
ESG ETF EU FAQ FBI FDA FTP GDP GPS GPU HDTV HR HTML HTTP HTTPS IBM ID IMF IP IPO IQ IRA IRS ISP IT
LAN LCD LED LLC MBA MP3 NASA NATO NGO OECD OEM OK PC PDF PIN PR PS PTSD RAM ROI ROM SEC SMS SQL SSD
SUV TV UN UPS URL US USA USB VAT VIP VPN WHO WTO XML CNN ADHD AIDS HIV ASAP FYI TBA TBD ETA RSVP
UFO UK OS RPM HQ GMT UTC IOU MRI DIY CC BCC KPI QA UX UI SDK CLI GUI IDE JSON YAML RSS WWW
AFAIK FWIW IMO IMHO LOL OMG BTW NDA LBO EPS EBIT EBITDA ROE ROA GAAP IFRS FX OTC NYSE
NASDAQ AGM CAD CAM CAGR COGS CPM CTR DAU MAU SEO SLA SKU POS EV AC DC HVAC ISO ANSI ASCII IEEE
APR BIOS CCTV COVID CV DBMS DJ DOS ECG EKG EUR FIFO FY GB GBP GIF GMO GNP GPA HDD HDMI ICU IVF JPEG KB
LGBT LIFO MB MIDI MOOC MP MPEG MVP NFC NLP NSFW NYC OCR OPEC PCB PVC QC REIT RISC RNA SAT SCSI SOP SOS
TOEFL TQM UNESCO UNICEF UNIX UPC USD UV VCR VGA VP WMD XL XS XXL
""".split())
TOKEN = re.compile(r"[A-Za-z]+")
VARIANT_FIELD = re.compile(r"^(\+|[ABZCD_][.=?v~V@x-]?( [ABZCD_][.=?v~V@x-]?)*( \{\d+\})?)$")
LEMMA = re.compile(r"^(.*?)(?: <([^>]*)>)?(?: \{[^}]*\})?(?: \([^)]*\))?$")
BAD_MARKS = set("-@!~")


def us_ok(info):
    """True when a variant field (or a derived form's variant info) is in the default US list."""
    info = info.strip()
    if not info or info == "+":
        return True
    info = re.sub(r"\s*\{\d+\}$", "", info)
    for code in info.split():
        if code[0] in "ABZCD_":
            spelling, level = code[0], code[1:]
        else:
            spelling, level = "_", code  # level only (derived forms): applies to every spelling
        if spelling in "A_" and level in ("", "."):
            return True
    return False


def clean_word(w):
    """The word without export marks, or None if it's marked uncommon/archaic/infrequent."""
    w = w.strip().replace("†", "")
    if not w or w == "-":
        return None
    if w[0] in BAD_MARKS or w[-1] in BAD_MARKS:
        return None
    w = w.rstrip("*")
    return w if TOKEN.fullmatch(w) else None


def split_top(text, sep):
    """Split on sep outside parentheses."""
    parts, depth, cur, i = [], 0, [], 0
    while i < len(text):
        c = text[i]
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
        if depth == 0 and text.startswith(sep, i):
            parts.append("".join(cur))
            cur = []
            i += len(sep)
            continue
        cur.append(c)
        i += 1
    parts.append("".join(cur))
    return parts


def forms(rest):
    """Preferred US inflected forms from the entry list."""
    out = []
    for entry in split_top(rest, ", "):
        entry = entry.strip()
        if entry.startswith("(") and entry.endswith(")"):
            for option in entry[1:-1].split("|"):
                info, sep, word = option.strip().rpartition(": ")
                if (not sep or us_ok(info)) and (w := clean_word(word)):
                    out.append(w)
                    break
        elif w := clean_word(entry):
            out.append(w)
    return out


def parse(line):
    """(size, pseudo-count, pos, class, lemma or None, forms) for a usable line, else None."""
    if line.startswith("#") or not line.strip():
        return None
    if len(line) > 1 and line[0] in "?+-=~" and line[1] == " ":
        return None  # adjust-file instructions
    line = line.split(" #", 1)[0].rstrip()
    fields = line.split(": ")
    if len(fields) < 2:
        return None
    info = fields[0].split()
    sizes = [int(t) for t in info if t.isdigit()]
    if not sizes or any(t in ("GB", "CA", "AU", "roman-numerals") for t in info):
        return None
    idx = 1
    if len(fields) > 2 and VARIANT_FIELD.match(fields[1]):
        if not us_ok(fields[1]):
            return None
        idx = 2
    m = LEMMA.match(fields[idx].strip())
    if not m:
        return None
    word, pos = m.group(1), m.group(2) or ""
    pos, _, cls = pos.partition("/")
    if word != "-" and word[:1] in BAD_MARKS:
        return None  # lemma marked uncommon/archaic/infrequent ("-" alone is the no-lemma slot)
    lemma = clean_word(word)
    rest = ": ".join(fields[idx + 1:])
    size = min(sizes)
    count = TIER_COUNT.get(size, 1)
    if size == 35 and "[ukfreq]" in info:
        count = UKFREQ_COUNT
    return size, count, pos, cls, lemma, forms(rest) if rest else []


def main():
    counts = {}
    acronyms, candidates = {}, set()
    nouns = set()
    entries = 0

    def add(word, n):
        if n > counts.get(word, 0):
            counts[word] = n

    for name in FILES:
        with open(os.path.join(DATA, name), encoding="utf-8", errors="replace") as f:
            for raw in f:
                p = parse(raw.rstrip("\n"))
                if not p:
                    continue
                size, n, pos, cls, lemma, infl = p
                if cls in DROP_CLASSES:
                    continue
                entries += 1
                if (lemma and size <= ACRONYM_MAX_SIZE and cls in ACRONYM_TAGS | {""} and
                        (pos == "abbr" or cls in ACRONYM_TAGS) and
                        re.fullmatch(r"[A-Z][A-Z0-9]{1,5}", lemma)):
                    candidates.add(lemma)
                    if lemma in BUSINESS_ACRONYMS:
                        acronyms[lemma] = max(acronyms.get(lemma, 0), n)
                    continue
                if size > LEXICON_MAX_SIZE or pos == "abbr":
                    continue
                lower = lemma is None or lemma.islower()
                cap = lemma is not None and lemma[:1].isupper() and lemma[1:].islower() and \
                    cls in ("upper", "upper?", "demonym")
                if not (lower or cap):
                    continue
                if lemma:
                    add(lemma, n)
                    if pos == "n" and lower:
                        nouns.add(lemma)
                for w in infl:
                    if w.islower() or (cap and w[:1].isupper() and w[1:].islower()):
                        add(w, n / 2)
    for a, n in acronyms.items():
        add(a, n)

    os.makedirs(OUT, exist_ok=True)
    rows = sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))
    with open(os.path.join(OUT, "en_words.tsv"), "w", encoding="utf-8", newline="\n") as f:
        for w, n in rows:
            f.write(f"{w}\t{n:g}\n")
    meta = {
        "source": "scowl-esdb",
        "tier": "moderate",
        "weight": 0.7,
        "uses": ["english-lexicon", "english-nouns", "acronyms", "english-key-model"],
        "license": "SCOWL permissive notice (Kevin Atkinson et al., LICENSE-Copyright.txt); "
                   "only sizes <= 60 used",
        "files": {"en_words.tsv": len(rows)},
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(f"{entries} entries read; en_words.tsv: {len(rows)} words "
          f"({len(nouns)} nouns, {len(acronyms)} of {len(candidates)} acronym candidates kept)")


if __name__ == "__main__":
    main()
