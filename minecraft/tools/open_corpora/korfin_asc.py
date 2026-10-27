"""Extracts KorFin-ASC (corpora/open/korfin-asc, huggingface.co/datasets/amphora/korfin-asc
revision 07cc4a29, CC BY-SA 4.0) into corpora/derived/korfin-asc/ for build_ranker.py. Only
derived sentences, counts and terms are used; the sentiment labels are ignored.

KorFin-ASC is 8,818 (sentence, aspect, polarity) rows of Korean financial text: news headlines
from KLUE-TC and sentences from Naver Finance analyst reports, one parquet file
(data/train-*.parquet) with the string columns SID, TYPE, SRC, ASPECT and SENTIMENT. (The source
evaluation called them title/aspect/...; the file says otherwise.) A sentence repeats once per
aspect, so SRC is deduplicated first (6,104 distinct). pyarrow isn't available, so the file is read
by a small stdlib parquet reader (Thrift compact footer, PLAIN/dictionary pages, uncompressed/
snappy/gzip/zstd, the same one hike_code_switching.py carries); only SRC and ASPECT are read.

Cleaning: NFC; full-width ASCII (＋ ％ ｇ) folded to ASCII, NBSP to space, ∼ to ~, – to -, ㈜
dropped; … · → are breaks (… ends a headline clause, so it splits the line); bullet markers
(1) (1) ① ...) and arrows/glyphs (▶ ▲ ■ ↑ ↓ • ∙) are removed rather than split on, so
"부진 사유는 1) A, 2) B 때문" stays one sentence. Lines are split into sentences after a Hangul or
digit/Latin character followed by . ? ! and a space (했다. / 발생. / 임.), and at ": " after a
heading (달랬다: 삼성전자의 ...). A sentence is dropped if it has a CJK ideograph (美, 中: hanja
headline abbreviations), fewer than 2 Hangul syllables or 2 tokens, or is mostly digits and
percent signs ((QoQ +39.6%, YoY +65.8%)). Whitespace is collapsed and sentences are exactly
deduplicated. There are no URLs or markup in the source.

- ko.txt: the sentences, in source order (Latin words, acronyms and digits kept: ARPU가, QoQ,
  삼성SDI의).
- mixed.txt: distinct whitespace tokens mixing Latin letters and Hangul (LG전자에, OLED가,
  M&A에, 1Q17부터) with edge punctuation stripped, plus the mixed ASPECT names (삼성SDI,
  SK텔레콤), sorted.
- en_words.tsv: Latin words (acronyms and English nouns) inside the Korean sentences with their
  counts (ARPU, QoQ, YoY, PER, EPS, OLED); a Latin stem glued to Hangul (KT가) counts as KT, and
  a letter glued after a digit (1Q17, 3Q16) is not a word.
- ko_words.tsv: the ASPECT column, a business-entity lexicon of company and market names (셀트리온,
  이마트, 코스피), Hangul-only single words of 2+ syllables, count = distinct sentences naming it.
- terms.tsv: glosses in the text, korean<TAB>english: an acronym or English word with a Hangul
  gloss in parentheses (ASP(평균판매가격), DP(디스플레이), Underperform(매도)) and a Hangul word
  with a one-word English one (유럽자동차공업협회(ACEA), 재배치(relocation)). Only the Hangul
  word right before the parenthesis is taken, so a few pairs are loose (현금/EBITDA, 실적/...);
  YoY/QoQ glosses and English of more than 3 words are skipped.

It's small (~6k sentences): best as a finance-domain boost and a mixed-script/acronym+particle eval
set. No holdout is made here.

Usage: python3 -I korfin_asc.py
"""

import collections
import glob
import gzip
import json
import os
import re
import struct
import unicodedata
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "korfin-asc")
OUT = os.path.join(ROOT, "corpora", "derived", "korfin-asc")

COLUMNS = ("SRC", "ASPECT")


# --- minimal parquet reader -------------------------------------------------------------------

class Thrift:
    """Thrift compact protocol, decoded generically: structs become {field_id: value}."""

    def __init__(self, buf, pos=0):
        self.buf = buf
        self.pos = pos

    def byte(self):
        b = self.buf[self.pos]
        self.pos += 1
        return b

    def varint(self):
        shift = result = 0
        while True:
            b = self.byte()
            result |= (b & 0x7F) << shift
            if not b & 0x80:
                return result
            shift += 7

    def zigzag(self):
        n = self.varint()
        return (n >> 1) ^ -(n & 1)

    def value(self, t):
        if t == 1:
            return True
        if t == 2:
            return False
        if t == 3:
            b = self.byte()
            return b - 256 if b > 127 else b
        if t in (4, 5, 6):
            return self.zigzag()
        if t == 7:
            v = struct.unpack_from("<d", self.buf, self.pos)[0]
            self.pos += 8
            return v
        if t == 8:
            n = self.varint()
            v = bytes(self.buf[self.pos:self.pos + n])
            self.pos += n
            return v
        if t in (9, 10):
            h = self.byte()
            n, et = h >> 4, h & 0x0F
            if n == 15:
                n = self.varint()
            if et in (1, 2):
                return [self.byte() == 1 for _ in range(n)]
            return [self.value(et) for _ in range(n)]
        if t == 11:
            n = self.varint()
            if n == 0:
                return {}
            h = self.byte()
            return {self.value(h >> 4): self.value(h & 0x0F) for _ in range(n)}
        if t == 12:
            return self.struct()
        raise ValueError(f"thrift type {t}")

    def struct(self):
        out = {}
        fid = 0
        while True:
            h = self.byte()
            if h == 0:
                return out
            delta, t = h >> 4, h & 0x0F
            fid = fid + delta if delta else self.zigzag()
            out[fid] = self.value(t)


def snappy(data):
    pos = 0
    shift = n = 0
    while True:
        b = data[pos]
        pos += 1
        n |= (b & 0x7F) << shift
        if not b & 0x80:
            break
        shift += 7
    out = bytearray()
    end = len(data)
    while pos < end:
        tag = data[pos]
        pos += 1
        kind = tag & 3
        if kind == 0:
            ln = tag >> 2
            if ln >= 60:
                nb = ln - 59
                ln = int.from_bytes(data[pos:pos + nb], "little")
                pos += nb
            ln += 1
            out += data[pos:pos + ln]
            pos += ln
            continue
        if kind == 1:
            ln = ((tag >> 2) & 7) + 4
            off = ((tag >> 5) << 8) | data[pos]
            pos += 1
        elif kind == 2:
            ln = (tag >> 2) + 1
            off = int.from_bytes(data[pos:pos + 2], "little")
            pos += 2
        else:
            ln = (tag >> 2) + 1
            off = int.from_bytes(data[pos:pos + 4], "little")
            pos += 4
        if off <= 0 or off > len(out):
            raise ValueError("bad snappy offset")
        start = len(out) - off
        if off >= ln:
            out += out[start:start + ln]
        else:
            for i in range(ln):
                out.append(out[start + i])
    if len(out) != n:
        raise ValueError("bad snappy length")
    return bytes(out)


def decompress(codec, data, size):
    if codec == 0:
        return bytes(data)
    if codec == 1:
        return snappy(data)
    if codec == 2:
        return gzip.decompress(data) if data[:2] == b"\x1f\x8b" else zlib.decompress(data)
    if codec == 6:
        from compression import zstd  # Python 3.14+
        return zstd.decompress(data)
    raise ValueError(f"unsupported parquet codec {codec}")


def hybrid(buf, pos, end, width, count):
    """RLE/bit-packed hybrid decoder; returns `count` ints."""
    out = []
    vbytes = (width + 7) // 8
    while len(out) < count and pos < end:
        h = 0
        shift = 0
        while True:
            b = buf[pos]
            pos += 1
            h |= (b & 0x7F) << shift
            if not b & 0x80:
                break
            shift += 7
        if h & 1:
            groups = h >> 1
            nbytes = groups * width
            bits = int.from_bytes(buf[pos:pos + nbytes], "little")
            pos += nbytes
            mask = (1 << width) - 1
            for i in range(groups * 8):
                out.append((bits >> (i * width)) & mask if width else 0)
        else:
            run = h >> 1
            v = int.from_bytes(buf[pos:pos + vbytes], "little") if vbytes else 0
            pos += vbytes
            out.extend([v] * run)
    return out[:count]


def plain_byte_arrays(buf, pos, count):
    out = []
    for _ in range(count):
        n = int.from_bytes(buf[pos:pos + 4], "little")
        pos += 4
        out.append(bytes(buf[pos:pos + n]))
        pos += n
    return out


def read_column(f, meta, max_def):
    """All values of one top-level BYTE_ARRAY column chunk (None for nulls)."""
    codec = meta[4]
    start = meta.get(11) or meta[9]
    if meta.get(11) is not None and meta[11] > meta[9]:
        start = meta[9]
    f.seek(start)
    chunk = f.read(meta[7])
    pos = 0
    dictionary = None
    values = []
    total = meta[5]
    while len(values) < total and pos < len(chunk):
        th = Thrift(chunk, pos)
        ph = th.struct()
        pos = th.pos
        csize = ph[3]
        raw = chunk[pos:pos + csize]
        pos += csize
        ptype = ph[1]
        if ptype == 2:
            dh = ph[7]
            page = decompress(codec, raw, ph[2])
            dictionary = plain_byte_arrays(page, 0, dh[1])
            continue
        if ptype == 0:
            dh = ph[5]
            n = dh[1]
            enc = dh[2]
            page = decompress(codec, raw, ph[2])
            p = 0
            if max_def:
                ln = int.from_bytes(page[0:4], "little")
                defs = hybrid(page, 4, 4 + ln, 1, n)
                p = 4 + ln
            else:
                defs = [1] * n
        elif ptype == 3:
            dh = ph[8]
            n = dh[1]
            enc = dh[4]
            dlen = dh.get(5, 0)
            rlen = dh.get(6, 0)
            levels = raw[:rlen + dlen]
            body = raw[rlen + dlen:]
            if dh.get(7, True):
                body = decompress(codec, body, ph[2] - rlen - dlen)
            defs = hybrid(levels, rlen, rlen + dlen, 1, n) if max_def else [1] * n
            page = body
            p = 0
        else:
            continue
        present = sum(1 for d in defs if d == max_def) if max_def else n
        if enc == 0:
            vals = plain_byte_arrays(page, p, present)
        elif enc in (2, 8):
            width = page[p]
            idx = hybrid(page, p + 1, len(page), width, present)
            vals = [dictionary[i] for i in idx]
        else:
            raise ValueError(f"unsupported parquet encoding {enc}")
        it = iter(vals)
        for d in defs:
            values.append(next(it) if d == max_def else None)
    return values


def read_parquet(path, wanted):
    """Yields dicts of the wanted top-level string columns, row group by row group."""
    with open(path, "rb") as f:
        f.seek(-8, 2)
        tail = f.read(8)
        if tail[4:] != b"PAR1":
            raise ValueError("not a parquet file")
        mlen = int.from_bytes(tail[:4], "little")
        f.seek(-8 - mlen, 2)
        fmd = Thrift(f.read(mlen)).struct()
        # Top-level optional columns have max definition level 1, required ones 0.
        schema = fmd[2]
        repetition = {}
        i = 1
        while i < len(schema):
            el = schema[i]
            name = el[4].decode("utf-8")
            repetition[name] = el.get(3, 0)
            # Skip nested children of group columns (audio).
            def skip(j):
                kids = schema[j].get(5, 0)
                j += 1
                for _ in range(kids):
                    j = skip(j)
                return j
            i = skip(i)
        for rg in fmd[4]:
            cols = {}
            for cc in rg[1]:
                meta = cc[3]
                path_ = [p.decode("utf-8") for p in meta[3]]
                if len(path_) == 1 and path_[0] in wanted:
                    max_def = 1 if repetition.get(path_[0]) == 1 else 0
                    cols[path_[0]] = read_column(f, meta, max_def)
            n = rg[3]
            for r in range(n):
                yield {c: (cols[c][r].decode("utf-8") if c in cols and cols[c][r] is not None
                           else None) for c in wanted}


# --- extraction --------------------------------------------------------------------------------

HANGUL = re.compile(r"[가-힣]")
LATIN = re.compile(r"[A-Za-z]")
CJK = re.compile(r"[㐀-䶿一-鿿豈-﫿]")
FULLWIDTH = {c: c - 0xFEE0 for c in range(0xFF01, 0xFF5F)}
FOLD = {0xA0: " ", 0x2009: " ", 0x223C: "~", 0x2013: "-", 0x3231: "", 0x2026: "\n"}
GLYPHS = re.compile(r"[▶▷▲△▼▽■□●○◆◇•∙↑↓★☆※]")
ARROW = re.compile(r"\s*[→←⇒]\s*")
CIRCLED = re.compile(r"[①-⑳]\s*")
BULLET = re.compile(r"(^|(?<=[\s,(]))\(?[1-9]\)\s*")
SENT_END = re.compile(r"(?<=[가-힣A-Za-z0-9%)][.?!])\s+")
HEADING = re.compile(r"(?<=[가-힣A-Za-z0-9]):\s+")
EDGE = "\"'“”‘’()[]{}<>《》〈〉「」『』.,;:!?·~-/"
MIXED = re.compile(r"(?=.*[A-Za-z])(?=.*[가-힣])[A-Za-z0-9가-힣&+.\-]+")
LATIN_WORD = re.compile(r"(?<![A-Za-z0-9])[A-Za-z][A-Za-z0-9&\-]*[A-Za-z0-9]|[A-Za-z]")
GLOSS_EN_KO = re.compile(r"(?<![A-Za-z0-9])([A-Za-z][A-Za-z0-9&\-]*(?: [A-Za-z][A-Za-z0-9&\-]*){0,2})"
                         r"\(([가-힣]+(?: [가-힣]+){0,3})\)")
GLOSS_KO_EN = re.compile(r"(?<![가-힣])([가-힣]{2,})\(([A-Za-z][A-Za-z0-9&\-]*)\)")
NOT_TERMS = {"yoy", "qoq", "mom", "e", "f", "p"}


def normalize(text):
    text = unicodedata.normalize("NFC", text or "")
    text = text.translate(FULLWIDTH).translate(FOLD)
    text = GLYPHS.sub(" ", text)
    text = ARROW.sub(" ", text)
    text = CIRCLED.sub("", text)
    return text


def sentences(text):
    for line in normalize(text).split("\n"):
        line = BULLET.sub(" ", line)
        for part in SENT_END.split(line):
            for sent in HEADING.split(part):
                sent = " ".join(sent.split()).strip(" ,;")
                if sent:
                    yield sent


def keep(sent):
    if CJK.search(sent):
        return "hanja"
    if len(HANGUL.findall(sent)) < 2 or len(sent.split()) < 2:
        return "short"
    body = sent.replace(" ", "")
    numeric = sum(1 for c in body if c.isdigit() or c in "%.,+-()")
    if numeric * 2 > len(body):
        return "numeric"
    return None


def write_lines(name, lines, files):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    files[name] = len(lines)


def main():
    paths = sorted(glob.glob(os.path.join(SRC, "data", "*.parquet")))
    if not paths:
        raise SystemExit(f"no parquet under {SRC}/data")
    sources = {}  # distinct SRC text -> set of aspects (insertion order = source order)
    for path in paths:
        for row in read_parquet(path, COLUMNS):
            src = row["SRC"]
            if not src:
                continue
            sources.setdefault(src, set())
            if row["ASPECT"]:
                sources[src].add(unicodedata.normalize("NFC", row["ASPECT"]).strip())

    out, seen = [], set()
    dropped = collections.Counter()
    terms = {}
    for src in sources:
        norm = normalize(src)
        for m in GLOSS_EN_KO.finditer(norm):
            en, ko = m.group(1), m.group(2)
            if en.lower() not in NOT_TERMS and len(en) >= 2:
                terms.setdefault((ko, en), None)
        for m in GLOSS_KO_EN.finditer(norm):
            ko, en = m.group(1), m.group(2)
            if en.lower() not in NOT_TERMS and len(en) >= 2:
                terms.setdefault((ko, en), None)
        for sent in sentences(src):
            why = keep(sent)
            if why:
                dropped[why] += 1
                continue
            if sent in seen:
                dropped["duplicate"] += 1
                continue
            seen.add(sent)
            out.append(sent)

    mixed = set()
    en_words = collections.Counter()
    for sent in out:
        for tok in sent.split():
            tok = tok.strip(EDGE)
            if tok and MIXED.fullmatch(tok):
                mixed.add(tok)
        for w in LATIN_WORD.findall(sent):
            if len(w) >= 2:
                en_words[w] += 1

    aspects = collections.Counter()
    for src, names in sources.items():
        for name in names:
            if MIXED.fullmatch(name):
                mixed.add(name)
            elif re.fullmatch(r"[가-힣]{2,}", name):
                aspects[name] += 1

    os.makedirs(OUT, exist_ok=True)
    files = {}
    write_lines("ko.txt", out, files)
    write_lines("mixed.txt", sorted(mixed), files)
    write_lines("en_words.tsv", [f"{w}\t{n}" for w, n in
                                 sorted(en_words.items(), key=lambda x: (-x[1], x[0]))], files)
    write_lines("ko_words.tsv", [f"{w}\t{n}" for w, n in
                                 sorted(aspects.items(), key=lambda x: (-x[1], x[0]))], files)
    write_lines("terms.tsv", [f"{ko}\t{en}" for ko, en in sorted(terms)], files)
    meta = {
        "source": "korfin-asc",
        "tier": "moderate",
        "weight": 1.2,
        "uses": ["korean-key-model", "korean-lexicon", "mixed-ko-en", "business-terms", "acronyms",
                 "english-nouns", "eval-only"],
        "license": "CC BY-SA 4.0",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    syllables = sum(len(HANGUL.findall(s)) for s in out)
    print(f"{len(sources)} distinct source texts -> {len(out)} sentences, {syllables} Hangul "
          f"syllables; dropped: {dict(dropped)}; " + ", ".join(f"{k}: {v}" for k, v in files.items()))


if __name__ == "__main__":
    main()
