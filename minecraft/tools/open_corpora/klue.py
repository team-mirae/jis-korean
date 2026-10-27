"""Extracts KLUE (corpora/open/klue, huggingface.co/datasets/klue/klue revision 349481ec, CC BY-SA
4.0) into corpora/derived/klue/ for build_ranker.py. Only derived counts and sentences are used.

KLUE is the Korean Language Understanding Evaluation benchmark: eight tasks over news, Wikipedia,
policy briefings, reviews and task-oriented dialogue, each a train and a validation Parquet file.
Labels are ignored; only the Korean text columns are read. Parquet needs no third-party package
here: the module carries a small stdlib-only reader (Thrift compact footer, snappy, PLAIN and
dictionary-encoded byte arrays, RLE/bit-packed levels) that reads just the string columns asked
for and refuses anything it does not understand.

- wos: dialogue.text, user and system turns of hotel/restaurant/taxi/... booking dialogues (the
  best chat register in KLUE).
- sts: sentence1, sentence2 (airbnb reviews, policy news, paraphrases).
- nli: premise, hypothesis.
- re: sentence (the entity markers live in separate columns, so the sentence is plain text).
- mrc: context (Wikipedia and news paragraphs, deduplicated, split into sentences) and question.
- dp: sentence (mostly new text; exact dedupe drops what it shares with the others), and its
  morpheme analysis (lemma + pos) for particles.txt.
- ynat: title (news headlines; they drop particles, so they come last and count once like any
  other line). ynat's url column is not read.
- ner is skipped: its sentence column carries inline <entity:TAG> markup and its text repeats the
  other news/Wikipedia sources.

Cleaning: NFC; mrc prose is split into sentences on . ? ! followed by whitespace, on
newlines, and between a Hangul . ? ! and the Hangul right after it (했다.이 같은);
citation markers ([1], [주해 1], [출처 필요]) and parentheses holding another script (hanja, kana,
Cyrillic: 서울(漢陽)) are removed, and any sentence still holding such characters is dropped, as is
any sentence with a URL, e-mail address or no Hangul. Whitespace is normalized and sentences are
exactly deduplicated across all tasks.

- ko.txt: the deduplicated sentences (Latin words, acronyms and digits kept).
- particles.txt: particles (J*) and endings (E*) from dp's morpheme analysis, seen at least
  MIN_PARTICLE times, most frequent first.
- mixed.txt: distinct tokens mixing Latin letters and Hangul (KT가, SNS에서, LG전자), sorted;
  wos's random reservation numbers (ABEF9입니다) are left out (they stay in ko.txt).

Usage: python3 -I klue.py
"""

import collections
import json
import os
import re
import struct
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "klue")
OUT = os.path.join(ROOT, "corpora", "derived", "klue")

SPLITS = ("train", "validation")
# task -> (columns, whether the text is prose to split into sentences). Columns are dotted paths
# through the Parquet schema; list levels are skipped when matching.
TASKS = (
    ("wos", ("dialogue.text",), False),
    ("sts", ("sentence1", "sentence2"), False),
    ("nli", ("premise", "hypothesis"), False),
    ("re", ("sentence",), False),
    ("mrc", ("context", "question"), True),
    ("dp", ("sentence",), False),
    ("ynat", ("title",), False),
)
MIN_PARTICLE = 3


# --- Minimal Parquet reader (string columns only) ---------------------------------------------

class ParquetError(Exception):
    pass


def snappy_decompress(data):
    pos, length, shift = 0, 0, 0
    while True:
        b = data[pos]
        pos += 1
        length |= (b & 0x7F) << shift
        if b < 0x80:
            break
        shift += 7
    out = bytearray()
    n = len(data)
    while pos < n:
        tag = data[pos]
        kind = tag & 3
        if kind == 0:
            size = tag >> 2
            if size < 60:
                pos += 1
            else:
                nb = size - 59
                size = int.from_bytes(data[pos + 1:pos + 1 + nb], "little")
                pos += 1 + nb
            size += 1
            out += data[pos:pos + size]
            pos += size
            continue
        if kind == 1:
            size = ((tag >> 2) & 7) + 4
            offset = ((tag >> 5) << 8) | data[pos + 1]
            pos += 2
        elif kind == 2:
            size = (tag >> 2) + 1
            offset = int.from_bytes(data[pos + 1:pos + 3], "little")
            pos += 3
        else:
            size = (tag >> 2) + 1
            offset = int.from_bytes(data[pos + 1:pos + 5], "little")
            pos += 5
        if offset == 0 or offset > len(out):
            raise ParquetError("bad snappy offset")
        start = len(out) - offset
        if offset >= size:
            out += out[start:start + size]
        else:
            for i in range(size):
                out.append(out[start + i])
    if len(out) != length:
        raise ParquetError("snappy length mismatch")
    return bytes(out)


class Thrift:
    """Thrift compact protocol: structs become {field id: value}."""

    def __init__(self, data, pos=0):
        self.data, self.pos = data, pos

    def byte(self):
        b = self.data[self.pos]
        self.pos += 1
        return b

    def varint(self):
        result, shift = 0, 0
        while True:
            b = self.byte()
            result |= (b & 0x7F) << shift
            if b < 0x80:
                return result
            shift += 7

    def zigzag(self):
        v = self.varint()
        return (v >> 1) ^ -(v & 1)

    def value(self, t):
        if t == 1:
            return True
        if t == 2:
            return False
        if t == 3:
            return struct.unpack("b", bytes([self.byte()]))[0]
        if t in (4, 5, 6):
            return self.zigzag()
        if t == 7:
            v = struct.unpack("<d", self.data[self.pos:self.pos + 8])[0]
            self.pos += 8
            return v
        if t == 8:
            n = self.varint()
            v = self.data[self.pos:self.pos + n]
            self.pos += n
            return v
        if t in (9, 10):
            h = self.byte()
            size, et = h >> 4, h & 0xF
            if size == 15:
                size = self.varint()
            if et in (1, 2):
                return [self.byte() == 1 for _ in range(size)]
            return [self.value(et) for _ in range(size)]
        if t == 11:
            size = self.varint()
            if size == 0:
                return {}
            h = self.byte()
            return {self.value(h >> 4): self.value(h & 0xF) for _ in range(size)}
        if t == 12:
            return self.struct()
        raise ParquetError(f"thrift type {t}")

    def struct(self):
        fields, last = {}, 0
        while True:
            h = self.byte()
            if h == 0:
                return fields
            delta, t = h >> 4, h & 0xF
            fid = last + delta if delta else self.zigzag()
            fields[fid] = self.value(t)
            last = fid


def rle_hybrid(data, pos, end, width, count):
    """Decodes count values of the RLE/bit-packed hybrid encoding from data[pos:end]."""
    out = []
    nbytes = (width + 7) // 8
    mask = (1 << width) - 1
    while len(out) < count and pos < end:
        header, shift = 0, 0
        while True:
            b = data[pos]
            pos += 1
            header |= (b & 0x7F) << shift
            if b < 0x80:
                break
            shift += 7
        if header & 1:
            groups = header >> 1
            size = groups * width
            chunk = int.from_bytes(data[pos:pos + size], "little")
            pos += size
            for i in range(groups * 8):
                out.append((chunk >> (i * width)) & mask)
        else:
            run = header >> 1
            v = int.from_bytes(data[pos:pos + nbytes], "little")
            pos += nbytes
            out.extend([v] * run)
    if len(out) < count:
        raise ParquetError("short level/index run")
    return out[:count]


def plain_strings(data, pos, count):
    out = []
    for _ in range(count):
        n = int.from_bytes(data[pos:pos + 4], "little")
        pos += 4
        out.append(data[pos:pos + n])
        pos += n
    return out


def read_strings(path, column):
    """Yields every non-null value (bytes) of a BYTE_ARRAY column, in file order.

    column is a dotted path of field names; the list/element wrappers of nested lists are
    matched implicitly.
    """
    with open(path, "rb") as f:
        data = f.read()
    if data[:4] != b"PAR1" or data[-4:] != b"PAR1":
        raise ParquetError(f"{path}: not Parquet")
    flen = int.from_bytes(data[-8:-4], "little")
    meta = Thrift(data, len(data) - 8 - flen).struct()
    schema = meta[2]
    # Walk the schema tree (depth-first, num_children) to find leaf levels.
    leaves = {}

    def walk(i, names, max_def, max_rep):
        el = schema[i]
        rep = el.get(3, 0)  # 0 REQUIRED, 1 OPTIONAL, 2 REPEATED
        if i:
            names = names + [el[4].decode()]
            max_def += rep != 0
            max_rep += rep == 2
        i += 1
        kids = el.get(5, 0)
        if not kids and i - 1:
            leaves[tuple(names)] = (max_def, max_rep, el.get(1))
        for _ in range(kids):
            i = walk(i, names, max_def, max_rep)
        return i

    walk(0, [], 0, 0)
    want = column.split(".")
    found = None
    for names, info in leaves.items():
        if [n for n in names if n not in ("list", "element", "item", "array")] == want:
            found = names, info
    if not found:
        raise ParquetError(f"{path}: no column {column}")
    names, (max_def, max_rep, ptype) = found
    if ptype != 6:
        raise ParquetError(f"{path}: {column} is not BYTE_ARRAY")
    for rg in meta[4]:
        for cc in rg[1]:
            cm = cc[3]
            if tuple(p.decode() for p in cm[3]) != names:
                continue
            codec = cm[4]
            if codec not in (0, 1):
                raise ParquetError(f"{path}: codec {codec}")
            yield from read_chunk(data, cm, codec, max_def, max_rep)


def read_chunk(data, cm, codec, max_def, max_rep):
    total = cm[5]
    pos = cm.get(11) or cm[9]
    if cm.get(11) is not None and cm[9] < pos:
        pos = cm[9]
    end = pos + cm[7]
    dictionary = None
    seen = 0
    while seen < total and pos < end:
        t = Thrift(data, pos)
        ph = t.struct()
        pos = t.pos
        csize = ph[3]
        body = data[pos:pos + csize]
        pos += csize
        ptype = ph[1]
        if ptype == 2:  # dictionary page
            raw = snappy_decompress(body) if codec == 1 else body
            dictionary = plain_strings(raw, 0, ph[7][1])
            continue
        if ptype == 0:  # data page v1
            raw = snappy_decompress(body) if codec == 1 else body
            hdr = ph[5]
            n, enc = hdr[1], hdr[2]
            p = 0
            if max_rep:
                ln = int.from_bytes(raw[p:p + 4], "little")
                p += 4 + ln
            if max_def:
                ln = int.from_bytes(raw[p:p + 4], "little")
                defs = rle_hybrid(raw, p + 4, p + 4 + ln, max_def.bit_length(), n)
                p += 4 + ln
            else:
                defs = None
        elif ptype == 3:  # data page v2
            hdr = ph[8]
            n, enc = hdr[1], hdr[4]
            dlen, rlen = hdr.get(5, 0), hdr.get(6, 0)
            if max_def:
                defs = rle_hybrid(body, rlen, rlen + dlen, max_def.bit_length(), n)
            else:
                defs = None
            rest = body[rlen + dlen:]
            raw = snappy_decompress(rest) if codec == 1 and hdr.get(7, True) else rest
            p = 0
        else:
            continue
        seen += n
        present = n if defs is None else sum(1 for d in defs if d == max_def)
        if enc == 0:
            values = plain_strings(raw, p, present)
        elif enc in (2, 8):
            if dictionary is None:
                raise ParquetError("dictionary index without dictionary")
            width = raw[p]
            idx = rle_hybrid(raw, p + 1, len(raw), width, present) if width else [0] * present
            values = [dictionary[i] for i in idx]
        else:
            raise ParquetError(f"encoding {enc}")
        yield from values


# --- Extraction --------------------------------------------------------------------------------

CITATION = re.compile(r"\[(\d+|\*|주해 ?\d+|출처 필요)\]")
FOREIGN = ("\u0370-\u03ff\u0400-\u04ff\u0590-\u05ff\u0600-\u06ff\u0750-\u077f\u0900-\u097f"
           "\u0e00-\u0e7f\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff")
FOREIGN_CHAR = re.compile(f"[{FOREIGN}]")
FOREIGN_PAREN = re.compile(rf"\s*\([^()]*[{FOREIGN}][^()]*\)")
URL = re.compile(r"https?://|www\.|\.(com|net|org|kr)\b|@[A-Za-z0-9-]+\.", re.I)
SENT_SPLIT = re.compile(r"(?<=[.?!])\s+|(?<=[가-힣][.?!])(?=[가-힣])|\n+")
HANGUL = re.compile(r"[가-힣]")
EDGE = "\"'“”‘’()[]{}<>《》〈〉「」『』.,;:!?…·~-"
MIXED = re.compile(r"(?=.*[A-Za-z])(?=.*[가-힣])[A-Za-z0-9가-힣&+.\-]+")
# wos reservation numbers (ABEF9입니다): random codes, not words.
BOOKING_CODE = re.compile(r"[A-Z]{4,}[0-9][가-힣]")
PARTICLE_TAG = re.compile(r"^(J[A-Z]+|E[A-Z]+)$")


def texts():
    for task, columns, prose in TASKS:
        for split in SPLITS:
            path = os.path.join(SRC, task, f"{split}-00000-of-00001.parquet")
            for column in columns:
                seen = set()
                for raw in read_strings(path, column):
                    if prose:  # mrc repeats a context once per question
                        if raw in seen:
                            continue
                        seen.add(raw)
                    yield task, raw.decode("utf-8", "replace"), prose


def clean(s):
    s = unicodedata.normalize("NFC", s)
    s = CITATION.sub("", s)
    s = FOREIGN_PAREN.sub("", s)
    return s


def particles():
    counts = collections.Counter()
    for split in SPLITS:
        path = os.path.join(SRC, "dp", f"{split}-00000-of-00001.parquet")
        for lemma, pos in zip(read_strings(path, "lemma"), read_strings(path, "pos")):
            ms = unicodedata.normalize("NFC", lemma.decode("utf-8", "replace")).split()
            ts = pos.decode("utf-8", "replace").split("+")
            if len(ms) != len(ts):
                continue
            for m, t in zip(ms, ts):
                if PARTICLE_TAG.match(t) and HANGUL.fullmatch(m[0]) and HANGUL.search(m):
                    counts[m] += 1
    return [m for m, c in sorted(counts.items(), key=lambda kv: (-kv[1], kv[0])) if c >= MIN_PARTICLE]


def write_lines(name, lines):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    return len(lines)


def main():
    seen = set()
    sentences = []
    per_task = collections.Counter()
    dropped = collections.Counter()
    for task, text, prose in texts():
        text = clean(text)
        parts = SENT_SPLIT.split(text) if prose else [text]
        for sent in parts:
            sent = " ".join(sent.split())
            if len(sent) < 2 or not HANGUL.search(sent) or sent in seen:
                continue
            seen.add(sent)
            if FOREIGN_CHAR.search(sent):
                dropped["foreign"] += 1
                continue
            if URL.search(sent):
                dropped["url"] += 1
                continue
            sentences.append(sent)
            per_task[task] += 1

    mixed = set()
    for sent in sentences:
        for tok in sent.split():
            tok = tok.strip(EDGE)
            if tok and MIXED.fullmatch(tok) and not BOOKING_CODE.match(tok):
                mixed.add(tok)

    os.makedirs(OUT, exist_ok=True)
    files = {"ko.txt": write_lines("ko.txt", sentences)}
    files["particles.txt"] = write_lines("particles.txt", particles())
    files["mixed.txt"] = write_lines("mixed.txt", sorted(mixed))
    meta = {
        "source": "klue",
        "tier": "high",
        "weight": 1.5,
        "uses": ["korean-key-model", "korean-lexicon", "mixed-ko-en", "acronyms", "particles-endings",
                 "business-terms", "typos"],
        "license": "CC BY-SA 4.0",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    eojeols = sum(len(s.split()) for s in sentences)
    print(f"{len(sentences)} sentences, {eojeols} eojeols; per task: "
          + ", ".join(f"{k} {v}" for k, v in per_task.items())
          + f"; dropped: {dict(dropped)}; " + ", ".join(f"{k}: {v}" for k, v in files.items()))


if __name__ == "__main__":
    main()
