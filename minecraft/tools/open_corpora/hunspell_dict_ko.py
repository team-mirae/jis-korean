"""Extracts the hunspell-dict-ko word data (corpora/open/hunspell-dict-ko) into
corpora/derived/hunspell-dict-ko for the predictor.

Source: https://github.com/spellcheck-ko/hunspell-dict-ko (release 0.7.94 plus src/data at HEAD),
the Korean spelling dictionary. Its per-entry YAML (src/data/entries/*.yaml) imports each headword
from 한국어기초사전 (krdict), 표준국어대사전 (stdict) or 우리말샘 (opendict), example sentences
included. License: mixed (LICENSE.md): the word data is mostly CC BY-SA 4.0, entries from 우리말샘
CC BY-SA 2.0 KR, legacy entries and code MPL-1.1/GPL-2.0/LGPL-2.1, the built ko.dic/ko.aff
GPL-3.0+. Only derived counts/sentences are used for training; the text itself is not shipped.

The entry files are a PyYAML block dump (multi-document, `---`-separated). PyYAML isn't stdlib,
so a small block-YAML reader below handles the subset the dump uses: block mappings, (indentless)
block sequences, plain scalars folded over continuation lines, single- and double-quoted scalars
(also multi-line) and the empty flow collections {} and [].

What it extracts:
- ko.txt: the example sentences of every entry: 한국어기초사전 `용례.문장` and `용례.대화`
  (dialogue lines), 우리말샘 `용례` items (the {braces} marking the headword removed), and the
  short 한국어기초사전 `용례.구` collocation phrases (배려가 눈에 띄다.). Lines with markup (<),
  URLs or under 2 Hangul syllables are dropped, whitespace is collapsed, and lines are
  deduplicated (first seen kept, files in sorted order).
- ko_words.tsv: the spell-checker headwords (`result: 맞춤법 검사: 표제어`) of entries not marked
  `제외` (틀린 말, 해당 품사 아님, ...), all-Hangul, 2+ syllables, particles excluded. Count = 1 +
  the number of ko.txt eojeols the headword starts (longest headword wins; verbs and adjectives
  match on the stem without 다).
- particles.txt: headwords whose spell-checker part of speech is 조사 (and not 제외).
- terms.tsv: korean<TAB>latin original for loanwords whose `원어` is Latin script (오디오 audio),
  plus 우리말샘 entries in business and technical fields (`분류`: 경제, 법률, 정보·통신, ...)
  with their Latin original when there is one.
- mixed.txt: distinct Latin+Hangul tokens in the sentences and headwords (CD를, X선).
- meta.json.
Ignored: 뜻풀이 (glosses, with HTML), pronunciation URLs, contributor names, ko.dic/ko.aff (the
GPL build of the same data) and src/dict-ko-data.yaml (a flat noun list duplicating the entries).

Usage: python3 -I hunspell_dict_ko.py
"""

import json
import os
import re
import unicodedata
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "hunspell-dict-ko")
ENTRIES = os.path.join(SRC, "src", "data", "entries")
OUT = os.path.join(ROOT, "corpora", "derived", "hunspell-dict-ko")

LICENSE = ("mixed: word data CC BY-SA 4.0 (우리말샘 entries CC BY-SA 2.0 KR); legacy entries and "
           "code MPL-1.1/GPL-2.0/LGPL-2.1; built ko.dic/ko.aff GPL-3.0+ (see LICENSE.md)")
USES = ["korean-key-model", "korean-lexicon", "particles-endings"]
CAP = 3_000_000

HANGUL = re.compile("[가-힣]")
HANGUL_WORD = re.compile("^[가-힣]{2,}$")
MIXED = re.compile("^(?=.*[A-Za-z])(?=.*[가-힣])[A-Za-z0-9가-힣]+$")
LATIN = re.compile(r"^[A-Za-z][A-Za-z .'&/-]*$")
TOKEN = re.compile(r"[A-Za-z0-9가-힣]+")
EOJEOL = re.compile("[가-힣]+")
BAD = re.compile(r"<|>|URL|https?:|www\.|\.(com|kr|net|org)\b", re.I)
# 우리말샘 분류 values counted as business or technical fields.
FIELDS = {"경제", "경영", "법률", "정보·통신", "전기·전자", "기계", "공업", "교통", "행정", "사회 일반",
          "매체", "산업", "금융", "무역", "회계", "컴퓨터", "통신", "정보 통신", "수학", "물리"}


# --- A minimal block-YAML reader for PyYAML dumps ---

class Yaml:
    def __init__(self, lines):
        self.lines = lines
        self.n = len(lines)

    @staticmethod
    def indent(line):
        return len(line) - len(line.lstrip(" "))

    def skip(self, i):
        while i < self.n and (not self.lines[i].strip() or self.lines[i].lstrip().startswith("#")):
            i += 1
        return i

    def node(self, i, ind):
        """Parses the node whose first line is i, at indent ind."""
        i = self.skip(i)
        if i >= self.n:
            return None, i
        content = self.lines[i][ind:]
        if content == "-" or content.startswith("- "):
            return self.seq(i, ind)
        if self.key_split(content) is not None:
            return self.mapping(i, ind)
        return self.scalar(i, ind, content, ind - 1)

    def seq(self, i, ind):
        items = []
        while True:
            i = self.skip(i)
            if i >= self.n or self.indent(self.lines[i]) != ind:
                break
            content = self.lines[i][ind:]
            if not (content == "-" or content.startswith("- ")):
                break
            rest = content[2:].lstrip(" ")
            if not rest:
                j = self.skip(i + 1)
                if j < self.n and self.indent(self.lines[j]) > ind:
                    value, i = self.node(j, self.indent(self.lines[j]))
                else:
                    value, i = None, i + 1
            else:
                sub = ind + len(content) - len(rest)
                self.lines[i] = " " * sub + rest
                value, i = self.node(i, sub)
            items.append(value)
        return items, i

    @staticmethod
    def key_split(content):
        """(key, rest) if content is a mapping entry, else None."""
        if content[:1] in ("'", '"'):
            q = content[0]
            j = 1
            while j < len(content):
                if content[j] == q:
                    if q == "'" and content[j + 1:j + 2] == "'":
                        j += 2
                        continue
                    break
                if q == '"' and content[j] == "\\":
                    j += 1
                j += 1
            after = content[j + 1:]
            if after == ":" or after.startswith(": "):
                return content[1:j].replace("''", "'"), after[1:].strip()
            return None
        m = re.match(r"^((?:[^:#'\"{\[]|:(?! |$))(?:[^:]|:(?! |$))*):(?: (.*)|)$", content)
        if not m:
            return None
        return m.group(1).strip(), (m.group(2) or "").strip()

    def mapping(self, i, ind):
        out = {}
        while True:
            i = self.skip(i)
            if i >= self.n or self.indent(self.lines[i]) != ind:
                break
            content = self.lines[i][ind:]
            if content.startswith("- ") or content == "-":
                break
            split = self.key_split(content)
            if split is None:
                i += 1
                continue
            key, rest = split
            if rest:
                value, i = self.scalar(i, ind, rest, ind)
            else:
                j = self.skip(i + 1)
                if j < self.n:
                    line = self.lines[j]
                    c = self.indent(line)
                    if c > ind or (c == ind and (line[c:] == "-" or line[c:].startswith("- "))):
                        value, i = self.node(j, c)
                    else:
                        value, i = None, i + 1
                else:
                    value, i = None, j
            out[key] = value
        return out, i

    def scalar(self, i, ind, text, owner):
        """A scalar starting with text on line i; continuation lines are indented past owner."""
        text = text.strip()
        if text in ("{}", "[]"):
            return ({} if text == "{}" else []), i + 1
        if text[:1] in ("'", '"'):
            q = text[0]
            parts = [text[1:]]
            j = i
            while True:
                body = " ".join(parts)
                end = self.closing(body, q)
                if end is not None:
                    raw = body[:end]
                    return (raw.replace("''", "'") if q == "'" else self.unescape(raw)), j + 1
                j += 1
                if j >= self.n:
                    return body, j
                parts.append(self.lines[j].strip())
        parts = [text]
        j = i + 1
        while j < self.n and self.lines[j].strip() and self.indent(self.lines[j]) > owner:
            parts.append(self.lines[j].strip())
            j += 1
        value = " ".join(parts)
        if value in ("null", "~"):
            value = None
        return value, j

    @staticmethod
    def closing(body, q):
        j = 0
        while j < len(body):
            c = body[j]
            if q == '"' and c == "\\":
                j += 2
                continue
            if c == q:
                if q == "'" and body[j + 1:j + 2] == "'":
                    j += 2
                    continue
                return j
            j += 1
        return None

    @staticmethod
    def unescape(raw):
        table = {"n": "\n", "t": "\t", '"': '"', "\\": "\\", "/": "/", "0": "", " ": " ",
                 "r": "", "a": "", "b": "", "e": "", "f": "", "v": "", "N": " ", "_": " "}
        out, j = [], 0
        while j < len(raw):
            c = raw[j]
            if c == "\\" and j + 1 < len(raw):
                d = raw[j + 1]
                width = {"x": 2, "u": 4, "U": 8}.get(d)
                if width:
                    try:
                        out.append(chr(int(raw[j + 2:j + 2 + width], 16)))
                    except (ValueError, OverflowError):
                        pass  # a malformed escape: drop it, the word is junk either way
                    j += 2 + width
                    continue
                out.append(table.get(d, d))
                j += 2
                continue
            out.append(c)
            j += 1
        return "".join(out)


def documents(path):
    with open(path, encoding="utf-8") as f:
        doc = []
        for line in f:
            line = line.rstrip("\n").rstrip("\r")
            if line.rstrip() == "---":
                if doc:
                    yield doc
                doc = []
            elif not line.startswith("#"):
                doc.append(line.replace("\t", "    "))
        if doc:
            yield doc


def parse(doc):
    try:
        value, _ = Yaml(doc).node(0, 0)
    except (RecursionError, IndexError, ValueError):
        return None
    return value if isinstance(value, dict) else None


# --- Extraction ---

def clean(s):
    if not isinstance(s, str):
        return None
    s = unicodedata.normalize("NFC", s)
    s = s.replace("{", "").replace("}", "")
    s = re.sub(r"\s+", " ", s).strip()
    if not s or BAD.search(s) or len(HANGUL.findall(s)) < 2:
        return None
    return s


def walk(node):
    """Every dict nested in node."""
    stack = [node]
    while stack:
        x = stack.pop()
        if isinstance(x, dict):
            yield x
            stack.extend(x.values())
        elif isinstance(x, list):
            stack.extend(x)


def strings(x):
    if isinstance(x, str):
        yield x
    elif isinstance(x, list):
        for y in x:
            if isinstance(y, str):
                yield y


def latin_original(d):
    """The Latin-script 원어 of an import block, if any."""
    w = d.get("원어")
    candidates = []
    if isinstance(w, str):
        candidates.append(w)
    elif isinstance(w, dict):
        candidates.append(w.get("원어"))
    elif isinstance(w, list):
        for y in w:
            candidates.append(y.get("원어") if isinstance(y, dict) else y)
    for c in candidates:
        if isinstance(c, str):
            c = re.sub(r"\s+", " ", c).strip()
            if LATIN.match(c) and len(c) >= 2 and c.lower() != "null":
                return c
    return None


def extract():
    sentences, seen = [], set()
    headwords = {}  # word -> pos
    particles = set()
    terms = {}
    docs = bad = 0

    def add(s):
        s = clean(s)
        if s and s not in seen:
            seen.add(s)
            sentences.append(s)

    for name in sorted(os.listdir(ENTRIES)):
        if not name.endswith(".yaml"):
            continue
        for doc in documents(os.path.join(ENTRIES, name)):
            entry = parse(doc)
            docs += 1
            if entry is None:
                bad += 1
                continue
            # Example sentences, from every source.
            phrases = []
            for d in walk(entry.get("import")):
                for key in ("용례.문장", "용례.대화"):
                    for s in strings(d.get(key)):
                        add(s)
                for s in strings(d.get("용례.구")):
                    phrases.append(s)
                u = d.get("용례")
                if isinstance(u, list):
                    for item in u:
                        if isinstance(item, dict):
                            for s in strings(item.get("용례")):
                                add(s)
                        elif isinstance(item, str):
                            add(item)
            for s in phrases:
                add(s)
            # Headword and part of speech from the spell-checker result.
            result = entry.get("result")
            check = result.get("맞춤법 검사") if isinstance(result, dict) else None
            if not isinstance(check, dict) or check.get("제외"):
                continue
            word = check.get("표제어")
            pos = check.get("품사")
            if not isinstance(word, str):
                continue
            word = unicodedata.normalize("NFC", word.strip())
            pos = pos if isinstance(pos, str) else ""
            if pos == "조사":
                if re.fullmatch("[가-힣]+", word):
                    particles.add(word)
                continue
            if HANGUL_WORD.match(word):
                headwords.setdefault(word, pos)
            elif MIXED.match(word):
                headwords.setdefault(word, pos)
            # Loanwords and field terms.
            imports = entry.get("import")
            if not isinstance(imports, dict):
                continue
            for source, d in imports.items():
                if not isinstance(d, dict):
                    continue
                original = latin_original(d)
                fields = set()
                for m in walk(d):
                    fields.update(strings(m.get("분류")))
                if HANGUL_WORD.match(word) and (original or fields & FIELDS):
                    if original or word not in terms:
                        terms[word] = original or terms.get(word, "")
    return sentences, headwords, particles, terms, docs, bad


def count_headwords(sentences, headwords):
    """1 + the number of eojeols each headword starts (longest headword wins)."""
    keys = {}
    for w, pos in headwords.items():
        if not HANGUL_WORD.match(w):
            continue
        keys.setdefault(w, w)
        if pos in ("동사", "형용사") and w.endswith("다") and len(w) >= 3:
            keys.setdefault(w[:-1], w)
    longest = max((len(k) for k in keys), default=0)
    counts = Counter({w: 1 for w in headwords})
    for s in sentences:
        for e in EOJEOL.findall(s):
            for n in range(min(len(e), longest), 1, -1):
                w = keys.get(e[:n])
                if w:
                    counts[w] += 1
                    break
    return counts


def sample(lines):
    if len(lines) <= CAP:
        return lines
    k = -(-len(lines) // CAP)
    return lines[::k]


def write_lines(path, lines):
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    return len(lines)


def main():
    os.makedirs(OUT, exist_ok=True)
    sentences, headwords, particles, terms, docs, bad = extract()
    print(f"{docs} entries ({bad} unparsed), {len(sentences)} sentences, {len(headwords)} headwords")
    files = {}
    files["ko.txt"] = write_lines(os.path.join(OUT, "ko.txt"), sample(sentences))
    counts = count_headwords(sentences, headwords)
    rows = sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))
    files["ko_words.tsv"] = write_lines(os.path.join(OUT, "ko_words.tsv"),
                                        sample([f"{w}\t{n}" for w, n in rows]))
    files["particles.txt"] = write_lines(os.path.join(OUT, "particles.txt"), sorted(particles))
    files["terms.tsv"] = write_lines(os.path.join(OUT, "terms.tsv"),
                                     [f"{k}\t{v}" for k, v in sorted(terms.items())])
    mixed = set(w for w in headwords if MIXED.match(w))
    for s in sentences:
        for tok in TOKEN.findall(s):
            if MIXED.match(tok):
                mixed.add(tok)
    files["mixed.txt"] = write_lines(os.path.join(OUT, "mixed.txt"), sorted(mixed))
    meta = {"source": "hunspell-dict-ko", "tier": "moderate", "weight": 1, "uses": USES,
            "license": LICENSE, "files": files}
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    for name, n in files.items():
        print(f"  {name}: {n}")


if __name__ == "__main__":
    main()
