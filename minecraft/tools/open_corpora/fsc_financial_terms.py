"""Extracts the FSC financial glossary into corpora/derived/fsc-financial-terms. License: 이용허락범위
제한 없음 (data.go.kr dataset 15160317, 금융위원회_금융용어사전; corpora/open/fsc-financial-terms/
LICENSE.txt and data.go.kr-15160317-catalog.json). The terms were fetched from the page the dataset
names, https://www.fsc.go.kr/in090301.

The glossary is ~229 entries (a few repeated) of financial-policy terms with a one-paragraph Korean
definition: 확정기여형(DC) 퇴직연금, 레버리지 ETF, TRS(Total Return Swap), IFRS재단. Only
fsc-terms.tsv (dicId, term, definition) is read; the raw html/ pages are skipped. Each definition is
cut at the page navigation ("목록으로 이전글 ... 다음글 ... <div"), a definition the page printed twice
is collapsed to one copy, stray spaces around punctuation (" , ", "( ") are closed up, and the text
is split into sentences and at its bullets (*, -, ①, /). Each term is split into its parts: the
Hangul head and its words (Korean lexicon), all-caps acronyms (DC, ABS, K-OTC, M&A, REITs), English
expansions in parentheses (Total Return Swap) and mixed Latin+Hangul tokens (DIP금융, IFRS재단).
"한글(ACR, English Name)" glosses inside the definitions (국제회계기준위원회(IASB, ...)) add more
term pairs. Two source typos are fixed: "Ventire" → Venture, "back-wardation" → backwardation.

Writes ko.txt (definition sentences), terms.tsv (korean<TAB>english: the English expansion and/or
acronym, empty if none), ko_words.tsv and en_words.tsv (term words and acronyms; the source has no
frequencies, so every count is 1), mixed.txt (distinct Latin+Hangul tokens) and meta.json. It's
tiny: useful as a business-term lexicon and acronym list, and the definitions as an eval set, not
as bulk training text.

Usage: python3 -I fsc_financial_terms.py
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..", "..")
SRC = os.path.join(ROOT, "corpora", "open", "fsc-financial-terms", "fsc-terms.tsv")
OUT = os.path.join(ROOT, "corpora", "derived", "fsc-financial-terms")

LICENSE = ("이용허락범위 제한 없음 (data.go.kr 15160317, 금융위원회_금융용어사전; "
           "fetched from https://www.fsc.go.kr/in090301)")
USES = ["business-terms", "acronyms", "korean-lexicon", "english-lexicon", "mixed-ko-en", "eval-only"]

HANGUL = re.compile(r"[가-힣]")
LATIN = re.compile(r"[A-Za-z]")
URL_PAREN = re.compile(r"\s*\((?:https?://|www[.-])[^)]*\)")
NAV = re.compile(r"\s*목록으로\s+이전글.*$|\s*<div.*$", re.S)
FIXES = [("Ventire", "Venture"), ("back-wardation", "backwardation"), ("Repot)", "Report)")]
SENT_END = re.compile(r"(?<=[.?!])\s+(?=\S)")
BULLET = re.compile(r"\s+(?:\*|-|/)\s+|\s+(?=[①-⑳])")
PAREN = re.compile(r"\(([^()]*)\)")
DEF_GLOSS = re.compile(r"([가-힣]+)\s?\(([A-Z][^()가-힣]*)\)")
GLOSS_STOP = {"금융회사", "금융기관", "매매거래일"}
TOKEN_SPLIT = re.compile(r"[\s,()「」“”\"'·ㆍ/*\[\]]+")
EDGE = re.compile(r"^[^\w&+]+|[^\w&+]+$")


def fix(text):
    for bad, good in FIXES:
        text = text.replace(bad, good)
    return text


def tidy(text):
    """Collapses whitespace and closes up the spaces the source left around punctuation."""
    text = " ".join(text.split())
    text = re.sub(r"\s+([,.)\]])", r"\1", text)
    text = re.sub(r"([(\[])\s+", r"\1", text)
    text = re.sub(r",(?=[^\s\d])", ", ", text)
    return text


def balanced(text):
    """Drops unmatched parentheses (left by the page's truncated definitions)."""
    out, depth = [], 0
    for ch in text:
        if ch == "(":
            depth += 1
        elif ch == ")":
            if depth == 0:
                continue
            depth -= 1
        out.append(ch)
    s = "".join(out)
    while depth > 0:  # unmatched "(": drop the last one
        i = s.rfind("(")
        s = s[:i] + s[i + 1:]
        depth -= 1
    return " ".join(s.split())


def clean_definition(text):
    text = NAV.sub("", fix(text))
    text = URL_PAREN.sub("", text)
    text = " ".join(text.split())
    # Footnote marks: "요건*을" → 요건을, "의견수렴 * 을 거쳐" → 의견수렴을 거쳐.
    text = re.sub(r"\s\*\s(?=[을를이가은는의에와과도로]\s)", "", text)
    text = re.sub(r"\*(?=[가-힣])", "", text)
    half = len(text) // 2
    # Some entries print the definition twice in a row.
    for cut in range(half - 2, half + 3):
        a, b = text[:cut].strip(), text[cut:].strip()
        if a and a == b:
            text = a
            break
    return tidy(text)


def segments(text):
    for sent in SENT_END.split(text):
        for seg in BULLET.split(sent):
            seg = balanced(tidy(seg)).strip(" -*/")
            if len(seg) >= 5 and HANGUL.search(seg) and not re.search(r"https?://|www[.-]", seg):
                yield seg


def is_acronym(tok):
    letters = [c for c in tok if c.isalpha()]
    if not letters or HANGUL.search(tok) or " " in tok or not tok[0].isupper() or len(tok) > 12:
        return False
    upper = sum(c.isupper() for c in letters)
    return upper >= 2 and upper * 2 >= len(letters)


def latin_words(phrase):
    for tok in TOKEN_SPLIT.split(phrase):
        tok = EDGE.sub("", tok)
        if tok and LATIN.search(tok) and not HANGUL.search(tok) and len(tok) > 1:
            yield tok


def split_term(term):
    """→ (korean head, acronyms, english phrases, korean glosses inside the parentheses)."""
    acronyms, english, glosses = [], [], []
    for inner in PAREN.findall(term):
        for part in re.split(r"[,:]\s*", inner):
            part = part.strip()
            if not part or "「" in part:
                continue
            # (구 정리금융공사): former name; (소위 5% rule): so-called
            part = re.sub(r"^(?:구|소위)\s+", "", part)
            if HANGUL.search(part):
                glosses.append(part)
            elif is_acronym(part):
                acronyms.append(part)
            elif LATIN.search(part):
                english.append(part)
    head = " ".join(PAREN.sub("", term).split())
    if not HANGUL.search(head):
        # TRS, Self-Indexing, Sale & Leaseback: no Korean head.
        if is_acronym(head):
            acronyms.insert(0, head)
        elif head:
            for tok in head.split():
                if is_acronym(tok):
                    acronyms.append(tok)
            if not all(is_acronym(t) for t in head.split()):
                english.insert(0, head)
        head = ""
    else:
        for tok in head.split():
            if is_acronym(tok):
                acronyms.append(tok)
    return head, acronyms, english, glosses


def mixed_tokens(text):
    for tok in TOKEN_SPLIT.split(text):
        tok = EDGE.sub("", tok)
        if HANGUL.search(tok) and LATIN.search(tok):
            yield tok


def korean_words(text):
    for tok in TOKEN_SPLIT.split(text):
        tok = EDGE.sub("", tok)
        if tok and re.fullmatch(r"[가-힣]+", tok) and len(tok) >= 2:
            yield tok


def main():
    with open(SRC, encoding="utf-8") as f:
        rows = [line.rstrip("\n").split("\t") for line in f][1:]
    rows = [r for r in rows if len(r) == 3]

    ko_lines, terms, ko_words, en_words, mixed = [], [], [], [], []

    def add(lst, item):
        if item and item not in seen[id(lst)]:
            seen[id(lst)].add(item)
            lst.append(item)

    seen = {id(x): set() for x in (ko_lines, terms, ko_words, en_words, mixed)}

    for _, term, definition in rows:
        term = tidy(fix(term))
        head, acronyms, english, glosses = split_term(term)
        en = ", ".join([a for a in acronyms if a not in head.split()] + english)
        korean = head or (glosses[0] if glosses else "")
        if korean:
            add(terms, (korean, en))
        for g in glosses:
            if g != korean:
                add(terms, (g, en))
        for text in [head] + glosses:
            for w in korean_words(text):
                add(ko_words, w)
        for a in acronyms:
            add(en_words, a)
        for phrase in english:
            for w in latin_words(phrase):
                add(en_words, w)
        for tok in mixed_tokens(term):
            add(mixed, tok)

        text = clean_definition(definition)
        for seg in segments(text):
            add(ko_lines, seg)
            for tok in mixed_tokens(seg):
                add(mixed, tok)
        # 국제회계기준위원회(IASB, International Accounting Standards Board)
        for kor, inner in DEF_GLOSS.findall(text):
            if text[:text.find(kor + "(") if kor + "(" in text else 0][-1:] in ("·", "ㆍ"):
                continue  # 환경·사회·지배구조(...): the gloss is for the whole list
            parts = [p.strip() for p in re.split(r",\s*", inner) if p.strip()]
            if not (len(kor) >= 3 and kor[-1] not in "로를을에의는가" and kor not in GLOSS_STOP
                    and (is_acronym(parts[0]) or len(parts[0].split()) >= 3)):
                continue  # 이를 기초로(Asset-Backed), 거래일(T+2): not a term pair
            add(terms, (kor, ", ".join(parts)))
            for w in korean_words(kor):
                add(ko_words, w)
            for p in parts:
                if is_acronym(p):
                    add(en_words, p)
                else:
                    for w in latin_words(p):
                        add(en_words, w)

    os.makedirs(OUT, exist_ok=True)
    files = {}

    def write(name, lines):
        if not lines:
            return
        with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as f:
            for line in lines:
                f.write(line + "\n")
        files[name] = len(lines)

    write("ko.txt", ko_lines)
    write("terms.tsv", [f"{k}\t{e}" for k, e in terms])
    write("ko_words.tsv", [f"{w}\t1" for w in ko_words])
    write("en_words.tsv", [f"{w}\t1" for w in en_words])
    write("mixed.txt", mixed)
    meta = {"source": "fsc-financial-terms", "tier": "moderate", "weight": 0.7, "uses": USES,
            "license": LICENSE, "files": files}
    with open(os.path.join(OUT, "meta.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, ensure_ascii=False, indent=2)
        f.write("\n")
    for name, n in files.items():
        print(f"{name}: {n}")


if __name__ == "__main__":
    main()
