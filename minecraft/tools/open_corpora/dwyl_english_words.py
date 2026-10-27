"""Extracts an English lexicon and acronym list from dwyl/english-words into
corpora/derived/dwyl-english-words/ (gitignored) for training the predictor's English side
(docs/spec.md §10.3; minecraft/tools/build_ranker.py).

Source: https://github.com/dwyl/english-words (corpora/open/dwyl-english-words/src), Unlicense
(public domain). ~370k lowercase words (words_alpha.txt) and ~466k mixed-case entries
(words.txt), unranked; about 60% of it is obscure.

What it writes (en_words.tsv, word<TAB>count; the source has no frequencies, so count is 1):
- Words from words_alpha.txt, kept only when a frequency list knows them: hermitdave
  FrequencyWords en_full.txt (count >= MIN_EN_FREQ) or engine/predict/common-en.txt. Words of 3
  letters or fewer need a higher count (MIN_SHORT_FREQ), since short obscure entries (aal, aam)
  are noise that would steal Korean keys. The frequency list only filters: its counts are not
  copied (CC BY-SA content, and another extractor ships it on its own).
- Words whose keys, typed the 2-set way (with JIS Korean's implied ㅇ before a bare vowel), make
  a known Korean word (hermitdave ko_full.txt count >= MIN_KO_FREQ, or common-ko.txt) are
  dropped unless the English word is itself common (count >= KEEP_EN_FREQ or in common-en.txt):
  `ak` (마) and `dl` (이) go, `dub` (여유) goes, `dog` (앻, no Korean word) stays.
- Acronyms: words.txt entries matching ^[A-Z]{2,6}$ that the frequency list knows in lowercase
  (count >= MIN_ACRONYM_FREQ) or common-en.txt has, written in UPPER case (CEO, JPEG). Entries of
  4+ letters that are also ordinary words in words_alpha.txt (TEMPO, CABS) are skipped. The
  ranker's own case variants come from the lowercase entries; these mark the acronym spelling.
The .zip/.json duplicates, the Moby files, the scripts and uk-us-dict.txt are ignored (the
spelling variants in uk-us-dict.txt are already in words_alpha.txt).

Usage: python3 -I dwyl_english_words.py
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
SRC = os.path.join(ROOT, "corpora", "open", "dwyl-english-words", "src")
FREQ = os.path.join(ROOT, "corpora", "open", "hermitdave-frequencywords")
PREDICT = os.path.join(ROOT, "engine", "predict")
OUT = os.path.join(ROOT, "corpora", "derived", "dwyl-english-words")

MIN_EN_FREQ = 5
MIN_SHORT_FREQ = 200
KEEP_EN_FREQ = 1000
MIN_KO_FREQ = 5
MIN_ACRONYM_FREQ = 20

# Standard 2-set (두벌식) jamo per lowercase key.
KEYS = {
    "q": "ㅂ", "w": "ㅈ", "e": "ㄷ", "r": "ㄱ", "t": "ㅅ", "y": "ㅛ", "u": "ㅕ", "i": "ㅑ",
    "o": "ㅐ", "p": "ㅔ", "a": "ㅁ", "s": "ㄴ", "d": "ㅇ", "f": "ㄹ", "g": "ㅎ", "h": "ㅗ",
    "j": "ㅓ", "k": "ㅏ", "l": "ㅣ", "z": "ㅋ", "x": "ㅌ", "c": "ㅊ", "v": "ㅍ", "b": "ㅠ",
    "n": "ㅜ", "m": "ㅡ",
}
INITIALS = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ"
VOWELS = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ"
FINALS = ["", "ㄱ", "ㄲ", "ㄳ", "ㄴ", "ㄵ", "ㄶ", "ㄷ", "ㄹ", "ㄺ", "ㄻ", "ㄼ", "ㄽ", "ㄾ", "ㄿ",
          "ㅀ", "ㅁ", "ㅂ", "ㅄ", "ㅅ", "ㅆ", "ㅇ", "ㅈ", "ㅊ", "ㅋ", "ㅌ", "ㅍ", "ㅎ"]
VOWEL_PAIRS = {("ㅗ", "ㅏ"): "ㅘ", ("ㅗ", "ㅐ"): "ㅙ", ("ㅗ", "ㅣ"): "ㅚ", ("ㅜ", "ㅓ"): "ㅝ",
               ("ㅜ", "ㅔ"): "ㅞ", ("ㅜ", "ㅣ"): "ㅟ", ("ㅡ", "ㅣ"): "ㅢ"}
FINAL_PAIRS = {("ㄱ", "ㅅ"): "ㄳ", ("ㄴ", "ㅈ"): "ㄵ", ("ㄴ", "ㅎ"): "ㄶ", ("ㄹ", "ㄱ"): "ㄺ",
               ("ㄹ", "ㅁ"): "ㄻ", ("ㄹ", "ㅂ"): "ㄼ", ("ㄹ", "ㅅ"): "ㄽ", ("ㄹ", "ㅌ"): "ㄾ",
               ("ㄹ", "ㅍ"): "ㄿ", ("ㄹ", "ㅎ"): "ㅀ", ("ㅂ", "ㅅ"): "ㅄ"}
SPLIT_FINAL = {v: k for k, v in FINAL_PAIRS.items()}


def syllable(cho, jung, jong):
    return chr(0xAC00 + (INITIALS.index(cho) * 21 + VOWELS.index(jung)) * 28 + FINALS.index(jong))


def hangul(word):
    """The 2-set reading of lowercase keys as whole syllables (a bare vowel gets the implied ㅇ),
    or None when a consonant is left without a vowel."""
    out = []
    cho = jung = jong = None  # the syllable being built
    for ch in word:
        j = KEYS.get(ch)
        if j is None:
            return None
        if j in VOWELS:
            if jung is not None and jong is None and (jung, j) in VOWEL_PAIRS:
                jung = VOWEL_PAIRS[(jung, j)]
                continue
            if jong:  # the final moves to the new syllable
                if jong in SPLIT_FINAL:
                    keep, move = SPLIT_FINAL[jong]
                else:
                    keep, move = "", jong
                out.append(syllable(cho, jung, keep))
                cho, jung, jong = move, j, None
            elif cho is not None and jung is None:
                jung = j
            else:
                if cho is not None:
                    out.append(syllable(cho, jung, ""))
                cho, jung, jong = "ㅇ", j, None
        else:
            if cho is None or jung is None:
                if cho is not None:  # consonant after a consonant with no vowel
                    return None
                cho = j
            elif jong is None:
                if j in FINALS:
                    jong = j
                else:  # ㄸ ㅃ ㅉ can't end a syllable (not typeable unshifted anyway)
                    out.append(syllable(cho, jung, ""))
                    cho, jung = j, None
            elif (jong, j) in FINAL_PAIRS:
                jong = FINAL_PAIRS[(jong, j)]
            else:
                out.append(syllable(cho, jung, jong))
                cho, jung, jong = j, None, None
    if cho is not None:
        if jung is None:
            return None
        out.append(syllable(cho, jung, jong or ""))
    return "".join(out)


def read_freq(path):
    counts = {}
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            parts = line.split()
            if len(parts) == 2 and parts[1].isdigit():
                counts[parts[0]] = counts.get(parts[0], 0) + int(parts[1])
    return counts


def read_list(path):
    with open(path, encoding="utf-8") as f:
        return {w.strip() for w in f if w.strip() and not w.startswith("#")}


def main():
    en_freq = read_freq(os.path.join(FREQ, "en_full.txt"))
    ko_freq = read_freq(os.path.join(FREQ, "ko_full.txt"))
    common_en = {w.lower() for w in read_list(os.path.join(PREDICT, "common-en.txt"))}
    common_ko = read_list(os.path.join(PREDICT, "common-ko.txt"))
    ko_known = {w for w, n in ko_freq.items() if n >= MIN_KO_FREQ} | common_ko

    words = set()
    alpha = set()
    stats = {"alpha": 0, "unknown": 0, "short": 0, "korean": 0}
    korean_dropped = []
    with open(os.path.join(SRC, "words_alpha.txt"), encoding="utf-8", errors="replace") as f:
        for line in f:
            w = line.strip()
            if not re.fullmatch(r"[a-z]+", w):
                continue
            stats["alpha"] += 1
            alpha.add(w)
            n = en_freq.get(w, 0)
            common = w in common_en
            if not common and n < MIN_EN_FREQ:
                stats["unknown"] += 1
                continue
            if len(w) <= 3 and not common and n < MIN_SHORT_FREQ:
                stats["short"] += 1
                continue
            if not common and n < KEEP_EN_FREQ:
                ko = hangul(w)
                if ko is not None and ko in ko_known:
                    stats["korean"] += 1
                    korean_dropped.append((w, ko))
                    continue
            words.add(w)

    acronyms = set()
    with open(os.path.join(SRC, "words.txt"), encoding="utf-8", errors="replace") as f:
        for line in f:
            w = line.strip()
            if re.fullmatch(r"[A-Z]{2,6}", w):
                low = w.lower()
                if len(w) >= 4 and low in alpha:  # a word in capitals (TEMPO, CABS)
                    continue
                if low in common_en or en_freq.get(low, 0) >= MIN_ACRONYM_FREQ:
                    acronyms.add(w)

    os.makedirs(OUT, exist_ok=True)
    rows = sorted(words) + sorted(acronyms)
    with open(os.path.join(OUT, "en_words.tsv"), "w", encoding="utf-8", newline="\n") as f:
        for w in rows:
            f.write(w + "\t1\n")
    meta = {
        "source": "dwyl-english-words",
        "tier": "moderate",
        "weight": 0.5,
        "uses": ["english-key-model", "english-lexicon", "acronyms"],
        "license": "Unlicense (public domain)",
        "files": {"en_words.tsv": len(rows)},
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print("words_alpha:", stats["alpha"], "kept:", len(words), "acronyms:", len(acronyms))
    print("dropped: unknown", stats["unknown"], "short", stats["short"],
          "korean", stats["korean"])
    print("korean examples:", " ".join(f"{w}={k}" for w, k in korean_dropped[:40]))


if __name__ == "__main__":
    main()
