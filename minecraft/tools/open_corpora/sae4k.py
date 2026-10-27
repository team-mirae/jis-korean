"""Extracts sae4k (corpora/open/sae4k, github.com/warnikchow/sae4k, CC BY-SA 4.0) into
corpora/derived/sae4k/ for build_ranker.py. Only derived counts and sampled sentences are used.

sae4k is ~50k Korean directive and question utterances (smart-speaker style: weather, mail,
schedules, devices, everyday advice), each paired with an extracted intent phrase in noun-phrase
style (오늘 오후 수영대회 여부, 민감한 내용 전체메일 보내지 않기). Only src/data/sae4k_v2.txt is
read; sae4k_v1.txt is a ~99% subset of it. Rows are label<TAB>utterance<TAB>intent phrase, with
CRLF line ends and a BOM, so the file is read as utf-8-sig and split with splitlines(). The text is
almost all Hangul; a handful of lines carry Latin (usb로, DSLR로) or digits.

- ko.txt: utterances (field 2), whitespace-normalized (NBSPs included), exact duplicates removed,
  lines with stray compatibility jamo (typos like 바랍ㄴ다) dropped. Templates are capped: at most
  CAP_PER_CLUSTER sentences share their first two eojeols (weather and mail phrasings dominate),
  sampled with random.Random(20261027), kept in file order.
- ko_words.tsv: space-separated tokens with counts over all deduplicated utterances (uncapped)
  plus the intent phrases (field 3) that aren't also an utterance, as extra noun/lexicon material.
  Leading/trailing punctuation is stripped from tokens.
- mixed.txt: distinct tokens mixing Latin letters and Hangul (DSLR로, usb로).

There is no usable English, and particles live inside the eojeols rather than as a list.

Usage: python3 -I sae4k.py
"""

import collections
import json
import os
import random
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "sae4k", "src", "data", "sae4k_v2.txt")
OUT = os.path.join(ROOT, "corpora", "derived", "sae4k")

CAP_PER_CLUSTER = 50
JAMO = re.compile(r"[ㄱ-ㆎᄀ-ᇿ]")
HANGUL = re.compile(r"[가-힣]")
LATIN = re.compile(r"[A-Za-z]")
PUNCT = "!?.,~;:…\"'()[]"


def clean(text):
    text = " ".join(text.replace(" ", " ").split())
    if not text or JAMO.search(text) or not HANGUL.search(text):
        return ""
    return text


def tokens(text):
    for tok in text.split():
        tok = tok.strip(PUNCT)
        if HANGUL.search(tok) or LATIN.search(tok):
            yield tok


def write_lines(name, lines):
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    return len(lines)


def main():
    with open(SRC, encoding="utf-8-sig") as f:
        raw_lines = f.read().splitlines()  # 9 MB; handles CR, LF and CRLF

    utterances = []  # deduplicated, file order
    seen = set()
    intents = []
    for raw in raw_lines:
        parts = raw.split("\t")
        if len(parts) < 2:
            continue
        text = clean(parts[1])
        if text and text not in seen:
            seen.add(text)
            utterances.append(text)
        if len(parts) >= 3:
            phrase = clean(parts[2])
            if phrase:
                intents.append(phrase)
    intent_set = []
    seen_intent = set()
    for phrase in intents:
        if phrase not in seen and phrase not in seen_intent:
            seen_intent.add(phrase)
            intent_set.append(phrase)

    # Sentences: templates capped by their first two eojeols.
    clusters = collections.defaultdict(list)
    for text in utterances:
        clusters[" ".join(text.split()[:2])].append(text)
    rng = random.Random(20261027)
    keep = set()
    for key in sorted(clusters):
        group = clusters[key]
        if len(group) > CAP_PER_CLUSTER:
            group = rng.sample(group, CAP_PER_CLUSTER)
        keep.update(group)
    sentences = [text for text in utterances if text in keep]

    words = collections.Counter()
    mixed = set()
    for text in utterances + intent_set:
        for tok in tokens(text):
            words[tok] += 1
            if LATIN.search(tok) and HANGUL.search(tok):
                mixed.add(tok)

    os.makedirs(OUT, exist_ok=True)
    files = {"ko.txt": write_lines("ko.txt", sentences)}
    ordered = sorted(words.items(), key=lambda kv: (-kv[1], kv[0]))
    files["ko_words.tsv"] = write_lines("ko_words.tsv", [f"{w}\t{c}" for w, c in ordered])
    if mixed:
        files["mixed.txt"] = write_lines("mixed.txt", sorted(mixed))
    meta = {
        "source": "sae4k",
        "tier": "moderate",
        "weight": 0.6,
        "uses": ["korean-key-model", "korean-lexicon", "particles-endings"],
        "license": "CC BY-SA 4.0",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(
        f"{len(utterances)} unique utterances, {len(intent_set)} extra intent phrases, "
        f"{len(clusters)} clusters; " + ", ".join(f"{k}: {v}" for k, v in files.items())
    )


main()
