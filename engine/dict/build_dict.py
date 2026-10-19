"""Builds hanja.tsv, the engine's hanja dictionary (spec §7), from:

- libhangul data/hanja/hanja.txt (BSD-3-Clause, see LICENSE-libhangul-hanja.txt)
- Unicode Unihan_Readings.txt, kHangul field (Unicode License v3, see LICENSE-unicode.txt)

Usage: python3 -I build_dict.py <hanja.txt> <Unihan_Readings.txt> [out.tsv]

Output: one candidate per line, `reading<TAB>hanja<TAB>comment`, readings in source order and
candidates best first. Kept basic on purpose: every single-syllable entry, the first
WORD_CANDIDATES forms of each word up to MAX_WORD_LENGTH syllables, without comments, and Unihan syllables in KS X 1001 or the education set.
"""

import re
import sys

WORD_CANDIDATES = 2
MAX_WORD_LENGTH = 4
HANGUL = re.compile(r"^[가-힣]+$")

hanja_txt, unihan_txt = sys.argv[1], sys.argv[2]
out = sys.argv[3] if len(sys.argv) > 3 else "hanja.tsv"

entries: dict[str, list[tuple[str, str]]] = {}

with open(hanja_txt, encoding="utf-8") as f:
    for line in f:
        if line.startswith("#") or ":" not in line:
            continue
        parts = line.rstrip("\n").split(":")
        if len(parts) < 2:
            continue
        reading, hanja, comment = parts[0], parts[1], ":".join(parts[2:])
        if not HANGUL.match(reading) or not hanja or HANGUL.search(hanja) or re.search(r"[가-힣ㄱ-ㅎ]", hanja):
            continue
        if len(hanja) != len(reading) or len(reading) > MAX_WORD_LENGTH:
            continue
        if len(reading) > 1:
            comment = ""  # word comments are mostly the reading again
        cands = entries.setdefault(reading, [])
        if len(reading) > 1 and len(cands) >= WORD_CANDIDATES:
            continue
        if all(h != hanja for h, _ in cands):
            cands.append((hanja, comment.replace("\t", " ")))

# kHangul: "U+4E00<TAB>kHangul<TAB>일:0E 일:N" — flags: 0 = KS X 1001, E = education set.
unihan: dict[str, list[tuple[int, str]]] = {}
with open(unihan_txt, encoding="utf-8") as f:
    for line in f:
        fields = line.rstrip("\n").split("\t")
        if len(fields) != 3 or fields[1] != "kHangul":
            continue
        hanja = chr(int(fields[0][2:], 16))
        for reading_flags in fields[2].split():
            reading, _, flags = reading_flags.partition(":")
            if "0" in flags or "E" in flags:
                rank = 0 if "E" in flags else 1
                unihan.setdefault(reading, []).append((rank, hanja))

added = 0
for reading, found in unihan.items():
    cands = entries.setdefault(reading, [])
    for _, hanja in sorted(found):
        if all(h != hanja for h, _ in cands):
            cands.append((hanja, ""))
            added += 1

with open(out, "w", encoding="utf-8") as f:
    for reading, cands in entries.items():
        for hanja, comment in cands:
            f.write(f"{reading}\t{hanja}\t{comment}\n")

print(f"{out}: {len(entries)} readings, {sum(map(len, entries.values()))} candidates ({added} from Unihan)")
