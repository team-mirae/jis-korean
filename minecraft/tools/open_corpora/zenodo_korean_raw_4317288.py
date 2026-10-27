"""Extracts noun and noun+particle word counts, particle priors and mixed Latin+Hangul forms from
Jungyeul Park's morpheme-tagged Korean raw text collection (corpora/open/zenodo-korean-raw-4317288)
into corpora/derived/zenodo-korean-raw-4317288. License: CC BY 4.0 as stated on the Zenodo record
(corpora/open/zenodo-korean-raw-4317288/LICENSE-CC-BY-4.0.txt); the underlying Sejong corpus and
Hankyoreh articles have their own copyright holders (see corpora/open/MANIFEST.md).

Source: "A Korean raw text collection for creating a language model"
(https://doi.org/10.5281/zenodo.4317288). Inputs, streamed line by line:

- sjraw.tok: Sejong corpus text, 3.0M sentences.
- hani-2016.tok: Hankyoreh newspaper 2016, 1.2M sentences.

Each line is one sentence of space-separated `form/TAG` morphemes (Sejong tag set, PRON for
pronouns); a morpheme starting with `+` attaches to the previous one, so the run of a plain
morpheme and its `+` followers is one eojeol (a space-delimited word). The tags give no surface
text for verbs (하+았 is 했, 보+아 is 봐 or 보아), so full sentences are NOT rebuilt (no ko.txt):
only eojeols whose surface form is recoverable exactly are used.

- Punctuation morphemes (S* tags other than SL/SH/SN) split an eojeol into segments
  (`"/SS +당내/NNG` gives the segment 당내).
- A noun segment is one or more noun-like morphemes (NNG, NNP, NNB, NR, NP, PRON, XPN, XSN)
  followed by zero or more particles (J*), nothing else. Its surface is the morphemes joined,
  with a jamo-initial particle (`+ㄴ/JX`, `+ㄹ/JKO`) merged into the final of the previous syllable
  (에+ㄴ is 엔, 나+ㄹ is 날); a segment where that is impossible is dropped.

Outputs:

- ko_words.tsv: word<TAB>count, all-Hangul only, count >= 3 (which drops tagger glue such as
  정계개편등). Two kinds of rows: NNG/NNP lemmas of 2+ syllables counted wherever they occur, and
  noun segment surfaces (신뢰를, 정치의, 당내에, 대표위원은). A form that is both keeps the larger
  count.
- particles.txt: particle sequences seen after a noun (을, 에서는, 으로부터), all-syllable forms
  only, count >= 50, most frequent first.
- mixed.txt: distinct segments mixing Latin letters and Hangul (MBC는, YG엔터테인먼트, KBS가),
  built from SL (+SN) morphemes followed by noun-like morphemes and particles, count >= 2, most
  frequent first. The Sejong text often title-cases acronyms (Tv, Kbs, Ngo); each Latin form is
  restored to its all-caps spelling when that makes up at least a tenth of its case variants in
  the corpus. Latin pieces without a capital letter (m의, acid의) are dropped, as are eojeols
  containing @ or a domain piece (www, com, co, kr, net, org, hani, http): emails and URLs.
- meta.json.

Usage: python3 -I zenodo_korean_raw_4317288.py
"""

import json
import os
import re
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "zenodo-korean-raw-4317288")
OUT = os.path.join(ROOT, "corpora", "derived", "zenodo-korean-raw-4317288")
NAME = "zenodo-korean-raw-4317288"
FILES = ["sjraw.tok", "hani-2016.tok"]
CAP = 3_000_000

NOUNISH = {"NNG", "NNP", "NNB", "NR", "NP", "PRON", "XPN", "XSN"}
LATIN = {"SL", "SN"}
DOMAIN = {"www", "com", "co", "kr", "net", "org", "hani", "http", "https"}
HANGUL = re.compile(r"^[가-힣]+$")
HAS_HANGUL = re.compile(r"[가-힣]")
HAS_LATIN = re.compile(r"[A-Za-z]")
MIXED_OK = re.compile(r"^[A-Za-z0-9가-힣]+$")
# Compatibility jamo (U+3131..) to jongseong index (0 = none).
FINALS = "ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ"


def join(parts):
    """Joins morpheme forms into a surface string; None if a jamo-initial form can't merge."""
    out = ""
    for p in parts:
        if not p:
            return None
        c = p[0]
        if "ㄱ" <= c <= "ㆎ":
            if not out or c not in FINALS:
                return None
            last = out[-1]
            code = ord(last) - 0xAC00
            if not (0 <= code < 11172) or code % 28:
                return None
            out = out[:-1] + chr(ord(last) + FINALS.index(c) + 1) + p[1:]
            if any("ㄱ" <= ch <= "ㆎ" for ch in p[1:]):
                return None
        else:
            out += p
    return out


def segments(eojeol):
    """Splits an eojeol's (form, tag) list at punctuation morphemes."""
    seg = []
    for form, tag in eojeol:
        if tag.startswith("S") and tag not in ("SL", "SH", "SN"):
            if seg:
                yield seg
            seg = []
        else:
            seg.append((form, tag))
    if seg:
        yield seg


def split_noun(seg, start_ok):
    """Returns (stem morphemes, particle morphemes) if seg is stem+ J*, else None."""
    i = 0
    while i < len(seg) and start_ok(seg[i][1]):
        i += 1
    if i == 0:
        return None
    j = i
    while j < len(seg) and seg[j][1].startswith("J"):
        j += 1
    if j != len(seg):
        return None
    return seg[:i], seg[i:]


def latin_case(latin):
    """Maps each SL form to its restored case. The Sejong text often title-cases acronyms (Tv,
    Kbs, Ngo for TV, KBS, NGO): when the all-caps spelling makes up at least a tenth of a word's
    case variants, it wins; otherwise the most frequent variant does."""
    groups = {}
    for form, n in latin.items():
        groups.setdefault(form.lower(), []).append((n, form))
    canon = {}
    for variants in groups.values():
        total = sum(n for n, _ in variants)
        upper = [(n, f) for n, f in variants if f.isupper()]
        best = max(upper) if upper and max(upper)[0] * 10 >= total else max(variants)
        for _, f in variants:
            canon[f] = best[1]
    return canon


def main():
    lemmas, words, parts, mixed, latin = Counter(), Counter(), Counter(), Counter(), Counter()
    sentences = 0
    for fname in FILES:
        with open(os.path.join(SRC, fname), encoding="utf-8", errors="replace") as f:
            for line in f:
                sentences += 1
                eojeols = []
                for tok in line.split():
                    attach = tok.startswith("+") and len(tok) > 1
                    if attach:
                        tok = tok[1:]
                    form, sep, tag = tok.rpartition("/")
                    if not sep or not form or not tag:
                        continue
                    if tag == "SL":
                        latin[form] += 1
                    if tag in ("NNG", "NNP") and len(form) >= 2 and HANGUL.match(form):
                        lemmas[form] += 1
                    if attach and eojeols:
                        eojeols[-1].append((form, tag))
                    else:
                        eojeols.append([(form, tag)])
                for e in eojeols:
                    url = any(t == "SL" and fm.lower() in DOMAIN for fm, t in e) or any(
                        "@" in fm for fm, _ in e)
                    for seg in segments(e):
                        tags = [t for _, t in seg]
                        if seg[0][1] == "SL" and not url:
                            sp = split_noun(seg, lambda t: t in LATIN or t in NOUNISH)
                            if sp and any(t in NOUNISH or t.startswith("J") for t in tags):
                                mixed[tuple((fm, t == "SL") for fm, t in seg)] += 1
                            continue
                        sp = split_noun(seg, lambda t: t in NOUNISH)
                        if not sp:
                            continue
                        stem, ps = sp
                        s = join([fm for fm, _ in seg])
                        if s and HANGUL.match(s):
                            words[s] += 1
                        if ps:
                            p = join([fm for fm, _ in ps])
                            if p and HANGUL.match(p):
                                parts[p] += 1
                if sentences % 500_000 == 0:
                    print(f"{sentences} sentences, {len(lemmas)} lemmas, {len(words)} words",
                          flush=True)
                    # Keep memory bounded: drop singletons of the open-ended counters.
                    if len(words) > 6_000_000:
                        words = Counter({k: v for k, v in words.items() if v > 1})

    canon = latin_case(latin)
    mixed_forms = Counter()
    for seg, n in mixed.items():
        forms = [canon.get(fm, fm) if sl else fm for fm, sl in seg]
        if not all(any(c.isupper() for c in f) for (_, sl), f in zip(seg, forms) if sl):
            continue
        m = join(forms)
        if (m and MIXED_OK.match(m) and HAS_HANGUL.search(m) and HAS_LATIN.search(m)
                and len(m) <= 30):
            mixed_forms[m] += n
    merged = Counter()
    for c in (lemmas, words):
        for k, v in c.items():
            if v >= 3 and v > merged[k]:
                merged[k] = v
    os.makedirs(OUT, exist_ok=True)
    files = {}

    def write(name, lines):
        lines = lines[:CAP]
        with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
            f.write("".join(l + "\n" for l in lines))
        files[name] = len(lines)

    write("ko_words.tsv", [f"{w}\t{n}" for w, n in sorted(merged.items(), key=lambda x: (-x[1], x[0]))])
    write("particles.txt", [p for p, n in sorted(parts.items(), key=lambda x: (-x[1], x[0])) if n >= 50])
    write("mixed.txt", [m for m, n in sorted(mixed_forms.items(), key=lambda x: (-x[1], x[0])) if n >= 2])
    meta = {
        "source": NAME,
        "tier": "moderate",
        "weight": 0.5,
        "uses": ["korean-lexicon", "particles-endings", "acronyms", "mixed-ko-en"],
        "license": "CC-BY-4.0",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(meta, ensure_ascii=False, indent=1) + "\n")
    print(f"{sentences} sentences:", files)


main()
