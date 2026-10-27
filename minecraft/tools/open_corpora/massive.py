"""Extracts Amazon MASSIVE's Korean utterances into corpora/derived/massive.

Source: corpora/open/massive/{train,validation,test}/ko.json.gz
(https://huggingface.co/datasets/mteb/amazon_massive_intent, the ko split of Amazon MASSIVE;
original https://github.com/alexa/massive).
License: CC BY 4.0 (the LICENSE beside it, from AmazonScience/massive; the mteb mirror's card says
apache-2.0).

About 16,500 Korean virtual-assistant utterances (알람 맞춰줘, 불 꺼, 노래 틀어줘): short colloquial
commands and questions, useful for imperative and casual endings. Each gzip file holds one JSON
object per line; only its "text" field is used (intent label and id are dropped). Whitespace is
collapsed and the lines are deduplicated in source order (train, validation, test). Then lines are
dropped if they have:
  - any Latin letter (dotted spelled-out acronyms like "k. b. s. 뉴스", brand names in lowercase),
  - a particle written as its own word (부산 에, 재생 목록 에서, 서울 은), an annotation artifact
    that would teach the predictor wrong spacing,
  - spelled-out Sino-Korean numbers (이십 퍼센트, 볼륨 오십으로, 십초, 천구백구십 년, 이 점 오), which people
    type as digits; native counts before 시 (아홉 시) are normal writing and are kept,
  - a bare jamo (어ㄸ, 들려저ㅜ, ㅋㅋㅋ) or fewer than 2 Hangul syllables.
That leaves about 12,200 utterances. Writes:
  ko.txt     the cleaned utterances, one per line
  meta.json  source, tier, weight, uses, license, line counts

Usage: python3 -I massive.py
"""

import gzip
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "massive")
OUT = os.path.join(ROOT, "corpora", "derived", "massive")
SPLITS = ("train", "validation", "test")

LATIN = re.compile("[A-Za-z]")
JAMO = re.compile("[\u1100-\u11ff\u3131-\u318e]")
HANGUL = re.compile("[가-힣]")
DETACHED = re.compile(r"[가-힣] (에|에서|의|을|를|은|는|이|가|으로|로|도|만)( |$)")
SINO = "일이삼사오육칠팔구"
SPELLED_NUMBER = re.compile(
    rf"(^| )[{SINO}]?[십백천][{SINO}십백천]*"
    r"( |$|분|초|번|도|퍼센트|개|주|년|월|일|원|위|시간|점|으로|만큼|까지|에|을|를|이|의)"
    rf"|(^| )[{SINO}] (분|번|도|초|퍼센트|달러|원|일|월|년)( |$)"
    rf"|(^| )[가-힣]+ 점 [{SINO}영공]"
)


def keep(line):
    return not (
        LATIN.search(line)
        or JAMO.search(line)
        or len(HANGUL.findall(line)) < 2
        or DETACHED.search(line)
        or SPELLED_NUMBER.search(line)
    )


def main():
    seen = set()
    lines = []
    total = 0
    for split in SPLITS:
        with gzip.open(os.path.join(SRC, split, "ko.json.gz"), "rt", encoding="utf-8") as f:
            for raw in f:
                raw = raw.strip()
                if not raw:
                    continue
                try:
                    text = json.loads(raw).get("text")
                except ValueError:
                    continue
                if not isinstance(text, str):
                    continue
                total += 1
                text = " ".join(text.split())
                if not text or text in seen:
                    continue
                seen.add(text)
                if keep(text):
                    lines.append(text)

    os.makedirs(OUT, exist_ok=True)
    with open(os.path.join(OUT, "ko.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.writelines(line + "\n" for line in lines)
    meta = {
        "source": "massive",
        "tier": "moderate",
        "weight": 0.4,
        "uses": ["korean-key-model", "korean-lexicon", "particles-endings"],
        "license": "CC BY 4.0 (Amazon MASSIVE; mteb mirror card says apache-2.0)",
        "files": {"ko.txt": len(lines)},
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(f"massive: {total} utterances, {len(seen)} distinct, {len(lines)} kept")


if __name__ == "__main__":
    main()
