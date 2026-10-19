"""Builds ranker.bin, the Minecraft mod's Korean-vs-English model, and reports its accuracy.

Both languages are scored on the same thing: the keys the user pressed. The English model learns
how English words are spelled; the Korean model learns the keys a JIS Korean typist presses for
Korean words (2-set, with an explicit ㅇ, or with the implied ㅇ and the double press). A word is
Korean when the Korean model explains its keys better.

Sources (all redistributable):
- English: dwyl/english-words words_alpha.txt (Unlicense), plus common-en.txt.
- Korean: readings in engine/dict/hanja.tsv (libhangul, BSD-3), plus common-ko.txt.

The common-*.txt lists are in src/main/resources/assets/jiskorean/; the mod also uses them.

Usage: python3 -I build_ranker.py <words_alpha.txt> [--eval-korean <text file>]
Writes src/main/resources/assets/jiskorean/ranker.bin next to this tool's parent.
"""

import math
import os
import random
import re
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
HANJA_TSV = os.path.join(ROOT, "..", "engine", "dict", "hanja.tsv")
OUT = os.path.join(ROOT, "src", "main", "resources", "assets", "jiskorean", "ranker.bin")

# --- Hand-written lists (shared with the mod): frequent words the big lists miss or under-weight ---

LISTS = os.path.join(ROOT, "src", "main", "resources", "assets", "jiskorean")


def read_list(name):
    with open(os.path.join(LISTS, name), encoding="utf-8") as f:
        return [w.strip() for w in f if w.strip() and not w.startswith("#")]


COMMON_EN = read_list("common-en.txt")
COMMON_KO = read_list("common-ko.txt")

PARTICLES = "은 는 이 가 을 를 에 에서 의 도 만 으로 로 와 과 이다 입니다 하다 했다 해요 하는 하고 적 들".split()

# --- Korean → 2-set keys ---

INITIAL_KEYS = dict(zip("ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ",
                        "r R s e E f a q Q t T d w W c z x v g".split()))
MEDIAL_KEYS = dict(zip("ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ",
                       "k o i O j p u P h hk ho hl y n nj np nl b m ml l".split()))
FINAL_KEYS = dict(zip("ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ",
                      "r R rt s sw sg e f fr fa fq ft fx fv fg a q qt t T d w c z x v g".split()))
JAMO_KEYS = {**INITIAL_KEYS, **MEDIAL_KEYS, **FINAL_KEYS}
INITIALS = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ"
MEDIALS = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ"
FINALS = " ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ"
# (previous medial, first key of the next vowel) pairs that would combine into a compound.
COMBINES = {("ㅗ", "k"), ("ㅗ", "o"), ("ㅗ", "l"), ("ㅜ", "j"), ("ㅜ", "p"), ("ㅜ", "l"), ("ㅡ", "l")}


def syllables(word):
    for ch in word:
        n = ord(ch) - 0xAC00
        if 0 <= n < 11172:
            yield INITIALS[n // 588], MEDIALS[n % 588 // 28], FINALS[n % 28].strip()
        elif ch in JAMO_KEYS:
            yield None, None, ch  # a bare jamo (ㅋㅋ)
        else:
            return


def korean_keys(word, jis):
    """The keys for a Korean word: explicit ㅇ (jis=False), or how JIS Korean lets you type it."""
    keys = []
    prev = None
    for initial, medial, final in syllables(word):
        if initial is None:
            keys.append(JAMO_KEYS[final])
            prev = None
            continue
        vowel = MEDIAL_KEYS[medial]
        if initial == "ㅇ" and jis:
            if prev and prev[2]:
                keys.append(vowel[0])  # double press: 말+ㅣ+ㅣ → 말이
            elif prev and (prev[1], vowel[0]) in COMBINES:
                keys.append("d")
            # else: implied ㅇ, no key
        else:
            keys.append(INITIAL_KEYS[initial])
        keys.append(vowel)
        if final:
            keys.append(FINAL_KEYS[final])
        prev = (initial, medial, final)
    return "".join(keys).lower()


# --- Model: key trigrams over a-z, with start (^) and end ($) ---

A = 26
START = 26
END = 26


def code(c):
    return ord(c) - 97


def train(weighted_words):
    c3 = [0.0] * (27 * 27 * 27)
    c2 = [0.0] * (27 * 27)
    c1 = [0.0] * 27
    ctx2 = [0.0] * (27 * 27)
    ctx1 = [0.0] * 27
    for word, w in weighted_words:
        a = b = START
        for ch in word + "$":
            c = END if ch == "$" else code(ch)
            c3[(a * 27 + b) * 27 + c] += w
            ctx2[a * 27 + b] += w
            c2[b * 27 + c] += w
            ctx1[b] += w
            c1[c] += w
            a, b = b, c
    total = sum(c1)
    table = bytearray(27 * 27 * 27)
    for a in range(27):
        for b in range(27):
            for c in range(27):
                p1 = (c1[c] + 1) / (total + 27)
                p2 = c2[b * 27 + c] / ctx1[b] if ctx1[b] else p1
                p3 = c3[(a * 27 + b) * 27 + c] / ctx2[a * 27 + b] if ctx2[a * 27 + b] else p2
                p = 0.6 * p3 + 0.3 * p2 + 0.1 * p1
                table[(a * 27 + b) * 27 + c] = min(255, round(-math.log2(p) * 16))
    return table


def score(table, keys):
    """Average log2 probability per key (higher = more likely)."""
    a = b = START
    total = 0.0
    for ch in keys + "$":
        c = END if ch == "$" else code(ch)
        total -= table[(a * 27 + b) * 27 + c] / 16
        a, b = b, c
    return total / (len(keys) + 1)


def is_keys(s):
    return bool(s) and bool(re.fullmatch(r"[a-z]+", s))


def main():
    words_alpha = sys.argv[1]
    eval_korean = sys.argv[sys.argv.index("--eval-korean") + 1] if "--eval-korean" in sys.argv else None
    rng = random.Random(20261019)

    english = [w.strip() for w in open(words_alpha, encoding="utf-8") if w.strip()]
    readings = sorted({line.split("\t", 1)[0] for line in open(HANJA_TSV, encoding="utf-8")})
    rng.shuffle(english)
    rng.shuffle(readings)
    en_test, en_train = english[: len(english) // 10], english[len(english) // 10:]
    ko_test, ko_train = readings[: len(readings) // 10], readings[len(readings) // 10:]

    common_en = COMMON_EN
    common_ko = COMMON_KO

    en_data = [(w, 1.0) for w in en_train] + [(w, 200.0) for w in common_en]
    ko_words = [(w, 1.0) for w in ko_train] + [(w, 200.0) for w in common_ko]
    # Words with particles, so the model has seen eojeol endings.
    ko_words += [(w + rng.choice(PARTICLES), 1.0) for w in ko_train if rng.random() < 0.5]
    ko_data = []
    for w, weight in ko_words:
        for jis in (False, True):
            k = korean_keys(w, jis)
            if is_keys(k):
                ko_data.append((k, weight / 2))

    en_table = train(en_data)
    ko_table = train(ko_data)

    def accuracy(name, keys_list, want_korean):
        keys_list = [k for k in keys_list if is_keys(k)]
        right = sum((score(ko_table, k) > score(en_table, k)) == want_korean for k in keys_list)
        print(f"  {name:<34} {right / len(keys_list):6.1%} of {len(keys_list)}")

    print("accuracy (held out):")
    accuracy("English words (dwyl)", en_test, False)
    accuracy("English common words (trained)", common_en, False)
    accuracy("Korean words, explicit ㅇ", [korean_keys(w, False) for w in ko_test], True)
    accuracy("Korean words, JIS Korean typing", [korean_keys(w, True) for w in ko_test], True)
    if eval_korean:
        eojeols = [t for t in re.findall(r"[가-힣]+", open(eval_korean, encoding="utf-8").read())]
        accuracy("Korean text eojeols, explicit ㅇ", [korean_keys(w, False) for w in eojeols], True)
        accuracy("Korean text eojeols, JIS typing", [korean_keys(w, True) for w in eojeols], True)

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "wb") as f:
        f.write(b"JKRK")
        f.write(struct.pack(">HH", 1, 27))
        f.write(en_table)
        f.write(ko_table)
    print(f"{OUT}: {os.path.getsize(OUT)} bytes")

    for w in ["hello", "minecraft", "diamond", "gg", "ok", "lol"]:
        print(f"  {w:<12} en={score(en_table, w):6.2f} ko={score(ko_table, w):6.2f}")
    for w in ["안녕하세요", "말이", "다이아", "ㅋㅋㅋ", "사람이", "학교에서"]:
        k = korean_keys(w, True)
        print(f"  {w:<8} {k:<14} en={score(en_table, k):6.2f} ko={score(ko_table, k):6.2f}")


main()
