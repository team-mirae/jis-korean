"""Extracts the OPUS en-ko parallel corpora (corpora/open/opus-en-ko, opus.nlpl.eu moses files; each
subfolder carries OPUS's LICENSE/README) into corpora/derived/opus-en-ko/ for build_ranker.py. Only
derived counts and sentences are used.

Licenses of the subsets read: ParaCrawl v9 and ParaCrawl-Bonus v9: CC0 for the packaging (web-crawled
text, ParaCrawl does not own the content); WikiMatrix v1 and wikimedia: CC BY-SA (Wikipedia);
GlobalVoices v2018q4: CC BY 3.0; tldr-pages: CC BY 4.0; translatewiki: CC BY 3.0 / GPL (MediaWiki
UI messages).

Read (only the *.ko side, one sentence/segment per line, streamed line by line):
- High (all kept after cleaning): tldr-pages, translatewiki, GlobalVoices, WikiMatrix, plus
  wikimedia with the machine-translation filter below (~0.7M raw lines).
- Bulk (sampled to fill the rest of the 3M-line cap): ParaCrawl, ParaCrawl-Bonus (~11.7M raw lines).
Skipped: LinguaTools-WikiTitles (title/transliteration pairs), PHP (English in the .ko file),
MDN_Web_Docs (code and macros), KDE4/GNOME/Ubuntu (tiny i18n fragments), ELRC-wikipedia_health
(tiny), every *.xml (alignment only) and every *.en (the recipe uses the Korean side only).

Cleaning, all subsets:
- NFC; non-breaking/ideographic spaces and tabs become spaces; curly quotes become ASCII quotes;
  U+2026 becomes "..."; reference marks like [1] removed; whitespace collapsed.
- tldr-pages: backticked single words are unwrapped (`dwebp`는 -> dwebp는).
- Parentheticals holding Han or kana (유라쿠초(有楽町)) are removed.
- Long lines are split into sentences after Hangul + [.?!] + space.
- Dropped: URLs, e-mail addresses, HTML/wiki markup ({{ }}, [[ ]], <...>, $1, %s, __X__, /* */,
  backticks left over, {\\displaystyle}), Wikipedia citation lines (...에 확인함), lines with
  characters outside Hangul syllables, printable ASCII and a few punctuation marks, lines whose
  letters are under 30% Hangul, lines with fewer than 2 Hangul syllables or under 4 characters.
- Machine-translation tells (wikimedia and bulk): "수있"/"수없" without a space, detached " 의 " /
  " 에 ", "그(것)".
Bulk only, on top:
- Dropped: Bible verse lines (^digits:digits), 데이터시트/datasheet, adult and gambling keywords,
  xHamster-style timestamps, slot-filled particle pairs (비엔나을(를)), lines under 10 characters.
- Templates collapsed: every line hash (digits masked) kept once; lines over 40 characters share a
  prefix+suffix key (first and last 20 characters, digits masked) kept at most 3 times (slot-filled
  templates such as "KAYAK은 ... 딱 맞는 X 특가 상품을 ..."); any token containing Latin letters
  (KAYAK은) may appear in at most 1000 kept lines.
- The surviving bulk lines are sampled with random.Random(20261027) so ko.txt stays under 3M lines.
Exact duplicates are removed across all subsets.

Outputs:
- ko.txt: Korean sentences (with their Latin words, acronyms and digits), high subsets first, then
  the bulk sample. For the key model, lexicon, particles/endings and business vocabulary.
- mixed.txt: distinct eojeol of Latin letters/digits followed by Hangul ([A-Za-z0-9]+[가-힣]+ with
  at least one letter: GPU를, LED조명), edge punctuation stripped, seen in the high subsets or at
  least twice in the filtered bulk.

Usage: python3 -I opus_en_ko.py
"""

import collections
import json
import os
import random
import re
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
SRC = os.path.join(ROOT, "corpora", "open", "opus-en-ko")
OUT = os.path.join(ROOT, "corpora", "derived", "opus-en-ko")
HIGH = ["tldr-pages", "translatewiki", "GlobalVoices", "WikiMatrix", "wikimedia"]
MT_FILTERED = {"wikimedia"}
BULK = ["ParaCrawl", "ParaCrawl-Bonus"]
MAX_LINES = 3_000_000
TOKEN_CAP = 1000
TEMPLATE_CAP = 3
SEED = 20261027

TRANSLATE = str.maketrans({
    " ": " ", "　": " ", "\t": " ", "​": "",
    "“": '"', "”": '"', "‘": "'", "’": "'",
    "…": "...",
})
ALLOWED = re.compile(r"[가-힣\x20-\x7e·《》〈〉「」『』–—°×]+")
CJK_PAREN = re.compile(r"\s?\([^()]*[぀-ヿ㐀-鿿豈-﫿][^()]*\)")
REF = re.compile(r"\s?\[(?:\d+|주 \d+|출처 필요)\]")
BACKTICK_WORD = re.compile(r"`([A-Za-z0-9_.+-]+)`")
SPACES = re.compile(r"\s+")
SENT_SPLIT = re.compile(r"(?<=[가-힣][.?!])\s+(?=\S)")
JUNK = re.compile(
    r"https?:|www\.|\.(?:com|net|org|kr)\b|@[A-Za-z0-9-]+\.|\{\{|\}\}|\[\[|\]\]|<[^>]*>|\$\d|%[sd]\b"
    r"|__[A-Za-z0-9$]+__|/\*|\*/|`|\\displaystyle|\{\\|에 확인함|&[a-z]+;|\|")
MT_TELLS = re.compile(r"수있|수없| 의 | 에 |그\(것\)")
BULK_JUNK = re.compile(
    r"^\d+:\d+|데이터시트|datasheet|섹스|포르노|야동|자위|성인용품|에로|누드|창녀|매춘|오르가즘"
    r"|[을를와과이가은는]\((?:을|를|와|과|이|가|은|는)\)"
    r"|porn|xxx|hentai|xhamster|milf|nude|\bsex|\b\d+ 개월 전\b|카지노|바카라|토토",
    re.IGNORECASE)
HANGUL = re.compile(r"[가-힣]")
LATIN = re.compile(r"[A-Za-z]")
MIXED = re.compile(r"[A-Za-z0-9]*[A-Za-z][A-Za-z0-9]*[가-힣]+")
EDGE = "\"'()[]{}<>.,!?;:·《》〈〉「」『』-–—"
DIGITS = re.compile(r"\d+")


def ko_path(name):
    return os.path.join(SRC, name, f"{name}.en-ko.ko")


def normalize(line):
    line = unicodedata.normalize("NFC", line).translate(TRANSLATE)
    line = REF.sub("", line)
    line = BACKTICK_WORD.sub(r"\1", line)
    line = CJK_PAREN.sub("", line)
    return SPACES.sub(" ", line).strip()


def acceptable(s, bulk, mt):
    if len(s) < (10 if bulk else 4) or JUNK.search(s) or not ALLOWED.fullmatch(s):
        return False
    hangul = len(HANGUL.findall(s))
    if hangul < 2 or hangul < 0.3 * (hangul + len(LATIN.findall(s))):
        return False
    if (mt or bulk) and MT_TELLS.search(s):
        return False
    if bulk and BULK_JUNK.search(s):
        return False
    return True


def sentences(path, bulk, mt):
    with open(path, encoding="utf-8", errors="replace") as f:
        for raw in f:
            line = normalize(raw)
            if not line:
                continue
            for s in SENT_SPLIT.split(line) if len(line) > 80 else (line,):
                if acceptable(s, bulk, mt):
                    yield s


def mixed_tokens(s):
    for tok in s.split(" "):
        tok = tok.strip(EDGE)
        if MIXED.fullmatch(tok):
            yield tok


def main():
    os.makedirs(OUT, exist_ok=True)
    seen = set()
    mixed_high = set()
    mixed_bulk = collections.Counter()
    ko_path_out = os.path.join(OUT, "ko.txt")
    tmp_path = os.path.join(OUT, "bulk.tmp")
    high = 0
    with open(ko_path_out, "w", encoding="utf-8", newline="\n") as out:
        for name in HIGH:
            n = 0
            for s in sentences(ko_path(name), False, name in MT_FILTERED):
                h = hash(s)
                if h in seen:
                    continue
                seen.add(h)
                out.write(s + "\n")
                mixed_high.update(mixed_tokens(s))
                n += 1
            print(f"{name}: {n}")
            high += n
    # Bulk pass 1: filter and collapse templates into a temp file.
    templates = collections.Counter()
    latin_tokens = collections.Counter()
    bulk = 0
    with open(tmp_path, "w", encoding="utf-8", newline="\n") as tmp:
        for name in BULK:
            n = 0
            for s in sentences(ko_path(name), True, True):
                masked = DIGITS.sub("0", s)
                h = hash(masked)
                if h in seen:
                    continue
                seen.add(h)
                if len(masked) > 40:
                    key = hash((masked[:20], masked[-20:]))
                    if templates[key] >= TEMPLATE_CAP:
                        continue
                    templates[key] += 1
                toks = {t.strip(EDGE) for t in s.split(" ") if LATIN.search(t)}
                if any(latin_tokens[t] >= TOKEN_CAP for t in toks):
                    continue
                for t in toks:
                    latin_tokens[t] += 1
                tmp.write(s + "\n")
                mixed_bulk.update(mixed_tokens(s))
                n += 1
            print(f"{name}: {n} after filtering")
            bulk += n
    del templates, latin_tokens, seen
    # Bulk pass 2: sample into ko.txt.
    budget = MAX_LINES - high
    rate = min(1.0, budget / bulk) if bulk else 0.0
    rng = random.Random(SEED)
    sampled = 0
    with open(tmp_path, encoding="utf-8") as tmp, \
            open(ko_path_out, "a", encoding="utf-8", newline="\n") as out:
        for s in tmp:
            if rng.random() < rate and sampled < budget:
                out.write(s)
                sampled += 1
    os.remove(tmp_path)
    print(f"bulk sampled: {sampled} of {bulk} (rate {rate:.3f})")
    mixed = mixed_high | {t for t, c in mixed_bulk.items() if c >= 2}
    with open(os.path.join(OUT, "mixed.txt"), "w", encoding="utf-8", newline="\n") as f:
        for t in sorted(mixed)[:MAX_LINES]:
            f.write(t + "\n")
    files = {"ko.txt": high + sampled, "mixed.txt": min(len(mixed), MAX_LINES)}
    meta = {
        "source": "opus-en-ko",
        "tier": "moderate",
        "weight": 0.7,
        "uses": ["korean-key-model", "mixed-ko-en", "acronyms", "particles-endings",
                 "business-terms", "korean-lexicon"],
        "license": "ParaCrawl/ParaCrawl-Bonus v9: CC0 packaging (web-crawled content); WikiMatrix, "
                   "wikimedia: CC BY-SA; GlobalVoices: CC BY 3.0; tldr-pages: CC BY 4.0; "
                   "translatewiki: CC BY 3.0 / GPL (via OPUS, opus.nlpl.eu)",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(json.dumps(files))


if __name__ == "__main__":
    main()
