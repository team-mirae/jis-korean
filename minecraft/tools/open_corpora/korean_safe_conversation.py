"""Extracts korean_safe_conversation (corpora/open/korean-safe-conversation) into
corpora/derived/korean-safe-conversation for the predictor.

Source: https://huggingface.co/datasets/jojo0217/korean_safe_conversation, about 27k Korean
chatbot instruction/answer pairs (KoAlpaca, Evol-Instruct, everyday-conversation, empathy and
hate-speech-rebuttal sets, answers written by GPT). License: Apache-2.0.

What it extracts, from train.jsonl only (raw/ holds the same rows split by origin; it is read
only to identify the hate-speech rows):
- Rows whose instruction appears in raw/hatespeech.jsonl are dropped whole (the instruction is
  the hate speech, and the answer echoes it).
- ko.txt: instructions first (the most user-like text: short questions and remarks), then the
  answers (GPT prose). The `input` field is empty in every row but is used if present. Fenced
  and inline code, URLs, markdown headings/emphasis and leading list markers (1. - * •) are
  removed, and lines with Evol-Instruct template slots (#새로운 질문#) are dropped; text is split on newlines and after sentence punctuation (. ? ! followed by a space, so 3.5 stays whole).
  Answer sentences carrying AI-assistant boilerplate (인공 지능, AI 모델, 언어 모델, 저는 ...
  경험/감정을 가질 수 없) are dropped. Lines with under 2 Hangul syllables or with less Hangul
  than Latin letters (English/code) are dropped, and the rest are deduplicated exactly.
- mixed.txt: distinct Latin+Hangul tokens (AI는, TV에서, GDP는, Python으로), acronym+particle
  examples.

The recipe weighs instructions about 1.0 and answers about 0.4; the derived format has one
ko.txt, so instruction lines come first and the per-part line counts are printed.

Usage: python3 -I korean_safe_conversation.py
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "korean-safe-conversation")
OUT = os.path.join(ROOT, "corpora", "derived", "korean-safe-conversation")

FENCE = re.compile(r"```.*?(?:```|\Z)", re.S)
INLINE_CODE = re.compile(r"`[^`\n]*`")
URL = re.compile(r"(?:https?://|www\.)\S+|\b[\w.-]+\.(?:com|net|org|kr|io)(?:/\S*)?", re.I)
LIST_MARK = re.compile(r"^\s*(?:#{1,6}\s+|\d{1,3}[.)]\s+|[-*•·]\s+|[①-⑳]\s*)+")
EMPHASIS = re.compile(r"\*\*|__")
TEMPLATE = re.compile(r"#[^#\n]{1,30}#")
SPLIT = re.compile(r"(?<=[.?!])\s+|\n+")
BOILER = re.compile(r"인공 ?지능|AI 모델|언어 ?모델|저는 .*(?:경험|감정)을 가질 수 없|AI 어시스턴트|챗봇")
HANGUL = re.compile(r"[가-힣]")
LATIN = re.compile(r"[A-Za-z]")
MIXED = re.compile(r"[A-Za-z][A-Za-z0-9]*[가-힣]+|[가-힣]+[A-Za-z]{2,}[A-Za-z0-9]*")


def sentences(text, answer):
    text = FENCE.sub("\n", text)
    text = INLINE_CODE.sub(" ", text)
    text = URL.sub(" ", text)
    for line in text.split("\n"):
        line = LIST_MARK.sub("", line)
        line = EMPHASIS.sub("", line)
        for part in SPLIT.split(line):
            part = re.sub(r"[ \t\r ​]+", " ", part).strip(" \"'")
            hangul = len(HANGUL.findall(part))
            if hangul < 2 or hangul < len(LATIN.findall(part)):
                continue
            if TEMPLATE.search(part) or (answer and BOILER.search(part)):
                continue
            yield part


def main():
    os.makedirs(OUT, exist_ok=True)
    hate = set()
    with open(os.path.join(SRC, "raw", "hatespeech.jsonl"), encoding="utf-8") as f:
        for line in f:
            if line.strip():
                hate.add(json.loads(line).get("instruction", "").strip())
    asks, answers, dropped = [], [], 0
    with open(os.path.join(SRC, "train.jsonl"), encoding="utf-8") as f:
        for line in f:
            if not line.strip():
                continue
            row = json.loads(line)
            instruction = str(row.get("instruction") or "")
            if instruction.strip() in hate:
                dropped += 1
                continue
            asks.append(instruction + "\n" + str(row.get("input") or ""))
            answers.append(str(row.get("output") or ""))
    seen, lines, mixed, parts = set(), [], set(), {}
    for name, texts, answer in (("instructions", asks, False), ("answers", answers, True)):
        before = len(lines)
        for text in texts:
            for s in sentences(text, answer):
                if s in seen:
                    continue
                seen.add(s)
                lines.append(s)
                mixed.update(MIXED.findall(s))
        parts[name] = len(lines) - before
    files = {}
    with open(os.path.join(OUT, "ko.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.writelines(line + "\n" for line in lines)
    files["ko.txt"] = len(lines)
    mixed = sorted(mixed)
    with open(os.path.join(OUT, "mixed.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.writelines(token + "\n" for token in mixed)
    files["mixed.txt"] = len(mixed)
    meta = {
        "source": "korean-safe-conversation",
        "tier": "moderate",
        "weight": 0.6,
        "uses": ["korean-key-model", "korean-lexicon", "particles-endings", "mixed-ko-en"],
        "license": "Apache-2.0",
        "files": files,
    }
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
        f.write("\n")
    print("dropped hate-speech rows:", dropped, "lines per part:", parts)
    print(files)


if __name__ == "__main__":
    main()
