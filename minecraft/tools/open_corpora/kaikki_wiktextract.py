"""Extracts Korean and English lexicons, Korean example sentences and particles from the kaikki.org
wiktextract dumps (corpora/open/kaikki-wiktextract) into corpora/derived/kaikki-wiktextract.
License: Wiktionary content, CC BY-SA 4.0 and GFDL (corpora/open/kaikki-wiktextract/
LICENSE-CC-BY-SA-4.0.txt). Only word counts derived from it are used.

Source: kaikki.org (wiktextract by Tatu Ylonen), the 2026-10-01 dumps: enwiktionary-Korean.jsonl.gz
(Korean entries in English Wiktionary), kowiktionary-ko-extract.jsonl.gz (all of Korean
Wiktionary) and enwiktionary-English.jsonl.gz (English entries in English Wiktionary). All three are
JSON lines, streamed through gzip. Wiktionary has no frequencies, so counts are flat:

- ko_words.tsv: Korean headwords written in Hangul syllables only (no spaces). English Wiktionary:
  every part of speech except character/syllable/romanization/symbol/punct, keeping an entry only
  if some sense is not tagged form-of, hanja, North-Korea, dialectal, historical, obsolete or
  archaic (so abbreviation, Internet and slang entries stay). Korean Wiktionary: lang_code ko,
  pos noun/verb/adj/adv/pron/num/det/conj, with some sense that is neither no-gloss nor form-of.
  Count is the number of the two dictionaries the word appears in, 1..2.
- particles.txt: Korean particles and endings, leading/trailing '-' stripped: English Wiktionary
  pos particle and suffix (same sense-tag filter; hanja-form suffixes such as 的 dropped by the
  Hangul-only rule) and Korean Wiktionary pos particle and postp. Entries with spaces or archaic
  jamo (U+1100..U+11FF) are dropped.
- ko.txt: example sentences, senses[].examples[].text of the Korean entries in both dictionaries
  plus the Korean translations (examples[].translation) of foreign-language entries in Korean
  Wiktionary. Lines are split on newlines and after a Hangul syllable followed by . ? or !; a
  "예문:" prefix is stripped. Dropped: related-word lists ("형용사: 부실하다", "참조: ...",
  "학명: ..."), ≪source≫ citations and parenthesised hanja (removed from the line), and whole
  lines still holding CJK ideographs, kana or private-use characters, URLs or wiki markup, with
  more Latin letters than Hangul, or under two Hangul syllables. Deduplicated.
- en.txt: the English translations (examples[].english) of the English Wiktionary Korean
  examples, one per line, deduplicated.
- en_words.tsv: English nouns (pos noun, ^[a-z]+$, some sense not tagged archaic, obsolete, rare,
  dialectal or form-of) plus their plural forms, kept only if the word occurs at least 20 times in
  the hermitdave-frequencywords en_full.txt subtitle list (used as a filter only, when present),
  and acronyms (^[A-Z][A-Z0-9]{1,5}$ with a sense tagged abbreviation, acronym or initialism, not
  all senses archaic/obsolete). Count is 1.
- terms.tsv: Korean headwords from English Wiktionary whose senses carry a business, finance,
  economics, computing or similar topic, with the first such English gloss (parentheticals
  dropped, at most 60 characters).
- mixed.txt: distinct tokens mixing Latin letters and Hangul (PC방, 챗GPT, WTO에), from headwords
  and example sentences.
- meta.json.

Usage: python3 -I kaikki_wiktextract.py
"""

import gzip
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "kaikki-wiktextract")
FREQ = os.path.join(ROOT, "corpora", "open", "hermitdave-frequencywords", "en_full.txt")
OUT = os.path.join(ROOT, "corpora", "derived", "kaikki-wiktextract")

CAP = 3_000_000
HANGUL_WORD = re.compile("^[가-힣]+$")
HANGUL = re.compile("[가-힣]")
LATIN = re.compile("[A-Za-z]")
MIXED_TOKEN = re.compile("[A-Za-z0-9가-힣]+")
OLD_JAMO = re.compile("[\u1100-\u11ff\u3164\u318d]")
CJK = re.compile("[\ue000-\uf8ff\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff]")
HANJA_PAREN = re.compile("\\([\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff·,、 ]+\\)")
CITATION = re.compile("[≪《〈][^≫》〉]*[≫》〉]")
URL = re.compile(r"https?://|www\.|\.com\b|\.kr\b", re.I)
LABEL = re.compile(r"^\s*([^\s:：]{1,10})\s*[:：]")
EXAMPLE_LABEL = re.compile(r"^\s*(예문|예|보기)\s*[:：]\s*")
SPACES = re.compile(r"\s+")
SENTENCE_END = re.compile(r"(?<=[가-힣][.?!])\s+")
NOUN = re.compile("^[a-z]+$")
ACRONYM = re.compile("^[A-Z][A-Z0-9]{1,5}$")
PAREN = re.compile(r"\s*\([^)]*\)")

EN_SKIP_POS = {"character", "syllable", "romanization", "symbol", "punct"}
KO_BAD_TAGS = {"form-of", "hanja", "North-Korea", "dialectal", "historical", "obsolete", "archaic"}
KOWIKI_POS = {"noun", "verb", "adj", "adv", "pron", "num", "det", "conj"}
EN_NOUN_BAD = {"archaic", "obsolete", "rare", "dialectal", "form-of"}
ACRONYM_TAGS = {"abbreviation", "acronym", "initialism"}
TERM_TOPICS = {
    "business", "finance", "economics", "computing", "accounting", "banking", "trading",
    "marketing", "management", "investment", "insurance", "taxation", "statistics",
    "programming", "software", "internet", "Internet", "communications", "telecommunications",
    "law", "commerce", "stock-market", "securities", "electronics", "information-technology",
}


def stream(name):
    with gzip.open(os.path.join(SRC, name), "rt", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    yield json.loads(line)
                except ValueError:
                    continue


def good_sense(sense, bad):
    return not (set(sense.get("tags", [])) & bad)


def strip_dashes(word):
    return word.strip().strip("-").strip()


def mixed_tokens(text, into):
    for tok in MIXED_TOKEN.findall(text):
        if LATIN.search(tok) and HANGUL.search(tok) and len(tok) <= 30:
            into.add(tok)


def clean_ko(text):
    """Yields cleaned Korean lines from one example text."""
    for raw in text.replace("\r", "\n").split("\n"):
        yield from clean_ko_line(raw)


def clean_ko_line(raw):
    for line in SENTENCE_END.split(raw.strip()):
        line = line.strip().lstrip("*:#-•").strip()
        m = EXAMPLE_LABEL.match(line)
        if m:
            line = line[m.end():]
        elif LABEL.match(line):
            continue
        line = CITATION.sub("", line)
        line = HANJA_PAREN.sub("", line)
        line = SPACES.sub(" ", line).strip()
        if not line or URL.search(line) or CJK.search(line) or OLD_JAMO.search(line):
            continue
        hangul = len(HANGUL.findall(line))
        if hangul < 2 or len(LATIN.findall(line)) > hangul:
            continue
        if "{{" in line or "[[" in line or "<" in line or "|" in line:
            continue
        yield line


def clean_en(text):
    line = SPACES.sub(" ", text or "").strip()
    if not line or HANGUL.search(line) or URL.search(line) or not LATIN.search(line):
        return None
    if "{{" in line or "[[" in line or "<" in line:
        return None
    return line


def short_gloss(gloss):
    g = PAREN.sub("", gloss).strip().rstrip(".;:").strip()
    if not g or len(g) > 60 or HANGUL.search(g) or "form of" in g:
        return None
    return g


def korean_from_enwiktionary(words, particles, sentences, english, terms, mixed):
    for d in stream("enwiktionary-Korean.jsonl.gz"):
        pos = d.get("pos")
        word = (d.get("word") or "").strip()
        senses = d.get("senses", [])
        good = [s for s in senses if good_sense(s, KO_BAD_TAGS)]
        for s in senses:
            for e in s.get("examples", []):
                text = e.get("text") or ""
                for line in clean_ko(text):
                    sentences.append(line)
                    mixed_tokens(line, mixed)
                en = clean_en(e.get("english") or e.get("translation"))
                if en and HANGUL.search(text):
                    english.append(en)
        if pos in EN_SKIP_POS or not good:
            continue
        if " " not in word and LATIN.search(word) and HANGUL.search(word):
            mixed_tokens(word, mixed)
        if pos in ("particle", "suffix", "postp"):
            p = strip_dashes(word)
            if p and " " not in p and not OLD_JAMO.search(p) and re.match("^[가-힣ㄱ-ㅎ]+$", p):
                particles.add(p)
            continue
        if not HANGUL_WORD.match(word):
            continue
        words.setdefault(word, set()).add("en")
        if word not in terms:
            for s in good:
                topics = set(s.get("topics", []))
                if topics & TERM_TOPICS and s.get("glosses"):
                    g = short_gloss(s["glosses"][0])
                    if g:
                        terms[word] = g
                        break


def korean_from_kowiktionary(words, particles, sentences, mixed):
    for d in stream("kowiktionary-ko-extract.jsonl.gz"):
        senses = d.get("senses", [])
        if d.get("lang_code") != "ko":
            # Foreign-language entries: keep only the Korean translations of their examples.
            for s in senses:
                for e in s.get("examples", []):
                    t = e.get("translation") or ""
                    if HANGUL.search(t):
                        for line in clean_ko(t):
                            sentences.append(line)
                            mixed_tokens(line, mixed)
            continue
        pos = d.get("pos")
        word = (d.get("word") or "").strip()
        for s in senses:
            for e in s.get("examples", []):
                for line in clean_ko(e.get("text") or ""):
                    sentences.append(line)
                    mixed_tokens(line, mixed)
        good = [s for s in senses if good_sense(s, {"no-gloss", "form-of"})]
        if not good:
            continue
        if " " not in word and LATIN.search(word) and HANGUL.search(word):
            mixed_tokens(word, mixed)
        if pos in ("particle", "postp"):
            p = strip_dashes(word)
            if p and not OLD_JAMO.search(p) and re.match("^[가-힣ㄱ-ㅎ]+$", p):
                particles.add(p)
        elif pos in KOWIKI_POS and HANGUL_WORD.match(word):
            words.setdefault(word, set()).add("ko")


def load_freq():
    freq = set()
    if not os.path.exists(FREQ):
        return None
    with open(FREQ, encoding="utf-8", errors="replace") as f:
        for line in f:
            parts = line.split()
            if len(parts) == 2 and parts[1].isdigit() and int(parts[1]) >= 20:
                freq.add(parts[0])
    return freq


def english_from_enwiktionary():
    freq = load_freq()
    nouns, acronyms = set(), set()
    for d in stream("enwiktionary-English.jsonl.gz"):
        if d.get("lang_code") != "en":
            continue
        word = (d.get("word") or "").strip()
        senses = d.get("senses", [])
        if d.get("pos") == "noun" and NOUN.match(word):
            if any(good_sense(s, EN_NOUN_BAD) for s in senses):
                if freq is None or word in freq:
                    nouns.add(word)
                for form in d.get("forms", []):
                    f = form.get("form") or ""
                    if "plural" in form.get("tags", []) and NOUN.match(f) and \
                            (freq is None or f in freq):
                        nouns.add(f)
        if ACRONYM.match(word):
            tags = set(t for s in senses for t in s.get("tags", []))
            if tags & ACRONYM_TAGS and any(good_sense(s, {"archaic", "obsolete"}) for s in senses):
                acronyms.add(word)
    return nouns, acronyms


def dedupe(lines):
    seen, out = set(), []
    for line in lines:
        if line not in seen:
            seen.add(line)
            out.append(line)
    return out


def cap(lines):
    if len(lines) <= CAP:
        return lines
    k = -(-len(lines) // CAP)
    return lines[::k]


def write(name, lines, files):
    if not lines:
        return
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    files[name] = len(lines)


def main():
    os.makedirs(OUT, exist_ok=True)
    words, particles, sentences, english, terms, mixed = {}, set(), [], [], {}, set()
    korean_from_enwiktionary(words, particles, sentences, english, terms, mixed)
    korean_from_kowiktionary(words, particles, sentences, mixed)
    nouns, acronyms = english_from_enwiktionary()

    files = {}
    write("ko.txt", cap(dedupe(sentences)), files)
    write("en.txt", cap(dedupe(english)), files)
    write("ko_words.tsv", [f"{w}\t{len(src)}" for w, src in sorted(words.items())], files)
    en_words = sorted(nouns | acronyms)
    write("en_words.tsv", [f"{w}\t1" for w in en_words], files)
    write("terms.tsv", [f"{k}\t{v}" for k, v in sorted(terms.items())], files)
    write("particles.txt", sorted(particles), files)
    write("mixed.txt", sorted(mixed), files)

    meta = {
        "source": "kaikki-wiktextract",
        "tier": "moderate",
        "weight": 0.7,
        "uses": ["english-nouns", "acronyms", "english-lexicon", "korean-lexicon",
                 "mixed-ko-en", "particles-endings"],
        "license": "CC BY-SA 4.0 and GFDL (Wiktionary content via kaikki.org wiktextract)",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    for name, n in files.items():
        print(f"{name}: {n}")


main()
