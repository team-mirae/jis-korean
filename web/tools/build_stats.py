"""Precomputes the key-press comparisons shown on the playground (web/src/stats.json).

Fetches each text from Korean Wikisource at a recorded revision, extracts the plain text, and counts
it with the engine's planner (engine/examples/count.rs). Only the counts and sources are stored;
the texts themselves aren't committed.

Usage: python3 -I web/tools/build_stats.py   (from the repo root; needs cargo)
"""

import json
import os
import re
import subprocess
import tempfile
import urllib.parse
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.path.join(ROOT, "web", "src", "stats.json")
API = "https://ko.wikisource.org/w/api.php"


def fetch(title):
    """(text, revision id, canonical title) of a page, following redirects."""
    query = urllib.parse.urlencode({
        "action": "query", "prop": "revisions", "rvprop": "ids|content", "rvslots": "main",
        "redirects": "1", "format": "json", "formatversion": "2", "titles": title,
    })
    req = urllib.request.Request(f"{API}?{query}", headers={"User-Agent": "jis-korean-stats/0.1"})
    with urllib.request.urlopen(req, timeout=60) as r:
        page = json.load(r)["query"]["pages"][0]
    revision = page["revisions"][0]
    return revision["slots"]["main"]["content"], revision["revid"], page["title"]


def plain(wiki):
    """Wiki markup → plain text lines."""
    text = re.sub(r"\{\{[^{}]*\}\}", "", wiki)
    text = re.sub(r"<br\s*/?>", "\n", text)
    text = re.sub(r"<[^>]+>", "", text)
    text = re.sub(r"\[\[(?:[^|\]]*\|)?([^\]]*)\]\]", r"\1", text)
    text = text.replace("'''", "").replace("''", "")
    lines = []
    for line in text.splitlines():
        line = re.sub(r"^[*:;#]+\s*", "", line.strip())
        line = re.sub(r"^=+\s*(.*?)\s*=+$", r"\1", line)
        if line:
            lines.append(re.sub(r"\s+", " ", line))
    return "\n".join(lines)


def constitution(wiki):
    body = wiki[wiki.index("== 전문 =="):wiki.index("== 연혁 ==")]
    return plain(body.replace("== 조문 ==", ""))


def commercial_articles_1_to_10(wiki):
    body = wiki[wiki.rfind("\n", 0, wiki.index("<span id='1'>")):wiki.rfind("\n", 0, wiki.index("<span id='11'>"))]
    return "\n".join(line for line in plain(body).splitlines() if re.match(r"제\d+조", line))


def anthem_sung(wiki):
    """The Hangul column, four verses with the chorus after each, as sung."""
    column = wiki.split("<div", 2)[2]
    verses = re.findall(r";\d+\n((?::.*\n?)+)", column)
    lines = [[plain(line[1:]) for line in verse.strip().splitlines()] for verse in verses]
    chorus = lines[0][1]
    return "\n".join(f"{verse[0]}\n{chorus}" for verse in lines[:4])


TEXTS = [
    ("constitution", "대한민국헌법", "Constitution of the Republic of Korea (full text)", "대한민국헌법 (제10호)", constitution),
    ("commercial-act", "상법 제1조~제10조", "Commercial Act, articles 1–10", "상법 (대한민국, 제14096호)", commercial_articles_1_to_10),
    ("anthem", "애국가 1~4절", "National anthem, all four verses (with the chorus, as sung)", "애국가 (대한민국)", anthem_sung),
]


def count(text):
    with tempfile.NamedTemporaryFile("w", suffix=".txt", encoding="utf-8", delete=False) as f:
        f.write(text)
    try:
        out = subprocess.run(
            ["cargo", "run", "--quiet", "--release", "--example", "count", "--", f.name],
            cwd=os.path.join(ROOT, "engine"), check=True, capture_output=True, text=True,
        ).stdout
    finally:
        os.unlink(f.name)
    return json.loads(out)


def main():
    stats = []
    for key, korean, english, title, extract in TEXTS:
        wiki, revid, canonical = fetch(title)
        counts = count(extract(wiki))
        source = f"https://ko.wikisource.org/w/index.php?title={urllib.parse.quote(canonical)}&oldid={revid}"
        stats.append({"id": key, "title": korean, "subtitle": english, "source": source, **counts})
        print(f"{korean}: {counts}")
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(stats, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(OUT)


main()
