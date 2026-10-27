"""Extracts rspeer/wordfreq's frequency lists from corpora/open/wordfreq into
corpora/derived/wordfreq. Source: https://github.com/rspeer/wordfreq (wordfreq/data). License:
code Apache-2.0, data CC BY-SA 4.0 (README, LICENSE-data-CC-BY-SA-4.0.txt); built from Wikipedia,
subtitles (OpenSubtitles, SUBTLEX), news, books (Google Books Ngrams), web text and Twitter.
wordfreq's README asks that the data not be reshaped into bare CSVs that drop its attribution:
these files are local training inputs only (corpora/ is gitignored), meta.json carries the
attribution, and only counts derived from them reach the shipped ranker. Tier high, weight 1.5.

The files are cBpack: gzipped msgpack of [header {"format": "cB", ...}, bucket 1, bucket 2, ...],
where bucket i (counting from 1) lists the words whose frequency is -(i-1) centibels, so
freq = 10 ** (-(i-1) / 100). Decoded by the small stdlib msgpack reader below (no third-party
packages; the input is only parsed, never executed). Counts are written as frequency per billion
words, rounded (at least 1), so build_ranker.py's relative frequencies come out right.

- en_words.tsv: large_en tokens matching ^[a-z]+('[a-z]+)?$ (no digits, symbols or other
  scripts), the most frequent EN_CAP of them, plus "a" and "i"; then small_ko's Latin tokens
  (below) that the English list didn't already have, at their small_ko frequency.
- ko_words.tsv: small_ko tokens made only of syllables U+AC00-U+D7A3 (compatibility jamo, CJK
  ideographs, symbols and digit placeholders dropped). These are wordfreq's tokens, which for
  Korean are mostly morphemes and short words rather than whole typed words: good for jamo-key
  trigrams and the stem/particle lexicon, not for whole-word prediction.
- meta.json: source, tier, weight, uses, license (with attribution), line counts.

small_ko's ^[a-z]{2,5}$ tokens (single letters dropped) are the Latin words and acronyms seen in
Korean text (gpu, api, kb); they go to en_words.tsv as said, and are printed for reference. There
are no Latin+Hangul tokens (wordfreq splits scripts), so no mixed.txt or particles.txt.

Usage: python3 -I wordfreq.py
"""

import gzip
import json
import os
import re
import struct

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "wordfreq")
OUT = os.path.join(ROOT, "corpora", "derived", "wordfreq")

NAME = "wordfreq"
TIER = "high"
WEIGHT = 1.5
USES = ["english-key-model", "english-lexicon", "korean-key-model", "korean-lexicon",
        "particles-endings", "acronyms"]
LICENSE = ("CC BY-SA 4.0 (data), Apache-2.0 (code); wordfreq by Robyn Speer et al., "
           "https://github.com/rspeer/wordfreq. Includes data from Google Books Ngrams "
           "(http://books.google.com/ngrams), OpenSubtitles (http://www.opensubtitles.org/), "
           "SUBTLEX (Brysbaert et al.), Wikipedia and Twitter word statistics.")

EN_CAP = 150_000
CAP = 3_000_000
PER = 1e9

ENGLISH = re.compile(r"^[a-z]+('[a-z]+)?$")
HANGUL = re.compile(r"^[가-힣]+$")
KO_LATIN = re.compile(r"^[a-z]{2,5}$")


class MsgpackError(ValueError):
    pass


def unpack(data):
    """Decodes one msgpack value from bytes (the subset cBpack uses, plus the other scalar types)."""
    pos = 0

    def take(n):
        nonlocal pos
        if pos + n > len(data):
            raise MsgpackError("truncated")
        chunk = data[pos:pos + n]
        pos += n
        return chunk

    def value(depth=0):
        if depth > 64:
            raise MsgpackError("too deep")
        b = take(1)[0]
        if b <= 0x7f:
            return b
        if b >= 0xe0:
            return b - 0x100
        if 0xa0 <= b <= 0xbf:
            return take(b & 0x1f).decode("utf-8", "replace")
        if 0x90 <= b <= 0x9f:
            return [value(depth + 1) for _ in range(b & 0x0f)]
        if 0x80 <= b <= 0x8f:
            return {value(depth + 1): value(depth + 1) for _ in range(b & 0x0f)}
        if b == 0xc0:
            return None
        if b == 0xc2:
            return False
        if b == 0xc3:
            return True
        if b in (0xc4, 0xc5, 0xc6):  # bin 8/16/32
            n = struct.unpack(">" + "BHI"[b - 0xc4], take(1 << (b - 0xc4)))[0]
            return take(n)
        if b in (0xd9, 0xda, 0xdb):  # str 8/16/32
            n = struct.unpack(">" + "BHI"[b - 0xd9], take(1 << (b - 0xd9)))[0]
            return take(n).decode("utf-8", "replace")
        if b in (0xdc, 0xdd):  # array 16/32
            n = struct.unpack(">" + "HI"[b - 0xdc], take(2 << (b - 0xdc)))[0]
            return [value(depth + 1) for _ in range(n)]
        if b in (0xde, 0xdf):  # map 16/32
            n = struct.unpack(">" + "HI"[b - 0xde], take(2 << (b - 0xde)))[0]
            return {value(depth + 1): value(depth + 1) for _ in range(n)}
        ints = {0xcc: ">B", 0xcd: ">H", 0xce: ">I", 0xcf: ">Q",
                0xd0: ">b", 0xd1: ">h", 0xd2: ">i", 0xd3: ">q", 0xca: ">f", 0xcb: ">d"}
        if b in ints:
            fmt = ints[b]
            return struct.unpack(fmt, take(struct.calcsize(fmt)))[0]
        raise MsgpackError(f"unsupported type byte 0x{b:02x}")

    result = value()
    return result


def read_cbpack(path):
    """{word: frequency} from a cBpack file; the first (most frequent) bucket wins on repeats."""
    with gzip.open(path, "rb") as f:
        data = unpack(f.read())
    if not isinstance(data, list) or not data or not isinstance(data[0], dict) \
            or data[0].get("format") != "cB":
        raise MsgpackError(f"{path}: not a cBpack file")
    freqs = {}
    for i, bucket in enumerate(data[1:]):
        if not isinstance(bucket, list):
            continue
        freq = 10 ** (-i / 100)
        for word in bucket:
            if isinstance(word, str) and word not in freqs:
                freqs[word] = freq
    return freqs


def count(freq):
    return max(1, round(freq * PER))


def write_lines(name, lines, files):
    lines = list(lines)
    if len(lines) > CAP:
        step = len(lines) / CAP
        lines = [lines[int(i * step)] for i in range(CAP)]
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    files[name] = len(lines)


def main():
    os.makedirs(OUT, exist_ok=True)
    files = {}
    by_freq = lambda d: sorted(d.items(), key=lambda kv: (-kv[1], kv[0]))

    large_en = read_cbpack(os.path.join(SRC, "large_en.msgpack.gz"))
    english = [(w, f) for w, f in by_freq(large_en) if ENGLISH.match(w)]
    en = dict(english[:EN_CAP])
    for w in ("a", "i"):
        if w in large_en:
            en[w] = large_en[w]

    small_ko = read_cbpack(os.path.join(SRC, "small_ko.msgpack.gz"))
    ko = {w: f for w, f in small_ko.items() if HANGUL.match(w)}
    ko_latin = {w: f for w, f in small_ko.items() if KO_LATIN.match(w)}
    added = 0
    for w, f in ko_latin.items():
        if w not in en:
            en[w] = f
            added += 1

    write_lines("en_words.tsv", (f"{w}\t{count(f)}" for w, f in by_freq(en)), files)
    write_lines("ko_words.tsv", (f"{w}\t{count(f)}" for w, f in by_freq(ko)), files)

    meta = {"source": NAME, "tier": TIER, "weight": WEIGHT, "uses": USES,
            "license": LICENSE, "files": files}
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(json.dumps(files))
    print(f"large_en: {len(large_en)} tokens, {len(english)} pure-Latin; small_ko: "
          f"{len(small_ko)} tokens, {len(ko)} Hangul, {len(ko_latin)} Latin ({added} new to en)")
    print("small_ko Latin, most frequent:", " ".join(w for w, _ in by_freq(ko_latin)[:60]))


if __name__ == "__main__":
    main()
