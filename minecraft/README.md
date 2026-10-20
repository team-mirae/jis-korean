# minecraft

**JIS Korean for Minecraft**: a Fabric mod (client only) that types Korean in Minecraft text
fields with the JIS Korean layout, and decides word by word whether you meant Korean or English.

Minecraft **[26.3, 26.4)** (`>=26.3 <26.4-`, which also excludes 26.4 snapshots). 26.3 moved
input from GLFW to SDL, and this mod reads SDL scancodes, so no earlier version can load it.
Requires Fabric Loader 0.19.3+ (the same as Fabric API 0.163.0+26.3), Fabric API and Java 25.

## How it works

Every key you type goes into a **staging area** (shown in a small window above the caret), and
the staging area is read two ways from the same keys:

- **Korean**: what JIS Korean composes (`gksrmf` → 한글), from the same engine as the macOS input
  method and the [web playground](https://jis-korean.sjun.me).
- **English**: what the keys type on a plain English layout of the same keyboard (`gksrmf`): JIS
  in Mode A, US ANSI in Mode B.

**Space, Enter or Tab** commits the more likely reading. Space adds a space, Enter then sends the
line, and Tab only commits. Minecraft's own suggestion line isn't used, so chat command
suggestions keep working.

| Key | Effect |
| --- | --- |
| Space / Enter / Tab | Commit the ranked reading |
| **F6** | Force Korean: the engine's F6, which commits (dropping a plain implied ㅇ) |
| **F7** | Force English (in place of the engine's hanja) |
| F8 / Shift+F8, F9, F10 | As in JIS Korean: 2-set escape and back, full- and half-width |
| Backspace | Edits the staging area by jamo, or deletes in the field when it's empty |
| Escape | Drops the staging area, then closes the screen |

F6 and F7 choices are remembered for the rest of the session: force `dkssud` to English once and
it stays English.

### Ranking

`me.sjun.jiskorean.rank.Ranker` compares two key-trigram models on the keys you pressed: one
trained on English spelling, one on the keys JIS Korean users press for Korean words (with the
implied ㅇ and the double press). The models are built by `tools/build_ranker.py` from
redistributable data:

- English: [dwyl/english-words](https://github.com/dwyl/english-words) (Unlicense) and
  `common-en.txt`
- Korean: the readings in `engine/dict/hanja.tsv` (libhangul, BSD-3-Clause) and `common-ko.txt`

Held-out accuracy (from the build script's report):

| Test set | Accuracy |
| --- | --- |
| English words | 99.5% |
| Korean words (JIS Korean typing) | 99.2% |
| Real Korean text, word by word (Samsung Electronics' "사업의 내용", not trained on) | about 96% |

Rules on top of the model:
- **Shift on a key that Korean never shifts** (anything but Q W E R T O P) means English.
- **Exact matches** in the common-word lists lean their way.
- **Leftover bare jamo** count against Korean, except chat words like ㅋㅋ.
- **A `/command` name** is English, unless it's one of the Mirae server's Korean aliases (`/귓말`,
  `/지역`, `/스폰`). Its arguments are ranked as usual.
- **Minecraft-only vocabulary** from 미래서버 (Mirae) is added to the word lists: the Mirae plugin's
  commands and aliases, and words from the server's public chat when the jar is built with them
  (`tools/mirae/README.md`; the chat logs are private and never committed).
- **The previous word's language** tips close calls.

## Build and test

```sh
./gradlew build               # build/libs/jis-korean-0.2.0-preview.jar (engine, model and Chicory inside)
./gradlew test                # staging, ranking and engine tests (no game)
./gradlew runClientGameTest   # boots the real client, opens chat and types through the mod
./gradlew runClient           # play with it
```

The engine is `../engine`, compiled to WebAssembly during the build (needs Rust with the
`wasm32-unknown-unknown` target) and run on [Chicory](https://chicory.dev), a pure-Java Wasm
runtime: one engine for every OS, no native libraries. Gradle runs on Java 25
(`gradle/gradle-daemon-jvm.properties`). To rebuild the ranker model:

```sh
python3 -I tools/build_ranker.py words_alpha.txt [--eval-korean <Korean text>]
```

## Notes

- **Always on, whatever the OS input source.** The mod is on in every single-line text field, and
  it types with its own engine from the raw key presses. For those fields it keeps the OS text
  input off (`TextInputManagerMixin`), so no OS input method, JIS Korean included, composes the
  same keys a second time; switch input sources (Ctrl+Space) as you like. Keys the engine doesn't
  know (the numeric keypad, Option combinations) type nothing in those fields.
- **The mode follows JIS Korean's.** On macOS the mod reads the input method's current mode
  (`defaults read me.sjun.inputmethod.JISKorean CurrentModeID`, spec §8.2) each time a screen
  opens, and types Mode A (JIS) or Mode B (US ANSI) to match. On Windows it reads the input
  method's `Mode` setting the same way (`reg query "HKCU\Software\JIS Korean" /v Mode`, then HKLM,
  spec §8.4). Elsewhere, or if JIS Korean has never been set up, it's Mode A. The macOS input method isn't changed for this: the mod only reads its public
  API.
- **Karabiner ろ rule:** on Macs where a Karabiner rule turns ろ into Option+¥ (as in
  hhkb-ekj-trilingual), the mod reads Option+¥ back as ろ.
- **Supported fields:** single-line text fields (`EditBox`): chat, commands, anvils, command
  blocks, search boxes. Books and signs use other widgets and aren't supported yet.
