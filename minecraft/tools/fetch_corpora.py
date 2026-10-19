"""Fetches the corpora build_ranker.py --corpora trains on into /corpora (gitignored). Licenses:
engine/predict/LICENSE-corpora.txt. Only word counts derived from them are shipped.

Usage: python3 -I fetch_corpora.py
"""

import os
import re
import tarfile
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "..", "corpora")

FILES = {
    "nsmc-ratings.txt": "https://raw.githubusercontent.com/e9t/nsmc/master/ratings.txt",
    "ChatbotData.csv": "https://raw.githubusercontent.com/songys/Chatbot_data/master/ChatbotData.csv",
    "eng_sentences.tsv.bz2": "https://downloads.tatoeba.org/exports/per_language/eng/eng_sentences.tsv.bz2",
    "tm1-self-dialogs.json": "https://raw.githubusercontent.com/google-research-datasets/Taskmaster/master/TM-1-2019/self-dialogs.json",
    "fibo.tar.gz": "https://codeload.github.com/edmcouncil/fibo/tar.gz/refs/heads/master",
    "std-terms.csv": "https://www.data.go.kr/cmm/cmm/fileDownload.do?atchFileId=FILE_000000003563657&fileDetailSn=1&insertDataPrcus=N",
    "std-words.csv": "https://www.data.go.kr/cmm/cmm/fileDownload.do?atchFileId=FILE_000000003563436&fileDetailSn=1&insertDataPrcus=N",
}


def main():
    os.makedirs(OUT, exist_ok=True)
    for name, url in FILES.items():
        path = os.path.join(OUT, name)
        if not os.path.exists(path):
            print("fetching", name)
            urllib.request.urlretrieve(url, path)
    # FIBO: the rdfs:label of every concept, one per line.
    labels = set()
    with tarfile.open(os.path.join(OUT, "fibo.tar.gz")) as tar:
        for member in tar:
            if member.name.endswith(".rdf"):
                text = tar.extractfile(member).read().decode("utf-8", "replace")
                labels.update(re.findall(r"<rdfs:label[^>]*>([^<]+)</rdfs:label>", text))
    with open(os.path.join(OUT, "fibo-labels.txt"), "w", encoding="utf-8") as f:
        f.write("\n".join(sorted(labels)) + "\n")
    print(f"{len(labels)} FIBO labels")


main()
