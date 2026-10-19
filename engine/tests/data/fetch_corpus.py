"""Fetches "II. 사업의 내용" from a DART filing and writes it as plain prose, for the robot typing test.

Usage: python3 -I fetch_corpus.py [rcpNo] [out]
Default: Samsung Electronics, 반기보고서 (2026.06). The text belongs to its filer, so the output is
gitignored rather than committed.
"""

import html.parser
import re
import sys
import urllib.request

RCP_NO = sys.argv[1] if len(sys.argv) > 1 else "20260814003699"
OUT = sys.argv[2] if len(sys.argv) > 2 else "samsung-business.txt"
SECTION = "II. 사업의 내용"
BASE = "https://dart.fss.or.kr"


def get(url: str) -> str:
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return r.read().decode("utf-8")


class Prose(html.parser.HTMLParser):
    """Collects text outside tables, one paragraph per block element."""

    BLOCKS = {"p", "div", "br", "li", "title", "h1", "h2", "h3", "h4", "tr"}

    def __init__(self) -> None:
        super().__init__()
        self.table_depth = 0
        self.paragraphs: list[str] = []
        self.current: list[str] = []

    def end_paragraph(self) -> None:
        text = re.sub(r"\s+", " ", "".join(self.current)).strip()
        if text:
            self.paragraphs.append(text)
        self.current = []

    def handle_starttag(self, tag, attrs):
        if tag == "table":
            self.end_paragraph()
            self.table_depth += 1
        elif tag in self.BLOCKS:
            self.end_paragraph()

    def handle_endtag(self, tag):
        if tag == "table":
            self.table_depth = max(0, self.table_depth - 1)
        elif tag in self.BLOCKS:
            self.end_paragraph()

    def handle_data(self, data):
        if self.table_depth == 0:
            self.current.append(data)


def is_prose(p: str) -> bool:
    """Keeps sentences; drops unit labels, table captions, bare headings and number lists."""
    hangul = len(re.findall(r"[가-힣]", p))
    return hangul >= 12 and hangul / len(p) > 0.4


def main() -> None:
    main_page = get(f"{BASE}/dsaf001/main.do?rcpNo={RCP_NO}")
    node = main_page[main_page.index(f'"{SECTION}"'):]
    params = {k: re.search(rf"node1\['{k}'\] = \"([^\"]*)\"", node).group(1)
              for k in ("dcmNo", "eleId", "offset", "length", "dtd")}
    section = get(f"{BASE}/report/viewer.do?rcpNo={RCP_NO}&dcmNo={params['dcmNo']}&eleId={params['eleId']}"
                  f"&offset={params['offset']}&length={params['length']}&dtd={params['dtd']}")
    parser = Prose()
    parser.feed(section)
    parser.end_paragraph()
    prose = [p for p in parser.paragraphs if is_prose(p)]
    with open(OUT, "w", encoding="utf-8") as f:
        f.write("\n".join(prose) + "\n")
    print(f"{OUT}: {len(prose)} paragraphs, {sum(map(len, prose))} characters")


main()
