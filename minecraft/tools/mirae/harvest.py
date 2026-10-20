# SPDX-License-Identifier: MIT
"""Harvests game vocabulary from Mirae server (미래서버) chat logs, for the Minecraft mod's ranker.

Source: the server logs of 미래서버 (Mirae), a live Minecraft server run in 2025 on Paper 1.21.8 with
the Mirae plugin (https://github.com/themrsung/Mirae). The logs are private and never committed:
put them in data/ (gitignored, see README.md). Only the word lists this script derives are used, and
those stay in data/ too.

What is read, and what never is (from the plugin's source, listener/player/PlayerListener.java,
social/DirectMessage.java):
- Kept: public chat and local chat. The plugin logs each as one line from the async chat thread:
  an optional "[지역] " (local chat), optional ":icon:" tokens (rank and title images), the sender's
  display name (their account name or a nickname set with /nick), " : ", and the message.
- Never read: direct messages and replies (logged by the server thread as "[sender -> recipient]
  message"), anything typed as a command ("issued server command"), muted players' messages
  ("[Muted] ", which only the sender saw), and plugin notices on the chat thread (GriefPrevention).

Usernames and nicknames never leave this script:
- Every name the logs show is collected first: account names (joins, leaves, logins, commands,
  shop and plugin lines, DMs), the display names in chat, and nicknames set with /nick.
- A word that contains any of those names, or is part of one, is dropped: "name님" and "@name" go,
  and so does any word that's a fragment of a nickname, even an ordinary one.
- Nicknames used only in conversation (never set with /nick) are caught by how Korean addresses
  people: a word that also occurs with a calling or honorific ending (~님 ~씨 ~형 ~쌤 ~아 ~야) is
  taken for a person and dropped in every form.
- Only single words are kept, with counts, never messages; and a word must have been used by at
  least MIN_SENDERS different people, so a name only one person typed can't slip through.
- Nothing about names is printed: the report is counts only.

English words that are really Korean typed with the keyboard in English mode (zzz for ㅋㅋㅋ, dk for
아, dlrj for 이거) are left out of the English list: a word whose keys compose to a word in the
Korean list, a letter repeated three times in a row (zzz is ㅋㅋㅋ), and single letters.

Usage: python3 -I harvest.py   (reads data/logs/*.log and *.log.gz; writes data/vocab-ko.txt and
data/vocab-en.txt, "word<TAB>count", most frequent first)
"""

import gzip
import re
import unicodedata
from collections import Counter, defaultdict
from pathlib import Path

DATA = Path(__file__).resolve().parent / "data"
MIN_SENDERS = 2  # people who must have used a word before it's kept
MIN_COUNT = 2  # times a word must appear
# Endings that address or honour a person: a word also seen with one of these is taken for a name.
ADDRESS_ENDINGS = ("님", "씨", "형", "쌤", "아", "야")

LINE = re.compile(r"^\[[^\]]*\] \[(?P<thread>[^\]]*)\]: (?P<text>.*)$")
# Public and local chat, as PlayerListener.renderChat logs it.
CHAT = re.compile(r"^(?P<local>\[지역\] )?(?P<icons>(?::[a-z0-9_]+: ?)*)(?P<name>[^:\[\]][^:]*?) : (?P<message>.*)$")
COMMAND = re.compile(r"^(?P<name>[A-Za-z0-9_]{2,16}) issued server command: /(?P<command>\S+)(?: (?P<args>.*))?$")
NICK_COMMANDS = {"nickname", "nick", "닉네임", "닉", "별명", "이름"}
# Lines that name an account (Paper and the Mirae plugin).
ACCOUNT_PATTERNS = [
    re.compile(r"^(?P<name>[A-Za-z0-9_]{2,16}) (?:joined|left) the game"),
    re.compile(r"^(?P<name>[A-Za-z0-9_]{2,16})\[/[^\]]*\] logged in"),
    re.compile(r"^UUID of player (?P<name>[A-Za-z0-9_]{2,16}) is"),
    re.compile(r"^(?P<name>[A-Za-z0-9_]{2,16}) lost connection"),
    re.compile(r"^\[Mirae\] (?P<name>[A-Za-z0-9_]{2,16}) : "),
    re.compile(r"^\[(?P<name>[^\] ]+) -> (?P<other>[^\] ]+)\] "),
    re.compile(r"^\[[+-]\] (?P<name>[A-Za-z0-9_]{2,16})"),
]
MINIMESSAGE_TAG = re.compile(r"<[^>]*>")
HANGUL = re.compile(r"^[가-힣ㄱ-ㅎㅏ-ㅣ]+$")
LATIN = re.compile(r"^[a-z]+$")
TRIM = re.compile(r"^[^\w가-힣ㄱ-ㅎㅏ-ㅣ]+|[^\w가-힣ㄱ-ㅎㅏ-ㅣ]+$")


def log_lines():
    for path in sorted(DATA.glob("logs/*.log")) + sorted(DATA.glob("logs/*.log.gz")):
        opener = gzip.open if path.suffix == ".gz" else open
        with opener(path, "rt", encoding="utf-8", errors="replace") as f:
            yield from f


# 2-set (KS X 5002): the jamo on each letter key, unshifted. Enough to read keys as Korean.
TWO_SET = dict(zip("qwertasdfgzxcvyuiophjklbnm", "ㅂㅈㄷㄱㅅㅁㄴㅇㄹㅎㅋㅌㅊㅍㅛㅕㅑㅐㅔㅗㅓㅏㅣㅠㅜㅡ"))
INITIALS = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ"
MEDIALS = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ"
FINALS = " ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ"
VOWEL_PAIRS = {"ㅗㅏ": "ㅘ", "ㅗㅐ": "ㅙ", "ㅗㅣ": "ㅚ", "ㅜㅓ": "ㅝ", "ㅜㅔ": "ㅞ", "ㅜㅣ": "ㅟ", "ㅡㅣ": "ㅢ"}
FINAL_PAIRS = {"ㄱㅅ": "ㄳ", "ㄴㅈ": "ㄵ", "ㄴㅎ": "ㄶ", "ㄹㄱ": "ㄺ", "ㄹㅁ": "ㄻ", "ㄹㅂ": "ㄼ", "ㄹㅅ": "ㄽ",
               "ㄹㅌ": "ㄾ", "ㄹㅍ": "ㄿ", "ㄹㅎ": "ㅀ", "ㅂㅅ": "ㅄ"}


def as_korean(word):
    """The Hangul the keys of an English word type on 2-set, or None if any jamo is left bare."""
    jamo = [TWO_SET.get(c) for c in word]
    if None in jamo:
        return None
    out, i = [], 0
    while i < len(jamo):
        initial = jamo[i]
        if initial not in INITIALS or i + 1 >= len(jamo) or jamo[i + 1] not in MEDIALS:
            return None
        medial, i = jamo[i + 1], i + 2
        if i < len(jamo) and medial + jamo[i] in VOWEL_PAIRS:
            medial, i = VOWEL_PAIRS[medial + jamo[i]], i + 1
        final = " "
        # A consonant is a final unless a vowel follows it (then it starts the next syllable).
        if i < len(jamo) and jamo[i] in FINALS.strip() and not (i + 1 < len(jamo) and jamo[i + 1] in MEDIALS):
            final, i = jamo[i], i + 1
            if i < len(jamo) and final + jamo[i] in FINAL_PAIRS and not (i + 1 < len(jamo) and jamo[i + 1] in MEDIALS):
                final, i = FINAL_PAIRS[final + jamo[i]], i + 1
        out.append(chr(0xAC00 + INITIALS.index(initial) * 588 + MEDIALS.index(medial) * 28 + FINALS.index(final)))
    return "".join(out)


def plain(name):
    """A display name or nickname without formatting tags or icons, lowercased."""
    name = MINIMESSAGE_TAG.sub("", name)
    name = re.sub(r":[a-z0-9_]+:", "", name)
    return unicodedata.normalize("NFC", name).strip().lower()


def main():
    names = set()
    chat = []  # (sender, message)
    for raw in log_lines():
        m = LINE.match(raw.rstrip("\n"))
        if not m:
            continue
        thread, text = m["thread"], m["text"]
        for pattern in ACCOUNT_PATTERNS:
            a = pattern.match(text)
            if a:
                names.add(plain(a["name"]))
                if "other" in a.groupdict() and a["other"]:
                    names.add(plain(a["other"]))
        c = COMMAND.match(text)
        if c:
            names.add(plain(c["name"]))
            if c["command"].lower() in NICK_COMMANDS and c["args"]:
                names.add(plain(c["args"]))
                names.update(plain(w) for w in c["args"].split())
            continue
        if thread.startswith("Async Chat Thread") and not text.startswith(("[Muted]", "[GriefPrevention]")):
            chat_line = CHAT.match(text)
            if chat_line:
                sender = plain(chat_line["name"])
                names.add(sender)
                names.update(plain(w) for w in chat_line["name"].split())
                chat.append((sender, chat_line["message"]))

    names = {n for n in names if n}

    def is_name(word):
        w = word.lower()
        if w in names:
            return True
        # Contains a name (name님, @name), or is a fragment of one.
        return any(n in w for n in names if len(n) >= 2) or any(w in n for n in names if len(w) >= 2)

    counts, senders = Counter(), defaultdict(set)
    dropped = 0
    for sender, message in chat:
        for token in message.split():
            word = TRIM.sub("", unicodedata.normalize("NFC", token))
            if not word:
                continue
            if is_name(word):
                dropped += 1
                continue
            word = word.lower()
            if HANGUL.match(word) or LATIN.match(word):
                counts[word] += 1
                senders[word].add(sender)

    # Informal nicknames: a word that also occurs with an address ending is a person (영희, 영희야).
    people = {w[:-1] for w in counts if HANGUL.match(w) and w.endswith(ADDRESS_ENDINGS) and len(w) >= 3 and w[:-1] in counts}
    addressed = {w for w in counts if any(w.startswith(p) for p in people)}
    kept = {w: n for w, n in counts.items()
            if n >= MIN_COUNT and len(senders[w]) >= MIN_SENDERS and w not in addressed}
    korean = sorted((w for w in kept if HANGUL.match(w)), key=lambda w: (-kept[w], w))
    korean_words = set(korean)

    def mistyped(w):
        """Korean typed with the keyboard in English mode, or a stray letter."""
        return len(w) < 2 or re.search(r"(.)\1\1", w) is not None or as_korean(w) in korean_words

    english = sorted((w for w in kept if LATIN.match(w) and not mistyped(w)), key=lambda w: (-kept[w], w))
    header = "# Derived from 미래서버 (Mirae) chat logs by minecraft/tools/mirae/harvest.py. Private: never commit.\n"
    (DATA / "vocab-ko.txt").write_text(header + "".join(f"{w}\t{kept[w]}\n" for w in korean), encoding="utf-8")
    (DATA / "vocab-en.txt").write_text(header + "".join(f"{w}\t{kept[w]}\n" for w in english), encoding="utf-8")
    print(f"chat lines kept: {len(chat)} from {len({s for s, _ in chat})} senders")
    print(f"names collected: {len(names)}; name words dropped: {dropped}; addressed words dropped: {len(addressed)}")
    print(f"words kept: {len(korean)} Korean, {len(english)} English (each used {MIN_COUNT}+ times by {MIN_SENDERS}+ people)")


if __name__ == "__main__":
    main()
