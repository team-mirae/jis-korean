"""Extracts Kakao's FunctionChat-Bench into corpora/derived/functionchat-bench. License: Apache-2.0
(corpora/open/functionchat-bench/LICENSE; https://github.com/kakao/FunctionChat-Bench).

FunctionChat-Bench is a small Korean tool-use chat benchmark: assistant-style dialogues full of app
and tech words (VIP, BMI를, 이메일). From src/data/*.jsonl it collects every string `content` of a
user or assistant message, wherever it sits in the object (system and tool messages, null content
and tool_calls are skipped), plus the Singlecall `query[].content` utterances; the Singlecall
ground_truth entries are JSON calls and are skipped. Tool and parameter descriptions (short Korean
noun phrases such as "사용자의 이메일 주소") are added after the dialogue lines, minus format specs
(YYYY-MM-DD, true/false) and English schema prose. Messages are split
into sentences; sentences with '{', '_', escaped quotes, URLs or e-mail addresses are dropped, and exact
duplicates (the repeated multi-turn history, the system prompt) collapse to one.

Writes ko.txt (one sentence per line), mixed.txt (distinct Latin+Hangul tokens) and meta.json.
It's only ~1K sentences: better as a held-out chat-register eval set than as training text.

Usage: python3 -I functionchat_bench.py
"""

import glob
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "functionchat-bench", "src", "data")
OUT = os.path.join(ROOT, "corpora", "derived", "functionchat-bench")

HANGUL = re.compile(r"[가-힣]")
LATIN = re.compile(r"[A-Za-z]")
BAD = re.compile(r"[{}_]|\\\"|https?://|www\.|[\w.+-]+@[\w-]+\.[\w.]+")
# Descriptions also carry format specs and English schema prose ("YYYY-MM-DD", "true이면").
DESC_BAD = re.compile(r"YYYY|HH:MM|true|false|[A-Za-z]+\(|[A-Za-z]{3,} [A-Za-z]{3,} [A-Za-z]{3,}")
SENT_END = re.compile(r"(?<=[.?!。])\s+|\n+")
TOKEN_EDGE = re.compile(r"^[^\w]+|[^\w]+$")


def walk(obj, lines, descs):
    """Collects user/assistant message contents into lines and tool descriptions into descs."""
    if isinstance(obj, dict):
        role = obj.get("role")
        if role in ("user", "assistant") and isinstance(obj.get("content"), str):
            lines.append(obj["content"])
        if isinstance(obj.get("description"), str):
            descs.append(obj["description"])
        for key, value in obj.items():
            if key == "ground_truth" and isinstance(value, list):
                continue  # Singlecall: JSON function calls
            if key == "query" and isinstance(value, list):
                for q in value:
                    if isinstance(q, dict) and "role" not in q and isinstance(q.get("content"), str):
                        lines.append(q["content"])
            walk(value, lines, descs)
    elif isinstance(obj, list):
        for item in obj:
            walk(item, lines, descs)


def sentences(text):
    for s in SENT_END.split(text):
        s = " ".join(s.split())
        if s and HANGUL.search(s) and not BAD.search(s) and not s.startswith("{"):
            yield s


def main():
    lines, descs = [], []
    for path in sorted(glob.glob(os.path.join(SRC, "*.jsonl"))):
        with open(path, encoding="utf-8") as f:
            for raw in f:
                raw = raw.strip()
                if not raw:
                    continue
                try:
                    walk(json.loads(raw), lines, descs)
                except ValueError:
                    continue
    seen, ko = set(), []
    for text, is_desc in [(t, False) for t in lines] + [(t, True) for t in descs]:
        for s in sentences(text):
            if is_desc and DESC_BAD.search(s):
                continue
            if s not in seen:
                seen.add(s)
                ko.append(s)
    mixed = set()
    for s in ko:
        for tok in s.split():
            tok = TOKEN_EDGE.sub("", tok)
            if HANGUL.search(tok) and LATIN.search(tok):
                mixed.add(tok)
    os.makedirs(OUT, exist_ok=True)
    files = {"ko.txt": ko, "mixed.txt": sorted(mixed)}
    for name, rows in files.items():
        with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
            f.writelines(r + "\n" for r in rows)
    meta = {
        "source": "functionchat-bench",
        "tier": "moderate",
        "weight": 0.5,
        "uses": ["korean-key-model", "mixed-ko-en", "particles-endings", "eval-only"],
        "license": "Apache-2.0",
        "files": {name: len(rows) for name, rows in files.items()},
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(meta["files"])


if __name__ == "__main__":
    main()
