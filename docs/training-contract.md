# Training contract: tuning JIS Korean's predictions from your own data

A **training file** tunes JIS Korean's predictions (spec §10) to one person. It's a single JSON file:
`docs/training-contract.schema.json` is its JSON Schema, and `docs/training-example.json` is a
worked example.

- **Where it comes from:** any tool can write one, such as a style corpus like type-like-me, a
  notes app, or an AI agent working with you.
- **Where it applies:** the web playground (drop the file on "Tune predictions") and the macOS input
  method (put it in a folder, see [Applying it](#applying-it)).

**The file holds:**
- **weights:** where the sliders sit;
- **vocabulary:** words you use that the built-in lists may not know;
- **examples:** what you meant when you typed something, optionally with the keys;
- **typos:** recorded for later, not used yet;
- **preferences:** a record of what an agent asked you and how it set the weights.

It holds no keystroke logs, no timings and no app names. Nothing is sent anywhere: the file is read
on your device.

**If you're an AI agent preparing this file for someone,** read [For AI agents](#for-ai-agents)
first.

---

## The file

```json
{
  "format": "jis-korean-training",
  "version": 1,
  "source": { "tool": "type-like-me", "created": "2026-10-19", "description": "style corpus, turns 001–003" },
  "weights": { "english": 0.8, "implied-l": 0 },
  "vocabulary": {
    "korean": [{ "word": "데이터센터", "count": 50, "confidence": "confirmed" }],
    "english": [{ "word": "paradox" }]
  },
  "examples": [{ "text": "기준입니다" }, { "text": "산이", "keys": "KeyT KeyK KeyS KeyL", "count": 2 }],
  "typos": [{ "intended": "질문", "typed": "질분", "class": "Q", "count": 1 }],
  "preferences": [
    { "id": "implied-l", "question": "Should 전력 be offered when you type 저녁's keys?", "answer": "no", "weights": { "implied-l": 0 } }
  ]
}
```

**Required:** `format` (exactly `"jis-korean-training"`) and `version` (`1`). Everything else is
optional, and readers ignore fields they don't know, so later versions can add fields. Limits: 2 MB,
5,000 vocabulary entries per language, 2,000 examples.

### `source` (optional)

Where the file came from: `tool`, `created` (ISO date), `description`. Shown to the user, and not
otherwise used.

### `weights` (optional)

These are the sliders of spec §10.5. Each weight multiplies how likely that kind of prediction is:
1 means unchanged, 2 means twice as likely, 0.5 half as likely. Values are clamped to 0.1–10. A
weight set here counts as **set by hand**, so it wins over anything learned from use.

| Key | Scenario | What raising it does |
|---|---|---|
| `korean` | the Korean reading of the keys | Korean wins close calls against English |
| `english` | the same keys read as English (`dkssud` → dkssud) | English wins close calls |
| `implied` | the implied ㅇ (ㅠ → 유) | 유유 over ㅠㅠ (lower it if you type chat jamo a lot) |
| `mixed` | English word + Korean particle (report를, GPU랙) | more English + particle readings |
| `two-set` | plain 2-set reading (number-row keys as digits) | more digit readings |
| `keep-finals` | a final kept before a vowel (말이 for 마리's keys) | more kept-final readings |
| `hanja` | hanja (always below the top) | hanja listed sooner |
| `implied-l` | an implied ㄹ after ㄴ (전력 for 저녁's keys) | more 전력-style readings |

**0 turns a scenario off** entirely. That works for every key except `korean` and `english`, which
are clamped to 0.1. Leave a key out to keep its current value, or its learned one.

### `vocabulary` (optional)

Words you use, added to the built-in word lists. A known word wins close calls. Known words also
decide spelling questions such as 바둑이 against 바두기, because a known word followed by a particle
keeps its final (spec §10.3).

- `korean`: Hangul words with no spaces, as you write them (데이터센터, 상향조정, 토큰량). Compounds
  you always write joined belong here.
- `english`: Latin words (kubernetes, paradox). Case doesn't matter.
- Each entry is `{ "word", "count"?, "confidence"? }`.
  - **count:** how common the word is for you. It's on the same scale as the built-in lists, which
    count uses in about 3 million sentences: 8 is just in, 100 is common, 1000+ is very common.
  - **confidence**, when there's no count: `tentative` = 10, `likely` = 20, `confirmed` = 50.
  - With neither, the word counts as very common (1000).
  - A word the lists already know keeps the larger of the two counts.

### `examples` (optional)

Words as you meant them, one word each (no spaces), optionally with the keys you pressed. Each one is
typed into the engine, and where the meant text shows among the predictions, that choice is learned
as if you'd picked it. It's the same learning as the opt-in "learn from my choices" (spec §10.11), and
the importer reports where each example landed.

- `text`: the word as meant, e.g. 산이, 기준입니다, GPU랙.
- `keys` (optional): what was pressed, as `KeyboardEvent.code` names separated by spaces, with `S-`
  before a key typed with Shift. 산이 typed lazily is `KeyT KeyK KeyS KeyL`; 있습니다 with Shift on
  ㅣ is `S-KeyL S-KeyT KeyT KeyM KeyQ KeyS KeyL KeyE KeyK`. Without `keys`, the cheapest JIS Korean
  keys for `text` are used. Use `keys` when you type a word some other way, for example lazily
  without the double press, or `9` for 의.
- `count` (optional, 1–20): how many times to learn it.

Examples move weights only gently (they're clamped to 0.5–2×, as learning always is). For a firm
setting, use `weights`.

### `typos` (optional, not used in version 1)

`{ "intended", "typed", "class"?, "count"? }` pairs, e.g. 질문 → 질분 (`class` `Q`, a neighbouring
key). Readers keep them but don't act on them yet. Record them anyway: a later version may use them.

### `preferences` (optional)

What an agent asked you, and what it set: `{ "id", "question", "answer", "weights"? }`. This is
informational; the `weights` that apply are the top-level ones. It lets you see later why a slider
sits where it does.

---

## Applying it

- **Web playground:** open "Tune predictions" and drop the file on the drop zone, or choose it.
  - The weights move the sliders. The vocabulary is kept in this browser until you clear it.
  - Examples feed the learner, which is forgotten after 24 hours like all learned choices there.
  - A summary shows what was applied.
- **macOS:** put the file at `~/Library/Application Support/JIS Korean/training.json`. You can drag
  it there in Finder; the input menu's "Open Training Folder" opens that folder.
  - It's read the next time you switch to JIS Korean, and again whenever the file changes.
  - Weights go to the `Weight.<Scenario>` preferences, the vocabulary is kept on your Mac, and
    examples feed the learner even with "Learn from My Choices" off.
- **Undo:**
  - Web: Reset in Tune predictions, and "Forget what it learned" or "Forget training words".
  - macOS: "Reset What It Learned", and remove `training.json`. The words go on the next switch.

---

## For AI agents

You may be asked to turn someone's data into this file: a style corpus, notes, typo logs, or a
conversation. Follow these rules.

1. **Only what the data supports.** Don't invent words, examples or weights. When the data doesn't
   settle a weight, leave the key out.
2. **Ask before setting what's a matter of taste.** Use the questions below, and only those that
   matter for this person. Record each in `preferences`. Ask before every question you'd otherwise
   guess on. You may skip asking when the user said "use your judgment"; then say what you set.
3. **Vocabulary:** words the person actually writes, spelled their way. Joined compounds count
   (데이터센터). Use rule confidence for `confidence`, or real counts for `count`.
4. **Examples:**
   - Words the person actually meant, especially ones a plain composition gets wrong (기준입니다,
     산이, 소비량은) and mixed words (GPU랙, paradox에).
   - Add `keys` only when you know how they typed it.
   - Don't paste whole paragraphs: one word per example, up to 2,000.
5. **Typos:** copy them into `typos` with their class. Don't put the typo spelling into vocabulary or
   examples.
6. **Privacy:** no personal names, addresses, or private text. Words and short examples only. Keep
   the file local.
7. **Validate** against `docs/training-contract.schema.json`, then tell the person how to apply it
   (see [Applying it](#applying-it)).

### Questions you may ask (and what each sets)

| # | Ask | Answer → weights |
|---|---|---|
| 1 | "When you type the keys of 저녁, should 전력 also be offered? (ㄴ then ㄹ sounds like ㄹㄹ)" | no → `implied-l: 0`; yes → leave out |
| 2 | "Do you write hanja?" | never → `hanja: 0`; sometimes → leave out; often → `hanja: 2` |
| 3 | "Do you type chat jamo like ㅠㅠ, ㅋㅋ, ㅇㅇ a lot?" | a lot → `implied: 0.7`; rarely → leave out |
| 4 | "How much English do you mix into Korean (report를, GPU랙)?" | a lot → `mixed: 1.5`, `english: 1.2`; little → `english: 0.8` |
| 5 | "Do you skip the double press, so 말이 comes out as 마리 and you fix it?" | often → `keep-finals: 1.5`; never → leave out |
| 6 | "Do you use the number-row keys for digits, as on plain 2-set?" | yes → `two-set: 1.5`; no → leave out |

**Signals in a typo log, when you have one:**
- **"Prediction picked the wrong language or script"** (wrong English) means the data supports
  `english` below 1, about 0.8. Ask before setting it.
- **A kept-final misfire** (이른 typed and 일은 committed) means the data supports `keep-finals`
  below 1, about 0.8. Ask before setting it.
