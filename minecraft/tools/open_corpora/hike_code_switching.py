"""Extracts HiKE (corpora/open/hike-code-switching, huggingface.co/datasets/thetaone-ai/HiKE,
Apache-2.0) into corpora/derived/hike-code-switching/ for build_ranker.py. Only the transcripts are
used; the audio is never decoded.

HiKE is a Korean-English code-switching speech benchmark: 1,121 utterances in one parquet file
(data/test-00000-of-00001.parquet), each with a transcript (`text`), a code-switching level
(`cs_level`: word, phrase or sentence), a topic `category` and a JSON list of the loanwords in it
(`loanwords`, [{"Korean": "버그", "English": "bug"}]). pyarrow isn't available, so the file is
read by a small stdlib parquet reader below (Thrift compact footer, PLAIN/dictionary pages,
uncompressed/snappy/gzip/zstd); it seeks past the audio column and reads only text, cs_level, category, loanwords and sample_id. text_normalized
(lowercased, no punctuation) and text_pier_labeled (markup) are skipped.

Holdout: the recipe reserves about half the utterances as an evaluation set for mixed-word
ranking. Rows are split per cs_level (sorted by sample_id, shuffled with random.Random(20261027),
first half kept), and every output below comes from the kept half only, so the held-out half stays
unseen. meta.json lists the held-out sample_ids under "holdout".

Cleaning: NFC, U+2010 to "-", whitespace collapsed, exact duplicates dropped. An utterance counts
as Korean-matrix when at least half its whitespace tokens hold Hangul and Hangul is at least 30% of
its letters, a Hangul syllable counting as two letters. (The recipe's plain "30% of letters" test
rejects ordinary sentences such as "이번 bug는 session management logic에 문제가 있었어.", since
each English word outweighs a syllable.) The `medical` category is not downweighted.

- ko.txt: the Korean-matrix utterances whole (Latin words kept), plus runs of 3+ Hangul tokens
  from the English-matrix ones (잘 보이려고만 만든), one per line.
- en.txt: runs of 4+ tokens without Hangul cut out of every kept utterance: the English sentences
  and phrases of sentence- and phrase-level switching (often clause fragments).
- mixed.txt: distinct whitespace tokens of a Latin stem followed by Hangul (sprint에서, API를,
  t-test를), edge punctuation stripped, sorted.
- particles.txt: the Hangul tails of those mixed tokens (를, 에서, 이랑, 하더라), most frequent
  first; tails longer than two syllables seen only once (title곡은 -> 곡은 stays, 기획부터 goes)
  are compounds and are dropped.
- en_words.tsv: Latin stems of the mixed tokens plus the Latin words standing alone inside
  Korean-matrix utterances, with counts (English nouns and acronyms used in Korean).
- terms.tsv: the `loanwords` labels, korean<TAB>english (프로젝트/project), deduplicated. The
  labels are hand-made and carry a few mismatches (네트워크/Netflix); they are kept as given.

Usage: python3 -I hike_code_switching.py
"""

import collections
import gzip
import json
import os
import random
import re
import struct
import unicodedata
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "hike-code-switching")
PARQUET = os.path.join(SRC, "data", "test-00000-of-00001.parquet")
OUT = os.path.join(ROOT, "corpora", "derived", "hike-code-switching")

COLUMNS = ("text", "cs_level", "category", "loanwords", "sample_id")


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
EDGE = "\"'“”‘’()[]{}<>《》〈〉「」『』.,;:!?…·~—-"
MIXED = re.compile(r"([A-Za-z][A-Za-z0-9.&+\-]*?)([가-힣]+)")
LATIN_WORD = re.compile(r"[A-Za-z][A-Za-z0-9.&+'\-]*[A-Za-z0-9]|[A-Za-z]")
SEED = 20261027


def clean(text):
    text = unicodedata.normalize("NFC", text or "")
    text = text.replace("‐", "-").replace(" ", " ")
    return " ".join(text.split())


def is_korean(text):
    """Korean-matrix utterance: Hangul-bearing tokens are at least half of the tokens and Hangul
    is at least 30% of the letters, counting a Hangul syllable as two letters (a jamo pair)."""
    toks = text.split()
    ko = sum(1 for t in toks if HANGUL.search(t))
    hg = len(HANGUL.findall(text)) * 2
    la = len(LATIN.findall(text))
    return toks and ko * 2 >= len(toks) and hg >= 0.3 * (hg + la)


def runs(text, korean_side, min_words):
    """Runs of consecutive tokens on one side (Hangul-bearing or not) with min_words+ words."""
    out = []
    run = []
    for tok in text.split() + [None]:
        if tok is not None and bool(HANGUL.search(tok)) == korean_side:
            run.append(tok)
            continue
        words = [t for t in run if (HANGUL if korean_side else LATIN).search(t)]
        if len(words) >= min_words:
            line = " ".join(run).strip(" ,;:—-")
            if line:
                out.append(line)
        run = []
    return out


def split_holdout(rows):
    by_level = collections.defaultdict(list)
    for r in rows:
        by_level[r["cs_level"] or ""].append(r)
    rng = random.Random(SEED)
    keep, hold = [], []
    for level in sorted(by_level):
        group = sorted(by_level[level], key=lambda r: r["sample_id"] or "")
        rng.shuffle(group)
        half = (len(group) + 1) // 2
        keep += group[:half]
        hold += group[half:]
    order = {id(r): i for i, r in enumerate(rows)}
    keep.sort(key=lambda r: order[id(r)])
    return keep, sorted(r["sample_id"] for r in hold)


def write_lines(name, lines, files):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    files[name] = len(lines)


def main():
    rows = []
    seen = set()
    for row in read_parquet(PARQUET, COLUMNS):
        row["text"] = clean(row["text"])
        if not row["text"] or row["text"] in seen:
            continue
        seen.add(row["text"])
        rows.append(row)
    keep, holdout = split_holdout(rows)

    ko, en = [], []
    en_seen = set()
    seen_ko = set()
    mixed = set()
    particles = collections.Counter()
    en_words = collections.Counter()
    terms = {}
    for row in keep:
        text = row["text"]
        korean = is_korean(text)
        if korean:
            ko.append(text)
        else:
            for run in runs(text, True, 3):
                if run not in seen_ko:
                    seen_ko.add(run)
                    ko.append(run)
        for run in runs(text, False, 4):
            if run not in en_seen:
                en_seen.add(run)
                en.append(run)
        for tok in text.split():
            tok = tok.strip(EDGE)
            m = MIXED.fullmatch(tok)
            if m:
                mixed.add(tok)
                particles[m.group(2)] += 1
                en_words[m.group(1).rstrip(".-")] += 1
            elif korean and not HANGUL.search(tok):
                for w in LATIN_WORD.findall(tok):
                    en_words[w] += 1
        try:
            pairs = json.loads(row["loanwords"] or "[]")
        except ValueError:
            pairs = []
        for p in pairs if isinstance(pairs, list) else []:
            if not isinstance(p, dict):
                continue
            k = clean(str(p.get("Korean") or ""))
            e = clean(str(p.get("English") or ""))
            if k and HANGUL.search(k) and "\t" not in k + e and (k, e) not in terms:
                terms[(k, e)] = True

    os.makedirs(OUT, exist_ok=True)
    files = {}
    write_lines("ko.txt", ko, files)
    write_lines("en.txt", en, files)
    write_lines("mixed.txt", sorted(mixed), files)
    # One-off long tails are compounds (title곡은, 기획부터), not particles.
    write_lines("particles.txt", [p for p, c in sorted(particles.items(),
                                                      key=lambda kv: (-kv[1], kv[0]))
                                  if c >= 2 or len(p) <= 2], files)
    write_lines("en_words.tsv", [f"{w}\t{c}" for w, c in sorted(en_words.items(),
                                                                key=lambda kv: (-kv[1], kv[0]))],
                files)
    write_lines("terms.tsv", [f"{k}\t{e}" for k, e in sorted(terms)], files)
    meta = {
        "source": "hike-code-switching",
        "tier": "moderate",
        "weight": 0.7,
        "uses": ["mixed-ko-en", "eval-only", "business-terms", "particles-endings",
                 "english-nouns", "acronyms"],
        "license": "Apache-2.0",
        "files": files,
        "rows": len(rows),
        "kept_rows": len(keep),
        "holdout": {
            "rule": "per cs_level, sorted by sample_id, random.Random(20261027).shuffle, "
                    "second half held out; no output above uses these rows",
            "sample_ids": holdout,
        },
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    for name, n in files.items():
        print(f"{name}: {n}")


if __name__ == "__main__":
    main()
