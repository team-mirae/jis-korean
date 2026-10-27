"""Extracts a frequency-ranked Korean lexicon, particle priors and a few clean sentences from the
Kiwi morphological analyzer's data (corpora/open/kiwi) into corpora/derived/kiwi. License:
Apache-2.0 (corpora/open/kiwi/LICENSE).

Source: bab2min/Kiwi (https://github.com/bab2min/Kiwi). Inputs:

- ModelGenerator/morphemes.txt: tab-separated form, POS, corpus frequency, then optional fields
  (allomorph links `==X`, variant links `=X`, `complex` decompositions, conditions such as
  `vowel`, and for dialect entries a weight `.N` plus a region tag `<경상>`). About half the rows
  are dialect entries; every row carrying a `<...>` region tag is dropped. Forms containing
  conjoining jamo (U+1100..U+11FF, e.g. the ending ᆫ) or arae-a (U+318D, U+119E) are not typed
  surface forms and are dropped.
- eval_data/web.txt, written.txt: tab-separated sentence and gold analysis; column 1 is clean
  running text. web_with_typos.txt and web_with_cont_typos.txt carry injected typos and have no
  aligned clean column, so they are not used (typos must not train the predictor).
- default.dict, multi.dict, dialect.dict, typo.dict, combiningRule.txt are analyzer data
  (Wikidata names, rules) and are ignored.

Outputs:

- ko_words.tsv: word<TAB>frequency for the free-standing word classes NNG, NNP, NNB, NP, NR, MAG,
  MAJ, IC, MM (nouns incl. bound nouns such as 것/수, pronouns, numerals, adverbs, conjunctive
  adverbs, interjections, determiners). Verb/adjective stems and endings (VV, VA, E*, X*) are not
  typed on their own and are skipped. Frequencies of the same form under several POS are summed.
- particles.txt: Kiwi's particles (POS J*: JKS/JKO/JKB/JKG/JKC/JKV/JKQ/JX/JC), all-syllable forms
  only, one per line, most frequent first.
- mixed.txt: distinct forms mixing Latin letters and Hangul (LG전자, 갤럭시S8, KB금융).
- ko.txt: the clean sentences from eval_data/web.txt and written.txt, deduplicated.
- meta.json.

Usage: python3 -I kiwi.py
"""

import json
import os
import re
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "kiwi")
OUT = os.path.join(ROOT, "corpora", "derived", "kiwi")

META = {
    "source": "kiwi",
    "tier": "moderate",
    "weight": 0.8,
    "uses": ["korean-lexicon", "particles-endings", "mixed-ko-en", "business-terms", "typos",
             "eval-only"],
    "license": "Apache-2.0",
}

WORD_POS = {"NNG", "NNP", "NNB", "NP", "NR", "MAG", "MAJ", "IC", "MM"}
# A typed word: Hangul syllables, optionally with Latin letters, digits and '&' (한화L&C).
WORD = re.compile("^(?=.*[가-힣])[가-힣A-Za-z0-9&]+$")
SYLLABLES = re.compile("^[가-힣]+$")
MIXED = re.compile("^(?=.*[A-Za-z])(?=.*[가-힣])[가-힣A-Za-z0-9&]+$")
DIALECT = re.compile("^<.*>$")
URL = re.compile(r"https?://|www\.", re.I)


def morphemes():
    """(form, pos, freq) for every non-dialect row of morphemes.txt."""
    with open(os.path.join(SRC, "ModelGenerator", "morphemes.txt"), encoding="utf-8") as f:
        for line in f:
            fields = line.rstrip("\n").split("\t")
            if len(fields) < 3 or any(DIALECT.match(x) for x in fields[3:]):
                continue
            form, pos, freq = fields[0].strip(), fields[1].strip(), fields[2].strip()
            try:
                n = int(float(freq))
            except ValueError:
                continue
            if n > 0 and form:
                yield form, pos, n


def sentences():
    seen = set()
    for name in ("web.txt", "written.txt"):
        with open(os.path.join(SRC, "eval_data", name), encoding="utf-8") as f:
            for line in f:
                text = " ".join(line.split("\t", 1)[0].split())
                if not text or URL.search(text) or not re.search("[가-힣]", text):
                    continue
                if text not in seen:
                    seen.add(text)
                    yield text


def write_lines(name, lines):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    return len(lines)


def main():
    words, particles, mixed = Counter(), Counter(), set()
    for form, pos, n in morphemes():
        if pos in WORD_POS and WORD.match(form):
            words[form] += n
            if MIXED.match(form):
                mixed.add(form)
        elif pos.startswith("J") and SYLLABLES.match(form):
            particles[form] += n

    os.makedirs(OUT, exist_ok=True)
    files = {
        "ko_words.tsv": write_lines("ko_words.tsv", [
            f"{w}\t{n}" for w, n in sorted(words.items(), key=lambda kv: (-kv[1], kv[0]))]),
        "particles.txt": write_lines("particles.txt", [
            p for p, _ in sorted(particles.items(), key=lambda kv: (-kv[1], kv[0]))]),
        "mixed.txt": write_lines("mixed.txt", sorted(mixed)),
        "ko.txt": write_lines("ko.txt", list(sentences())),
    }
    meta = dict(META, files=files)
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    for name, n in files.items():
        print(f"{name}: {n}")


if __name__ == "__main__":
    main()
